use std::{cell::Cell, rc::Rc, sync::Arc};
use tgskk_core::input::{
    CompositionMode, ConversionCandidate, ConversionError, ConversionRequest, ConversionResponse,
    CskkDictionary, CskkKeyEvent, InputMode, InputOutcome, InputSession, LearningPolicy,
};

fn session() -> InputSession {
    InputSession::open(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/SKK-JISYO.demo.utf8"
        ),
        "utf-8",
    )
    .unwrap()
}

fn key(session: &mut InputSession, name: &str) -> InputOutcome {
    session.process_key(&name.parse::<CskkKeyEvent>().unwrap())
}

fn type_keys(session: &mut InputSession, names: &str) -> InputOutcome {
    names
        .split_whitespace()
        .map(|name| key(session, name))
        .last()
        .unwrap()
}

fn ranked(_: &ConversionRequest) -> ConversionResponse {
    ConversionResponse {
        order: Some(vec![1, 0, 2]),
        learning: Some(LearningPolicy::ExplicitSelectionOnly),
        ..Default::default()
    }
}

#[test]
fn normal_skk_input_cancel_backspace_and_commit() {
    let mut input = session();
    assert_eq!(type_keys(&mut input, "k a").committed, "か");
    assert_eq!(type_keys(&mut input, "K a n j i space").preedit, "▼漢字");
    assert_eq!(key(&mut input, "C-g").preedit, "▽かんじ");
    assert_eq!(key(&mut input, "BackSpace").preedit, "▽かん");
    assert_eq!(key(&mut input, "C-g").preedit, "");
    assert_eq!(input.composition_mode(), CompositionMode::Direct);
    type_keys(&mut input, "K a n j i space");
    assert_eq!(key(&mut input, "Return").committed, "漢字");
    assert_eq!(input.composition_mode(), CompositionMode::Direct);
    assert!(!key(&mut input, "Return").consumed);
}

#[test]
fn internal_order_controls_forward_reverse_and_label_confirmation() {
    let mut input = session();
    input.set_conversion_provider(Some(Box::new(ranked)));
    assert_eq!(type_keys(&mut input, "K a n j i space").preedit, "▼感じ");
    assert_eq!(input.candidates()[0].annotation.as_deref(), Some("感覚"));
    assert_eq!(key(&mut input, "space").preedit, "▼漢字");
    assert_eq!(key(&mut input, "x").preedit, "▼感じ");
    assert_eq!(input.confirm_candidate(2).committed, "幹事");
    assert_eq!(input.composition_mode(), CompositionMode::Direct);
    assert!(!input.confirm_candidate(0).consumed);
}

#[test]
fn last_candidate_enters_registration_and_abort_restores_selection() {
    let mut input = session();
    input.set_conversion_provider(Some(Box::new(ranked)));
    type_keys(&mut input, "K a n j i space space space");
    assert_eq!(input.candidates()[2].text, "幹事");
    key(&mut input, "space");
    assert_eq!(input.composition_mode(), CompositionMode::Direct);
    assert!(key(&mut input, "C-g").preedit.contains("幹事"));
    assert_eq!(
        input.composition_mode(),
        CompositionMode::CompositionSelection
    );
}

#[test]
fn external_candidates_use_current_okuri_and_native_confirmation() {
    let mut input = session();
    input.set_conversion_provider(Some(Box::new(|request: &ConversionRequest| {
        assert_eq!(request.reading, "か");
        assert_eq!(request.okuri.as_deref(), Some("く"));
        ConversionResponse {
            additional_candidates: vec![ConversionCandidate {
                text: "欠".into(),
                annotation: Some("external".into()),
            }],
            order: Some(vec![2, 0, 1]),
            ..Default::default()
        }
    })));
    assert_eq!(type_keys(&mut input, "K a K u").preedit, "▼欠く");
    assert_eq!(key(&mut input, "space").preedit, "▼書く");
    assert_eq!(input.confirm_candidate(0).committed, "欠く");
}

#[test]
fn external_only_reading_does_not_enter_registration() {
    let mut input = session();
    input.set_conversion_provider(Some(Box::new(|request: &ConversionRequest| {
        assert!(request.candidates.is_empty());
        ConversionResponse {
            additional_candidates: vec![ConversionCandidate {
                text: "東京".into(),
                annotation: None,
            }],
            ..Default::default()
        }
    })));
    assert_eq!(
        type_keys(&mut input, "T o u k y o u space").preedit,
        "▼東京"
    );
    assert_eq!(key(&mut input, "Return").committed, "東京");
}

#[test]
fn duplicate_external_candidate_keeps_local_annotation_and_position() {
    let mut input = session();
    input.set_conversion_provider(Some(Box::new(|_: &ConversionRequest| ConversionResponse {
        additional_candidates: vec![ConversionCandidate {
            text: "漢字".into(),
            annotation: Some("external".into()),
        }],
        order: Some(vec![1, 0, 2]),
        ..Default::default()
    })));
    type_keys(&mut input, "K a n j i space");
    assert_eq!(input.candidates().len(), 3);
    assert_eq!(input.candidates()[1].annotation.as_deref(), Some("文字"));
}

#[test]
fn invalid_response_falls_back_atomically_and_reports_error() {
    for order in [vec![0], vec![0, 1, 2, 9], vec![0, 1, 1, 3]] {
        let mut input = session();
        input.set_conversion_provider(Some(Box::new(move |_: &ConversionRequest| {
            ConversionResponse {
                additional_candidates: vec![ConversionCandidate {
                    text: "監事".into(),
                    annotation: None,
                }],
                order: Some(order.clone()),
                ..Default::default()
            }
        })));
        let outcome = type_keys(&mut input, "K a n j i space");
        assert_eq!(
            outcome.conversion_error,
            Some(ConversionError::InvalidOrder)
        );
        assert_eq!(outcome.preedit, "▼漢字");
        assert_eq!(input.candidates().len(), 3);
        assert!(key(&mut input, "Return").conversion_error.is_none());
    }
}

#[test]
fn unsafe_external_dictionary_syntax_is_rejected() {
    for text in ["", "[注]", "(concat x)", "a\nb", "#0回", "a\\b"] {
        let mut input = session();
        input.set_conversion_provider(Some(Box::new(move |_: &ConversionRequest| {
            ConversionResponse {
                additional_candidates: vec![ConversionCandidate {
                    text: text.into(),
                    annotation: None,
                }],
                ..Default::default()
            }
        })));
        assert_eq!(
            type_keys(&mut input, "K a n j i space").conversion_error,
            Some(ConversionError::InvalidCandidate)
        );
        assert_eq!(key(&mut input, "Return").committed, "漢字");
    }
}

#[test]
fn provider_is_called_once_per_conversion_and_cannot_change_displayed_order() {
    let mut input = session();
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    input.set_conversion_provider(Some(Box::new(move |request: &ConversionRequest| {
        observed.set(observed.get() + 1);
        ranked(request)
    })));
    type_keys(&mut input, "K a n j i space space x");
    assert_eq!(calls.get(), 1);
    input.set_conversion_provider(None);
    assert_eq!(key(&mut input, "Return").committed, "感じ");
    assert_eq!(type_keys(&mut input, "K a n j i space").preedit, "▼漢字");
}

#[test]
fn mode_change_event_only_occurs_on_actual_mode_change() {
    let mut input = session();
    assert!(key(&mut input, "k").mode_change.is_none());
    key(&mut input, "C-g");
    let change = key(&mut input, "q").mode_change.unwrap();
    assert_eq!(change.from, InputMode::Hiragana);
    assert_eq!(change.to, InputMode::Katakana);
    assert_eq!(change.label(), "ア");
    let change = key(&mut input, "l").mode_change.unwrap();
    assert_eq!(change.label(), "A");
    assert!(key(&mut input, "a").mode_change.is_none());
    assert_eq!(key(&mut input, "C-j").mode_change.unwrap().label(), "あ");
    assert!(key(&mut input, "C-j").mode_change.is_none());
}

#[test]
fn reset_discards_unconfirmed_conversion_and_late_results() {
    let mut input = session();
    input.set_conversion_provider(Some(Box::new(ranked)));
    type_keys(&mut input, "K a n j i space");
    let _ = input.reset();
    assert_eq!(input.composition_mode(), CompositionMode::Direct);
    assert!(input.candidates().is_empty());
    assert_eq!(type_keys(&mut input, "k a").committed, "か");
}

#[allow(
    clippy::arc_with_non_send_sync,
    reason = "CSKK requires Arc dictionaries; each fixture stays on its input thread"
)]
fn learning_session() -> (tempfile::TempDir, Arc<CskkDictionary>, InputSession) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("user.dict");
    std::fs::write(&path, "").unwrap();
    let user =
        Arc::new(CskkDictionary::new_user_dict(path.to_str().unwrap(), "utf-8", false).unwrap());
    let local = Arc::new(
        CskkDictionary::new_static_dict(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/SKK-JISYO.demo.utf8"
            ),
            "utf-8",
            false,
        )
        .unwrap(),
    );
    let input = InputSession::with_dictionaries(vec![user.clone(), local]).unwrap();
    (dir, user, input)
}

#[test]
fn explicit_selection_policy_distinguishes_manual_and_implicit_confirmation() {
    for (action, committed, learned) in [
        ("Return", "感じ", "漢字"),
        ("C-j", "感じ", "漢字"),
        ("a", "感じあ", "漢字"),
        ("space Return", "幹事", "幹事"),
        ("space x Return", "感じ", "感じ"),
        ("label0", "感じ", "感じ"),
        ("label1", "幹事", "幹事"),
    ] {
        let (_dir, _user, mut input) = learning_session();
        input.set_conversion_provider(Some(Box::new(|_: &ConversionRequest| ConversionResponse {
            order: Some(vec![1, 2, 0]),
            learning: Some(LearningPolicy::ExplicitSelectionOnly),
            ..Default::default()
        })));
        type_keys(&mut input, "K a n j i space");
        let outcome = match action {
            "label0" => input.confirm_candidate(0),
            "label1" => input.confirm_candidate(1),
            _ => type_keys(&mut input, action),
        };
        assert_eq!(outcome.committed, committed, "{action}");
        let _ = input.reset();
        input.set_conversion_provider(None);
        assert_eq!(
            type_keys(&mut input, "K a n j i space").preedit,
            format!("▼{learned}"),
            "{action}"
        );
    }
}

#[test]
fn default_learning_policy_preserves_native_learning_but_not_provider_choices() {
    for (response, committed, learned) in [
        (ConversionResponse::default(), "漢字", true),
        (
            ConversionResponse {
                order: Some(vec![0, 1, 2]),
                ..Default::default()
            },
            "漢字",
            true,
        ),
        (
            ConversionResponse {
                order: Some(vec![0, 2, 1]),
                ..Default::default()
            },
            "漢字",
            true,
        ),
        (
            ConversionResponse {
                additional_candidates: vec![ConversionCandidate {
                    text: "漢字".into(),
                    annotation: None,
                }],
                ..Default::default()
            },
            "漢字",
            true,
        ),
        (
            ConversionResponse {
                learning: Some(LearningPolicy::ExplicitSelectionOnly),
                ..Default::default()
            },
            "漢字",
            false,
        ),
        (
            ConversionResponse {
                order: Some(vec![1, 0, 2]),
                ..Default::default()
            },
            "感じ",
            false,
        ),
        (
            ConversionResponse {
                order: Some(vec![1, 0, 2]),
                learning: Some(LearningPolicy::Always),
                ..Default::default()
            },
            "感じ",
            true,
        ),
        (
            ConversionResponse {
                additional_candidates: vec![ConversionCandidate {
                    text: "監事".into(),
                    annotation: None,
                }],
                ..Default::default()
            },
            "漢字",
            true,
        ),
    ] {
        let (_dir, user, mut input) = learning_session();
        input.set_conversion_provider(Some(Box::new(move |_: &ConversionRequest| {
            response.clone()
        })));
        type_keys(&mut input, "K a n j i space");
        assert_eq!(key(&mut input, "Return").committed, committed);
        let mut user_only = InputSession::with_dictionaries(vec![user]).unwrap();
        type_keys(&mut user_only, "K a n j i space");
        let candidates = user_only.candidates();
        if learned {
            assert_eq!(candidates[0].text, committed);
        } else {
            assert!(candidates.is_empty());
        }
    }
}

#[test]
fn abort_after_kana_confirmation_never_revives_completed_composition() {
    for (prefix, mode, subsequent, expected) in [
        ("K a n j i C-j", InputMode::Hiragana, "k a", "か"),
        ("K a n j i C-j l", InputMode::Ascii, "a", "a"),
        ("K a q L", InputMode::Zenkaku, "a", "ａ"),
        ("K a C-q", InputMode::Hiragana, "k a", "か"),
    ] {
        let mut input = session();
        type_keys(&mut input, prefix);
        for _ in 0..3 {
            let outcome = key(&mut input, "C-g");
            assert!(outcome.preedit.is_empty(), "{prefix}: {}", outcome.preedit);
            assert!(outcome.committed.is_empty());
            assert_eq!(
                input.composition_mode(),
                CompositionMode::Direct,
                "{prefix}"
            );
            assert_eq!(input.input_mode(), mode, "{prefix}");
        }
        assert_eq!(
            type_keys(&mut input, subsequent).committed,
            expected,
            "{prefix}"
        );
    }
}

#[test]
fn abort_after_purge_never_revives_empty_candidate_selection() {
    let mut input = session();
    type_keys(&mut input, "K a n j i space X");
    assert_eq!(input.composition_mode(), CompositionMode::Direct);
    let outcome = key(&mut input, "C-g");
    assert!(outcome.preedit.is_empty());
    assert_eq!(input.composition_mode(), CompositionMode::Direct);
    assert_eq!(key(&mut input, "space").committed, " ");
    assert_eq!(type_keys(&mut input, "k a").committed, "か");
}

#[test]
fn empty_reading_never_queries_provider_or_enters_candidate_selection() {
    for (start, expected_mode) in [
        ("Q", CompositionMode::PreComposition),
        ("slash", CompositionMode::Abbreviation),
    ] {
        let mut input = session();
        let calls = Rc::new(Cell::new(0));
        let observed = calls.clone();
        input.set_conversion_provider(Some(Box::new(move |_: &ConversionRequest| {
            observed.set(observed.get() + 1);
            ConversionResponse {
                additional_candidates: vec![ConversionCandidate {
                    text: "候補".into(),
                    annotation: None,
                }],
                ..Default::default()
            }
        })));
        key(&mut input, start);
        for _ in 0..2 {
            let outcome = key(&mut input, "space");
            assert!(outcome.committed.is_empty());
            assert_eq!(input.composition_mode(), expected_mode);
            assert!(input.candidates().is_empty());
        }
        assert_eq!(calls.get(), 0);
    }
}

fn session_with_dictionary(source: &str) -> (tempfile::TempDir, InputSession) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("local.dict");
    std::fs::write(&path, source).unwrap();
    let input = InputSession::open(path.to_str().unwrap(), "utf-8").unwrap();
    (dir, input)
}

#[test]
fn local_duplicates_keep_their_indices_and_annotations() {
    let (_dir, mut input) =
        session_with_dictionary(";; okuri-nasi entries.\nかんじ /漢字;文字/漢字/感じ/\n");
    input.set_conversion_provider(Some(Box::new(|request: &ConversionRequest| {
        assert_eq!(
            request
                .candidates
                .iter()
                .map(|c| c.text.as_str())
                .collect::<Vec<_>>(),
            ["漢字", "漢字", "感じ"]
        );
        ConversionResponse {
            additional_candidates: vec![
                ConversionCandidate {
                    text: "漢字".into(),
                    annotation: None,
                },
                ConversionCandidate {
                    text: "監事".into(),
                    annotation: None,
                },
            ],
            order: Some(vec![3, 1, 2, 0]),
            ..Default::default()
        }
    })));
    let outcome = type_keys(&mut input, "K a n j i space");
    assert!(outcome.conversion_error.is_none());
    let candidates = input.candidates();
    assert_eq!(candidates.len(), 4);
    assert_eq!(candidates[0].text, "監事");
    assert_eq!(candidates[1].annotation, None);
    assert_eq!(candidates[3].annotation.as_deref(), Some("文字"));
    assert_eq!(key(&mut input, "Return").committed, "監事");
}

#[test]
fn numeric_candidates_keep_distinct_dictionary_indices_with_the_same_output() {
    let (_dir, mut input) =
        session_with_dictionary(";; okuri-nasi entries.\n1かい /1回/\n#かい /#0回/#1回/\n");
    input.set_conversion_provider(Some(Box::new(|request: &ConversionRequest| {
        assert_eq!(
            request
                .candidates
                .iter()
                .map(|c| c.text.as_str())
                .collect::<Vec<_>>(),
            ["1回", "1回", "１回"]
        );
        ConversionResponse {
            order: Some(vec![2, 1, 0]),
            ..Default::default()
        }
    })));
    assert_eq!(type_keys(&mut input, "Q 1 k a i space").preedit, "▼１回");
    assert_eq!(key(&mut input, "Return").committed, "１回");
}

#[test]
fn annotations_that_cannot_round_trip_are_rejected_atomically() {
    for annotation in ["", "[note]", "a\nb", "a\0b", r"a\b;c", r#"(concat "zz")"#] {
        let mut input = session();
        input.set_conversion_provider(Some(Box::new(move |_: &ConversionRequest| {
            ConversionResponse {
                additional_candidates: vec![ConversionCandidate {
                    text: "監事".into(),
                    annotation: Some(annotation.into()),
                }],
                order: Some(vec![3, 0, 1, 2]),
                ..Default::default()
            }
        })));
        let outcome = type_keys(&mut input, "K a n j i space");
        assert_eq!(
            outcome.conversion_error,
            Some(ConversionError::InvalidCandidate),
            "{annotation:?}"
        );
        assert_eq!(outcome.preedit, "▼漢字");
        assert_eq!(input.candidates().len(), 3);
    }
}

#[test]
fn external_punctuation_and_annotations_survive_dictionary_save_and_reload() {
    for text in ["km/h", "1/2", "(笑)", "C++(言語)", "a;note", r#"p"q;r"#] {
        let (dir, user, mut input) = learning_session();
        let annotation = r#"p"q;r/s"#;
        input.set_conversion_provider(Some(Box::new(move |_: &ConversionRequest| {
            ConversionResponse {
                additional_candidates: vec![ConversionCandidate {
                    text: text.into(),
                    annotation: Some(annotation.into()),
                }],
                order: Some(vec![3, 0, 1, 2]),
                learning: Some(LearningPolicy::Always),
            }
        })));
        let outcome = type_keys(&mut input, "K a n j i space");
        assert!(outcome.conversion_error.is_none(), "{text:?}");
        assert_eq!(key(&mut input, "Return").committed, text);
        let mut writer = cskk::CskkContext::new_with_bundled_rule(vec![user]).unwrap();
        writer.save_dictionary();
        let path = dir.path().join("user.dict");
        let mut reloaded = InputSession::open(path.to_str().unwrap(), "utf-8").unwrap();
        type_keys(&mut reloaded, "K a n j i space");
        let candidates = reloaded.candidates();
        assert_eq!(candidates[0].text, text);
        assert_eq!(candidates[0].annotation.as_deref(), Some(annotation));
        assert_eq!(key(&mut reloaded, "Return").committed, text);
    }
}

#[test]
fn reset_reports_mode_change_when_registration_restores_the_outer_mode() {
    let mut input = session();
    key(&mut input, "q");
    let entered = type_keys(&mut input, "T o u k y o u space");
    assert_eq!(entered.mode_change.unwrap().to, InputMode::Hiragana);
    let change = input.reset().unwrap();
    assert_eq!(change.from, InputMode::Hiragana);
    assert_eq!(change.to, InputMode::Katakana);
    let outcome = key(&mut input, "a");
    assert_eq!(outcome.committed, "ア");
    assert!(outcome.mode_change.is_none());
    assert!(input.reset().is_none());
}

#[test]
fn conversion_preview_does_not_select_learn_or_call_the_provider() {
    let mut input = session();
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    input.set_conversion_provider(Some(Box::new(move |_: &ConversionRequest| {
        seen.set(seen.get() + 1);
        ConversionResponse::default()
    })));
    assert!(input.preview_conversion().is_none());
    type_keys(&mut input, "K a n j i");
    for _ in 0..2 {
        let preview = input.preview_conversion().unwrap();
        assert_eq!(preview.reading, "かんじ");
        assert_eq!(preview.candidates[0].text, "漢字");
        assert!(preview.protected_indices.is_empty());
        assert_eq!(input.composition_mode(), CompositionMode::PreComposition);
    }
    assert_eq!(calls.get(), 0);
    assert_eq!(key(&mut input, "space").preedit, "▼漢字");
    assert_eq!(calls.get(), 1);
    assert!(input.preview_conversion().is_none());
}

#[test]
#[allow(clippy::arc_with_non_send_sync)]
fn preview_protects_custom_entries_but_allows_context_to_override_common_learning() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("user.utf8");
    std::fs::write(
        &path,
        ";; okuri-ari entries.\n;; okuri-nasi entries.\nかんじ /独自登録/感じ;自分の注釈/\n",
    )
    .unwrap();
    let user = CskkDictionary::new_user_dict(path.to_str().unwrap(), "utf-8", true).unwrap();
    let system = CskkDictionary::new_static_dict(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/SKK-JISYO.demo.utf8"
        ),
        "utf-8",
        true,
    )
    .unwrap();
    let mut input =
        InputSession::with_dictionaries(vec![Arc::new(user), Arc::new(system)]).unwrap();
    type_keys(&mut input, "K a n j i");
    let request = input.preview_conversion().unwrap();
    assert_eq!(request.candidates[0].text, "独自登録");
    assert_eq!(request.candidates[1].text, "感じ");
    assert_eq!(request.protected_indices, vec![0]);
    key(&mut input, "space");
    let _ = input.reset();
    type_keys(&mut input, "K a n j i");
    assert_eq!(
        input.preview_conversion().unwrap().protected_indices,
        vec![0]
    );
}

#[test]
fn preview_flushes_pending_n_without_changing_the_preedit() {
    let mut input = InputSession::open(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/SKK-JISYO.zenz.utf8"
        ),
        "utf-8",
    )
    .unwrap();
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    input.set_conversion_provider(Some(Box::new(move |request: &ConversionRequest| {
        assert_eq!(request.reading, "こうえん");
        observed.set(observed.get() + 1);
        ConversionResponse {
            order: Some(vec![1, 0, 2, 3]),
            ..Default::default()
        }
    })));
    assert_eq!(type_keys(&mut input, "K o u e n").preedit, "▽こうえn");
    let preview = input.preview_conversion().unwrap();
    assert_eq!(preview.reading, "こうえん");
    assert_eq!(preview.candidates[1].text, "講演");
    assert_eq!(calls.get(), 0);
    assert_eq!(key(&mut input, "space").preedit, "▼講演");
    assert_eq!(calls.get(), 1);
}

#[test]
#[allow(clippy::arc_with_non_send_sync)]
fn bounded_preview_preserves_identity_and_does_not_truncate_actual_conversion() {
    let directory = tempfile::tempdir().unwrap();
    let system_path = directory.path().join("system.utf8");
    let user_path = directory.path().join("user.utf8");
    let candidates: String = (0..32).map(|i| format!("候補{i}/")).collect();
    std::fs::write(
        &system_path,
        format!(";; okuri-ari entries.\n;; okuri-nasi entries.\nてすと /{candidates}\n"),
    )
    .unwrap();
    std::fs::write(
        &user_path,
        ";; okuri-ari entries.\n;; okuri-nasi entries.\nてすと /独自/候補1/\n",
    )
    .unwrap();
    let system =
        CskkDictionary::new_static_dict(system_path.to_str().unwrap(), "utf-8", true).unwrap();
    let user = CskkDictionary::new_user_dict(user_path.to_str().unwrap(), "utf-8", true).unwrap();
    let mut input =
        InputSession::with_dictionaries(vec![Arc::new(user), Arc::new(system)]).unwrap();
    type_keys(&mut input, "T e s u t o");
    let all = input.preview_conversion().unwrap();
    assert_eq!(all.candidates.len(), 33);
    let head = input.preview_conversion_limited(8).unwrap();
    assert_eq!(head.candidates, all.candidates[..8]);
    assert_eq!(head.protected_indices, vec![0]);
    assert!(
        input
            .preview_conversion_limited(0)
            .unwrap()
            .candidates
            .is_empty()
    );
    input.set_conversion_provider(Some(Box::new(|request: &ConversionRequest| {
        assert_eq!(request.candidates.len(), 33);
        assert_eq!(request.protected_indices, vec![0]);
        ConversionResponse::default()
    })));
    assert_eq!(key(&mut input, "space").preedit, "▼独自");
    assert_eq!(input.candidates().len(), 33);
}
