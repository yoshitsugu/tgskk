#![cfg(windows)]
use std::time::Duration;
use tgskk_skkserv::{Config, Worker};
#[test]
#[ignore = "requires the Windows CMake child fixture"]
fn managed_stdio_helper_returns_utf8_candidates_on_windows() {
    let path = std::env::var_os("TGSKK_TEST_CHILD").expect("TGSKK_TEST_CHILD");
    let worker = Worker::start(Config::google(std::path::Path::new(&path)).unwrap()).unwrap();
    for _ in 0..2 {
        let candidates = worker.lookup("きしゃ", Duration::from_secs(1)).unwrap();
        assert_eq!(candidates[0].text, "汽車");
    }
}
