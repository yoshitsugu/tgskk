use super::*;
use std::collections::BTreeMap;
use tgskk_core::input::CskkKeyEvent;

struct Random(u64);
impl Random {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }
}

enum Event {
    Change,
    Offer,
    Start,
    Finish(Job, u64, u64),
    Convert,
}

struct Simulation {
    random: Random,
    events: BTreeMap<(u64, u64), Event>,
    trace: Vec<(u64, &'static str)>,
    input: InputSession,
    assistance: Assistance,
    worker: Worker,
    version: u64,
    pending_version: u64,
    busy: bool,
    coverage: [bool; 7],
    alternate_reading: bool,
}

impl Simulation {
    fn new(seed: u64) -> Self {
        let worker = tests::worker();
        let mut input = tests::input();
        let assistance = Assistance::attach(&mut input, worker.clone());
        assistance.context("駅で", "", true);
        let mut simulation = Self {
            random: Random(seed),
            events: BTreeMap::new(),
            trace: Vec::new(),
            input,
            assistance,
            worker,
            version: 0,
            pending_version: 0,
            busy: false,
            coverage: [false; 7],
            alternate_reading: false,
        };
        simulation.compose();
        simulation.offer();
        simulation.schedule(1, Event::Start);
        let change = 1 + simulation.random.next() % 7;
        simulation.schedule(change, Event::Change);
        let convert = change + simulation.random.next() % 7;
        simulation.schedule(convert, Event::Convert);
        for time in 2..24 {
            simulation.schedule(time, Event::Start);
            if time % 3 == 0 {
                simulation.schedule(time, Event::Offer);
            }
        }
        simulation
    }

    fn schedule(&mut self, time: u64, event: Event) {
        let priority = self.random.next();
        assert!(self.events.insert((time, priority), event).is_none());
    }

    fn compose(&mut self) {
        let keys = if self.alternate_reading {
            ["K", "a", "n", "j", "i"]
        } else {
            ["K", "i", "s", "h", "a"]
        };
        for key in keys {
            let _ = self
                .input
                .process_key(&CskkKeyEvent::from_string_representation(key).unwrap());
        }
    }

    fn offer(&mut self) {
        self.assistance.prefetch(&self.input);
        if let Some(job) = self.worker.0.queue.pending.lock().unwrap().as_ref() {
            let current = self.assistance.0.borrow().epoch.load(Ordering::Relaxed);
            if job.key.epoch == current {
                self.pending_version = self.version;
            }
        }
    }

    fn run(&mut self) {
        while let Some(((time, _), event)) = self.events.pop_first() {
            let queue = self.worker.0.queue.clone();
            match event {
                Event::Change => {
                    self.trace.push((time, "context/reset/privacy change"));
                    let change = self.random.next() % 4;
                    if change != 3 {
                        self.version += 1;
                    }
                    match change {
                        0 => self.assistance.context("新聞の", "", true),
                        1 => {
                            self.assistance.reset();
                            self.assistance.context("駅で", "", true);
                        }
                        2 => self.assistance.context("password", "", false),
                        _ => self.alternate_reading = true,
                    }
                    let _ = self.input.reset();
                    self.compose();
                    self.offer();
                }
                Event::Offer => {
                    self.trace.push((time, "prefetch"));
                    self.offer();
                }
                Event::Start => {
                    self.trace.push((time, "worker dequeue"));
                    if !self.busy {
                        let job = queue.pending.lock().unwrap().take();
                        if let Some(job) = job {
                            let stale = self.pending_version != self.version;
                            let cached = queue
                                .ready
                                .lock()
                                .unwrap()
                                .iter()
                                .any(|(key, _)| key == &job.key);
                            let ready = job_is_ready(&queue, &job).unwrap();
                            assert_eq!(ready, !stale && !cached, "wrong dequeue eligibility");
                            self.coverage[0] |= stale;
                            self.coverage[4] |= !stale && cached;
                            if ready {
                                self.busy = true;
                                let finish = time + self.random.next() % 9;
                                let failure = self.random.next() % 3;
                                self.schedule(
                                    finish,
                                    Event::Finish(job, self.pending_version, failure),
                                );
                            }
                        }
                    }
                }
                Event::Finish(job, version, failure) => {
                    self.trace.push((time, "score success/timeout/refusal"));
                    self.busy = false;
                    let before = queue.ready.lock().unwrap().clone();
                    let pending = queue
                        .pending
                        .lock()
                        .unwrap()
                        .as_ref()
                        .map(|j| j.key.clone());
                    let key = job.key.clone();
                    let result = match failure {
                        0 => {
                            let mut scores = vec![(-6., 0.); key.candidates.len()];
                            scores[1].0 = -0.2;
                            Ok(scores)
                        }
                        1 => Err(io::Error::from(io::ErrorKind::TimedOut)),
                        _ => Err(io::Error::from(io::ErrorKind::InvalidInput)),
                    };
                    let result = complete_job(&queue, job, result);
                    assert_eq!(result.is_err(), failure == 1);
                    if version != self.version {
                        self.coverage[1] |= failure == 0;
                        self.coverage[2] |= failure == 1;
                        assert_eq!(*queue.ready.lock().unwrap(), before, "stale result cached");
                        assert_eq!(
                            queue
                                .pending
                                .lock()
                                .unwrap()
                                .as_ref()
                                .map(|j| j.key.clone()),
                            pending,
                            "stale job was requeued or displaced newer work"
                        );
                    } else if failure == 0 {
                        assert!(self.worker.ready(&key).is_some(), "valid result lost");
                    } else {
                        assert_eq!(*queue.ready.lock().unwrap(), before, "failed score cached");
                        let after = queue
                            .pending
                            .lock()
                            .unwrap()
                            .as_ref()
                            .map(|job| job.key.clone());
                        if failure == 2 {
                            self.coverage[5] = true;
                            assert_eq!(after, pending, "refused job must not be retried");
                        } else if let Some(pending) = pending {
                            self.coverage[3] |= pending != key;
                            assert_eq!(after, Some(pending), "newer pending job displaced");
                        } else {
                            self.coverage[6] = true;
                            assert_eq!(after, Some(key), "failed job must be retried");
                        }
                    }
                }
                Event::Convert => {
                    self.trace.push((time, "show candidates"));
                    let request = self.input.preview_conversion().unwrap();
                    let ranked = self
                        .assistance
                        .0
                        .borrow()
                        .key(&request)
                        .is_some_and(|key| self.worker.ready(&key).is_some());
                    let expected_first = request.candidates[usize::from(ranked)].text.clone();
                    let _ = self
                        .input
                        .process_key(&CskkKeyEvent::from_string_representation("space").unwrap());
                    let candidates = self.input.candidates();
                    assert!(
                        !candidates.is_empty(),
                        "dictionary input must remain usable"
                    );
                    assert_eq!(
                        candidates[0].text, expected_first,
                        "wrong conversion ranked"
                    );
                }
            }
        }
    }
}

#[test]
fn deterministic_scoring_schedules() {
    let replay = std::env::var("TGSKK_DST_SEED");
    let seeds: Vec<u64> = replay.clone().map_or_else(
        |_| (0..256).collect(),
        |value| vec![value.parse().expect("TGSKK_DST_SEED must be a u64")],
    );
    let mut coverage = [false; 7];
    for seed in seeds {
        let mut simulation = Simulation::new(seed);
        assert!(
            std::panic::catch_unwind(AssertUnwindSafe(|| simulation.run())).is_ok(),
            "DST seed={seed}; replay with TGSKK_DST_SEED={seed}; trace={:?}",
            simulation.trace
        );
        for (covered, observed) in coverage.iter_mut().zip(simulation.coverage) {
            *covered |= observed;
        }
    }
    if replay.is_err() {
        assert!(
            coverage.into_iter().all(|covered| covered),
            "missing schedule coverage: {coverage:?}"
        );
    }
}

#[test]
fn delayed_completion_after_selection_keeps_the_displayed_list_and_commit() {
    let worker = tests::worker();
    let mut input = tests::input();
    let assistance = Assistance::attach(&mut input, worker.clone());
    assistance.context("駅で", "", true);
    for key in ["K", "i", "s", "h", "a"] {
        let _ = input.process_key(&CskkKeyEvent::from_string_representation(key).unwrap());
    }
    let request = input.preview_conversion().unwrap();
    assistance.prefetch(&input);
    let key = assistance.0.borrow().key(&request).unwrap();
    let job = worker
        .0
        .queue
        .pending
        .lock()
        .unwrap()
        .take()
        .expect("prefetch must queue a job");
    assert_eq!(job.key, key, "queued job does not match the display");
    assert!(job_is_ready(&worker.0.queue, &job).unwrap());
    assert!(worker.ready(&key).is_none());

    let _ = input.process_key(&CskkKeyEvent::from_string_representation("space").unwrap());
    let displayed = input.candidates();
    assert_eq!(
        displayed, request.candidates,
        "displayed list is not dictionary order"
    );
    assert!(displayed.len() > 1, "no non-first candidate");

    let selected = 1;
    let expected = &displayed[selected].text;
    assert_ne!(
        expected, &displayed[0].text,
        "selected candidate must be non-first"
    );
    assert!(
        input.select_candidate(selected).consumed,
        "selection rejected"
    );
    let cursor = input.candidate_cursor();
    assert_eq!(cursor, Some(selected), "wrong cursor after selection");
    let mut scores = vec![(-6., 0.); job.key.candidates.len()];
    scores[selected].0 = -0.2;
    complete_job(&worker.0.queue, job, Ok(scores)).unwrap();
    let cached = worker.ready(&key).expect("late score was not cached");
    assert_eq!(
        winner(&cached, &key.protected),
        Some(selected),
        "score does not reorder the list"
    );

    assert_eq!(
        input.candidates(),
        displayed,
        "late score changed the displayed list"
    );
    assert_eq!(
        input.candidate_cursor(),
        cursor,
        "late score moved the cursor"
    );
    let outcome = input.process_key(&CskkKeyEvent::from_string_representation("Return").unwrap());
    assert_eq!(
        &outcome.committed, expected,
        "Return committed another candidate"
    );
}

#[test]
fn same_seed_replays_the_same_schedule() {
    let mut first = Simulation::new(42);
    first.run();
    let mut second = Simulation::new(42);
    second.run();
    assert_eq!(first.trace, second.trace);
    assert_eq!(first.coverage, second.coverage);
}
