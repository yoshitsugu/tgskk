//! SKK server protocol support.
//!
//! The codec ([`encode_request`], [`decode_response`]) performs no I/O. The
//! [`fetch`] client performs one bounded TCP exchange per call. [`Worker`] runs
//! lookups on a single background thread with a bounded cache and can lazily
//! own a managed Google helper subprocess; retries, logging and provider
//! composition belong to the caller.

mod client;
mod worker;

pub use client::{ClientError, ProtocolError, fetch};
pub use worker::{Config, ConfigError, LookupError, Worker};

use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt;
use std::str::FromStr;

/// Maximum accepted byte length of a lookup reading, measured as UTF-8.
pub const MAX_READING_BYTES: usize = 1024;

/// Maximum accepted byte length of a response frame.
pub const MAX_RESPONSE_BYTES: usize = 64 * 1024;

/// Maximum accepted number of candidates in one response.
pub const MAX_CANDIDATES: usize = 256;

/// Wire encoding used for a server conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    EucJp,
    Utf8,
}

impl Encoding {
    fn codec(self) -> &'static encoding_rs::Encoding {
        match self {
            Self::EucJp => encoding_rs::EUC_JP,
            Self::Utf8 => encoding_rs::UTF_8,
        }
    }
}

impl FromStr for Encoding {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.eq_ignore_ascii_case("euc-jp") {
            Ok(Self::EucJp)
        } else if value.eq_ignore_ascii_case("utf-8") {
            Ok(Self::Utf8)
        } else {
            Err(Error::InvalidEncodingName)
        }
    }
}

/// One dictionary candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub text: String,
    pub annotation: Option<String>,
}

/// Codec error.
///
/// Messages never contain the offending reading or response bytes. Details of
/// which check failed are available through the variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// An encoding name other than `euc-jp` or `utf-8` was requested.
    InvalidEncodingName,
    /// The lookup reading was empty.
    EmptyReading,
    /// The lookup reading exceeded [`MAX_READING_BYTES`] in UTF-8 bytes.
    ReadingTooLong,
    /// The lookup reading contained whitespace or a control character.
    InvalidReadingCharacter,
    /// The lookup reading cannot be represented in the requested encoding.
    UnrepresentableReading,
    /// The response frame exceeded [`MAX_RESPONSE_BYTES`].
    ResponseTooLarge,
    /// The response contained more than [`MAX_CANDIDATES`] candidates.
    TooManyCandidates,
    /// The server reported a protocol error (`0`).
    ServerError,
    /// The response was not valid SKK server framing.
    MalformedResponse,
    /// The response bytes were not valid in the requested encoding.
    InvalidResponseEncoding,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidEncodingName => "unknown SKK server encoding",
            Self::EmptyReading => "SKK lookup reading is empty",
            Self::ReadingTooLong => "SKK lookup reading is too long",
            Self::InvalidReadingCharacter => {
                "SKK lookup reading contains whitespace or a control character"
            }
            Self::UnrepresentableReading => {
                "SKK lookup reading cannot be represented in the requested encoding"
            }
            Self::ResponseTooLarge => "SKK server response is too large",
            Self::TooManyCandidates => "SKK server response has too many candidates",
            Self::ServerError => "SKK server reported an error",
            Self::MalformedResponse => "malformed SKK server response",
            Self::InvalidResponseEncoding => {
                "SKK server response is not valid in the requested encoding"
            }
        };
        f.write_str(message)
    }
}

impl StdError for Error {}

/// Encodes a dictionary lookup request as an ASCII `1`, the reading in the
/// requested encoding, and an ASCII space.
pub fn encode_request(reading: &str, encoding: Encoding) -> Result<Vec<u8>, Error> {
    if reading.is_empty() {
        return Err(Error::EmptyReading);
    }
    if reading.len() > MAX_READING_BYTES {
        return Err(Error::ReadingTooLong);
    }
    if reading.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(Error::InvalidReadingCharacter);
    }

    let (encoded, _, had_errors) = encoding.codec().encode(reading);
    if had_errors {
        return Err(Error::UnrepresentableReading);
    }

    let mut request = Vec::with_capacity(encoded.len() + 2);
    request.push(b'1');
    request.extend_from_slice(&encoded);
    request.push(b' ');
    Ok(request)
}

/// Decodes a dictionary lookup response.
///
/// A hit is `1/candidate;annotation/candidate/` terminated by LF or CRLF.
/// A body starting with `4` (optionally followed by the echoed reading and a
/// space) is a miss, `1/` is an empty hit, and a standalone `0` is a typed
/// server error. A trailing LF is required; a bare `0` is the only accepted
/// frame without one, matching the upstream error response.
pub fn decode_response(frame: &[u8], encoding: Encoding) -> Result<Vec<Candidate>, Error> {
    if frame.len() > MAX_RESPONSE_BYTES {
        return Err(Error::ResponseTooLarge);
    }

    let decoded = decode_strict(frame, encoding)?;
    if &*decoded == "0" {
        return Err(Error::ServerError);
    }
    let body = decoded.strip_suffix('\n').ok_or(Error::MalformedResponse)?;
    let body = body.strip_suffix('\r').unwrap_or(body);
    if body.is_empty() || body.chars().any(|c| c == '\r' || c == '\n') {
        return Err(Error::MalformedResponse);
    }

    parse_body(body)
}

fn decode_strict(frame: &[u8], encoding: Encoding) -> Result<Cow<'_, str>, Error> {
    match encoding {
        Encoding::Utf8 => std::str::from_utf8(frame)
            .map(Cow::Borrowed)
            .map_err(|_| Error::InvalidResponseEncoding),
        Encoding::EucJp => encoding_rs::EUC_JP
            .decode_without_bom_handling_and_without_replacement(frame)
            .ok_or(Error::InvalidResponseEncoding),
    }
}

fn parse_body(body: &str) -> Result<Vec<Candidate>, Error> {
    if body == "0" {
        return Err(Error::ServerError);
    }
    if body.starts_with('4') {
        validate_text(body)?;
        return Ok(Vec::new());
    }

    let rest = body.strip_prefix("1/").ok_or(Error::MalformedResponse)?;
    if rest.is_empty() {
        return Ok(Vec::new());
    }
    let list = rest.strip_suffix('/').ok_or(Error::MalformedResponse)?;
    if list.is_empty() {
        return Err(Error::MalformedResponse);
    }

    let mut candidates = Vec::new();
    for slot in list.split('/') {
        if candidates.len() == MAX_CANDIDATES {
            return Err(Error::TooManyCandidates);
        }
        candidates.push(parse_candidate(slot)?);
    }
    Ok(candidates)
}

fn parse_candidate(slot: &str) -> Result<Candidate, Error> {
    if slot.is_empty() {
        return Err(Error::MalformedResponse);
    }
    let (text, annotation) = match slot.split_once(';') {
        Some((text, annotation)) => (text, Some(annotation)),
        None => (slot, None),
    };
    if text.is_empty() {
        return Err(Error::MalformedResponse);
    }
    validate_text(text)?;
    if let Some(annotation) = annotation {
        validate_text(annotation)?;
    }

    let text = decode_constant_string(text)?;
    if text.is_empty() {
        return Err(Error::MalformedResponse);
    }
    let annotation = match annotation {
        Some(annotation) => {
            let decoded = decode_constant_string(annotation)?;
            (!decoded.is_empty()).then_some(decoded)
        }
        None => None,
    };
    Ok(Candidate { text, annotation })
}

/// Decodes the canonical SKK `(concat "..." "...")` constant-string form.
/// Text that does not start with `(concat` is returned unchanged. Only quoted
/// constant arguments are supported; nothing is evaluated.
fn decode_constant_string(value: &str) -> Result<String, Error> {
    let Some(rest) = value.strip_prefix("(concat") else {
        return Ok(value.to_string());
    };
    let mut cursor = rest;
    let mut decoded = String::new();
    let mut arguments = 0;
    loop {
        cursor = cursor.trim_start_matches(' ');
        if let Some(after_quote) = cursor.strip_prefix('"') {
            let (argument, remaining) = decode_quoted_string(after_quote)?;
            decoded.push_str(&argument);
            arguments += 1;
            cursor = remaining;
            continue;
        }
        let Some(trailing) = cursor.strip_prefix(')') else {
            return Err(Error::MalformedResponse);
        };
        if arguments == 0 || !trailing.is_empty() {
            return Err(Error::MalformedResponse);
        }
        if decoded.chars().any(char::is_control) {
            return Err(Error::MalformedResponse);
        }
        return Ok(decoded);
    }
}

fn decode_quoted_string(input: &str) -> Result<(String, &str), Error> {
    let mut decoded = String::new();
    let mut characters = input.char_indices().peekable();
    while let Some((index, character)) = characters.next() {
        match character {
            '"' => return Ok((decoded, &input[index + 1..])),
            '\\' => {
                let Some((_, escape)) = characters.next() else {
                    return Err(Error::MalformedResponse);
                };
                match escape {
                    '"' => decoded.push('"'),
                    '\\' => decoded.push('\\'),
                    '0'..='7' => {
                        let mut value = (escape as u32) - ('0' as u32);
                        let mut digits = 1;
                        while digits < 3 {
                            let Some(&(_, next)) = characters.peek() else {
                                break;
                            };
                            let Some(digit) = next.to_digit(8) else {
                                break;
                            };
                            characters.next();
                            value = value * 8 + digit;
                            digits += 1;
                        }
                        let Some(character) = char::from_u32(value) else {
                            return Err(Error::MalformedResponse);
                        };
                        decoded.push(character);
                    }
                    _ => return Err(Error::MalformedResponse),
                }
            }
            other => decoded.push(other),
        }
    }
    Err(Error::MalformedResponse)
}

fn validate_text(value: &str) -> Result<(), Error> {
    if value.chars().any(char::is_control) {
        return Err(Error::MalformedResponse);
    }
    Ok(())
}
