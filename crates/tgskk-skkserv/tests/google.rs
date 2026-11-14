#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use tgskk_skkserv::{Candidate, Config, ConfigError, Encoding, Error, LookupError, Worker};

static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(label: &str) -> Self {
        let unique = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "tgskk-skkserv-d2-{}-{label}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create test dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn candidate(text: &str) -> Candidate {
    Candidate {
        text: text.to_string(),
        annotation: None,
    }
}

const SUCCESS: &str =
    r#"sys.stdout.buffer.write(bytes.fromhex("312fe585882f0a")); sys.stdout.buffer.flush()"#;
const FRAGMENTED: &str = r#"sys.stdout.buffer.write(bytes.fromhex("312f")); sys.stdout.buffer.flush(); time.sleep(0.02); sys.stdout.buffer.write(bytes.fromhex("e5")); sys.stdout.buffer.flush(); time.sleep(0.02); sys.stdout.buffer.write(bytes.fromhex("8588")); sys.stdout.buffer.flush(); time.sleep(0.02); sys.stdout.buffer.write(bytes.fromhex("2f0a")); sys.stdout.buffer.flush()"#;
const ZERO: &str = r#"sys.stdout.buffer.write(b"0"); sys.stdout.buffer.flush()"#;
const MALFORMED: &str = r#"sys.stdout.buffer.write(b"garbage\n"); sys.stdout.buffer.flush()"#;
const CRASH: &str = "sys.exit(0)";
const TRAILING: &str = r#"sys.stdout.buffer.write(b"1/a/\nEXTRA"); sys.stdout.buffer.flush()"#;
const HUNG: &str = "time.sleep(30)";

#[expect(
    clippy::unnecessary_debug_formatting,
    reason = "Python source needs a quoted and escaped path literal."
)]
fn write_script(dir: &Path, name: &str, log: &Path, action: &str) -> PathBuf {
    let path = dir.join(name);
    let script = format!(
        r#"#!/usr/bin/env python3
import os
import sys
import time

LOG = {log:?}

def append(line):
    with open(LOG, "a", encoding="utf-8") as handle:
        handle.write(line + "\n")

def respond():
    {action}

append("start " + str(os.getpid()) + " " + " ".join(sys.argv[1:]))

while True:
    code = sys.stdin.buffer.read(1)
    if not code:
        break
    if code == b"0":
        break
    if code != b"1":
        sys.stdout.buffer.write(b"0")
        sys.stdout.buffer.flush()
        break
    reading = bytearray()
    while True:
        byte = sys.stdin.buffer.read(1)
        if not byte:
            sys.exit(0)
        if byte == b" ":
            break
        reading += byte
    append("request " + str(os.getpid()) + " " + reading.hex())
    respond()
"#
    );
    // A concurrent fork can inherit a writer to this inode and cause ETXTBSY.
    // Only the copy subprocess opens the executable for writing.
    let source = path.with_extension("source");
    fs::write(&source, script).expect("write helper source");
    assert!(
        Command::new("cp")
            .arg(&source)
            .arg(&path)
            .status()
            .expect("copy helper")
            .success()
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod helper");
    path
}

fn worker_for(script: &Path) -> Worker {
    Worker::start(Config::google(script.to_path_buf()).expect("config")).expect("worker")
}

fn log_lines(log: &Path) -> Vec<String> {
    fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

fn start_pids(log: &Path) -> Vec<i32> {
    log_lines(log)
        .iter()
        .filter_map(|line| line.strip_prefix("start "))
        .filter_map(|rest| rest.split_whitespace().next())
        .filter_map(|pid| pid.parse().ok())
        .collect()
}

fn requests(log: &Path) -> Vec<Vec<String>> {
    log_lines(log)
        .into_iter()
        .filter(|line| line.starts_with("request "))
        .map(|line| line.split_whitespace().map(str::to_string).collect())
        .collect()
}

fn wait_for(mut condition: impl FnMut() -> bool, timeout: Duration, what: &str) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if condition() {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(condition(), "timed out waiting for {what}");
}

fn process_alive(pid: i32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .status()
        .is_ok_and(|status| status.success())
}

fn wait_error(worker: &Worker, timeout: Duration) -> String {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Some(error) = worker.take_error() {
            return error;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for a worker diagnostic");
}

#[test]
fn google_helper_is_lazy_reused_and_cached() {
    let dir = TestDir::new("lazy");
    let log = dir.path().join("helper.log");
    let script = write_script(dir.path(), "helper.py", &log, SUCCESS);
    let worker = worker_for(&script);
    assert!(!log.exists(), "helper started before the first lookup");

    let first = worker
        .lookup("さき", Duration::from_secs(1))
        .expect("first lookup");
    assert_eq!(first, vec![candidate("先")]);

    wait_for(
        || !start_pids(&log).is_empty(),
        Duration::from_secs(1),
        "helper start marker",
    );
    let start = log_lines(&log)
        .into_iter()
        .find(|line| line.starts_with("start "))
        .expect("start line");
    let parts: Vec<_> = start.split_whitespace().collect();
    assert_eq!(&parts[2..], &["--stdio", "--encoding=utf-8"][..], "{start}");

    let second = worker
        .lookup("なか", Duration::from_secs(1))
        .expect("second lookup");
    assert_eq!(second, vec![candidate("先")]);

    let observed = requests(&log);
    assert_eq!(observed.len(), 2);
    assert_eq!(observed[0][1], observed[1][1], "helper process was reused");
    assert_eq!(observed[0][2], "e38195e3818d");
    assert_eq!(observed[1][2], "e381aae3818b");

    let cached = worker
        .lookup("さき", Duration::ZERO)
        .expect("cached lookup");
    assert_eq!(cached, vec![candidate("先")]);
    thread::sleep(Duration::from_millis(150));
    assert_eq!(requests(&log).len(), 2, "cache hit reached the helper");

    let pid = observed[0][1].parse::<i32>().expect("pid");
    drop(worker);
    wait_for(
        || !process_alive(pid),
        Duration::from_secs(2),
        "helper process reap",
    );
}

#[test]
fn google_reassembles_fragmented_utf8_response() {
    let dir = TestDir::new("fragmented");
    let log = dir.path().join("helper.log");
    let script = write_script(dir.path(), "helper.py", &log, FRAGMENTED);
    let worker = worker_for(&script);

    let result = worker
        .lookup("さき", Duration::from_secs(1))
        .expect("lookup");
    assert_eq!(result, vec![candidate("先")]);
    let observed = requests(&log);
    assert_eq!(observed[0][2], "e38195e3818d");
}

#[test]
fn google_standalone_zero_resets_and_next_lookup_respawns() {
    let dir = TestDir::new("zero");
    let log = dir.path().join("helper.log");
    let script = write_script(dir.path(), "helper.py", &log, ZERO);
    let worker = worker_for(&script);

    for attempt in 0..2 {
        match worker.lookup("ひみつ", Duration::from_secs(1)) {
            Err(LookupError::Unavailable) => {}
            other => panic!("unexpected result: {other:?}"),
        }
        let error = worker.take_error().expect("diagnostic");
        assert!(
            error.contains("reported an error"),
            "attempt {attempt}: {error}, log={:?}",
            log_lines(&log)
        );
        assert!(!error.contains("ひみつ"));
    }

    wait_for(
        || start_pids(&log).len() == 2,
        Duration::from_secs(2),
        &format!("two helper starts; log={:?}", log_lines(&log)),
    );
    let pids = start_pids(&log);
    assert_ne!(pids[0], pids[1], "helper was not restarted after 0");
}

#[test]
fn google_malformed_response_reports_protocol_error_and_resets() {
    let dir = TestDir::new("malformed");
    let log = dir.path().join("helper.log");
    let script = write_script(dir.path(), "helper.py", &log, MALFORMED);
    let worker = worker_for(&script);

    match worker.lookup("ひみつ", Duration::from_secs(1)) {
        Err(LookupError::Unavailable) => {}
        other => panic!("unexpected result: {other:?}"),
    }
    let error = worker.take_error().expect("diagnostic");
    assert!(error.contains("malformed SKK server response"), "{error}");
    assert!(!error.contains("ひみつ"));

    match worker.lookup("ひみつ", Duration::from_secs(1)) {
        Err(LookupError::Unavailable) => {}
        other => panic!("unexpected result: {other:?}"),
    }
    wait_for(
        || start_pids(&log).len() == 2,
        Duration::from_secs(2),
        &format!("two helper starts; log={:?}", log_lines(&log)),
    );
    let pids = start_pids(&log);
    assert_ne!(pids[0], pids[1]);
}

#[test]
fn google_crash_eof_starts_fresh_helper() {
    let dir = TestDir::new("crash");
    let log = dir.path().join("helper.log");
    let script = write_script(dir.path(), "helper.py", &log, CRASH);
    let worker = worker_for(&script);

    match worker.lookup("ひみつ", Duration::from_secs(1)) {
        Err(LookupError::Unavailable) => {}
        other => panic!("unexpected result: {other:?}"),
    }
    let error = worker.take_error().expect("diagnostic");
    assert!(error.contains("closed the connection"), "{error}");
    assert!(!error.contains("ひみつ"));

    match worker.lookup("ひみつ", Duration::from_secs(1)) {
        Err(LookupError::Unavailable) => {}
        other => panic!("unexpected result: {other:?}"),
    }
    wait_for(
        || start_pids(&log).len() == 2,
        Duration::from_secs(2),
        &format!("two helper starts; log={:?}", log_lines(&log)),
    );
    let pids = start_pids(&log);
    assert_ne!(pids[0], pids[1]);
}

#[test]
fn google_hung_helper_is_bounded_and_reaped() {
    let dir = TestDir::new("hung");
    let log = dir.path().join("helper.log");
    let script = write_script(dir.path(), "helper.py", &log, HUNG);
    let worker = worker_for(&script);

    let started = Instant::now();
    match worker.lookup("ひみつ", Duration::from_millis(300)) {
        Err(LookupError::Timeout) => {}
        other => panic!("unexpected result: {other:?}"),
    }
    assert!(started.elapsed() < Duration::from_secs(1));

    wait_for(
        || !start_pids(&log).is_empty(),
        Duration::from_secs(1),
        "helper start marker",
    );
    let pid = start_pids(&log)[0];

    let error = wait_error(&worker, Duration::from_secs(5));
    assert!(error.contains("timed out"), "{error}");
    assert!(!error.contains("ひみつ"));
    wait_for(
        || !process_alive(pid),
        Duration::from_secs(2),
        "hung helper reap",
    );
}

#[test]
fn dropping_last_worker_reaps_idle_helper_promptly() {
    let dir = TestDir::new("drop");
    let log = dir.path().join("helper.log");
    let script = write_script(dir.path(), "helper.py", &log, SUCCESS);
    let worker = worker_for(&script);
    worker
        .lookup("さき", Duration::from_secs(1))
        .expect("lookup");
    let pid = start_pids(&log)[0];

    let started = Instant::now();
    drop(worker);
    assert!(started.elapsed() < Duration::from_secs(1));
    wait_for(
        || !process_alive(pid),
        Duration::from_secs(2),
        "helper reap after drop",
    );
}

#[test]
fn google_startup_failure_reports_static_diagnostic() {
    let dir = TestDir::new("startup");
    let missing = dir.path().join("missing-helper");
    let worker = Worker::start(Config::google(missing.clone()).expect("config")).expect("worker");

    match worker.lookup("ひみつ", Duration::from_secs(1)) {
        Err(LookupError::Unavailable) => {}
        other => panic!("unexpected result: {other:?}"),
    }
    let error = worker.take_error().expect("diagnostic");
    assert!(error.contains("could not start"), "{error}");
    assert!(!error.contains("ひみつ"));
    assert!(!error.contains("missing-helper"));
}

#[test]
fn google_persistent_stream_rejects_trailing_bytes() {
    let dir = TestDir::new("trailing");
    let log = dir.path().join("helper.log");
    let script = write_script(dir.path(), "helper.py", &log, TRAILING);
    let worker = worker_for(&script);

    match worker.lookup("ひみつ", Duration::from_secs(1)) {
        Err(LookupError::Unavailable) => {}
        other => panic!("unexpected result: {other:?}"),
    }
    let error = worker.take_error().expect("diagnostic");
    assert!(error.contains("trailing data"), "{error}");
    assert!(!error.contains("ひみつ"));

    match worker.lookup("ひみつ", Duration::from_secs(1)) {
        Err(LookupError::Unavailable) => {}
        other => panic!("unexpected result: {other:?}"),
    }
    wait_for(
        || start_pids(&log).len() == 2,
        Duration::from_secs(2),
        &format!("two helper starts; log={:?}", log_lines(&log)),
    );
}

#[test]
fn google_rejects_invalid_reading_before_spawn() {
    let dir = TestDir::new("reading");
    let log = dir.path().join("helper.log");
    let script = write_script(dir.path(), "helper.py", &log, SUCCESS);
    let worker = worker_for(&script);

    assert_eq!(
        worker.lookup("か な", Duration::from_secs(1)).err(),
        Some(LookupError::InvalidReading(Error::InvalidReadingCharacter))
    );
    assert!(!log.exists(), "invalid reading started the helper");
}

#[test]
fn google_config_validates_the_executable_path() {
    assert_eq!(
        Config::google("").err(),
        Some(LookupError::InvalidConfig(ConfigError::EmptyExecutable))
    );
    assert_eq!(
        Config::google(PathBuf::from("a\0b")).err(),
        Some(LookupError::InvalidConfig(ConfigError::InvalidExecutable))
    );

    let dir = TestDir::new("config");
    assert!(Config::google(dir.path().join("does-not-exist")).is_ok());
    assert!(Config::tcp("127.0.0.1", 1178, Encoding::Utf8).is_ok());
}
