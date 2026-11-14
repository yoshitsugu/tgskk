use tgskk_core::{Dictionary, Engine, UpdateOutcome};

fn engine() -> Engine {
    Engine::new(
        Dictionary::parse_utf8(include_str!("../../../examples/SKK-JISYO.demo.utf8")).unwrap(),
        2,
    )
}

#[test]
fn parses_annotations_okuri_keys_and_merges_duplicate_readings() {
    let dictionary =
        Dictionary::parse_utf8(";; header\r\n\nかk /書;write/描/書/\r\nかk /欠/\n").unwrap();
    let candidates = dictionary.lookup("かk");
    assert_eq!(
        candidates
            .iter()
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>(),
        ["書", "描", "欠"]
    );
    assert_eq!(candidates[0].annotation.as_deref(), Some("write"));
    assert!(dictionary.lookup("missing").is_empty());
}

#[test]
fn rejects_malformed_and_unsupported_dictionary_entries() {
    for entry in [
        "bad",
        "bad 候補",
        "bad //",
        "bad /;note/",
        "bad /a//",
        "bad /[く/書/]/",
        "bad /(concat x)/",
    ] {
        let error = Dictionary::parse_utf8(&format!(";; header\n{entry}"))
            .err()
            .unwrap();
        assert_eq!(error.line, 2, "{entry}");
    }
}

#[test]
fn ranking_changes_order_without_changing_candidates_or_annotations() {
    let mut engine = engine();
    let request = engine.begin("かんじ", "文字を入力するための");
    assert_eq!(request.left_context(), "文字を入力するための");
    assert_eq!(request.reading(), "かんじ");
    assert_eq!(engine.candidates()[0].text, "漢字");
    assert_eq!(
        engine.apply_ranking(request.ranked(vec![1, 2, 0])),
        UpdateOutcome::Applied
    );
    assert_eq!(
        engine
            .candidates()
            .iter()
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>(),
        ["感じ", "幹事", "漢字"]
    );
    assert_eq!(engine.commit().unwrap().annotation.as_deref(), Some("感覚"));
    assert_eq!(engine.candidates(), []);
}

#[test]
fn rejects_old_results_even_for_the_same_reading() {
    let mut engine = engine();
    let old = engine.begin("かんじ", "old context");
    engine.begin("かんじ", "new context");
    assert_eq!(
        engine.apply_ranking(old.ranked(vec![2, 1, 0])),
        UpdateOutcome::Stale
    );
    assert_eq!(engine.candidates()[0].text, "漢字");
}

#[test]
fn selection_and_first_ranking_freeze_order() {
    let mut engine = engine();
    let request = engine.begin("かんじ", "");
    assert_eq!(engine.select(1).unwrap().text, "感じ");
    assert_eq!(
        engine.apply_ranking(request.ranked(vec![2, 0, 1])),
        UpdateOutcome::Frozen
    );
    assert_eq!(engine.commit().unwrap().text, "感じ");
    let request = engine.begin("かんじ", "");
    assert_eq!(
        engine.apply_ranking(request.clone().ranked(vec![2, 0, 1])),
        UpdateOutcome::Applied
    );
    assert_eq!(
        engine.apply_ranking(request.ranked(vec![1, 0, 2])),
        UpdateOutcome::Frozen
    );
}

#[test]
fn invalid_orders_leave_session_usable() {
    let mut engine = engine();
    let request = engine.begin("かんじ", "");
    for order in [
        vec![0, 0, 2],
        vec![0, 1],
        vec![0, 1, 3],
        vec![0, 1, usize::MAX],
    ] {
        assert_eq!(
            engine.apply_ranking(request.clone().ranked(order)),
            UpdateOutcome::InvalidOrder
        );
        assert_eq!(engine.candidates()[0].text, "漢字");
    }
    assert!(engine.select(100).is_none());
    assert_eq!(
        engine.apply_ranking(request.ranked(vec![2, 1, 0])),
        UpdateOutcome::Applied
    );
}

#[test]
fn cancellation_commit_and_dictionary_reload_invalidate_results() {
    let mut engine = engine();
    let request = engine.begin("かんじ", "");
    engine.cancel();
    assert_eq!(
        engine.apply_ranking(request.ranked(vec![2, 1, 0])),
        UpdateOutcome::Stale
    );
    assert!(engine.commit().is_none());
    let request = engine.begin("かんじ", "");
    engine.commit();
    assert_eq!(
        engine.apply_ranking(request.ranked(vec![2, 1, 0])),
        UpdateOutcome::Stale
    );
    let request = engine.begin("かんじ", "");
    engine.replace_dictionary(Dictionary::parse_utf8("かんじ /新候補/").unwrap());
    assert_eq!(
        engine.apply_ranking(request.ranked(vec![2, 1, 0])),
        UpdateOutcome::Stale
    );
    assert_eq!(engine.lookup("かんじ")[0].text, "新候補");
}

#[test]
fn empty_lookup_and_disabled_cache_work() {
    let mut engine = Engine::new(Dictionary::default(), 0);
    engine.begin("unknown", "");
    assert_eq!(engine.candidates(), []);
    assert!(engine.commit().is_none());
    assert!(engine.lookup("unknown").is_empty());
}

#[test]
fn requests_from_other_engines_are_rejected() {
    let mut first = engine();
    let mut second = engine();
    let request = first.begin("かんじ", "");
    second.begin("かんじ", "");
    assert_eq!(
        second.apply_ranking(request.ranked(vec![2, 1, 0])),
        UpdateOutcome::Stale
    );
}

#[test]
fn presentation_can_freeze_order_before_any_selection() {
    let mut engine = engine();
    let request = engine.begin("かんじ", "");
    engine.freeze();
    assert_eq!(
        engine.apply_ranking(request.ranked(vec![2, 1, 0])),
        UpdateOutcome::Frozen
    );
    assert_eq!(engine.commit().unwrap().text, "漢字");
    engine.cancel();
    engine.freeze();
    let request = engine.begin("かんじ", "");
    assert_eq!(
        engine.apply_ranking(request.ranked(vec![1, 0, 2])),
        UpdateOutcome::Applied
    );
}
