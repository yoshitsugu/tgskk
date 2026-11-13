use std::sync::Arc;

use cskk::CskkContext;

pub use cskk::{
    dictionary::{CskkDictionary, DictionaryUpdates, SaveVersion},
    error::CskkError,
};

pub use cskk::conversion::{
    ConversionCandidate, ConversionError, ConversionProvider, ConversionRequest,
    ConversionResponse, LearningPolicy, MissingConversionProvider, MissingConversionResponse,
};
pub use cskk::keyevent::CskkKeyEvent;
pub use cskk::skk_modes::{CompositionMode, InputMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModeChange {
    pub from: InputMode,
    pub to: InputMode,
}

impl ModeChange {
    pub fn label(&self) -> &'static str {
        mode_label(self.to)
    }
}

fn mode_label(mode: InputMode) -> &'static str {
    match mode {
        InputMode::Hiragana => "あ",
        InputMode::Katakana => "ア",
        InputMode::HankakuKatakana => "ｱ",
        InputMode::Zenkaku => "Ａ",
        InputMode::Ascii => "A",
    }
}

#[derive(Debug)]
pub struct InputOutcome {
    pub consumed: bool,
    pub committed: String,
    pub preedit: String,
    pub mode_change: Option<ModeChange>,
    pub conversion_error: Option<ConversionError>,
}

pub struct InputSession {
    context: CskkContext,
}

impl InputSession {
    #[allow(
        clippy::arc_with_non_send_sync,
        reason = "CSKK requires Arc dictionaries; the session remains on the input thread"
    )]
    pub fn open(dictionary_path: &str, encoding: &str) -> Result<Self, CskkError> {
        let dictionary = CskkDictionary::new_static_dict(dictionary_path, encoding, true)?;
        Self::with_dictionaries(vec![Arc::new(dictionary)])
    }

    pub fn with_dictionaries(dictionaries: Vec<Arc<CskkDictionary>>) -> Result<Self, CskkError> {
        Ok(Self {
            context: CskkContext::new_with_bundled_rule(dictionaries)?,
        })
    }

    pub fn set_conversion_provider(&mut self, provider: Option<Box<dyn ConversionProvider>>) {
        self.context.set_conversion_provider(provider);
    }

    pub fn set_missing_conversion_provider(
        &mut self,
        provider: Option<Box<dyn MissingConversionProvider>>,
    ) {
        self.context.set_missing_conversion_provider(provider);
    }

    pub fn preview_conversion(&self) -> Option<ConversionRequest> {
        self.context.preview_conversion()
    }

    pub fn preview_conversion_limited(&self, limit: usize) -> Option<ConversionRequest> {
        self.context.preview_conversion_limited(limit)
    }

    pub fn may_process(&self, event: &CskkKeyEvent) -> bool {
        self.context.will_process(event)
    }

    pub fn process_key(&mut self, event: &CskkKeyEvent) -> InputOutcome {
        let before = self.context.get_current_input_mode();
        let consumed = self.context.process_key_event(event);
        self.outcome(before, consumed)
    }

    pub fn confirm_candidate(&mut self, index: usize) -> InputOutcome {
        let before = self.context.get_current_input_mode();
        let consumed = self.is_selecting()
            && cskk::skk_context_confirm_candidate_at_rs(&mut self.context, index);
        self.outcome(before, consumed)
    }

    pub fn select_candidate(&mut self, index: usize) -> InputOutcome {
        let before = self.context.get_current_input_mode();
        let consumed = self.is_selecting()
            && index < cskk::skk_context_get_current_candidate_count_rs(&self.context)
            && i32::try_from(index).is_ok_and(|index| {
                cskk::skk_context_select_candidate_at_rs(&mut self.context, index)
            });
        self.outcome(before, consumed)
    }

    fn outcome(&mut self, before: InputMode, consumed: bool) -> InputOutcome {
        let mode_change = self.mode_change_from(before);
        InputOutcome {
            consumed,
            committed: self.context.poll_output().unwrap_or_default(),
            preedit: self.context.get_preedit().unwrap_or_default(),
            mode_change,
            conversion_error: self.context.take_conversion_error(),
        }
    }

    pub fn composition_mode(&self) -> CompositionMode {
        self.context.get_current_composition_mode()
    }

    pub fn input_mode(&self) -> InputMode {
        self.context.get_current_input_mode()
    }
    pub fn set_input_mode(&mut self, mode: InputMode) -> InputOutcome {
        let before = self.input_mode();
        cskk::skk_context_reset_rs(&mut self.context);
        cskk::skk_context_set_input_mode_rs(&mut self.context, mode);
        self.outcome(before, false)
    }

    pub fn mode_label(&self) -> &'static str {
        mode_label(self.input_mode())
    }

    fn is_selecting(&self) -> bool {
        matches!(
            self.composition_mode(),
            CompositionMode::CompositionSelection | CompositionMode::Completion
        )
    }

    pub fn candidate_cursor(&self) -> Option<usize> {
        self.is_selecting()
            .then(|| cskk::skk_context_get_current_candidate_cursor_position_rs(&self.context).ok())
            .flatten()
    }

    pub fn candidates(&self) -> Vec<ConversionCandidate> {
        cskk::skk_context_get_current_candidates_rs(&self.context)
            .iter()
            .map(|candidate| ConversionCandidate {
                text: candidate.text().to_owned(),
                annotation: candidate.annotation().map(str::to_owned),
            })
            .collect()
    }

    fn mode_change_from(&self, before: InputMode) -> Option<ModeChange> {
        let after = self.context.get_current_input_mode();
        (before != after).then_some(ModeChange {
            from: before,
            to: after,
        })
    }

    #[must_use = "update the frontend mode indicator if reset restores another input mode"]
    pub fn reset(&mut self) -> Option<ModeChange> {
        let before = self.context.get_current_input_mode();
        cskk::skk_context_reset_rs(&mut self.context);
        self.mode_change_from(before)
    }
}

#[cfg(test)]
mod eligibility_tests {
    use super::*;
    #[test]
    fn ineligible_keys_are_never_consumed_across_modes_and_compositions() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/SKK-JISYO.demo.utf8");
        let prefixes: &[&[(u32, u32)]] = &[
            &[],
            &[(b'l' as u32, 0)],
            &[(b'L' as u32, 1)],
            &[(b'q' as u32, 0)],
            &[(b'q' as u32, 4)],
            &[(b'k' as u32, 0)],
            &[(b'K' as u32, 1), (b'a' as u32, 0)],
            &[
                (b'K' as u32, 1),
                (b'i' as u32, 0),
                (b's' as u32, 0),
                (b'h' as u32, 0),
                (b'a' as u32, 0),
                (b' ' as u32, 0),
            ],
            &[(b'M' as u32, 1), (b'i' as u32, 0), (b' ' as u32, 0)],
        ];
        for prefix in prefixes {
            let mut input = InputSession::open(path.to_str().unwrap(), "utf-8").unwrap();
            for modifiers in [0, 1, 4, 5, 8] {
                for symbol in (0x20..=0x7e).chain([
                    0xff08, 0xff09, 0xff0d, 0xff1b, 0xff51, 0xff52, 0xff53, 0xff54, 0xff55, 0xff56,
                ]) {
                    let _ = input.reset();
                    input.process_key(&CskkKeyEvent::from_fcitx_keyevent(b'j' as u32, 4, false));
                    for &(key, mods) in *prefix {
                        input.process_key(&CskkKeyEvent::from_fcitx_keyevent(key, mods, false));
                    }
                    let key = CskkKeyEvent::from_fcitx_keyevent(symbol, modifiers, false);
                    let eligible = input.may_process(&key);
                    let outcome = input.process_key(&key);
                    assert!(
                        eligible || !outcome.consumed,
                        "ineligible key was consumed: prefix={prefix:?} symbol={symbol:#x} modifiers={modifiers}"
                    );
                }
            }
        }
    }
    #[test]
    fn uppercase_start_and_repeated_queries_preserve_input_state() {
        for modifiers in [0, 1] {
            let mut checked = InputSession::with_dictionaries(vec![]).unwrap();
            let mut direct = InputSession::with_dictionaries(vec![]).unwrap();
            for symbol in "KAnji1,.!".chars() {
                let key = CskkKeyEvent::from_fcitx_keyevent(symbol as u32, modifiers, false);
                for _ in 0..3 {
                    assert!(checked.may_process(&key));
                }
                let actual = checked.process_key(&key);
                let expected = direct.process_key(&key);
                assert_eq!(actual.preedit, expected.preedit);
                assert_eq!(actual.committed, expected.committed);
                assert_eq!(actual.consumed, expected.consumed);
            }
        }
    }
}
