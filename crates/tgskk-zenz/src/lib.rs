//! Local candidate scoring. The input thread only offers jobs and reads ready
//! results with `try_lock`; model loading and IPC belong to the worker thread.
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use tgskk_core::input::{ConversionCandidate, ConversionRequest, ConversionResponse, InputSession};

const MAX_CANDIDATES: usize = 8;
const MAX_BYTES: usize = 1024;
const CACHE_SIZE: usize = 16;
const MIN_MARGIN: f64 = 1.5;
// A changed wire format must use a different handshake, including refusals.
const PROTOCOL_MAGIC: u32 = 0x325A_4754;
const REJECTED_REQUEST: u32 = u32::MAX;
const SHUTDOWN_POLL_INTERVAL: Duration = Duration::from_millis(100);
const UNAVAILABLE_AFTER_FAILURES: usize = 4;
const RETRY_DELAYS: [Duration; 5] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
    Duration::from_secs(15),
    Duration::from_secs(30),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Status {
    Loading = 0,
    Ready = 1,
    Unavailable = 2,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Key {
    epoch: u64,
    context: String,
    reading: String,
    candidates: Vec<ConversionCandidate>,
    protected: Vec<usize>,
}
struct Job {
    key: Key,
    epoch: Arc<AtomicU64>,
}
struct Queue {
    pending: Mutex<Option<Job>>,
    wake: Condvar,
    ready: Mutex<VecDeque<(Key, Vec<f64>)>>,
    stopped: AtomicBool,
    status: AtomicU8,
    scored: AtomicBool,
    diagnostic: Mutex<Option<String>>,
}
struct Service {
    queue: Arc<Queue>,
}
impl Drop for Service {
    fn drop(&mut self) {
        self.queue.stopped.store(true, Ordering::Release);
        self.queue.wake.notify_one();
    }
}
/// Shared model service. Failure leaves dictionary conversion usable.
#[derive(Clone)]
pub struct Worker(Arc<Service>);
impl Worker {
    pub fn start(executable: PathBuf, model: PathBuf) -> io::Result<Self> {
        let queue = Arc::new(Queue {
            pending: Mutex::new(None),
            wake: Condvar::new(),
            ready: Mutex::new(VecDeque::new()),
            stopped: AtomicBool::new(false),
            status: AtomicU8::new(Status::Loading as u8),
            scored: AtomicBool::new(false),
            diagnostic: Mutex::new(None),
        });
        let background = queue.clone();
        std::thread::Builder::new()
            .name("tgskk-zenz".into())
            .spawn(move || {
                supervise(
                    &background,
                    || run(&background, &executable, &model),
                    &RETRY_DELAYS,
                );
            })?;
        Ok(Self(Arc::new(Service { queue })))
    }
    #[must_use]
    pub fn status(&self) -> Status {
        match self.0.queue.status.load(Ordering::Acquire) {
            1 => Status::Ready,
            2 => Status::Unavailable,
            _ => Status::Loading,
        }
    }
    /// Returns each diagnostic once, without logging or retaining input text.
    #[must_use]
    pub fn take_error(&self) -> Option<String> {
        self.0.queue.diagnostic.try_lock().ok()?.take()
    }
    fn offer(&self, job: Job) -> bool {
        if self.status() == Status::Unavailable {
            return false;
        }
        if let Ok(mut slot) = self.0.queue.pending.try_lock() {
            *slot = Some(job);
            self.0.queue.wake.notify_one();
            true
        } else {
            false
        }
    }
    fn ready(&self, key: &Key) -> Option<Vec<f64>> {
        if self.status() == Status::Unavailable {
            return None;
        }
        self.0
            .queue
            .ready
            .try_lock()
            .ok()?
            .iter()
            .find(|(cached, _)| cached == key)
            .map(|(_, scores)| scores.clone())
    }
}
struct Process {
    child: Child,
    socket: UnixStream,
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Process {
    fn start(executable: &Path, model: &Path) -> io::Result<Self> {
        std::fs::File::open(model)
            .map_err(|e| io::Error::new(e.kind(), format!("Cannot open Zenz model: {e}")))?;
        let (socket, child_socket) = UnixStream::pair()?;
        socket.set_read_timeout(Some(Duration::from_secs(5)))?;
        socket.set_write_timeout(Some(Duration::from_millis(500)))?;
        let child = Command::new(executable)
            .arg(model)
            .stdin(Stdio::from(OwnedFd::from(child_socket.try_clone()?)))
            .stdout(Stdio::from(OwnedFd::from(child_socket)))
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| io::Error::new(e.kind(), format!("Cannot start Zenz worker: {e}")))?;
        let mut process = Self { child, socket };
        if read_word(&mut process.socket)? != PROTOCOL_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid Zenz worker handshake",
            ));
        }
        process
            .socket
            .set_read_timeout(Some(Duration::from_millis(500)))?;
        Ok(process)
    }
    fn score(&mut self, key: &Key) -> io::Result<Vec<f64>> {
        let mut frame = Vec::new();
        frame.extend_from_slice(
            &u32::try_from(key.candidates.len())
                .map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidInput, "Too many Zenz candidates")
                })?
                .to_le_bytes(),
        );
        for text in std::iter::once(&key.context)
            .chain(std::iter::once(&key.reading))
            .chain(key.candidates.iter().map(|c| &c.text))
        {
            if text.len() > MAX_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Zenz input too long",
                ));
            }
            frame.extend_from_slice(
                &u32::try_from(text.len())
                    .expect("MAX_BYTES fits in u32")
                    .to_le_bytes(),
            );
            frame.extend_from_slice(text.as_bytes());
        }
        self.socket.write_all(&frame)?;
        let count = read_word(&mut self.socket)?;
        if count == REJECTED_REQUEST {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Zenz rejected this conversion request",
            ));
        }
        if count as usize != key.candidates.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid Zenz score count",
            ));
        }
        let mut scores = Vec::new();
        for _ in &key.candidates {
            let mut bytes = [0; 8];
            self.socket.read_exact(&mut bytes)?;
            let score = f64::from_le_bytes(bytes);
            if !score.is_finite() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid Zenz score",
                ));
            }
            scores.push(score);
        }
        Ok(scores)
    }
}
fn read_word(socket: &mut UnixStream) -> io::Result<u32> {
    let mut bytes = [0; 4];
    socket.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}
fn run(queue: &Queue, executable: &Path, model: &Path) -> io::Result<()> {
    let mut process = Process::start(executable, model)?;
    queue.status.store(Status::Ready as u8, Ordering::Release);
    worker_loop(queue, |key| process.score(key))
}

fn supervise(queue: &Queue, mut run: impl FnMut() -> io::Result<()>, delays: &[Duration]) {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut attempt = 0usize;
        queue.status.store(Status::Loading as u8, Ordering::Release);
        loop {
            if queue.stopped.load(Ordering::Acquire) {
                return;
            }
            let error = match run() {
                Ok(()) => return,
                Err(error) => error,
            };
            if queue.scored.swap(false, Ordering::AcqRel) {
                attempt = 0;
            }
            if let Ok(mut diagnostic) = queue.diagnostic.lock() {
                *diagnostic = Some(format!("Zenz attempt {} failed: {error}", attempt + 1));
            }
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound
                    | io::ErrorKind::PermissionDenied
                    | io::ErrorKind::InvalidInput
                    | io::ErrorKind::InvalidData
            ) {
                return;
            }
            let Some(delay) = delays.get(attempt.min(delays.len().saturating_sub(1))) else {
                return;
            };
            attempt = attempt.saturating_add(1);
            let status = if attempt >= UNAVAILABLE_AFTER_FAILURES {
                Status::Unavailable
            } else {
                Status::Loading
            };
            queue.status.store(status as u8, Ordering::Release);
            let deadline = Instant::now() + *delay;
            let Ok(mut slot) = queue.pending.lock() else {
                return;
            };
            while !queue.stopped.load(Ordering::Acquire) && Instant::now() < deadline {
                let Ok((next, _)) = queue.wake.wait_timeout(
                    slot,
                    deadline
                        .saturating_duration_since(Instant::now())
                        .min(SHUTDOWN_POLL_INTERVAL),
                ) else {
                    return;
                };
                slot = next;
            }
        }
    }));
    if result.is_err()
        && let Ok(mut diagnostic) = queue.diagnostic.lock()
    {
        *diagnostic = Some("Zenz worker thread panicked; using dictionary order".into());
    }
    queue
        .status
        .store(Status::Unavailable as u8, Ordering::Release);
}

fn worker_loop(
    queue: &Queue,
    mut score: impl FnMut(&Key) -> io::Result<Vec<f64>>,
) -> io::Result<()> {
    loop {
        let job = {
            let mut slot = queue
                .pending
                .lock()
                .map_err(|_| io::Error::other("Zenz queue poisoned"))?;
            while slot.is_none() && !queue.stopped.load(Ordering::Acquire) {
                slot = queue
                    .wake
                    .wait_timeout(slot, SHUTDOWN_POLL_INTERVAL)
                    .map_err(|_| io::Error::other("Zenz queue poisoned"))?
                    .0;
            }
            if queue.stopped.load(Ordering::Acquire) {
                return Ok(());
            }
            slot.take().expect("pending job")
        };
        if job.epoch.load(Ordering::Acquire) != job.key.epoch {
            continue;
        }
        if queue
            .ready
            .lock()
            .map_err(|_| io::Error::other("Zenz cache poisoned"))?
            .iter()
            .any(|(key, _)| key == &job.key)
        {
            continue;
        }
        let scores = match score(&job.key) {
            Ok(scores) => {
                queue.scored.store(true, Ordering::Release);
                scores
            }
            Err(error) if error.kind() == io::ErrorKind::InvalidInput => {
                if let Ok(mut diagnostic) = queue.diagnostic.lock() {
                    *diagnostic =
                        Some("Zenz rejected one conversion request; using dictionary order".into());
                }
                continue;
            }
            Err(error) => {
                let mut slot = queue
                    .pending
                    .lock()
                    .map_err(|_| io::Error::other("Zenz queue poisoned"))?;
                if slot.is_none() && job.epoch.load(Ordering::Acquire) == job.key.epoch {
                    *slot = Some(job);
                }
                return Err(error);
            }
        };
        if job.epoch.load(Ordering::Acquire) != job.key.epoch {
            continue;
        }
        let mut ready = queue
            .ready
            .lock()
            .map_err(|_| io::Error::other("Zenz cache poisoned"))?;
        if ready.len() == CACHE_SIZE {
            ready.pop_front();
        }
        ready.push_back((job.key, scores));
    }
}
static NEXT_EPOCH: AtomicU64 = AtomicU64::new(1);
fn next_epoch() -> u64 {
    NEXT_EPOCH.fetch_add(1, Ordering::Relaxed)
}
struct State {
    worker: Worker,
    context: String,
    enabled: bool,
    epoch: Arc<AtomicU64>,
    applied: bool,
    hits: u64,
    misses: u64,
}
impl Drop for State {
    fn drop(&mut self) {
        self.epoch.store(next_epoch(), Ordering::Release);
    }
}
impl State {
    fn key(&self, request: &ConversionRequest) -> Option<Key> {
        if !self.enabled
            || request.okuri.is_some()
            || request.reading.len() > 192
            || request.reading.chars().any(|c| !('ぁ'..='ゖ').contains(&c))
        {
            return None;
        }
        if request
            .candidates
            .iter()
            .take(MAX_CANDIDATES)
            .any(|c| c.annotation.as_ref().is_some_and(|a| a.len() > 512))
        {
            return None;
        }
        let candidates: Vec<_> = request
            .candidates
            .iter()
            .take(MAX_CANDIDATES)
            .cloned()
            .collect();
        if candidates.iter().any(|c| {
            c.text.chars().count() > 24
                || c.text.len() > 96
                || c.text.chars().any(|c| {
                    c.is_ascii() || c == '\u{3000}' || ('\u{ee00}'..='\u{ee0f}').contains(&c)
                })
        }) {
            return None;
        }
        Some(Key {
            epoch: self.epoch.load(Ordering::Relaxed),
            context: self.context.clone(),
            reading: request
                .reading
                .chars()
                .map(|c| char::from_u32(c as u32 + 0x60).unwrap_or(c))
                .collect(),
            candidates,
            protected: request
                .protected_indices
                .iter()
                .copied()
                .filter(|i| *i < MAX_CANDIDATES)
                .collect(),
        })
    }
    fn response(&mut self, request: &ConversionRequest) -> ConversionResponse {
        self.applied = false;
        let Some(key) = self.key(request) else {
            return ConversionResponse::default();
        };
        if key.candidates.len() < 2 || key.protected.contains(&0) {
            return ConversionResponse::default();
        }
        let Some(scores) = self.worker.ready(&key) else {
            self.misses += 1;
            return ConversionResponse::default();
        };
        self.hits += 1;
        if let Some(index) = confident_winner(&scores, &key.protected) {
            let mut order: Vec<_> = (0..request.candidates.len()).collect();
            // Protected entries keep their exact positions; other entries keep
            // their relative order when the winner moves to the front.
            let slots: Vec<_> = (0..=index).filter(|i| !key.protected.contains(i)).collect();
            for pair in slots.windows(2) {
                order[pair[1]] = pair[0];
            }
            order[0] = index;
            self.applied = true;
            return ConversionResponse {
                order: Some(order),
                ..Default::default()
            };
        }
        ConversionResponse::default()
    }
}
fn confident_winner(scores: &[f64], protected: &[usize]) -> Option<usize> {
    let mut indices: Vec<_> = (0..scores.len()).collect();
    indices.sort_by(|a, b| scores[*b].total_cmp(&scores[*a]));
    let winner = *indices.first()?;
    (winner != 0
        && !protected.contains(&winner)
        && scores[winner] - scores[*indices.get(1)?] >= MIN_MARGIN)
        .then_some(winner)
}
#[derive(Clone)]
pub struct Assistance(Rc<RefCell<State>>);
impl Assistance {
    pub fn attach(input: &mut InputSession, worker: Worker) -> Self {
        let state = Rc::new(RefCell::new(State {
            worker,
            context: String::new(),
            enabled: false,
            epoch: Arc::new(AtomicU64::new(next_epoch())),
            applied: false,
            hits: 0,
            misses: 0,
        }));
        let provider = state.clone();
        input.set_conversion_provider(Some(Box::new(move |request: &ConversionRequest| {
            provider.borrow_mut().response(request)
        })));
        Self(state)
    }
    pub fn context(&self, text: &str, enabled: bool) {
        let text: String = text
            .chars()
            .filter(|c| !('\u{ee00}'..='\u{ee0f}').contains(c))
            .rev()
            .take(40)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let mut state = self.0.borrow_mut();
        if state.context != text || state.enabled != enabled {
            state.context = if enabled { text } else { String::new() };
            state.enabled = enabled;
            state.epoch.store(next_epoch(), Ordering::Release);
        }
    }
    pub fn reset(&self) {
        let mut state = self.0.borrow_mut();
        state.epoch.store(next_epoch(), Ordering::Release);
        state.context.clear();
        state.enabled = false;
        state.applied = false;
    }
    pub fn prefetch(&self, input: &InputSession) {
        let state = self.0.borrow();
        if !state.enabled || state.worker.status() == Status::Unavailable {
            return;
        }
        let Some(request) = input.preview_conversion_limited(MAX_CANDIDATES) else {
            return;
        };
        let Some(key) = state.key(&request) else {
            return;
        };
        if key.candidates.len() == 1 || state.worker.ready(&key).is_some() {
            return;
        }
        state.worker.offer(Job {
            key,
            epoch: state.epoch.clone(),
        });
    }
    #[must_use]
    pub fn applied(&self) -> bool {
        self.0.borrow().applied
    }
    #[must_use]
    pub fn counts(&self) -> (u64, u64) {
        let s = self.0.borrow();
        (s.hits, s.misses)
    }
}

/// Blocking scorer for evaluation tools; never call from a key event handler.
pub struct Evaluator(Process);
impl Evaluator {
    pub fn open(executable: &Path, model: &Path) -> io::Result<Self> {
        Ok(Self(Process::start(executable, model)?))
    }
    pub fn score(
        &mut self,
        left: &str,
        katakana: &str,
        candidates: &[String],
    ) -> io::Result<Vec<f64>> {
        if candidates.len() > MAX_CANDIDATES {
            return Err(io::Error::other("At most eight candidates"));
        }
        self.0.score(&Key {
            epoch: 0,
            context: left.into(),
            reading: katakana.into(),
            candidates: candidates
                .iter()
                .map(|text| ConversionCandidate {
                    text: text.clone(),
                    annotation: None,
                })
                .collect(),
            protected: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Instant;

    fn worker() -> Worker {
        Worker(Arc::new(Service {
            queue: Arc::new(Queue {
                pending: Mutex::new(None),
                wake: Condvar::new(),
                ready: Mutex::new(VecDeque::new()),
                stopped: AtomicBool::new(false),
                status: AtomicU8::new(Status::Ready as u8),
                scored: AtomicBool::new(false),
                diagnostic: Mutex::new(None),
            }),
        }))
    }
    fn request() -> ConversionRequest {
        ConversionRequest {
            reading: "きしゃ".into(),
            okuri: None,
            candidates: ["記者", "汽車", "帰社", "貴社"]
                .iter()
                .map(|text| ConversionCandidate {
                    text: (*text).into(),
                    annotation: None,
                })
                .collect(),
            protected_indices: Vec::new(),
        }
    }
    fn input() -> InputSession {
        InputSession::open(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/SKK-JISYO.demo.utf8"
            ),
            "utf-8",
        )
        .unwrap()
    }
    fn ready(assistance: &Assistance, req: &ConversionRequest, scores: Vec<f64>) {
        let state = assistance.0.borrow();
        let key = state.key(req).unwrap();
        state
            .worker
            .0
            .queue
            .ready
            .lock()
            .unwrap()
            .push_back((key, scores));
    }
    fn wait_ready(worker: &Worker, key: &Key) {
        let until = Instant::now() + Duration::from_secs(2);
        while worker.ready(key).is_none() {
            assert!(Instant::now() < until, "worker did not finish");
            std::thread::yield_now();
        }
    }
    #[test]
    fn confident_promotion_keeps_protected_positions_and_remaining_order() {
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker());
        assistance.context("取引先である", true);
        let mut req = request();
        req.protected_indices = vec![1];
        ready(&assistance, &req, vec![-6., -8., -5., -0.2]);
        let response = assistance.0.borrow_mut().response(&req);
        assert_eq!(response.order, Some(vec![3, 1, 0, 2]));
        assert_eq!(response.learning, None);
        assert!(assistance.applied());
    }
    #[test]
    fn close_scores_and_protected_first_candidate_prevent_intervention() {
        assert_eq!(confident_winner(&[-1., -0.2, -5.], &[]), None);
        assert_eq!(confident_winner(&[-5., -0.2, -6.], &[1]), None);
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker());
        assistance.context("駅で", true);
        let mut req = request();
        req.protected_indices = vec![0];
        ready(&assistance, &req, vec![-5., -0.2, -6., -7.]);
        assert_eq!(assistance.0.borrow_mut().response(&req).order, None);
        assert!(!assistance.applied());
    }
    #[test]
    fn results_must_match_context_reading_candidate_identity_and_protection() {
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker());
        assistance.context("駅で", true);
        let req = request();
        ready(&assistance, &req, vec![-6., -0.2, -8., -9.]);
        assert_eq!(
            assistance.0.borrow_mut().response(&req).order,
            Some(vec![1, 0, 2, 3])
        );
        for changed in 0..4 {
            let mut other = req.clone();
            match changed {
                0 => other.reading = "かんじ".into(),
                1 => other.candidates[1].text = "期社".into(),
                2 => other.candidates[1].annotation = Some("個人".into()),
                _ => other.protected_indices = vec![1],
            }
            assert!(assistance.0.borrow_mut().response(&other).order.is_none());
        }
        assistance.context("新聞の", true);
        assert!(assistance.0.borrow_mut().response(&req).order.is_none());
        assistance.context("駅で", true);
        assert!(assistance.0.borrow_mut().response(&req).order.is_none());
    }
    #[test]
    fn sensitive_reset_and_dropped_sessions_invalidate_queued_work() {
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker());
        assistance.context("駅で", true);
        let req = request();
        ready(&assistance, &req, vec![-6., -0.2, -8., -9.]);
        let epoch = assistance.0.borrow().epoch.clone();
        let before = epoch.load(Ordering::Acquire);
        assistance.context("password", false);
        assert_ne!(before, epoch.load(Ordering::Acquire));
        assert_eq!(assistance.0.borrow().context, "");
        assert!(assistance.0.borrow_mut().response(&req).order.is_none());
        assistance.context("駅で", true);
        assistance.reset();
        assert!(assistance.0.borrow_mut().response(&req).order.is_none());
        let before = epoch.load(Ordering::Acquire);
        drop(assistance);
        drop(input);
        assert_ne!(before, epoch.load(Ordering::Acquire));
    }
    #[test]
    fn input_thread_never_waits_for_busy_queue_or_cache() {
        let worker = worker();
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker.clone());
        assistance.context("駅で", true);
        let req = request();
        let key = assistance.0.borrow().key(&req).unwrap();
        let _pending = worker.0.queue.pending.lock().unwrap();
        let _cache = worker.0.queue.ready.lock().unwrap();
        assert!(!worker.offer(Job {
            key,
            epoch: assistance.0.borrow().epoch.clone()
        }));
        assert!(assistance.0.borrow_mut().response(&req).order.is_none());
    }
    #[test]
    fn latest_pending_job_replaces_old_work_and_stale_inflight_results_are_discarded() {
        let worker = worker();
        let queue = worker.0.queue.clone();
        let (started_tx, started_rx) = mpsc::channel();
        let (continue_tx, continue_rx) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            worker_loop(&queue, |key| {
                started_tx.send(key.reading.clone()).unwrap();
                continue_rx.recv_timeout(Duration::from_secs(2)).unwrap();
                Ok(vec![-1.; key.candidates.len()])
            })
        });
        let epoch = Arc::new(AtomicU64::new(1));
        let key = |reading: &str, number| Key {
            epoch: number,
            context: "駅で".into(),
            reading: reading.into(),
            candidates: request().candidates,
            protected: vec![],
        };
        let first = key("キ", 1);
        assert!(worker.offer(Job {
            key: first.clone(),
            epoch: epoch.clone()
        }));
        assert_eq!(
            started_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            "キ"
        );
        assert!(worker.offer(Job {
            key: key("キシ", 1),
            epoch: epoch.clone()
        }));
        epoch.store(2, Ordering::Release);
        let last = key("キシャ", 2);
        assert!(worker.offer(Job {
            key: last.clone(),
            epoch: epoch.clone()
        }));
        continue_tx.send(()).unwrap();
        assert_eq!(
            started_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            "キシャ"
        );
        continue_tx.send(()).unwrap();
        wait_ready(&worker, &last);
        assert!(worker.ready(&first).is_none());
        drop(worker);
        assert!(thread.join().unwrap().is_ok());
    }
    #[test]
    fn transient_failure_restarts_and_retries_the_latest_valid_job() {
        let worker = worker();
        let queue = worker.0.queue.clone();
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker.clone());
        assistance.context("駅で", true);
        let key = assistance.0.borrow().key(&request()).unwrap();
        worker.offer(Job {
            key: key.clone(),
            epoch: assistance.0.borrow().epoch.clone(),
        });
        let thread = std::thread::spawn(move || {
            let mut attempts = 0;
            supervise(
                &queue,
                || {
                    attempts += 1;
                    queue.status.store(Status::Ready as u8, Ordering::Release);
                    worker_loop(&queue, |_| {
                        if attempts == 1 {
                            Err(io::Error::new(io::ErrorKind::TimedOut, "test timeout"))
                        } else {
                            Ok(vec![-5., -0.2, -8., -9.])
                        }
                    })
                },
                &[Duration::from_millis(1)],
            );
            attempts
        });
        wait_ready(&worker, &key);
        assert_eq!(worker.status(), Status::Ready);
        assert!(worker.take_error().unwrap().contains("test timeout"));
        assert!(worker.take_error().is_none());
        assert_eq!(
            assistance.0.borrow_mut().response(&request()).order,
            Some(vec![1, 0, 2, 3])
        );
        drop(assistance);
        drop(input);
        drop(worker);
        assert_eq!(thread.join().unwrap(), 2);
    }
    #[test]
    fn transient_failures_recover_after_the_old_retry_limit() {
        let worker = worker();
        let mut attempts = 0;
        supervise(
            &worker.0.queue,
            || {
                attempts += 1;
                if attempts <= 8 {
                    Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "simulated CPU contention",
                    ))
                } else {
                    Ok(())
                }
            },
            &[Duration::ZERO; 3],
        );
        assert_eq!(attempts, 9);
        assert!(worker.take_error().unwrap().contains("attempt 8"));
    }
    #[test]
    fn repeated_startup_failures_are_visible_and_scoring_resets_the_backoff() {
        let worker = worker();
        let queue = &worker.0.queue;
        let mut attempts = 0;
        let mut observations = Vec::new();
        supervise(
            queue,
            || {
                attempts += 1;
                match attempts {
                    1..=4 => Err(io::Error::from(io::ErrorKind::UnexpectedEof)),
                    5 => {
                        observations.push((worker.status(), worker.take_error()));
                        queue.status.store(Status::Ready as u8, Ordering::Release);
                        queue.scored.store(true, Ordering::Release);
                        Err(io::Error::from(io::ErrorKind::TimedOut))
                    }
                    6 => {
                        observations.push((worker.status(), worker.take_error()));
                        Ok(())
                    }
                    _ => unreachable!(),
                }
            },
            &[Duration::ZERO],
        );
        assert_eq!(attempts, 6);
        assert_eq!(observations[0].0, Status::Unavailable);
        assert_eq!(observations[1].0, Status::Loading);
        assert!(
            observations[1]
                .1
                .as_ref()
                .unwrap()
                .contains("attempt 1 failed")
        );
    }
    #[test]
    fn permanent_failures_and_panics_are_not_retried() {
        let worker = worker();
        let mut attempts = 0;
        supervise(
            &worker.0.queue,
            || {
                attempts += 1;
                Err(io::Error::new(io::ErrorKind::NotFound, "missing worker"))
            },
            &[Duration::ZERO; 3],
        );
        assert_eq!(attempts, 1);
        assert_eq!(worker.status(), Status::Unavailable);
        supervise(&worker.0.queue, || panic!("test worker panic"), &[]);
        assert_eq!(worker.status(), Status::Unavailable);
        assert!(worker.take_error().unwrap().contains("panicked"));
    }
    #[test]
    fn rejecting_one_job_does_not_restart_or_stop_the_service() {
        let worker = worker();
        let queue = worker.0.queue.clone();
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker.clone());
        assistance.context("駅で", true);
        let valid = assistance.0.borrow().key(&request()).unwrap();
        let mut invalid = valid.clone();
        invalid.reading = "BAD".into();
        let epoch = assistance.0.borrow().epoch.clone();
        let (rejected_tx, rejected_rx) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            worker_loop(&queue, |key| {
                if key.reading == "BAD" {
                    rejected_tx.send(()).unwrap();
                    Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "invalid request",
                    ))
                } else {
                    Ok(vec![-5., -0.2, -8., -9.])
                }
            })
        });
        worker.offer(Job {
            key: invalid.clone(),
            epoch: epoch.clone(),
        });
        rejected_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let until = Instant::now() + Duration::from_secs(2);
        while !worker.offer(Job {
            key: valid.clone(),
            epoch: epoch.clone(),
        }) {
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }
        wait_ready(&worker, &valid);
        assert_eq!(worker.status(), Status::Ready);
        assert!(worker.0.queue.scored.load(Ordering::Acquire));
        assert!(worker.ready(&invalid).is_none());
        assert!(
            worker
                .take_error()
                .unwrap()
                .contains("one conversion request")
        );
        drop(assistance);
        drop(input);
        drop(worker);
        assert!(thread.join().unwrap().is_ok());
    }
    #[test]
    fn context_updates_keep_the_indicator_for_an_already_ranked_list() {
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker());
        assistance.context("駅で", true);
        ready(&assistance, &request(), vec![-6., -0.2, -8., -9.]);
        assert!(
            assistance
                .0
                .borrow_mut()
                .response(&request())
                .order
                .is_some()
        );
        assistance.context("新しい文脈", true);
        assert!(assistance.applied());
        assistance.reset();
        assert!(!assistance.applied());
    }
    #[test]
    fn prompt_tags_in_context_are_removed_and_tagged_candidates_are_not_scored() {
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker());
        assistance.context("駅\u{ee01}で", true);
        assert_eq!(assistance.0.borrow().context, "駅で");
        let mut req = request();
        req.candidates[1].text.push('\u{ee01}');
        assert!(assistance.0.borrow().key(&req).is_none());
    }
    #[test]
    fn stopping_the_service_interrupts_retry_backoff() {
        let worker = worker();
        let queue = worker.0.queue.clone();
        let (started_tx, started_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            supervise(
                &queue,
                || {
                    started_tx.send(()).unwrap();
                    Err(io::Error::new(io::ErrorKind::TimedOut, "test timeout"))
                },
                &[Duration::from_secs(60)],
            );
            done_tx.send(()).unwrap();
        });
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        drop(worker);
        done_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        thread.join().unwrap();
    }
    #[test]
    fn empty_reading_warms_context_but_unavailable_results_do_not_rank() {
        let worker = worker();
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker.clone());
        assistance.context("駅で", true);
        input.process_key(&"K".parse().unwrap());
        assistance.prefetch(&input);
        let job = worker.0.queue.pending.lock().unwrap().take().unwrap();
        assert_eq!(job.key.context, "駅で");
        assert_eq!(job.key.reading, "");
        assert!(job.key.candidates.is_empty(), "{:?}", job.key.candidates);
        ready(&assistance, &request(), vec![-6., -0.2, -8., -9.]);
        worker
            .0
            .queue
            .status
            .store(Status::Unavailable as u8, Ordering::Release);
        assert!(
            assistance
                .0
                .borrow_mut()
                .response(&request())
                .order
                .is_none()
        );
    }
    #[test]
    fn model_failure_is_reported_without_blocking_input() {
        let worker = Worker::start("/nonexistent/tgskk-worker".into(), "unused".into()).unwrap();
        let until = Instant::now() + Duration::from_secs(2);
        while worker.status() != Status::Unavailable {
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker);
        assistance.context("駅で", true);
        assert!(
            assistance
                .0
                .borrow_mut()
                .response(&request())
                .order
                .is_none()
        );
    }
    #[test]
    fn kana_technical_and_okuri_policy_bounds_worker_inputs() {
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker());
        assistance.context(&"あ".repeat(200), true);
        assert_eq!(assistance.0.borrow().context.chars().count(), 40);
        let mut req = request();
        req.candidates[1].text = "Rust".into();
        assert!(assistance.0.borrow().key(&req).is_none());
        let mut req = request();
        req.okuri = Some("る".into());
        assert!(assistance.0.borrow().key(&req).is_none());
        let mut req = request();
        req.reading = "#".into();
        assert!(assistance.0.borrow().key(&req).is_none());
    }
}

#[cfg(test)]
mod protocol_tests {
    use super::*;
    fn key() -> Key {
        Key {
            epoch: 0,
            context: "駅で".into(),
            reading: "キシャ".into(),
            candidates: vec![ConversionCandidate {
                text: "汽車".into(),
                annotation: None,
            }],
            protected: vec![],
        }
    }
    fn pair() -> (Process, UnixStream) {
        let (socket, server) = UnixStream::pair().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_millis(30)))
            .unwrap();
        socket
            .set_write_timeout(Some(Duration::from_millis(30)))
            .unwrap();
        let child = Command::new("/bin/true").spawn().unwrap();
        (Process { child, socket }, server)
    }
    #[test]
    fn ipc_framing_and_little_endian_scores_round_trip() {
        let (mut process, mut server) = pair();
        let thread = std::thread::spawn(move || {
            assert_eq!(read_word(&mut server).unwrap(), 1);
            for expected in ["駅で", "キシャ", "汽車"] {
                let len = read_word(&mut server).unwrap();
                let mut bytes = vec![0; len as usize];
                server.read_exact(&mut bytes).unwrap();
                assert_eq!(String::from_utf8(bytes).unwrap(), expected);
            }
            server.write_all(&1u32.to_le_bytes()).unwrap();
            server.write_all(&(-0.25f64).to_le_bytes()).unwrap();
        });
        assert_eq!(process.score(&key()).unwrap(), vec![-0.25]);
        thread.join().unwrap();
    }
    #[test]
    fn refused_request_leaves_the_stream_usable() {
        let (mut process, mut server) = pair();
        server.write_all(&REJECTED_REQUEST.to_le_bytes()).unwrap();
        server.write_all(&1u32.to_le_bytes()).unwrap();
        server.write_all(&(-0.25f64).to_le_bytes()).unwrap();
        assert_eq!(
            process.score(&key()).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(process.score(&key()).unwrap(), vec![-0.25]);
    }
    #[test]
    fn invalid_count_nan_partial_response_and_stall_fail_closed() {
        for response in [
            2u32.to_le_bytes().to_vec(),
            [
                1u32.to_le_bytes().as_slice(),
                f64::NAN.to_le_bytes().as_slice(),
            ]
            .concat(),
            vec![1, 0],
        ] {
            let (mut process, mut server) = pair();
            server.write_all(&response).unwrap();
            assert!(process.score(&key()).is_err());
        }
        let (mut process, _server) = pair();
        assert!(process.score(&key()).is_err());
        let (mut process, _server) = pair();
        let mut oversized = key();
        oversized.context = "a".repeat(MAX_BYTES + 1);
        assert!(process.score(&oversized).is_err());
    }
}
