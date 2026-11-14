use crate::dictionary::{candidate::Candidate, get_all_candidates, CompositeKey, CskkDictionary};
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversionCandidate {
    pub text: String,
    pub annotation: Option<String>,
}

impl ConversionCandidate {
    /// Validates a candidate supplied by an external source before it is merged
    /// into the dictionary candidate list.
    pub fn validate_external(&self) -> Result<(), ConversionError> {
        if self.text.is_empty()
            || unsupported_literal(&self.text)
            || self.text.contains('#')
            || self
                .annotation
                .as_ref()
                .is_some_and(|annotation| annotation.is_empty() || unsupported_literal(annotation))
        {
            return Err(ConversionError::InvalidCandidate);
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ConversionRequest {
    pub reading: String,
    pub okuri: Option<String>,
    pub candidates: Vec<ConversionCandidate>,
    /// Candidates only found in writable dictionaries retain their positions.
    pub protected_indices: Vec<usize>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LearningPolicy {
    #[default]
    Always,
    ExplicitSelectionOnly,
}

#[derive(Clone, Debug, Default)]
pub struct ConversionResponse {
    pub additional_candidates: Vec<ConversionCandidate>,
    /// Indices into request.candidates followed by new additional candidates.
    /// Local duplicates are preserved. Additional candidates with an output text
    /// already present in the list are omitted before assigning indices.
    pub order: Option<Vec<usize>>,
    /// None keeps normal learning if the original first dictionary candidate
    /// remains first; otherwise learning requires explicit selection.
    pub learning: Option<LearningPolicy>,
}

/// Called synchronously before the first candidate is presented. Implementations
/// must use ready results and must not perform network, disk, or inference work.
pub trait ConversionProvider {
    fn prepare(&mut self, request: &ConversionRequest) -> ConversionResponse;
}

impl<F: FnMut(&ConversionRequest) -> ConversionResponse> ConversionProvider for F {
    fn prepare(&mut self, request: &ConversionRequest) -> ConversionResponse {
        self(request)
    }
}

#[derive(Clone, Debug)]
pub enum MissingConversionResponse {
    Ready(ConversionResponse),
    Pending,
}
/// Called only when a nonempty reading produced no local dictionary candidates
/// and an actual conversion is starting. Unlike [`ConversionProvider`], an
/// implementation may wait for a bounded background result (for example a
/// short network lookup under the caller's deadline) because this is never
/// reached by preview or completion while typing, and never by a local hit.
pub trait MissingConversionProvider {
    fn prepare(&mut self, request: &ConversionRequest) -> MissingConversionResponse;
}

impl<F: FnMut(&ConversionRequest) -> ConversionResponse> MissingConversionProvider for F {
    fn prepare(&mut self, request: &ConversionRequest) -> MissingConversionResponse {
        MissingConversionResponse::Ready(self(request))
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ConversionError {
    #[error("candidate order must be a complete permutation of the merged candidates")]
    InvalidOrder,
    #[error("external candidate contains empty text or unsupported dictionary delimiters")]
    InvalidCandidate,
}

impl ConversionRequest {
    /// Dictionary key for this request, including the SKK okuri prefix when an
    /// okuri is present (reading `か` with okuri `く` becomes `かk`).
    pub fn dictionary_key(&self) -> String {
        CompositeKey::new(&self.reading, self.okuri.clone()).get_dict_key()
    }

    pub(crate) fn new_missing(key: &CompositeKey) -> Self {
        Self {
            reading: key.get_to_composite().to_owned(),
            okuri: key.get_okuri().clone(),
            candidates: Vec::new(),
            protected_indices: Vec::new(),
        }
    }

    pub(crate) fn new(
        key: &CompositeKey,
        candidates: &[Candidate],
        dictionaries: &[Arc<CskkDictionary>],
    ) -> Self {
        Self::new_limited(key, candidates, dictionaries, usize::MAX)
    }

    pub(crate) fn new_limited(
        key: &CompositeKey,
        candidates: &[Candidate],
        dictionaries: &[Arc<CskkDictionary>],
        limit: usize,
    ) -> Self {
        let candidates = &candidates[..candidates.len().min(limit)];
        let readonly: Vec<_> = dictionaries
            .iter()
            .filter(|dictionary| dictionary.is_read_only())
            .cloned()
            .collect();
        let static_candidates = get_all_candidates(&readonly, key);
        // Match Candidate::eq: annotations, rendered numbers and okuri do not
        // distinguish a learned system entry from its dictionary identity.
        let static_identities: HashSet<_> = static_candidates
            .iter()
            .map(|candidate| (candidate.midashi.as_str(), candidate.kouho_text.as_str()))
            .collect();
        Self {
            reading: key.get_to_composite().to_owned(),
            okuri: key.get_okuri().clone(),
            candidates: candidates
                .iter()
                .map(|candidate| ConversionCandidate {
                    text: candidate.output.clone(),
                    annotation: candidate.annotation.clone(),
                })
                .collect(),
            protected_indices: candidates
                .iter()
                .enumerate()
                .filter_map(|(index, candidate)| {
                    (!static_identities
                        .contains(&(candidate.midashi.as_str(), candidate.kouho_text.as_str())))
                    .then_some(index)
                })
                .collect(),
        }
    }
}

pub(crate) fn apply_response(
    candidates: &mut Vec<Candidate>,
    key: &CompositeKey,
    response: ConversionResponse,
) -> Result<LearningPolicy, ConversionError> {
    for extra in &response.additional_candidates {
        extra.validate_external()?;
    }
    let mut merged = candidates.clone();
    for extra in response.additional_candidates {
        if !merged
            .iter()
            .any(|candidate| candidate.output == extra.text)
        {
            merged.push(Candidate::new(
                key.get_dict_key(),
                key.get_okuri().clone(),
                key.has_okuri(),
                extra.text.clone(),
                extra.annotation,
                extra.text,
            ));
        }
    }
    if let Some(order) = response.order {
        let mut seen = vec![false; merged.len()];
        if order.len() != seen.len() {
            return Err(ConversionError::InvalidOrder);
        }
        for &index in &order {
            let Some(used) = seen.get_mut(index) else {
                return Err(ConversionError::InvalidOrder);
            };
            if *used {
                return Err(ConversionError::InvalidOrder);
            }
            *used = true;
        }
        let mut slots: Vec<_> = merged.into_iter().map(Some).collect();
        merged = order
            .into_iter()
            .map(|index| slots[index].take().unwrap())
            .collect();
    }
    let learning = response
        .learning
        .unwrap_or(if merged.first() == candidates.first() {
            LearningPolicy::Always
        } else {
            LearningPolicy::ExplicitSelectionOnly
        });
    *candidates = merged;
    Ok(learning)
}

fn unsupported_literal(text: &str) -> bool {
    text.contains("(concat") || text.chars().any(|c| c.is_control() || "[]\\".contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_output_with_different_dictionary_identity_requires_explicit_learning() {
        let key = CompositeKey::new("1かい", None);
        let mut candidates = vec![
            Candidate::new(
                "1かい".into(),
                None,
                false,
                "1回".into(),
                None,
                "1回".into(),
            ),
            Candidate::new(
                "#かい".into(),
                None,
                false,
                "#0回".into(),
                None,
                "1回".into(),
            ),
        ];
        let learning = apply_response(
            &mut candidates,
            &key,
            ConversionResponse {
                order: Some(vec![1, 0]),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(learning, LearningPolicy::ExplicitSelectionOnly);
    }

    #[test]
    fn dictionary_key_includes_skk_okuri_prefix() {
        let no_okuri = ConversionRequest {
            reading: "か".into(),
            okuri: None,
            candidates: Vec::new(),
            protected_indices: Vec::new(),
        };
        assert_eq!(no_okuri.dictionary_key(), "か");

        let with_okuri = ConversionRequest {
            reading: "か".into(),
            okuri: Some("く".into()),
            candidates: Vec::new(),
            protected_indices: Vec::new(),
        };
        assert_eq!(with_okuri.dictionary_key(), "かk");
    }

    #[test]
    fn external_candidate_validation_matches_merge_rules() {
        for text in ["東京", "km/h", "1/2", "(笑)", "a;note"] {
            let candidate = ConversionCandidate {
                text: text.into(),
                annotation: None,
            };
            assert!(candidate.validate_external().is_ok(), "{:?}", text);
        }
        for text in ["", "[注]", "(concat x)", "a\nb", "#0回", "a\\b"] {
            let candidate = ConversionCandidate {
                text: text.into(),
                annotation: None,
            };
            assert_eq!(
                candidate.validate_external(),
                Err(ConversionError::InvalidCandidate),
                "{text:?}"
            );
        }
        let annotated = ConversionCandidate {
            text: "東京".into(),
            annotation: Some("首都".into()),
        };
        assert!(annotated.validate_external().is_ok());
        for annotation in ["", "[注]", "a\nb", "(concat x)"] {
            let candidate = ConversionCandidate {
                text: "東京".into(),
                annotation: Some(annotation.into()),
            };
            assert_eq!(
                candidate.validate_external(),
                Err(ConversionError::InvalidCandidate),
                "{annotation:?}"
            );
        }
    }
}
