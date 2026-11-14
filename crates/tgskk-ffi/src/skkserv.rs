use super::{TgskkSession, diagnostic, report, string_arg};
use std::cell::Cell;
use std::collections::HashSet;
use std::ffi::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::rc::Rc;
use std::time::Duration;
use tgskk_core::input::{
    ConversionCandidate, ConversionRequest, ConversionResponse, LearningPolicy,
    MissingConversionProvider, MissingConversionResponse,
};
use tgskk_skkserv::{Candidate, Config, Worker};

#[derive(Clone, Copy)]
enum Source {
    Google,
    Custom,
}

pub(super) struct TgskkSkkserv {
    worker: Worker,
    source: Source,
    wait: Duration,
}

fn new_server(config: Config, source: Source, wait_ms: u32) -> Result<*mut TgskkSkkserv, String> {
    if wait_ms > 1000 {
        return Err("SKK server wait must be between 0 and 1000 milliseconds".into());
    }
    let worker = Worker::start(config).map_err(|_| "Cannot start SKK server worker".to_owned())?;
    Ok(Box::into_raw(Box::new(TgskkSkkserv {
        worker,
        source,
        wait: Duration::from_millis(u64::from(wait_ms)),
    })))
}

#[unsafe(no_mangle)]
pub(super) unsafe extern "C" fn tgskk_skkserv_google_new(
    executable: *const c_char,
    wait_ms: u32,
    error: *mut *mut c_char,
) -> *mut TgskkSkkserv {
    unsafe {
        report(error, || {
            let config = Config::google(string_arg(executable)?).map_err(|e| e.to_string())?;
            new_server(config, Source::Google, wait_ms)
        })
    }
    .unwrap_or(ptr::null_mut())
}

#[unsafe(no_mangle)]
pub(super) unsafe extern "C" fn tgskk_skkserv_custom_new(
    host: *const c_char,
    port: u16,
    encoding: *const c_char,
    wait_ms: u32,
    error: *mut *mut c_char,
) -> *mut TgskkSkkserv {
    unsafe {
        report(error, || {
            let encoding = string_arg(encoding)?
                .parse()
                .map_err(|e: tgskk_skkserv::Error| e.to_string())?;
            let config =
                Config::tcp(string_arg(host)?, port, encoding).map_err(|e| e.to_string())?;
            new_server(config, Source::Custom, wait_ms)
        })
    }
    .unwrap_or(ptr::null_mut())
}

#[unsafe(no_mangle)]
pub(super) unsafe extern "C" fn tgskk_skkserv_free(server: *mut TgskkSkkserv) {
    if !server.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| unsafe { drop(Box::from_raw(server)) }));
    }
}

#[unsafe(no_mangle)]
pub(super) unsafe extern "C" fn tgskk_skkserv_take_error(
    server: *const TgskkSkkserv,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        unsafe { server.as_ref() }
            .and_then(|s| s.worker.take_error())
            .map_or(ptr::null_mut(), |message| diagnostic(&message).into_raw())
    }))
    .unwrap_or(ptr::null_mut())
}

#[unsafe(no_mangle)]
pub(super) unsafe extern "C" fn tgskk_session_network_allowed(
    session: *mut TgskkSession,
    allowed: u8,
) -> u8 {
    let Some(session) = (unsafe { session.as_mut() }).filter(|s| !s.poisoned) else {
        return 0;
    };
    session.network_allowed.set(allowed != 0);
    1
}

#[unsafe(no_mangle)]
pub(super) unsafe extern "C" fn tgskk_session_set_skkserv(
    session: *mut TgskkSession,
    server: *const TgskkSkkserv,
) -> u8 {
    let Some(session) = (unsafe { session.as_mut() }).filter(|s| !s.poisoned) else {
        return 0;
    };
    session.network_allowed.set(false);
    let result = catch_unwind(AssertUnwindSafe(|| {
        let provider = unsafe { server.as_ref() }.map(|server| {
            Box::new(MissingLookup {
                worker: server.worker.clone(),
                source: server.source,
                wait: server.wait,
                allowed: session.network_allowed.clone(),
                pending: session.lookup_pending.clone(),
                defer: session.defer_missing.clone(),
            }) as Box<dyn MissingConversionProvider>
        });
        session.input.set_missing_conversion_provider(provider);
    }));
    if result.is_err() {
        session.poisoned = true;
        return 0;
    }
    1
}

struct MissingLookup {
    worker: Worker,
    source: Source,
    wait: Duration,
    allowed: Rc<Cell<bool>>,
    pending: Rc<Cell<bool>>,
    defer: Rc<Cell<bool>>,
}
impl MissingConversionProvider for MissingLookup {
    fn prepare(&mut self, request: &ConversionRequest) -> MissingConversionResponse {
        let empty = || MissingConversionResponse::Ready(ConversionResponse::default());
        if !self.allowed.get() {
            return empty();
        }
        let Some(reading) = lookup_reading(request, self.source) else {
            return empty();
        };
        let candidates = match self.worker.lookup(&reading, self.wait) {
            Ok(candidates) => candidates,
            Err(error) => {
                let pending = matches!(
                    error,
                    tgskk_skkserv::LookupError::Timeout | tgskk_skkserv::LookupError::Busy
                );
                self.pending.set(pending);
                return if self.wait.is_zero() && self.defer.get() && pending {
                    MissingConversionResponse::Pending
                } else {
                    empty()
                };
            }
        };
        let additional_candidates = convert_candidates(candidates, request, self.source);
        let learning = (!additional_candidates.is_empty()).then_some(LearningPolicy::Always);
        MissingConversionResponse::Ready(ConversionResponse {
            additional_candidates,
            learning,
            ..Default::default()
        })
    }
}

fn lookup_reading(request: &ConversionRequest, source: Source) -> Option<String> {
    match source {
        Source::Custom => Some(request.dictionary_key()),
        Source::Google => {
            let reading = format!(
                "{}{}",
                request.reading,
                request.okuri.as_deref().unwrap_or_default()
            );
            (!reading.is_empty()
                && reading
                    .chars()
                    .all(|c| matches!(c, '\u{3041}'..='\u{3096}' | 'ゝ' | 'ゞ' | 'ゟ' | 'ー')))
            .then_some(reading)
        }
    }
}

fn convert_candidates(
    candidates: Vec<Candidate>,
    request: &ConversionRequest,
    source: Source,
) -> Vec<ConversionCandidate> {
    let mut seen = HashSet::new();
    candidates
        .into_iter()
        .filter_map(|candidate| {
            let text = match (source, request.okuri.as_deref()) {
                (Source::Google, Some(okuri)) => candidate.text.strip_suffix(okuri)?.to_owned(),
                _ => candidate.text,
            };
            let candidate = ConversionCandidate {
                text,
                annotation: candidate.annotation,
            };
            if candidate.text.len() > 1024
                || candidate.annotation.as_ref().is_some_and(|a| a.len() > 512)
                || candidate.validate_external().is_err()
                || !seen.insert(candidate.text.clone())
            {
                return None;
            }
            Some(candidate)
        })
        .collect()
}

#[cfg(test)]
mod tests;
