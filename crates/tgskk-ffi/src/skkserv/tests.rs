use super::*;
use crate::{tgskk_session_key, tgskk_session_reset, tgskk_session_set_zenz, tgskk_string_free};
use std::ffi::{CStr, CString};
use std::io::{Read, Write};
use std::net::TcpListener;
#[cfg(unix)]
use std::process::Command;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::thread;
use std::time::Instant;
use tgskk_core::input::{CskkDictionary, InputSession};

struct Server {
    port: u16,
    count: Arc<AtomicUsize>,
    requests: std::sync::mpsc::Receiver<Vec<u8>>,
    stop: std::sync::mpsc::Sender<()>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Server {
    fn new(response: &[u8], delay: Duration) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let count = Arc::new(AtomicUsize::new(0));
        let observed = count.clone();
        let (stop, stopped) = std::sync::mpsc::channel();
        let (send, requests) = std::sync::mpsc::channel();
        let response = response.to_vec();
        let thread = thread::spawn(move || {
            while stopped.try_recv().is_err() {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(1)))
                            .unwrap();
                        observed.fetch_add(1, Ordering::SeqCst);
                        let mut request = Vec::new();
                        let mut byte = [0];
                        while stream.read_exact(&mut byte).is_ok() {
                            request.push(byte[0]);
                            if byte[0] == b' ' {
                                break;
                            }
                        }
                        let _ = send.send(request);
                        if stopped.recv_timeout(delay).is_ok() {
                            return;
                        }
                        let _ = stream.write_all(&response);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if stopped.recv_timeout(Duration::from_millis(2)).is_ok() {
                            return;
                        }
                    }
                    Err(e) => panic!("accept failed: {e}"),
                }
            }
        });
        Self {
            port,
            count,
            requests,
            stop,
            thread: Some(thread),
        }
    }

    fn handle(&self, encoding: &str, wait_ms: u32) -> *mut TgskkSkkserv {
        let mut error = ptr::null_mut();
        let encoding = CString::new(encoding).unwrap();
        let handle = unsafe {
            tgskk_skkserv_custom_new(
                c"127.0.0.1".as_ptr(),
                self.port,
                encoding.as_ptr(),
                wait_ms,
                &mut error,
            )
        };
        assert!(error.is_null());
        assert!(!handle.is_null());
        handle
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        self.thread.take().unwrap().join().unwrap();
    }
}

fn session() -> Box<TgskkSession> {
    let input = InputSession::open(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/SKK-JISYO.demo.utf8"
        ),
        "utf-8",
    )
    .unwrap();
    Box::new(TgskkSession::new(input).unwrap())
}

fn keys(session: &mut TgskkSession, trace: &str) {
    for token in trace.split_whitespace() {
        let (key, modifiers) = match token {
            "space" => (32, 0),
            "Return" => (0xff0d, 0),
            "C-g" => (u32::from(b'g'), 4),
            _ => (
                u32::from(token.as_bytes()[0]),
                u32::from(token.as_bytes()[0].is_ascii_uppercase()),
            ),
        };
        assert!(!unsafe { tgskk_session_key(session, key, modifiers, 0) }.is_null());
    }
}

fn preedit(session: &TgskkSession) -> &str {
    session.preedit.to_str().unwrap()
}

#[test]
fn permission_is_per_session_defaults_off_and_reset_revokes_it() {
    let server = Server::new("1/東京/\n".as_bytes(), Duration::ZERO);
    let handle = server.handle("utf-8", 300);
    let mut first = session();
    let mut second = session();
    unsafe {
        assert_eq!(tgskk_session_set_skkserv(&mut *first, handle), 1);
        assert_eq!(tgskk_session_set_skkserv(&mut *second, handle), 1);
        keys(&mut first, "T o u k y o u space");
        assert_eq!(server.count.load(Ordering::SeqCst), 0);
        tgskk_session_reset(&mut *first);
        tgskk_session_network_allowed(&mut *first, 1);
        keys(&mut first, "T o u k y o u space");
        assert_eq!(preedit(&first), "▼東京");
        assert_eq!(
            server
                .requests
                .recv_timeout(Duration::from_secs(1))
                .unwrap(),
            "1とうきょう ".as_bytes()
        );
        keys(&mut second, "T o u k y o u space");
        assert!(!preedit(&second).starts_with("▼東京"));
        tgskk_session_reset(&mut *first);
        keys(&mut first, "T o u k y o u space");
        assert!(!preedit(&first).starts_with("▼東京"));
        assert_eq!(server.count.load(Ordering::SeqCst), 1);
        tgskk_skkserv_free(handle);
    }
}

#[test]
fn local_hits_bypass_server_with_and_without_global_provider() {
    let server = Server::new(b"0", Duration::from_secs(1));
    let handle = server.handle("utf-8", 1000);
    for global in [false, true] {
        let mut session = session();
        unsafe {
            tgskk_session_set_skkserv(&mut *session, handle);
            tgskk_session_network_allowed(&mut *session, 1);
        }
        if global {
            session
                .input
                .set_conversion_provider(Some(Box::new(|_: &ConversionRequest| {
                    ConversionResponse::default()
                })));
        }
        keys(&mut session, "K a n j i space");
        assert_eq!(preedit(&session), "▼漢字");
        assert_eq!(server.count.load(Ordering::SeqCst), 0);
    }
    unsafe {
        tgskk_skkserv_free(handle);
    }
}

#[test]
fn malformed_items_are_filtered_individually_and_handle_lifetime_is_retained() {
    let server = Server::new(
        "1/#0/[bad]/東京;首都/東京;duplicate/(concat \"DOS\\057V\")/\n".as_bytes(),
        Duration::ZERO,
    );
    let handle = server.handle("utf-8", 300);
    let mut session = session();
    unsafe {
        tgskk_session_set_skkserv(&mut *session, handle);
        tgskk_skkserv_free(handle);
        tgskk_session_set_zenz(&mut *session, ptr::null());
        tgskk_session_network_allowed(&mut *session, 1);
    }
    keys(&mut session, "T o u k y o u space");
    assert_eq!(preedit(&session), "▼東京");
    assert_eq!(session.candidates.len(), 2);
    assert_eq!(
        session.candidates[0].1.as_ref().unwrap().to_str().unwrap(),
        "首都"
    );
    assert_eq!(session.candidates[1].0.to_str().unwrap(), "DOS/V");
    unsafe {
        tgskk_session_set_skkserv(&mut *session, ptr::null());
    }
    assert_eq!(preedit(&session), "▼東京");
    assert_eq!(session.candidates.len(), 2);
}

#[test]
fn timeout_returns_to_registration_and_late_result_is_cached_without_ui_change() {
    let server = Server::new("1/東京/\n".as_bytes(), Duration::from_millis(100));
    let handle = server.handle("utf-8", 10);
    let mut session = session();
    unsafe {
        tgskk_session_set_skkserv(&mut *session, handle);
        tgskk_session_network_allowed(&mut *session, 1);
    }
    let start = Instant::now();
    keys(&mut session, "T o u k y o u space");
    assert!(start.elapsed() < Duration::from_millis(500));
    let before = preedit(&session).to_owned();
    assert!(session.candidates.is_empty());
    assert!(before.contains("【"), "expected registration, got {before}");
    let worker = unsafe { &*handle }.worker.clone();
    let deadline = Instant::now() + Duration::from_secs(2);
    while worker.try_cached("とうきょう").is_none() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(2));
    }
    assert!(worker.try_cached("とうきょう").is_some());
    assert_eq!(preedit(&session), before);
    unsafe {
        tgskk_session_reset(&mut *session);
        tgskk_session_network_allowed(&mut *session, 1);
    }
    keys(&mut session, "T o u k y o u space");
    assert_eq!(preedit(&session), "▼東京");
    assert_eq!(server.count.load(Ordering::SeqCst), 1);
    unsafe {
        tgskk_skkserv_free(handle);
    }
}

#[test]
fn custom_euc_jp_uses_skk_okuri_key_and_learns_confirmed_stem() {
    let server = Server::new(b"1/\xbd\xf1/\n", Duration::ZERO);
    let handle = server.handle("euc-jp", 300);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("user");
    std::fs::write(&path, ";; okuri-ari entries.\n;; okuri-nasi entries.\n").unwrap();
    #[allow(clippy::arc_with_non_send_sync)]
    let user =
        Arc::new(CskkDictionary::new_user_dict(path.to_str().unwrap(), "utf-8", true).unwrap());
    let input = InputSession::with_dictionaries(vec![user.clone()]).unwrap();
    let mut session = Box::new(TgskkSession::new(input).unwrap());
    unsafe {
        tgskk_session_set_skkserv(&mut *session, handle);
        tgskk_session_network_allowed(&mut *session, 1);
    }
    keys(&mut session, "K a K u");
    assert_eq!(preedit(&session), "▼書く");
    assert_eq!(
        server
            .requests
            .recv_timeout(Duration::from_secs(1))
            .unwrap(),
        b"1\xa4\xabk "
    );
    keys(&mut session, "Return");
    assert_eq!(session.committed.to_str().unwrap(), "書く");
    drop(session);
    let mut learned =
        Box::new(TgskkSession::new(InputSession::with_dictionaries(vec![user]).unwrap()).unwrap());
    keys(&mut learned, "K a K u");
    assert_eq!(preedit(&learned), "▼書く");
    unsafe {
        tgskk_skkserv_free(handle);
    }
}

#[test]
fn google_uses_full_kana_and_strips_only_exact_okuri() {
    let request = ConversionRequest {
        reading: "か".into(),
        okuri: Some("く".into()),
        candidates: vec![],
        protected_indices: vec![],
    };
    assert_eq!(
        lookup_reading(&request, Source::Google).as_deref(),
        Some("かく")
    );
    assert_eq!(
        lookup_reading(&request, Source::Custom).as_deref(),
        Some("かk")
    );
    let raw = ["書く", "書く", "欠ける", "く"]
        .into_iter()
        .map(|text| Candidate {
            text: text.into(),
            annotation: None,
        })
        .collect();
    let result = convert_candidates(raw, &request, Source::Google);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].text, "書");
    let mut abbreviation = request.clone();
    abbreviation.reading = "HTTP".into();
    assert!(lookup_reading(&abbreviation, Source::Google).is_none());
}

#[cfg(unix)]
#[test]
fn google_ffi_uses_managed_utf8_helper_and_confirms_okuri_once() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("helper.py");
    let request_log = directory.path().join("request");
    // The worker subprocess runs in this private directory; no external API.
    let source = executable.with_extension("source");
    std::fs::write(
        &source,
        r#"#!/usr/bin/env python3
import os, sys
assert sys.argv[1:] == ['--stdio', '--encoding=utf-8']
reading = bytearray()
assert sys.stdin.buffer.read(1) == b'1'
while True:
    byte = sys.stdin.buffer.read(1)
    if byte == b' ':
        break
    assert byte
    reading += byte
with open(os.path.join(os.path.dirname(__file__), 'request'), 'wb') as log:
    log.write(reading)
sys.stdout.buffer.write('1/書く/描く/欠ける/\n'.encode('utf-8'))
sys.stdout.buffer.flush()
sys.stdin.buffer.read()
"#,
    )
    .unwrap();
    // Keep the executable's writer out of other tests' forked processes.
    assert!(
        Command::new("cp")
            .arg(&source)
            .arg(&executable)
            .status()
            .unwrap()
            .success()
    );
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let executable = CString::new(executable.to_str().unwrap()).unwrap();
    let mut error = ptr::null_mut();
    let handle = unsafe { tgskk_skkserv_google_new(executable.as_ptr(), 1000, &mut error) };
    assert!(!handle.is_null());
    assert!(error.is_null());
    // The regular demo dictionary already has かk; use an empty local dictionary
    // so the assertion exercises the actual missing-candidate hook.
    let mut session =
        Box::new(TgskkSession::new(InputSession::with_dictionaries(vec![]).unwrap()).unwrap());
    unsafe {
        tgskk_session_set_skkserv(&mut *session, handle);
        tgskk_session_network_allowed(&mut *session, 1);
    }
    assert!(!request_log.exists());
    keys(&mut session, "K a K u");
    assert_eq!(std::fs::read(&request_log).unwrap(), "かく".as_bytes());
    assert_eq!(preedit(&session), "▼書く");
    assert_eq!(session.candidates.len(), 2);
    assert_eq!(session.candidates[0].0.to_str().unwrap(), "書");
    assert_eq!(session.candidates[1].0.to_str().unwrap(), "描");
    unsafe {
        tgskk_skkserv_free(handle);
    }
    keys(&mut session, "Return");
    assert_eq!(session.committed.to_str().unwrap(), "書く");
}

#[test]
fn abi_rejects_invalid_settings_and_nulls_without_network() {
    unsafe {
        for (port, encoding, wait) in [
            (0, c"utf-8", 300),
            (1178, c"bad", 300),
            (1178, c"utf-8", 1001),
        ] {
            let mut error = ptr::null_mut();
            assert!(
                tgskk_skkserv_custom_new(
                    c"localhost".as_ptr(),
                    port,
                    encoding.as_ptr(),
                    wait,
                    &mut error
                )
                .is_null()
            );
            assert!(!error.is_null());
            assert!(!CStr::from_ptr(error).to_bytes().is_empty());
            tgskk_string_free(error);
        }
        let mut error = ptr::null_mut();
        assert!(tgskk_skkserv_google_new(ptr::null(), 300, &mut error).is_null());
        tgskk_string_free(error);
        assert_eq!(tgskk_session_network_allowed(ptr::null_mut(), 1), 0);
        assert_eq!(tgskk_session_set_skkserv(ptr::null_mut(), ptr::null()), 0);
        assert!(tgskk_skkserv_take_error(ptr::null()).is_null());
        tgskk_skkserv_free(ptr::null_mut());
    }
}
