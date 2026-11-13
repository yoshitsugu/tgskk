#![cfg(windows)]
use std::time::{Duration, Instant};
use tgskk_skkserv::{Config, LookupError, Worker};
#[test]
#[ignore = "requires the Windows CMake child fixture"]
fn managed_stdio_helper_returns_utf8_candidates_on_windows() {
    let path = std::env::var_os("TGSKK_TEST_CHILD").expect("TGSKK_TEST_CHILD");
    let worker = Worker::start(Config::google(std::path::Path::new(&path)).unwrap()).unwrap();
    for (reading, expected, annotation) in [
        ("きしゃ", "汽車", None),
        ("かんじ", "漢字", Some("文字")),
        ("きしゃ", "汽車", None),
    ] {
        let deadline = Instant::now() + Duration::from_secs(1);
        let candidates = loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match worker.lookup(reading, remaining) {
                Ok(candidates) => break candidates,
                Err(LookupError::Busy) if !remaining.is_zero() => std::thread::yield_now(),
                Err(error) => panic!("Windows helper lookup failed: {error:?}"),
            }
        };
        assert_eq!(candidates[0].text, expected);
        assert_eq!(candidates[0].annotation.as_deref(), annotation);
    }
}
