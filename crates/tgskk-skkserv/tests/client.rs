use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use tgskk_skkserv::{
    Candidate, ClientError, Encoding, Error, MAX_RESPONSE_BYTES, ProtocolError, fetch,
};

fn in_ms(millis: u64) -> Instant {
    Instant::now() + Duration::from_millis(millis)
}

fn candidate(text: &str, annotation: Option<&str>) -> Candidate {
    Candidate {
        text: text.to_string(),
        annotation: annotation.map(str::to_string),
    }
}

struct TestServer {
    addr: SocketAddr,
    handle: Option<thread::JoinHandle<()>>,
}

impl TestServer {
    fn start<F>(script: F) -> Self
    where
        F: FnOnce(TcpStream) + Send + 'static,
    {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
        let addr = listener.local_addr().expect("local addr");
        listener
            .set_nonblocking(true)
            .expect("nonblocking listener");
        let handle = thread::spawn(move || {
            let accept_deadline = Instant::now() + Duration::from_secs(5);
            let stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                        if Instant::now() >= accept_deadline {
                            return;
                        }
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => return,
                }
            };
            stream.set_nonblocking(false).expect("blocking stream");
            stream.set_write_timeout(Some(Duration::from_secs(1))).ok();
            script(stream);
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
            Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
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

fn serve_and_fetch(
    response: Vec<u8>,
    reading: &str,
    encoding: Encoding,
    deadline: Instant,
) -> Result<Vec<Candidate>, ClientError> {
    let server = TestServer::start(move |mut stream| {
        let _ = read_request(&mut stream);
        if send(&mut stream, &response) {
            let _ = stream.shutdown(Shutdown::Write);
        }
    });
    fetch(server.addr(), reading, encoding, deadline)
}

#[test]
fn fetch_sends_utf8_request_and_decodes_response() {
    let (tx, rx) = mpsc::channel();
    let server = TestServer::start(move |mut stream| {
        let request = read_request(&mut stream);
        let _ = tx.send(request);
        let response = b"1/\xe6\x97\xa5\xe6\x9c\xac;\xe3\x81\xab\xe3\x81\xbb\xe3\x82\x93/\n";
        if send(&mut stream, response) {
            let _ = stream.shutdown(Shutdown::Write);
        }
    });

    let result = fetch(server.addr(), "にほん", Encoding::Utf8, in_ms(3000)).expect("fetch");
    assert_eq!(result, vec![candidate("日本", Some("にほん"))]);

    let observed = rx.recv_timeout(Duration::from_secs(1)).expect("request");
    assert_eq!(
        observed.as_slice(),
        b"1\xe3\x81\xab\xe3\x81\xbb\xe3\x82\x93 "
    );
}

#[test]
fn fetch_sends_euc_jp_request_and_decodes_response() {
    let (tx, rx) = mpsc::channel();
    let server = TestServer::start(move |mut stream| {
        let request = read_request(&mut stream);
        let _ = tx.send(request);
        if send(&mut stream, b"1/\xc6\xfc\xcb\xdc/\n") {
            let _ = stream.shutdown(Shutdown::Write);
        }
    });

    let result = fetch(server.addr(), "か", Encoding::EucJp, in_ms(3000)).expect("fetch");
    assert_eq!(result, vec![candidate("日本", None)]);

    let observed = rx.recv_timeout(Duration::from_secs(1)).expect("request");
    assert_eq!(observed.as_slice(), b"1\xa4\xab ");
}

#[test]
fn fetch_reassembles_fragmented_response_split_inside_utf8_char() {
    let chunks: Vec<Vec<u8>> = vec![
        b"1/".to_vec(),
        b"\xe6".to_vec(),
        b"\x97\xa5".to_vec(),
        b"\xe6\x9c".to_vec(),
        b"\xac/\n".to_vec(),
    ];
    let server = TestServer::start(move |mut stream| {
        let _ = read_request(&mut stream);
        for chunk in &chunks {
            if !send(&mut stream, chunk) {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        let _ = stream.shutdown(Shutdown::Write);
    });

    let result = fetch(server.addr(), "にほん", Encoding::Utf8, in_ms(3000)).expect("fetch");
    assert_eq!(result, vec![candidate("日本", None)]);
}

#[test]
fn fetch_stops_at_lf_and_ignores_trailing_bytes() {
    let result = serve_and_fetch(
        b"1/\xe6\x97\xa5\xe6\x9c\xac/\nTRAILING".to_vec(),
        "にほん",
        Encoding::Utf8,
        in_ms(3000),
    )
    .expect("fetch");
    assert_eq!(result, vec![candidate("日本", None)]);
}

#[test]
fn fetch_accepts_echoed_utf8_miss() {
    let result = serve_and_fetch(
        b"4\xe3\x81\x8b \n".to_vec(),
        "か",
        Encoding::Utf8,
        in_ms(3000),
    )
    .expect("fetch");
    assert_eq!(result, Vec::<Candidate>::new());
}

#[test]
fn fetch_accepts_echoed_euc_jp_miss() {
    let result = serve_and_fetch(b"4\xa4\xab \n".to_vec(), "か", Encoding::EucJp, in_ms(3000))
        .expect("fetch");
    assert_eq!(result, Vec::<Candidate>::new());
}

#[test]
fn fetch_rejects_miss_echo_with_nul() {
    let result = serve_and_fetch(b"4\x00\n".to_vec(), "か", Encoding::Utf8, in_ms(3000));
    match result {
        Err(ClientError::Protocol(ProtocolError::Codec(Error::MalformedResponse))) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn fetch_reports_standalone_zero_without_lf() {
    let result = serve_and_fetch(b"0".to_vec(), "か", Encoding::Utf8, in_ms(3000));
    match result {
        Err(ClientError::Protocol(ProtocolError::ServerError)) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn fetch_rejects_eof_before_lf() {
    let result = serve_and_fetch(b"1/\xe6\x97".to_vec(), "か", Encoding::Utf8, in_ms(3000));
    match result {
        Err(ClientError::Protocol(ProtocolError::Truncated)) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn fetch_rejects_invalid_response_bytes() {
    let result = serve_and_fetch(b"1/\xff/\n".to_vec(), "か", Encoding::Utf8, in_ms(3000));
    match result {
        Err(ClientError::Protocol(ProtocolError::Codec(Error::InvalidResponseEncoding))) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn fetch_accepts_response_at_size_limit() {
    let server = TestServer::start(move |mut stream| {
        let _ = read_request(&mut stream);
        let mut body = Vec::with_capacity(MAX_RESPONSE_BYTES);
        body.extend_from_slice(b"1/");
        body.resize(MAX_RESPONSE_BYTES - 2, b'a');
        body.extend_from_slice(b"/\n");
        let _ = stream.write_all(&body);
        let _ = stream.flush();
        let _ = stream.shutdown(Shutdown::Write);
    });

    let candidates = fetch(server.addr(), "か", Encoding::Utf8, in_ms(5000)).expect("fetch");
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].text.len(), MAX_RESPONSE_BYTES - 4);
}

#[test]
fn fetch_rejects_oversized_terminated_response() {
    let server = TestServer::start(move |mut stream| {
        let _ = read_request(&mut stream);
        let mut body = vec![b'1'; MAX_RESPONSE_BYTES];
        body.push(b'\n');
        let _ = stream.write_all(&body);
        let _ = stream.flush();
        let _ = stream.shutdown(Shutdown::Write);
    });

    let result = fetch(server.addr(), "か", Encoding::Utf8, in_ms(5000));
    match result {
        Err(ClientError::Protocol(ProtocolError::Oversized)) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn fetch_rejects_oversized_response_without_lf() {
    let server = TestServer::start(move |mut stream| {
        let _ = read_request(&mut stream);
        let body = vec![b'1'; MAX_RESPONSE_BYTES + 1];
        let _ = stream.write_all(&body);
        let _ = stream.flush();
        let _ = stream.shutdown(Shutdown::Write);
    });

    let result = fetch(server.addr(), "か", Encoding::Utf8, in_ms(5000));
    match result {
        Err(ClientError::Protocol(ProtocolError::Oversized)) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn fetch_times_out_when_server_does_not_respond() {
    let (done_tx, done_rx) = mpsc::channel();
    let server = TestServer::start(move |mut stream| {
        let _ = read_request(&mut stream);
        let _ = done_rx.recv_timeout(Duration::from_secs(2));
    });

    let started = Instant::now();
    let result = fetch(server.addr(), "か", Encoding::Utf8, in_ms(150));
    match result {
        Err(ClientError::Timeout) => {}
        other => panic!("unexpected result: {other:?}"),
    }
    assert!(started.elapsed() < Duration::from_secs(2));
    let _ = done_tx.send(());
}

#[test]
fn fetch_enforces_total_deadline_for_trickled_response() {
    let (done_tx, done_rx) = mpsc::channel();
    let server = TestServer::start(move |mut stream| {
        let _ = read_request(&mut stream);
        for _ in 0..8 {
            if !send(&mut stream, b"1") {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        let _ = done_rx.recv_timeout(Duration::from_secs(2));
    });

    let result = fetch(server.addr(), "か", Encoding::Utf8, in_ms(180));
    match result {
        Err(ClientError::Timeout) => {}
        other => panic!("unexpected result: {other:?}"),
    }
    let _ = done_tx.send(());
}

#[test]
fn fetch_rejects_expired_deadline_before_connecting() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let addr = listener.local_addr().expect("local addr");
    let expired = Instant::now();
    thread::sleep(Duration::from_millis(2));

    let result = fetch(addr, "か", Encoding::Utf8, expired);
    match result {
        Err(ClientError::Timeout) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn fetch_reports_unencodable_reading() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let addr = listener.local_addr().expect("local addr");

    let result = fetch(addr, "😀", Encoding::EucJp, in_ms(1000));
    match result {
        Err(ClientError::Encoding(Error::UnrepresentableReading)) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn client_error_display_does_not_leak_wire_data() {
    let result = serve_and_fetch(b"secret\n".to_vec(), "秘密", Encoding::Utf8, in_ms(3000));
    let error = result.expect_err("response must be rejected");
    let message = error.to_string();
    assert!(!message.contains("secret"));
    assert!(!message.contains("秘密"));
}
