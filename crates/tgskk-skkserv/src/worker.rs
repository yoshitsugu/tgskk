//! One background SKK server worker with a bounded in-memory cache.
//!
//! A `Worker` validates and queues readings from caller threads, then performs
//! DNS resolution and the blocking lookup on a single background thread. The
//! TCP backend opens one connection per request; the managed Google backend
//! lazily owns one helper process. Callers only ever wait for a bounded budget;
//! network work never runs on the calling thread. Dropping the last `Worker`
//! stops the background thread without joining it.

use std::collections::VecDeque;
use std::error::Error as StdError;
use std::fmt;
use std::io;
use std::net::ToSocketAddrs;
#[cfg(unix)]
use std::os::fd::OwnedFd;
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, TryLockError};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(unix)]
use crate::client::TrailingData;
use crate::{Candidate, ClientError, Encoding, Error, ProtocolError, encode_request, fetch};

const MAX_HOST_BYTES: usize = 253;
const MAX_CALLER_WAIT: Duration = Duration::from_secs(1);
const FETCH_DEADLINE: Duration = Duration::from_secs(3);
#[cfg(unix)]
const HELPER_DEADLINE: Duration = Duration::from_secs(2);
const MAX_RESOLVED_ADDRESSES: usize = 16;
const CACHE_CAPACITY: usize = 128;
const POSITIVE_TTL: Duration = Duration::from_secs(300);
const NEGATIVE_TTL: Duration = Duration::from_secs(5);

/// Endpoint settings for a [`Worker`].
#[derive(Clone)]
pub struct Config {
    endpoint: Endpoint,
}

#[derive(Clone)]
enum Endpoint {
    Tcp {
        host: String,
        port: u16,
        encoding: Encoding,
    },
    Google {
        executable: PathBuf,
    },
}

impl Config {
    /// Creates TCP endpoint settings without resolving the hostname.
    pub fn tcp(host: &str, port: u16, encoding: Encoding) -> Result<Self, LookupError> {
        if host.is_empty() {
            return Err(LookupError::InvalidConfig(ConfigError::EmptyHost));
        }
        if host.len() > MAX_HOST_BYTES {
            return Err(LookupError::InvalidConfig(ConfigError::HostTooLong));
        }
        if host.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err(LookupError::InvalidConfig(
                ConfigError::InvalidHostCharacter,
            ));
        }
        if port == 0 {
            return Err(LookupError::InvalidConfig(ConfigError::ZeroPort));
        }
        Ok(Self {
            endpoint: Endpoint::Tcp {
                host: host.to_string(),
                port,
                encoding,
            },
        })
    }

    /// Creates managed Google-helper settings. The path is validated but not
    /// checked, spawned or resolved here; the helper always uses UTF-8.
    pub fn google(executable: impl Into<PathBuf>) -> Result<Self, LookupError> {
        let executable = executable.into();
        if executable.as_os_str().is_empty() {
            return Err(LookupError::InvalidConfig(ConfigError::EmptyExecutable));
        }
        if executable_has_nul(&executable) {
            return Err(LookupError::InvalidConfig(ConfigError::InvalidExecutable));
        }
        Ok(Self {
            endpoint: Endpoint::Google { executable },
        })
    }

    fn encoding(&self) -> Encoding {
        match &self.endpoint {
            Endpoint::Tcp { encoding, .. } => *encoding,
            Endpoint::Google { .. } => Encoding::Utf8,
        }
    }
}

#[cfg(unix)]
fn executable_has_nul(executable: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    executable.as_os_str().as_bytes().contains(&0)
}

#[cfg(not(unix))]
fn executable_has_nul(executable: &Path) -> bool {
    executable.to_string_lossy().contains('\0')
}

/// Why a [`Config`] was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    EmptyHost,
    HostTooLong,
    InvalidHostCharacter,
    ZeroPort,
    EmptyExecutable,
    InvalidExecutable,
}

/// Failure of a bounded worker lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookupError {
    InvalidConfig(ConfigError),
    InvalidReading(Error),
    Timeout,
    Busy,
    Unavailable,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyHost => "SKK server hostname is empty",
            Self::HostTooLong => "SKK server hostname is too long",
            Self::InvalidHostCharacter => {
                "SKK server hostname contains whitespace or a control character"
            }
            Self::ZeroPort => "SKK server port is zero",
            Self::EmptyExecutable => "SKK helper executable path is empty",
            Self::InvalidExecutable => "SKK helper executable path contains NUL",
        };
        f.write_str(message)
    }
}

impl StdError for ConfigError {}

impl fmt::Display for LookupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(err) => write!(f, "invalid SKK server config: {err}"),
            Self::InvalidReading(err) => write!(f, "invalid SKK lookup reading: {err}"),
            Self::Timeout => f.write_str("SKK server lookup timed out"),
            Self::Busy => f.write_str("SKK server lookup is busy"),
            Self::Unavailable => f.write_str("SKK server lookup is unavailable"),
        }
    }
}

impl StdError for LookupError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::InvalidConfig(err) => Some(err),
            Self::InvalidReading(err) => Some(err),
            Self::Timeout | Self::Busy | Self::Unavailable => None,
        }
    }
}

/// Internal failure detail used for worker diagnostics.
#[derive(Debug)]
enum LookupFailure {
    Dns,
    #[cfg(unix)]
    Spawn,
    #[cfg(not(unix))]
    Unsupported,
    Timeout,
    Io,
    Encoding(Error),
    Protocol(ProtocolError),
}

impl LookupFailure {
    fn from_client(error: ClientError) -> Self {
        match error {
            ClientError::Timeout => Self::Timeout,
            ClientError::Io(_) => Self::Io,
            ClientError::Encoding(err) => Self::Encoding(err),
            ClientError::Protocol(err) => Self::Protocol(err),
        }
    }
}

impl fmt::Display for LookupFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dns => f.write_str("SKK server hostname resolution failed"),
            #[cfg(unix)]
            Self::Spawn => f.write_str("SKK helper process could not start"),
            #[cfg(not(unix))]
            Self::Unsupported => {
                f.write_str("SKK helper process is not supported on this platform")
            }
            Self::Timeout => f.write_str("SKK server lookup timed out"),
            Self::Io => f.write_str("SKK server socket error"),
            Self::Encoding(err) => write!(f, "SKK lookup encoding error: {err}"),
            Self::Protocol(err) => write!(f, "SKK server protocol error: {err}"),
        }
    }
}

struct Queue {
    state: Mutex<State>,
    wake: Condvar,
    stopped: AtomicBool,
    diagnostic: Mutex<Option<String>>,
}

struct State {
    cache: Cache,
    in_flight: Option<String>,
    pending: Option<String>,
}

struct Service {
    queue: Arc<Queue>,
    config: Config,
}

impl Drop for Service {
    fn drop(&mut self) {
        self.queue.stopped.store(true, Ordering::Release);
        let _guard = lock_state(&self.queue);
        self.queue.wake.notify_all();
    }
}

/// Cloneable handle to one background SKK server worker.
#[derive(Clone)]
pub struct Worker(Arc<Service>);

impl Worker {
    /// Starts the single background thread. The thread stays idle until the
    /// first lookup request; no DNS or connection is attempted here.
    pub fn start(config: Config) -> io::Result<Self> {
        let queue = Arc::new(Queue {
            state: Mutex::new(State {
                cache: Cache::default(),
                in_flight: None,
                pending: None,
            }),
            wake: Condvar::new(),
            stopped: AtomicBool::new(false),
            diagnostic: Mutex::new(None),
        });
        let background = Arc::clone(&queue);
        let background_config = config.clone();
        thread::Builder::new()
            .name("tgskk-skkserv".into())
            .spawn(move || supervise(background, background_config))?;
        Ok(Self(Arc::new(Service { queue, config })))
    }

    /// Returns a cached result immediately, or queues a lookup and waits at
    /// most `wait` (capped at one second). A timeout does not cancel the
    /// background request, which may fill the cache later.
    pub fn lookup(&self, reading: &str, wait: Duration) -> Result<Vec<Candidate>, LookupError> {
        encode_request(reading, self.0.config.encoding()).map_err(LookupError::InvalidReading)?;

        let wait = wait.min(MAX_CALLER_WAIT);
        let deadline = Instant::now() + wait;
        let mut state = try_lock_state(&self.0.queue)?;

        if let Some(cached) = state.cache.get(reading, Instant::now()) {
            return Ok(cached);
        }
        if self.0.queue.stopped.load(Ordering::Acquire) {
            return Err(LookupError::Unavailable);
        }

        let already_queued = state.in_flight.as_deref() == Some(reading)
            || state.pending.as_deref() == Some(reading);
        if !already_queued {
            state.pending = Some(reading.to_string());
            self.0.queue.wake.notify_all();
        }
        if wait.is_zero() {
            return Err(LookupError::Timeout);
        }

        loop {
            let now = Instant::now();
            if let Some(cached) = state.cache.get(reading, now) {
                return Ok(cached);
            }
            if self.0.queue.stopped.load(Ordering::Acquire) {
                return Err(LookupError::Unavailable);
            }
            if now >= deadline {
                return Err(LookupError::Timeout);
            }
            let queued = state.in_flight.as_deref() == Some(reading)
                || state.pending.as_deref() == Some(reading);
            if !queued {
                return Err(LookupError::Unavailable);
            }
            let (next, _) = self
                .0
                .queue
                .wake
                .wait_timeout(state, deadline - now)
                .unwrap_or_else(PoisonError::into_inner);
            state = next;
        }
    }

    /// Returns the cached result without queueing, network work or waiting.
    /// Expired entries count as misses; a cached negative miss is `Some(empty)`.
    pub fn try_cached(&self, reading: &str) -> Option<Vec<Candidate>> {
        let mut state = match self.0.queue.state.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::WouldBlock) => return None,
            Err(TryLockError::Poisoned(poison)) => poison.into_inner(),
        };
        state.cache.get(reading, Instant::now())
    }

    /// Returns one diagnostic at a time without blocking on contention.
    pub fn take_error(&self) -> Option<String> {
        match self.0.queue.diagnostic.try_lock() {
            Ok(mut slot) => slot.take(),
            Err(TryLockError::WouldBlock) => None,
            Err(TryLockError::Poisoned(poison)) => poison.into_inner().take(),
        }
    }
}

fn lock_state(queue: &Queue) -> MutexGuard<'_, State> {
    queue.state.lock().unwrap_or_else(PoisonError::into_inner)
}

fn try_lock_state(queue: &Queue) -> Result<MutexGuard<'_, State>, LookupError> {
    match queue.state.try_lock() {
        Ok(guard) => Ok(guard),
        Err(TryLockError::WouldBlock) => Err(LookupError::Busy),
        Err(TryLockError::Poisoned(poison)) => Ok(poison.into_inner()),
    }
}

fn set_diagnostic(queue: &Queue, message: String) {
    let mut slot = queue
        .diagnostic
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    *slot = Some(message);
}

fn clear_diagnostic(queue: &Queue) {
    let mut slot = queue
        .diagnostic
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    *slot = None;
}

fn supervise(queue: Arc<Queue>, config: Config) {
    let result = catch_unwind(AssertUnwindSafe(|| worker_main(&queue, &config)));
    if result.is_err() {
        set_diagnostic(&queue, "SKK server worker stopped after a panic".into());
        queue.stopped.store(true, Ordering::Release);
        let _guard = lock_state(&queue);
        queue.wake.notify_all();
    }
}

fn worker_main(queue: &Queue, config: &Config) {
    let mut backend = Backend::new(config);
    loop {
        let reading = {
            let mut state = lock_state(queue);
            loop {
                if queue.stopped.load(Ordering::Acquire) {
                    return;
                }
                if let Some(reading) = state.pending.take() {
                    state.in_flight = Some(reading.clone());
                    break reading;
                }
                state = queue
                    .wake
                    .wait(state)
                    .unwrap_or_else(PoisonError::into_inner);
            }
        };

        // One absolute deadline covers DNS, connections and managed exchanges.
        let deadline = Instant::now() + FETCH_DEADLINE;
        let result = backend.lookup(&reading, deadline);

        let mut state = lock_state(queue);
        if !queue.stopped.load(Ordering::Acquire) {
            match result {
                Ok(candidates) => {
                    state.cache.insert(reading, candidates, Instant::now());
                    clear_diagnostic(queue);
                }
                Err(failure) => {
                    set_diagnostic(queue, format!("SKK server lookup failed: {failure}"));
                }
            }
        }
        state.in_flight = None;
        drop(state);
        queue.wake.notify_all();
    }
}

enum Backend {
    Tcp {
        host: String,
        port: u16,
        encoding: Encoding,
    },
    #[cfg(unix)]
    Google {
        executable: PathBuf,
        helper: Option<GoogleHelper>,
    },
    #[cfg(not(unix))]
    UnsupportedGoogle,
}

impl Backend {
    fn new(config: &Config) -> Self {
        match &config.endpoint {
            Endpoint::Tcp {
                host,
                port,
                encoding,
            } => Self::Tcp {
                host: host.clone(),
                port: *port,
                encoding: *encoding,
            },
            Endpoint::Google { executable } => {
                #[cfg(unix)]
                {
                    Self::Google {
                        executable: executable.clone(),
                        helper: None,
                    }
                }
                #[cfg(not(unix))]
                {
                    let _ = executable;
                    Self::UnsupportedGoogle
                }
            }
        }
    }

    fn lookup(
        &mut self,
        reading: &str,
        overall_deadline: Instant,
    ) -> Result<Vec<Candidate>, LookupFailure> {
        match self {
            Self::Tcp {
                host,
                port,
                encoding,
            } => tcp_lookup(host, *port, *encoding, reading, overall_deadline),
            #[cfg(unix)]
            Self::Google { executable, helper } => {
                google_lookup(executable, helper, reading, overall_deadline)
            }
            #[cfg(not(unix))]
            Self::UnsupportedGoogle => Err(LookupFailure::Unsupported),
        }
    }
}

/// Resolves the endpoint and tries addresses in order under one deadline.
///
/// std performs DNS on this worker thread and the OS resolver call cannot be
/// interrupted; the deadline is rechecked before each connection so an
/// overrunning resolver never leads to a connection after expiry. The last
/// typed failure is retained for diagnostics.
fn tcp_lookup(
    host: &str,
    port: u16,
    encoding: Encoding,
    reading: &str,
    deadline: Instant,
) -> Result<Vec<Candidate>, LookupFailure> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|_| LookupFailure::Dns)?;

    let mut last = None;
    for addr in addresses.take(MAX_RESOLVED_ADDRESSES) {
        if Instant::now() >= deadline {
            return Err(last.unwrap_or(LookupFailure::Timeout));
        }
        match fetch(addr, reading, encoding, deadline) {
            Ok(candidates) => return Ok(candidates),
            Err(ClientError::Timeout) => return Err(LookupFailure::Timeout),
            Err(error) => last = Some(LookupFailure::from_client(error)),
        }
    }
    Err(last.unwrap_or(LookupFailure::Dns))
}

#[cfg(unix)]
struct GoogleHelper {
    child: Child,
    stream: UnixStream,
}

#[cfg(unix)]
impl Drop for GoogleHelper {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(unix)]
impl GoogleHelper {
    fn start(executable: &Path) -> io::Result<Self> {
        let (stream, child_stream) = UnixStream::pair()?;
        let stdin = Stdio::from(OwnedFd::from(child_stream.try_clone()?));
        let stdout = Stdio::from(OwnedFd::from(child_stream));
        let child = Command::new(executable)
            .arg("--stdio")
            .arg("--encoding=utf-8")
            .stdin(stdin)
            .stdout(stdout)
            .stderr(Stdio::null())
            .spawn()?;
        Ok(Self { child, stream })
    }

    fn exchange(
        &mut self,
        request: &[u8],
        deadline: Instant,
    ) -> Result<Vec<Candidate>, LookupFailure> {
        crate::client::exchange(
            &mut self.stream,
            request,
            Encoding::Utf8,
            deadline,
            TrailingData::Reject,
        )
        .map_err(LookupFailure::from_client)
    }
}

/// Lazy managed helper exchange. The child is owned by this worker thread and
/// is dropped (killed and waited) on any failure, so the next lookup starts a
/// fresh process. The helper exchange has its own 2 second bound inside the
/// worker's overall 3 second deadline.
#[cfg(unix)]
fn google_lookup(
    executable: &Path,
    helper: &mut Option<GoogleHelper>,
    reading: &str,
    overall_deadline: Instant,
) -> Result<Vec<Candidate>, LookupFailure> {
    let deadline = overall_deadline.min(Instant::now() + HELPER_DEADLINE);
    if helper.is_none() {
        match GoogleHelper::start(executable) {
            Ok(process) => *helper = Some(process),
            Err(_) => return Err(LookupFailure::Spawn),
        }
    }
    if Instant::now() >= deadline {
        *helper = None;
        return Err(LookupFailure::Timeout);
    }

    let request = encode_request(reading, Encoding::Utf8).map_err(LookupFailure::Encoding)?;
    let response = match helper.as_mut() {
        Some(process) => process.exchange(&request, deadline),
        None => return Err(LookupFailure::Spawn),
    };
    match response {
        Ok(candidates) => Ok(candidates),
        Err(error) => {
            *helper = None;
            Err(error)
        }
    }
}

struct CacheEntry {
    reading: String,
    candidates: Vec<Candidate>,
    stored_at: Instant,
}

impl CacheEntry {
    fn ttl(&self) -> Duration {
        if self.candidates.is_empty() {
            NEGATIVE_TTL
        } else {
            POSITIVE_TTL
        }
    }

    fn is_fresh(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.stored_at) < self.ttl()
    }
}

#[derive(Default)]
struct Cache {
    entries: VecDeque<CacheEntry>,
}

impl Cache {
    fn get(&mut self, reading: &str, now: Instant) -> Option<Vec<Candidate>> {
        self.entries.retain(|entry| entry.is_fresh(now));
        self.entries
            .iter()
            .find(|entry| entry.reading == reading)
            .map(|entry| entry.candidates.clone())
    }

    fn insert(&mut self, reading: String, candidates: Vec<Candidate>, now: Instant) {
        self.entries
            .retain(|entry| entry.is_fresh(now) && entry.reading != reading);
        while self.entries.len() >= CACHE_CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back(CacheEntry {
            reading,
            candidates,
            stored_at: now,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> Candidate {
        Candidate {
            text: value.to_string(),
            annotation: None,
        }
    }

    #[test]
    fn positive_entries_expire_after_ttl() {
        let base = Instant::now();
        let mut cache = Cache::default();
        cache.insert("よみ".into(), vec![text("読")], base);

        assert_eq!(
            cache.get("よみ", base + POSITIVE_TTL - Duration::from_millis(1)),
            Some(vec![text("読")])
        );
        assert_eq!(cache.get("よみ", base + POSITIVE_TTL), None);
    }

    #[test]
    fn negative_entries_expire_after_short_ttl() {
        let base = Instant::now();
        let mut cache = Cache::default();
        cache.insert("なし".into(), Vec::new(), base);

        assert_eq!(
            cache.get("なし", base + NEGATIVE_TTL - Duration::from_millis(1)),
            Some(Vec::new())
        );
        assert_eq!(cache.get("なし", base + NEGATIVE_TTL), None);
    }

    #[test]
    fn capacity_evicts_oldest_entry() {
        let base = Instant::now();
        let mut cache = Cache::default();
        for index in 0..CACHE_CAPACITY {
            cache.insert(format!("r{index}"), vec![text("候補")], base);
        }

        assert!(cache.get("r0", base).is_some());
        cache.insert("overflow".into(), vec![text("候補")], base);

        assert_eq!(cache.get("r0", base), None);
        assert!(cache.get("r1", base).is_some());
        assert!(cache.get("overflow", base).is_some());
    }

    #[test]
    fn reinserting_refreshes_expiry() {
        let base = Instant::now();
        let mut cache = Cache::default();
        cache.insert("よみ".into(), vec![text("古")], base);
        let refreshed = base + Duration::from_secs(1);
        cache.insert("よみ".into(), vec![text("新")], refreshed);

        assert_eq!(cache.get("よみ", refreshed), Some(vec![text("新")]));
        let later = base + POSITIVE_TTL + Duration::from_millis(500);
        assert_eq!(cache.get("よみ", later), Some(vec![text("新")]));
    }

    #[test]
    fn expired_entries_are_dropped_on_insert() {
        let base = Instant::now();
        let mut cache = Cache::default();
        for index in 0..CACHE_CAPACITY {
            cache.insert(format!("r{index}"), vec![text("候補")], base);
        }

        let later = base + POSITIVE_TTL + Duration::from_secs(1);
        cache.insert("new".into(), vec![text("候補")], later);
        assert!(cache.get("new", later).is_some());
        assert_eq!(cache.get("r0", later), None);
    }
}
