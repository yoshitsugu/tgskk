#![cfg(windows)]
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn executable() -> PathBuf {
    std::env::var_os("TGSKK_TEST_CHILD")
        .expect("build the CMake child fixture and set TGSKK_TEST_CHILD")
        .into()
}
#[test]
#[ignore = "requires the Windows CMake child fixture"]
fn private_stdio_round_trip_and_partial_eof() {
    let (mut child, mut pipe) = tgskk_process::spawn(&executable(), ["echo"]).unwrap();
    pipe.write_all(b"hello").unwrap();
    let mut bytes = [0; 5];
    pipe.read_exact(&mut bytes).unwrap();
    assert_eq!(&bytes, b"hello");
    assert!(child.wait().unwrap().success());
    drop(pipe);
    let (mut child, mut pipe) = tgskk_process::spawn(&executable(), ["partial"]).unwrap();
    assert_eq!(
        pipe.read_exact(&mut [0; 4]).unwrap_err().kind(),
        std::io::ErrorKind::UnexpectedEof
    );
    assert_eq!(pipe.read(&mut [0; 1]).unwrap(), 0);
    assert!(child.wait().unwrap().success());
}
#[test]
#[ignore = "requires the Windows CMake child fixture"]
fn stalled_reads_and_writes_are_bounded_and_child_is_reaped() {
    for mode in ["stall", "write-stall"] {
        let (mut child, mut pipe) = tgskk_process::spawn(&executable(), [mode]).unwrap();
        if mode == "write-stall" {
            pipe.read_exact(&mut [0; 5]).unwrap();
        }
        pipe.set_read_timeout(Some(Duration::from_millis(30)))
            .unwrap();
        pipe.set_write_timeout(Some(Duration::from_millis(30)))
            .unwrap();
        let start = Instant::now();
        let error = if mode == "stall" {
            pipe.read_exact(&mut [0; 1]).unwrap_err()
        } else {
            pipe.write_all(&vec![0; 256 * 1024]).unwrap_err()
        };
        assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
        assert!(start.elapsed() < Duration::from_secs(1));
        assert_eq!(
            pipe.write_all(b"next").unwrap_err().kind(),
            std::io::ErrorKind::BrokenPipe
        );
        assert_eq!(
            pipe.read(&mut [0; 1]).unwrap_err().kind(),
            std::io::ErrorKind::BrokenPipe
        );
        child.kill().unwrap();
        child.wait().unwrap();
        drop(pipe);
        assert!(start.elapsed() < Duration::from_secs(2));
    }
}
