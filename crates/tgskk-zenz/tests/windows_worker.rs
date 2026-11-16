#![cfg(windows)]
use std::path::PathBuf;
use tgskk_zenz::Evaluator;

#[test]
#[ignore = "requires the native Windows worker and verified model"]
fn managed_windows_worker_ranks_and_recovers_from_a_refused_request() {
    let executable =
        PathBuf::from(std::env::var_os("TGSKK_TEST_WORKER").expect("TGSKK_TEST_WORKER"));
    let model = PathBuf::from(std::env::var_os("TGSKK_TEST_MODEL").expect("TGSKK_TEST_MODEL"));
    let mut evaluator = Evaluator::open(&executable, &model).unwrap();
    let candidates = vec!["記者".into(), "汽車".into()];
    let scores = evaluator.score("駅で", "", "キシャ", &candidates).unwrap();
    assert_eq!(scores.len(), 2);
    assert!(scores[1].0 > scores[0].0);
    let scores = evaluator
        .score("新聞の", "", "キシャ", &candidates)
        .unwrap();
    assert!(scores[0].0 > scores[1].0);
    assert_eq!(
        evaluator
            .score(&"あ".repeat(241), "", "キシャ", &candidates)
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::InvalidInput
    );
    let scores = evaluator.score("駅で", "", "キシャ", &candidates).unwrap();
    assert!(scores[1].0 > scores[0].0);
}
