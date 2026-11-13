use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use tgskk_skkserv::{
    Candidate, Config, ConfigError, Encoding, Error, LookupError, Worker, encode_request,
};

fn candidate(text: &str) -> Candidate {
    Candidate {
        text: text.to_string(),
        annotation: None,
    }
}

struct TestServer {
    addr: SocketAddr,
    handle: Option<thread::JoinHandle<()>>,
}

impl TestServer {
    fn start<F>(connections: usize, handler: F) -> Self
    where
        F: FnMut(TcpStream, usize) + Send + 'static,
    {
        Self::on(
            TcpListener::bind(("127.0.0.1", 0)).expect("bind"),
            connections,
            handler,
        )
    }
    fn on<F>(listener: TcpListener, connections: usize, mut handler: F) -> Self
    where
        F: FnMut(TcpStream, usize) + Send + 'static,
    {
        let addr = listener.local_addr().expect("local addr");
        listener
            .set_nonblocking(true)
            .expect("nonblocking listener");
        let handle = thread::spawn(move || {
            for index in 0..connections {
                let accept_deadline = Instant::now() + Duration::from_secs(5);
                let stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                            if Instant::now() >= accept_deadline {
                                return;
                            }
                            thread::sleep(Duration::from_millis(2));
                        }
                        Err(_) => return,
                    }
                };
                stream.set_nonblocking(false).expect("blocking stream");
                stream.set_read_timeout(Some(Duration::from_secs(1))).ok();
                stream.set_write_timeout(Some(Duration::from_secs(1))).ok();
                handler(stream, index);
            }
        });
        Self {
            addr,
            handle: Some(handle),
        }
    }

    fn addr(&self) -> SocketAddr {
        self.addr
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn read_request(stream: &mut TcpStream) -> Vec<u8> {
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .expect("read timeout");
    let mut request = Vec::new();
    let mut byte = [0u8; 1];
    while request.len() < 4096 {
        match stream.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                request.push(byte[0]);
                if byte[0] == b' ' {
                    break;
                }
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    request
}

fn send(stream: &mut TcpStream, bytes: &[u8]) -> bool {
    stream
        .write_all(bytes)
        .and_then(|()| stream.flush())
        .is_ok()
}

fn tcp_config(server: &TestServer, encoding: Encoding) -> Config {
    Config::tcp("127.0.0.1", server.addr().port(), encoding).expect("config")
}

fn expect_timeout(result: Result<Vec<Candidate>, LookupError>) {
    match result {
        Err(LookupError::Timeout) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn positive_cache_reuses_utf8_result_without_second_request() {
    let (request_tx, request_rx) = mpsc::channel();
    let server = TestServer::start(1, move |mut stream, _| {
        let request = read_request(&mut stream);
        let _ = request_tx.send(request);
        let _ = send(&mut stream, b"1/\xe6\x97\xa5\xe6\x9c\xac/\n");
    });
    let worker = Worker::start(tcp_config(&server, Encoding::Utf8)).expect("worker");

    let first = worker
        .lookup("にほん", Duration::from_secs(1))
        .expect("first lookup");
    assert_eq!(first, vec![candidate("日本")]);

    let request = request_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("request");
    assert_eq!(
        request.as_slice(),
        b"1\xe3\x81\xab\xe3\x81\xbb\xe3\x82\x93 "
    );

    let second = worker
        .lookup("にほん", Duration::ZERO)
        .expect("cached lookup");
    assert_eq!(second, first);
    assert!(request_rx.try_recv().is_err());
    assert_eq!(worker.try_cached("にほん"), Some(first));
}

#[test]
fn positive_cache_reuses_euc_jp_result_without_second_request() {
    let (request_tx, request_rx) = mpsc::channel();
    let server = TestServer::start(1, move |mut stream, _| {
        let request = read_request(&mut stream);
        let _ = request_tx.send(request);
        let _ = send(&mut stream, b"1/\xc6\xfc\xcb\xdc/\n");
    });
    let worker = Worker::start(tcp_config(&server, Encoding::EucJp)).expect("worker");

    let first = worker
        .lookup("か", Duration::from_secs(1))
        .expect("first lookup");
    assert_eq!(first, vec![candidate("日本")]);

    let request = request_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("request");
    assert_eq!(request.as_slice(), b"1\xa4\xab ");

    let second = worker.lookup("か", Duration::ZERO).expect("cached lookup");
    assert_eq!(second, first);
    assert!(request_rx.try_recv().is_err());
}

#[test]
fn negative_cache_reuses_miss() {
    let (request_tx, request_rx) = mpsc::channel();
    let server = TestServer::start(1, move |mut stream, _| {
        let request = read_request(&mut stream);
        let _ = request_tx.send(request);
        let _ = send(&mut stream, b"4\xe3\x81\x8b \n");
    });
    let worker = Worker::start(tcp_config(&server, Encoding::Utf8)).expect("worker");

    let first = worker
        .lookup("か", Duration::from_secs(1))
        .expect("first lookup");
    assert_eq!(first, Vec::<Candidate>::new());

    let request = request_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("request");
    assert_eq!(request.as_slice(), b"1\xe3\x81\x8b ");

    let second = worker.lookup("か", Duration::ZERO).expect("cached miss");
    assert_eq!(second, Vec::<Candidate>::new());
    assert!(request_rx.try_recv().is_err());
    assert_eq!(worker.try_cached("か"), Some(Vec::new()));
}

#[test]
fn zero_wait_enqueues_without_blocking() {
    let (request_tx, request_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let server = TestServer::start(1, move |mut stream, _| {
        let request = read_request(&mut stream);
        let _ = request_tx.send(request);
        let _ = release_rx.recv_timeout(Duration::from_secs(5));
        let _ = send(&mut stream, b"1/\xe6\x97\xa5\xe6\x9c\xac/\n");
    });
    let worker = Worker::start(tcp_config(&server, Encoding::Utf8)).expect("worker");

    let started = Instant::now();
    expect_timeout(worker.lookup("にほん", Duration::ZERO));
    assert!(started.elapsed() < Duration::from_millis(500));

    let _ = request_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("queued request");
    let _ = release_tx.send(());

    let later = worker
        .lookup("にほん", Duration::from_secs(1))
        .expect("late result");
    assert_eq!(later, vec![candidate("日本")]);
    assert!(request_rx.try_recv().is_err());
}

#[test]
fn timeout_does_not_cancel_and_cache_fills_later() {
    let (request_tx, request_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let server = TestServer::start(1, move |mut stream, _| {
        let request = read_request(&mut stream);
        let _ = request_tx.send(request);
        let _ = release_rx.recv_timeout(Duration::from_secs(5));
        let _ = send(&mut stream, b"1/\xe6\x97\xa5\xe6\x9c\xac/\n");
    });
    let worker = Worker::start(tcp_config(&server, Encoding::Utf8)).expect("worker");

    let started = Instant::now();
    expect_timeout(worker.lookup("にほん", Duration::from_millis(120)));
    assert!(started.elapsed() < Duration::from_secs(1));

    let _ = request_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("request");
    assert_eq!(worker.try_cached("にほん"), None);
    let _ = release_tx.send(());

    let later = worker
        .lookup("にほん", Duration::from_secs(1))
        .expect("late result");
    assert_eq!(later, vec![candidate("日本")]);
    assert!(request_rx.try_recv().is_err());
}

#[test]
fn duplicate_readings_share_one_request() {
    let (request_tx, request_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let server = TestServer::start(1, move |mut stream, _| {
        let request = read_request(&mut stream);
        let _ = request_tx.send(request);
        let _ = release_rx.recv_timeout(Duration::from_secs(5));
        let _ = send(&mut stream, b"1/\xe6\x97\xa5\xe6\x9c\xac/\n");
    });
    let worker = Worker::start(tcp_config(&server, Encoding::Utf8)).expect("worker");

    let (result_tx, result_rx) = mpsc::channel();
    let mut handles = Vec::new();
    for _ in 0..2 {
        let worker = worker.clone();
        let result_tx = result_tx.clone();
        handles.push(thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(1);
            let result = loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                match worker.lookup("にほん", remaining) {
                    // Queue contention is allowed by the nonblocking lookup API.
                    Err(LookupError::Busy) if !remaining.is_zero() => thread::yield_now(),
                    result => break result,
                }
            };
            let _ = result_tx.send(result);
        }));
    }
    drop(result_tx);

    let _ = request_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("request");
    let _ = release_tx.send(());

    for _ in 0..2 {
        let result = result_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("lookup result");
        assert_eq!(result.expect("lookup"), vec![candidate("日本")]);
    }
    for handle in handles {
        let _ = handle.join();
    }
    assert!(request_rx.try_recv().is_err());
}

#[test]
fn latest_pending_reading_supersedes_earlier() {
    let (log_tx, log_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let server = TestServer::start(2, move |mut stream, index| {
        let request = read_request(&mut stream);
        let _ = log_tx.send(request);
        if index == 0 {
            let _ = release_rx.recv_timeout(Duration::from_secs(5));
        }
        let response: &[u8] = if index == 0 {
            b"1/\xe5\x85\x88/\n"
        } else {
            b"1/\xe5\xbe\x8c/\n"
        };
        let _ = send(&mut stream, response);
    });
    let worker = Worker::start(tcp_config(&server, Encoding::Utf8)).expect("worker");

    let first_worker = worker.clone();
    let first = thread::spawn(move || first_worker.lookup("さき", Duration::from_secs(1)));
    let _ = log_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("first request");

    expect_timeout(worker.lookup("なか", Duration::ZERO));
    expect_timeout(worker.lookup("あと", Duration::ZERO));
    let _ = release_tx.send(());

    let first_result = first.join().expect("join first lookup");
    assert_eq!(first_result.expect("first"), vec![candidate("先")]);

    let last = worker
        .lookup("あと", Duration::from_secs(1))
        .expect("last lookup");
    assert_eq!(last, vec![candidate("後")]);

    let second_request = log_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("second request");
    assert_eq!(
        second_request.as_slice(),
        encode_request("あと", Encoding::Utf8).unwrap().as_slice()
    );
    assert!(log_rx.try_recv().is_err());
}

#[test]
fn failure_is_not_cached_as_miss() {
    let (log_tx, log_rx) = mpsc::channel();
    let server = TestServer::start(2, move |mut stream, index| {
        let request = read_request(&mut stream);
        let _ = log_tx.send(request);
        if index == 1 {
            let _ = send(&mut stream, b"1/\xe6\x97\xa5\xe6\x9c\xac/\n");
        }
    });
    let worker = Worker::start(tcp_config(&server, Encoding::Utf8)).expect("worker");

    match worker.lookup("にほん", Duration::from_secs(1)) {
        Err(LookupError::Unavailable) => {}
        other => panic!("unexpected result: {other:?}"),
    }
    assert_eq!(worker.try_cached("にほん"), None);
    assert!(worker.take_error().is_some());
    let _ = log_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("first request");

    let second = worker
        .lookup("にほん", Duration::from_secs(1))
        .expect("second lookup");
    assert_eq!(second, vec![candidate("日本")]);
    let _ = log_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("second request");
    assert!(log_rx.try_recv().is_err());
    assert!(worker.take_error().is_none());
}

#[test]
fn caller_wait_is_capped_at_one_second() {
    let (request_tx, request_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let server = TestServer::start(1, move |mut stream, _| {
        let request = read_request(&mut stream);
        let _ = request_tx.send(request);
        let _ = release_rx.recv_timeout(Duration::from_secs(5));
        let _ = send(&mut stream, b"1/\xe6\x97\xa5\xe6\x9c\xac/\n");
    });
    let worker = Worker::start(tcp_config(&server, Encoding::Utf8)).expect("worker");

    let started = Instant::now();
    expect_timeout(worker.lookup("にほん", Duration::from_secs(30)));
    assert!(started.elapsed() < Duration::from_secs(2));

    let _ = request_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("request");
    let _ = release_tx.send(());
}

#[test]
fn dropping_last_worker_does_not_block() {
    let config = Config::tcp("127.0.0.1", 1178, Encoding::Utf8).expect("config");
    let worker = Worker::start(config).expect("worker");

    let started = Instant::now();
    drop(worker);
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn config_validation_is_typed() {
    assert_eq!(
        Config::tcp("", 1178, Encoding::Utf8).err(),
        Some(LookupError::InvalidConfig(ConfigError::EmptyHost))
    );

    let too_long = "a".repeat(254);
    assert_eq!(
        Config::tcp(&too_long, 1178, Encoding::Utf8).err(),
        Some(LookupError::InvalidConfig(ConfigError::HostTooLong))
    );
    let at_limit = "a".repeat(253);
    assert!(Config::tcp(&at_limit, 1178, Encoding::Utf8).is_ok());

    for host in ["a b", "a\tb", "a\u{0}b"] {
        assert_eq!(
            Config::tcp(host, 1178, Encoding::Utf8).err(),
            Some(LookupError::InvalidConfig(
                ConfigError::InvalidHostCharacter
            )),
            "host {host:?}"
        );
    }

    assert_eq!(
        Config::tcp("127.0.0.1", 0, Encoding::Utf8).err(),
        Some(LookupError::InvalidConfig(ConfigError::ZeroPort))
    );
    assert!(Config::tcp("localhost", 1178, Encoding::EucJp).is_ok());
    let _clone = Config::tcp("127.0.0.1", 1178, Encoding::Utf8)
        .expect("config")
        .clone();
}

#[test]
fn invalid_readings_are_rejected_before_network() {
    let config = Config::tcp("127.0.0.1", 1178, Encoding::EucJp).expect("config");
    let worker = Worker::start(config).expect("worker");

    assert_eq!(
        worker.lookup("", Duration::from_secs(1)).err(),
        Some(LookupError::InvalidReading(Error::EmptyReading))
    );
    assert_eq!(
        worker.lookup("か な", Duration::from_secs(1)).err(),
        Some(LookupError::InvalidReading(Error::InvalidReadingCharacter))
    );
    assert_eq!(
        worker.lookup("😀", Duration::from_secs(1)).err(),
        Some(LookupError::InvalidReading(Error::UnrepresentableReading))
    );
    let too_long = "a".repeat(1025);
    assert_eq!(
        worker.lookup(&too_long, Duration::from_secs(1)).err(),
        Some(LookupError::InvalidReading(Error::ReadingTooLong))
    );
}

#[test]
fn hostname_host_is_resolved_by_worker() {
    let (request_tx, request_rx) = mpsc::channel();
    let server = TestServer::on(
        TcpListener::bind(("localhost", 0)).expect("bind hostname"),
        1,
        move |mut stream, _| {
            let request = read_request(&mut stream);
            let _ = request_tx.send(request);
            let _ = send(&mut stream, b"1/\xe6\x97\xa5\xe6\x9c\xac/\n");
        },
    );
    let config = Config::tcp("localhost", server.addr().port(), Encoding::Utf8).expect("config");
    let worker = Worker::start(config).expect("worker");

    let result = worker
        .lookup("にほん", Duration::from_secs(1))
        .expect("lookup");
    assert_eq!(result, vec![candidate("日本")]);
    let _ = request_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("request");
}
