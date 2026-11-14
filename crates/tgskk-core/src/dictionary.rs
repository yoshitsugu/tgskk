use std::{collections::HashMap, error::Error, fmt, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub text: String,
    pub annotation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionaryError {
    pub line: usize,
    pub reason: &'static str,
}

impl fmt::Display for DictionaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "dictionary line {}: {}", self.line, self.reason)
    }
}

impl Error for DictionaryError {}

#[derive(Default)]
pub struct Dictionary {
    entries: HashMap<String, Arc<[Candidate]>>,
}

impl Dictionary {
    pub fn parse_utf8(input: &str) -> Result<Self, DictionaryError> {
        let mut entries: HashMap<String, Vec<Candidate>> = HashMap::new();
        for (index, line) in input.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') {
                continue;
            }
            let error = |reason| DictionaryError {
                line: index + 1,
                reason,
            };
            let (reading, body) = line
                .split_once(char::is_whitespace)
                .ok_or_else(|| error("expected reading and /candidates/"))?;
            let body = body.trim();
            let body = body
                .strip_prefix('/')
                .and_then(|body| body.strip_suffix('/'))
                .ok_or_else(|| error("candidates must be enclosed in /"))?;
            let candidates = entries.entry(reading.to_owned()).or_default();
            for raw in body.split('/') {
                if raw.is_empty() {
                    return Err(error("empty candidate"));
                }
                if raw.starts_with('[') || raw.starts_with(']') || raw.starts_with('(') {
                    return Err(error(
                        "okuri blocks and Lisp expressions are not supported yet",
                    ));
                }
                let (text, annotation) = raw
                    .split_once(';')
                    .map_or((raw, None), |(t, a)| (t, Some(a)));
                if text.is_empty() {
                    return Err(error("empty candidate text"));
                }
                if !candidates.iter().any(|candidate| candidate.text == text) {
                    candidates.push(Candidate {
                        text: text.to_owned(),
                        annotation: annotation.map(str::to_owned),
                    });
                }
            }
        }
        Ok(Self {
            entries: entries
                .into_iter()
                .map(|(key, value)| (key, value.into()))
                .collect(),
        })
    }

    #[must_use]
    pub fn lookup(&self, reading: &str) -> Arc<[Candidate]> {
        self.entries.get(reading).cloned().unwrap_or_default()
    }
}
