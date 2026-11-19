//! Local candidate scoring. The input thread only offers jobs and reads ready
//! results with `try_lock`; model loading and IPC belong to the worker thread.
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::Child;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use tgskk_core::input::{ConversionCandidate, ConversionRequest, ConversionResponse, InputSession};

const MAX_CANDIDATES: usize = 8;
const MAX_BYTES: usize = 1024;
const CACHE_SIZE: usize = 16;
const MIN_MARGIN: f64 = 1.5;
// Overriding a confident left-context leader needs a wider margin.
const REVERSAL_MARGIN: f64 = 2.1;
// Measured: these improved ranking most without hurting left-context-only cases.
const RIGHT_WEIGHT: f64 = 0.4;
const RIGHT_CHARS: usize = 8;
// A changed wire format must use a different handshake, including refusals.
const PROTOCOL_MAGIC: u32 = 0x335A_4754;
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

/// The candidate's log probability, and the joined candidate and right context
/// log probability minus it.
pub type Score = (f64, f64);

#[derive(Clone, Debug, PartialEq, Eq)]
struct Key {
    epoch: u64,
    context: String,
    right: String,
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
    ready: Mutex<VecDeque<(Key, Vec<Score>)>>,
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
    fn ready(&self, key: &Key) -> Option<Vec<Score>> {
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
    socket: tgskk_process::Stream,
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
        let (child, socket) = tgskk_process::spawn(executable, [model.as_os_str()])
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
    fn score(&mut self, key: &Key) -> io::Result<Vec<Score>> {
        let mut frame = Vec::new();
        frame.extend_from_slice(
            &u32::try_from(key.candidates.len())
                .map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidInput, "Too many Zenz candidates")
                })?
                .to_le_bytes(),
        );
        for text in [&key.context, &key.right, &key.reading]
            .into_iter()
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
            scores.push((read_score(&mut self.socket)?, read_score(&mut self.socket)?));
        }
        Ok(scores)
    }
}
fn read_word(socket: &mut impl Read) -> io::Result<u32> {
    let mut bytes = [0; 4];
    socket.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}
fn read_score(socket: &mut impl Read) -> io::Result<f64> {
    let mut bytes = [0; 8];
    socket.read_exact(&mut bytes)?;
    let score = f64::from_le_bytes(bytes);
    if !score.is_finite() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid Zenz score",
        ));
    }
    Ok(score)
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
    mut score: impl FnMut(&Key) -> io::Result<Vec<Score>>,
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
        if job_is_ready(queue, &job)? {
            let result = score(&job.key);
            complete_job(queue, job, result)?;
        }
    }
}

fn job_is_ready(queue: &Queue, job: &Job) -> io::Result<bool> {
    if job.epoch.load(Ordering::Acquire) != job.key.epoch {
        return Ok(false);
    }
    Ok(!queue
        .ready
        .lock()
        .map_err(|_| io::Error::other("Zenz cache poisoned"))?
        .iter()
        .any(|(key, _)| key == &job.key))
}

fn complete_job(queue: &Queue, job: Job, result: io::Result<Vec<Score>>) -> io::Result<()> {
    let scores = match result {
        Ok(scores) => {
            queue.scored.store(true, Ordering::Release);
            scores
        }
        Err(error) if error.kind() == io::ErrorKind::InvalidInput => {
            if let Ok(mut diagnostic) = queue.diagnostic.lock() {
                *diagnostic =
                    Some("Zenz rejected one conversion request; using dictionary order".into());
            }
            return Ok(());
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
        return Ok(());
    }
    let mut ready = queue
        .ready
        .lock()
        .map_err(|_| io::Error::other("Zenz cache poisoned"))?;
    if ready.len() == CACHE_SIZE {
        ready.pop_front();
    }
    ready.push_back((job.key, scores));
    Ok(())
}

#[cfg(test)]
mod simulation;

fn is_tag(c: char) -> bool {
    ('\u{ee00}'..='\u{ee0f}').contains(&c)
}

static NEXT_EPOCH: AtomicU64 = AtomicU64::new(1);
fn next_epoch() -> u64 {
    NEXT_EPOCH.fetch_add(1, Ordering::Relaxed)
}
struct State {
    worker: Worker,
    context: String,
    right: String,
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
                || c.text
                    .chars()
                    .any(|c| c.is_ascii() || c == '\u{3000}' || is_tag(c))
        }) {
            return None;
        }
        Some(Key {
            epoch: self.epoch.load(Ordering::Relaxed),
            context: self.context.clone(),
            right: self.right.clone(),
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
        if let Some(index) = winner(&scores, &key.protected) {
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
/// Combines a candidate's score with how well the right context follows it.
#[must_use]
pub fn mixed((base, right): Score) -> f64 {
    base + RIGHT_WEIGHT * right
}
fn winner(scores: &[Score], protected: &[usize]) -> Option<usize> {
    let mixed: Vec<_> = scores.iter().copied().map(mixed).collect();
    let base: Vec<_> = scores.iter().map(|(base, _)| *base).collect();
    let first = match (leader(&base), leader(&mixed)) {
        (Some((left, left_margin)), Some((combined, combined_margin)))
            if left_margin >= MIN_MARGIN && combined != left =>
        {
            if combined_margin >= REVERSAL_MARGIN {
                combined
            } else {
                left
            }
        }
        (_, Some((combined, margin))) if margin >= MIN_MARGIN => combined,
        (Some((left, margin)), _) if margin >= MIN_MARGIN => left,
        _ => return None,
    };
    (first != 0 && !protected.contains(&first)).then_some(first)
}
fn leader(scores: &[f64]) -> Option<(usize, f64)> {
    let mut indices: Vec<_> = (0..scores.len()).collect();
    indices.sort_by(|a, b| scores[*b].total_cmp(&scores[*a]));
    let first = *indices.first()?;
    Some((first, scores[first] - scores[*indices.get(1)?]))
}
#[derive(Clone)]
pub struct Assistance(Rc<RefCell<State>>);
impl Assistance {
    pub fn attach(input: &mut InputSession, worker: Worker) -> Self {
        let state = Rc::new(RefCell::new(State {
            worker,
            context: String::new(),
            right: String::new(),
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
    pub fn context(&self, left: &str, right: &str, enabled: bool) {
        let (left, right) = if enabled {
            (
                left.chars()
                    .filter(|c| !is_tag(*c))
                    .rev()
                    .take(40)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect(),
                right
                    .chars()
                    .take_while(|c| !matches!(c, '\n' | '\r'))
                    .filter(|c| !is_tag(*c))
                    .take(RIGHT_CHARS)
                    .collect(),
            )
        } else {
            (String::new(), String::new())
        };
        let mut state = self.0.borrow_mut();
        if state.context != left || state.right != right || state.enabled != enabled {
            state.context = left;
            state.right = right;
            state.enabled = enabled;
            state.epoch.store(next_epoch(), Ordering::Release);
        }
    }
    pub fn reset(&self) {
        let mut state = self.0.borrow_mut();
        state.epoch.store(next_epoch(), Ordering::Release);
        state.context.clear();
        state.right.clear();
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
pub struct Evaluator(Option<Process>);
impl Evaluator {
    pub fn open(executable: &Path, model: &Path) -> io::Result<Self> {
        Ok(Self(Some(Process::start(executable, model)?)))
    }
    pub fn score(
        &mut self,
        left: &str,
        right: &str,
        katakana: &str,
        candidates: &[String],
    ) -> io::Result<Vec<Score>> {
        if candidates.len() > MAX_CANDIDATES {
            return Err(io::Error::other("At most eight candidates"));
        }
        let process = self.0.as_mut().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "Evaluator failed; open a new evaluator",
            )
        })?;
        let result = process.score(&Key {
            epoch: 0,
            context: left.into(),
            right: right.into(),
            reading: katakana.into(),
            candidates: candidates
                .iter()
                .map(|text| ConversionCandidate {
                    text: text.clone(),
                    annotation: None,
                })
                .collect(),
            protected: Vec::new(),
        });
        if result
            .as_ref()
            .is_err_and(|error| error.kind() != io::ErrorKind::InvalidInput)
        {
            self.0 = None;
        }
        result
    }
    /// Evaluation entry for the live ranking with no protected candidates.
    #[must_use]
    pub fn first_candidate(scores: &[Score]) -> usize {
        winner(scores, &[]).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Instant;

    pub(super) fn worker() -> Worker {
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
    pub(super) fn input() -> InputSession {
        InputSession::open(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/SKK-JISYO.demo.utf8"
            ),
            "utf-8",
        )
        .unwrap()
    }
    fn base(scores: &[f64]) -> Vec<Score> {
        scores.iter().map(|score| (*score, 0.)).collect()
    }
    fn ready(assistance: &Assistance, req: &ConversionRequest, scores: Vec<Score>) {
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
        assistance.context("取引先である", "", true);
        let mut req = request();
        req.protected_indices = vec![1];
        ready(&assistance, &req, base(&[-6., -8., -5., -0.2]));
        let response = assistance.0.borrow_mut().response(&req);
        assert_eq!(response.order, Some(vec![3, 1, 0, 2]));
        assert_eq!(response.learning, None);
        assert!(assistance.applied());
    }
    #[test]
    fn close_scores_and_protected_first_candidate_prevent_intervention() {
        assert_eq!(winner(&base(&[-1., -0.2, -5.]), &[]), None);
        assert_eq!(winner(&base(&[-5., -0.2, -6.]), &[1]), None);
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker());
        assistance.context("駅で", "", true);
        let mut req = request();
        req.protected_indices = vec![0];
        ready(&assistance, &req, base(&[-5., -0.2, -6., -7.]));
        assert_eq!(assistance.0.borrow_mut().response(&req).order, None);
        assert!(!assistance.applied());
    }
    #[test]
    fn results_must_match_context_reading_candidate_identity_and_protection() {
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker());
        assistance.context("駅で", "", true);
        let req = request();
        ready(&assistance, &req, base(&[-6., -0.2, -8., -9.]));
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
        assistance.context("新聞の", "", true);
        assert!(assistance.0.borrow_mut().response(&req).order.is_none());
        assistance.context("駅で", "", true);
        assert!(assistance.0.borrow_mut().response(&req).order.is_none());
    }
    #[test]
    fn results_must_match_the_right_context() {
        let mut input = input();
        let worker = worker();
        let assistance = Assistance::attach(&mut input, worker.clone());
        assistance.context("駅で", "に乗る", true);
        let req = request();
        ready(&assistance, &req, base(&[-6., -0.2, -8., -9.]));
        let key = assistance.0.borrow().key(&req).unwrap();
        assert_eq!(key.right, "に乗る");
        let mut other = key.clone();
        other.right = "を書く".into();
        assert!(worker.ready(&key).is_some());
        assert!(worker.ready(&other).is_none());
        assistance.context("駅で", "を書く", true);
        assert!(assistance.0.borrow_mut().response(&req).order.is_none());
        assistance.context("駅で", "", true);
        assert!(assistance.0.borrow_mut().response(&req).order.is_none());
    }
    #[test]
    fn empty_right_context_keeps_the_left_only_ranking() {
        for (scores, protected, expected) in [
            (vec![-6., -8., -5., -0.2], vec![1], Some(3)),
            (vec![-6., -0.2, -8., -9.], vec![], Some(1)),
            (vec![-1., -0.2, -5.], vec![], None),
            (vec![-5., -0.2, -6.], vec![1], None),
            (vec![-0.2, -5., -6.], vec![], None),
            (vec![-1.7, -0.2, -8.], vec![], Some(1)),
            (vec![-0.2], vec![], None),
        ] {
            assert_eq!(
                winner(&base(&scores), &protected),
                expected,
                "{scores:?} {protected:?}"
            );
        }
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker());
        assistance.context("取引先である", "", true);
        let mut req = request();
        req.protected_indices = vec![1];
        ready(&assistance, &req, base(&[-6., -8., -5., -0.2]));
        assert_eq!(
            assistance.0.borrow_mut().response(&req).order,
            Some(vec![3, 1, 0, 2])
        );
    }
    #[test]
    fn confident_right_context_promotes_a_candidate_the_left_alone_does_not() {
        let scores = [(-1., -10.), (-1.5, -2.), (-5., -10.), (-6., -10.)];
        let left_only: Vec<_> = scores.iter().map(|(base, _)| *base).collect();
        assert_eq!(winner(&base(&left_only), &[]), None);
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker());
        assistance.context("", "に乗る", true);
        let req = request();
        ready(&assistance, &req, scores.to_vec());
        assert_eq!(
            assistance.0.borrow_mut().response(&req).order,
            Some(vec![1, 0, 2, 3])
        );
        assert!(assistance.applied());
        assert_eq!(winner(&scores, &[1]), None);
    }
    #[test]
    fn confident_right_context_keeps_the_first_candidate_against_the_left() {
        let scores = [(-5., 0.), (-0.2, -20.), (-8., -20.), (-9., -20.)];
        let left_only: Vec<_> = scores.iter().map(|(base, _)| *base).collect();
        assert_eq!(winner(&base(&left_only), &[]), Some(1));
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker());
        assistance.context("駅で", "を書く", true);
        let req = request();
        ready(&assistance, &req, scores.to_vec());
        assert_eq!(assistance.0.borrow_mut().response(&req).order, None);
        assert!(!assistance.applied());
    }
    #[test]
    fn close_mixed_scores_fall_back_to_the_left_context() {
        let scores = [(-6., 0.), (-0.2, -5.), (-2., 0.), (-9., 0.)];
        assert_eq!(winner(&scores, &[]), Some(1));
        assert_eq!(winner(&scores, &[1]), None);
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker());
        assistance.context("駅で", "を", true);
        let req = request();
        ready(&assistance, &req, scores.to_vec());
        assert_eq!(
            assistance.0.borrow_mut().response(&req).order,
            Some(vec![1, 0, 2, 3])
        );
    }
    #[test]
    fn right_context_must_beat_a_confident_left_leader_by_2_1() {
        // The left leader is index 2 with the 1.5 margin; the right context lifts
        // index 1 by only 1.6, so the left-context leader stays first.
        assert_eq!(winner(&[(-6., 0.), (-4.5, 7.75), (-3., 0.)], &[]), Some(2));
        // A mixed margin of exactly 2.0 is still too small for the reversal.
        assert_eq!(winner(&[(-6., 0.), (-5., 10.), (-3., 0.)], &[]), Some(2));
        // Exactly 2.1 admits it; the mixed scores are 0.0 and -2.1.
        assert_eq!(winner(&[(-6., 0.), (-4., 10.), (-2.1, 0.)], &[]), Some(1));
        // The same rule decides when the left leader is the dictionary head.
        assert_eq!(winner(&[(-3., 0.), (-5., 9.)], &[]), None);
        assert_eq!(winner(&[(-2.1, 0.), (-4., 10.)], &[]), Some(1));
        // An ambiguous left context keeps the 1.5 margin for the right context.
        assert_eq!(winner(&[(-9., 0.), (-5.2, 6.), (-4.4, 0.)], &[]), Some(1));
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker());
        assistance.context("駅で", "に乗る", true);
        let req = request();
        ready(
            &assistance,
            &req,
            [(-6., 0.), (-4., 10.), (-2.1, 0.), (-9., 0.)].to_vec(),
        );
        assert_eq!(
            assistance.0.borrow_mut().response(&req).order,
            Some(vec![1, 0, 2, 3])
        );
    }
    #[test]
    fn right_context_stops_at_a_line_break_and_drops_prompt_tags() {
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker());
        for (right, expected) in [
            ("を見る\n次の行", "を見る"),
            ("に乗る\r\n次の行", "に乗る"),
            ("\n次の行", ""),
            ("あいうえおかきくけこ", "あいうえおかきく"),
            ("を\u{ee01}見\u{ee0f}るために出かける", "を見るために出か"),
        ] {
            assistance.context("駅で", right, true);
            assert_eq!(assistance.0.borrow().right, expected, "{right:?}");
        }
        assistance.context("駅で", "を見る", false);
        assert_eq!(assistance.0.borrow().right, "");
        assistance.context("駅で", "を見る", true);
        assistance.reset();
        assert_eq!(assistance.0.borrow().right, "");
    }
    #[test]
    fn sensitive_reset_and_dropped_sessions_invalidate_queued_work() {
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker());
        assistance.context("駅で", "", true);
        let req = request();
        ready(&assistance, &req, base(&[-6., -0.2, -8., -9.]));
        let epoch = assistance.0.borrow().epoch.clone();
        let before = epoch.load(Ordering::Acquire);
        assistance.context("password", "", false);
        assert_ne!(before, epoch.load(Ordering::Acquire));
        assert_eq!(assistance.0.borrow().context, "");
        assert!(assistance.0.borrow_mut().response(&req).order.is_none());
        assistance.context("駅で", "", true);
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
        assistance.context("駅で", "", true);
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
        let epoch = Arc::new(AtomicU64::new(1));
        let key = |reading: &str, number| Key {
            epoch: number,
            context: "駅で".into(),
            right: String::new(),
            reading: reading.into(),
            candidates: request().candidates,
            protected: vec![],
        };
        let first = key("キ", 1);
        assert!(worker.offer(Job {
            key: first.clone(),
            epoch: epoch.clone()
        }));
        // Offer before starting the consumer so try_lock cannot contend at startup.
        let thread = std::thread::spawn(move || {
            worker_loop(&queue, |key| {
                started_tx.send(key.reading.clone()).unwrap();
                continue_rx.recv_timeout(Duration::from_secs(2)).unwrap();
                Ok(vec![(-1., 0.); key.candidates.len()])
            })
        });
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
        assistance.context("駅で", "", true);
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
                            Ok(base(&[-5., -0.2, -8., -9.]))
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
        assistance.context("駅で", "", true);
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
                    Ok(base(&[-5., -0.2, -8., -9.]))
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
        assistance.context("駅で", "", true);
        ready(&assistance, &request(), base(&[-6., -0.2, -8., -9.]));
        assert!(
            assistance
                .0
                .borrow_mut()
                .response(&request())
                .order
                .is_some()
        );
        assistance.context("新しい文脈", "", true);
        assert!(assistance.applied());
        assistance.reset();
        assert!(!assistance.applied());
    }
    #[test]
    fn prompt_tags_in_context_are_removed_and_tagged_candidates_are_not_scored() {
        let mut input = input();
        let assistance = Assistance::attach(&mut input, worker());
        assistance.context("駅\u{ee01}で", "", true);
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
        assistance.context("駅で", "", true);
        input.process_key(&"K".parse().unwrap());
        assistance.prefetch(&input);
        let job = worker.0.queue.pending.lock().unwrap().take().unwrap();
        assert_eq!(job.key.context, "駅で");
        assert_eq!(job.key.reading, "");
        assert!(job.key.candidates.is_empty(), "{:?}", job.key.candidates);
        ready(&assistance, &request(), base(&[-6., -0.2, -8., -9.]));
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
        assistance.context("駅で", "", true);
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
        assistance.context(&"あ".repeat(200), "", true);
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

#[cfg(all(test, unix))]
mod protocol_tests {
    use super::*;
    use std::os::unix::net::UnixStream;
    use std::process::Command;
    fn key() -> Key {
        Key {
            epoch: 0,
            context: "駅で".into(),
            right: "を見る".into(),
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
            for expected in ["駅で", "を見る", "キシャ", "汽車"] {
                let len = read_word(&mut server).unwrap();
                let mut bytes = vec![0; len as usize];
                server.read_exact(&mut bytes).unwrap();
                assert_eq!(String::from_utf8(bytes).unwrap(), expected);
            }
            server.write_all(&1u32.to_le_bytes()).unwrap();
            server.write_all(&(-0.25f64).to_le_bytes()).unwrap();
            server.write_all(&(-1.5f64).to_le_bytes()).unwrap();
        });
        assert_eq!(process.score(&key()).unwrap(), vec![(-0.25, -1.5)]);
        thread.join().unwrap();
    }
    #[test]
    fn refused_request_leaves_the_stream_usable() {
        let (process, mut server) = pair();
        let mut evaluator = Evaluator(Some(process));
        server.write_all(&REJECTED_REQUEST.to_le_bytes()).unwrap();
        server.write_all(&1u32.to_le_bytes()).unwrap();
        server.write_all(&(-0.25f64).to_le_bytes()).unwrap();
        server.write_all(&0f64.to_le_bytes()).unwrap();
        assert_eq!(
            evaluator
                .score("駅で", "", "キシャ", &["汽車".into()])
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            evaluator
                .score("駅で", "", "キシャ", &["汽車".into()])
                .unwrap(),
            vec![(-0.25, 0.)]
        );
    }
    #[test]
    fn evaluator_discards_a_misaligned_or_timed_out_process() {
        for response in [Some(2u32.to_le_bytes()), None] {
            let (process, mut server) = pair();
            let mut evaluator = Evaluator(Some(process));
            if let Some(response) = response {
                server.write_all(&response).unwrap();
            }
            let error = evaluator
                .score("駅で", "", "キシャ", &["汽車".into()])
                .unwrap_err();
            assert!(matches!(
                error.kind(),
                io::ErrorKind::InvalidData | io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
            ));
            assert_eq!(
                evaluator
                    .score("駅で", "", "キシャ", &["汽車".into()])
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::BrokenPipe
            );
        }
    }
    #[test]
    fn invalid_count_nan_partial_response_and_stall_fail_closed() {
        for response in [
            2u32.to_le_bytes().to_vec(),
            [
                1u32.to_le_bytes().as_slice(),
                f64::NAN.to_le_bytes().as_slice(),
                0f64.to_le_bytes().as_slice(),
            ]
            .concat(),
            [
                1u32.to_le_bytes().as_slice(),
                (-0.25f64).to_le_bytes().as_slice(),
                f64::INFINITY.to_le_bytes().as_slice(),
            ]
            .concat(),
            [
                1u32.to_le_bytes().as_slice(),
                (-0.25f64).to_le_bytes().as_slice(),
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
        let (mut process, _server) = pair();
        let mut oversized = key();
        oversized.right = "a".repeat(MAX_BYTES + 1);
        assert!(process.score(&oversized).is_err());
    }
}
