//! C ABI for a single input thread. Pointer ownership and lifetimes are defined
//! in include/tgskk.h; no Rust allocation may be released by the C allocator.

use std::ffi::{CStr, CString, c_char};
use std::io::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::sync::Arc;

use tgskk_zenz::{Assistance, Status, Worker};

use tgskk_core::input::{CskkDictionary, CskkKeyEvent, InputOutcome, InputSession, ModeChange};

struct TgskkZenz {
    worker: Worker,
}

struct TgskkRuntime {
    dictionaries: Vec<Arc<CskkDictionary>>,
}

#[repr(C)]
struct TgskkView {
    consumed: u8,
    mode_changed: u8,
    selecting: u8,
    zenz_applied: u8,
    selected: usize,
    candidate_count: usize,
    committed: *const c_char,
    preedit: *const c_char,
    mode_label: *const c_char,
}

struct TgskkSession {
    input: InputSession,
    assistance: Option<Assistance>,
    view: TgskkView,
    committed: CString,
    preedit: CString,
    label: CString,
    candidates: Vec<(CString, Option<CString>)>,
    error: CString,
    poisoned: bool,
}

impl TgskkSession {
    fn new(input: InputSession) -> Result<Self, String> {
        let mut session = Self {
            input,
            assistance: None,
            view: TgskkView {
                consumed: 0,
                mode_changed: 0,
                selecting: 0,
                zenz_applied: 0,
                selected: 0,
                candidate_count: 0,
                committed: ptr::null(),
                preedit: ptr::null(),
                mode_label: ptr::null(),
            },
            committed: CString::default(),
            preedit: CString::default(),
            label: CString::default(),
            candidates: Vec::new(),
            error: CString::default(),
            poisoned: false,
        };
        session.update(empty_outcome(None))?;
        Ok(session)
    }

    fn update(&mut self, outcome: InputOutcome) -> Result<(), String> {
        self.committed = CString::new(outcome.committed).map_err(|e| e.to_string())?;
        self.preedit = CString::new(outcome.preedit).map_err(|e| e.to_string())?;
        self.label = CString::new(self.input.mode_label()).map_err(|e| e.to_string())?;
        let cursor = self.input.candidate_cursor();
        self.candidates.clear();
        if cursor.is_some() {
            for candidate in self.input.candidates() {
                self.candidates.push((
                    CString::new(candidate.text).map_err(|e| e.to_string())?,
                    candidate
                        .annotation
                        .map(CString::new)
                        .transpose()
                        .map_err(|e| e.to_string())?,
                ));
            }
        }
        self.error = diagnostic(
            outcome
                .conversion_error
                .map(|e| format!("{e:?}"))
                .unwrap_or_default(),
        );
        self.view = TgskkView {
            consumed: u8::from(outcome.consumed),
            mode_changed: u8::from(outcome.mode_change.is_some()),
            selecting: u8::from(cursor.is_some()),
            zenz_applied: u8::from(
                cursor.is_some() && self.assistance.as_ref().is_some_and(Assistance::applied),
            ),
            selected: cursor.unwrap_or(0),
            candidate_count: self.candidates.len(),
            committed: self.committed.as_ptr(),
            preedit: self.preedit.as_ptr(),
            mode_label: self.label.as_ptr(),
        };
        Ok(())
    }
}

fn empty_outcome(mode_change: Option<ModeChange>) -> InputOutcome {
    InputOutcome {
        consumed: false,
        committed: String::new(),
        preedit: String::new(),
        mode_change,
        conversion_error: None,
    }
}

fn diagnostic(message: String) -> CString {
    CString::new(message.replace('\0', "\\0")).expect("NULs were escaped")
}

unsafe fn string_arg<'a>(value: *const c_char) -> Result<&'a str, String> {
    if value.is_null() {
        return Err("Required string is NULL".to_owned());
    }
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map_err(|e| e.to_string())
}

unsafe fn report<T>(
    error: *mut *mut c_char,
    action: impl FnOnce() -> Result<T, String>,
) -> Option<T> {
    if !error.is_null() {
        unsafe { *error = ptr::null_mut() };
    }
    let result = catch_unwind(AssertUnwindSafe(action))
        .unwrap_or_else(|_| Err("Rust panic in tgskk".to_owned()));
    match result {
        Ok(value) => Some(value),
        Err(message) => {
            if !error.is_null() {
                unsafe { *error = diagnostic(message).into_raw() };
            }
            None
        }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_zenz_new(
    executable: *const c_char,
    model: *const c_char,
    error: *mut *mut c_char,
) -> *mut TgskkZenz {
    unsafe {
        report(error, || {
            let worker = Worker::start(string_arg(executable)?.into(), string_arg(model)?.into())
                .map_err(|e| e.to_string())?;
            Ok(Box::into_raw(Box::new(TgskkZenz { worker })))
        })
        .unwrap_or(ptr::null_mut())
    }
}
#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_zenz_free(zenz: *mut TgskkZenz) {
    if !zenz.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| unsafe { drop(Box::from_raw(zenz)) }));
    }
}
#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_zenz_status(zenz: *const TgskkZenz) -> u8 {
    unsafe { zenz.as_ref() }.map_or(Status::Unavailable, |z| z.worker.status()) as u8
}
#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_zenz_take_error(zenz: *const TgskkZenz) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        unsafe { zenz.as_ref() }
            .and_then(|z| z.worker.take_error())
            .map_or(ptr::null_mut(), |message| diagnostic(message).into_raw())
    }))
    .unwrap_or(ptr::null_mut())
}
#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_session_set_zenz(
    session: *mut TgskkSession,
    zenz: *const TgskkZenz,
) -> u8 {
    let Some(session) = (unsafe { session.as_mut() }) else {
        return 0;
    };
    if session.poisoned {
        return 0;
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        if let Some(assistance) = session.assistance.take() {
            assistance.reset();
        }
        session.input.set_conversion_provider(None);
        if let Some(zenz) = unsafe { zenz.as_ref() } {
            session.assistance = Some(Assistance::attach(&mut session.input, zenz.worker.clone()));
        }
    }));
    if result.is_err() {
        session.poisoned = true;
        return 0;
    }
    1
}
#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_session_context(
    session: *mut TgskkSession,
    left: *const c_char,
    enabled: u8,
) -> u8 {
    let Some(session) = (unsafe { session.as_mut() }) else {
        return 0;
    };
    if session.poisoned {
        return 0;
    }
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<(), String> {
        let text = if enabled == 0 {
            ""
        } else {
            unsafe { string_arg(left)? }
        };
        if let Some(assistance) = &session.assistance {
            assistance.context(text, enabled != 0);
        }
        Ok(())
    }));
    match result {
        Ok(Ok(())) => 1,
        Ok(Err(message)) => {
            session.error = diagnostic(message);
            0
        }
        Err(_) => {
            session.poisoned = true;
            0
        }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_runtime_new(
    dictionary: *const c_char,
    encoding: *const c_char,
    user_dictionary: *const c_char,
    error: *mut *mut c_char,
) -> *mut TgskkRuntime {
    unsafe {
        report(error, || {
            let path = string_arg(dictionary)?;
            let encoding = string_arg(encoding)?;
            if !encoding.eq_ignore_ascii_case("utf-8") && !encoding.eq_ignore_ascii_case("euc-jp") {
                return Err("Dictionary encoding must be utf-8 or euc-jp".to_owned());
            }
            let dictionary =
                CskkDictionary::new_static_dict(path, encoding, true).map_err(|e| e.to_string())?;
            let mut dictionaries = Vec::new();
            if !user_dictionary.is_null() {
                let user_path = string_arg(user_dictionary)?;
                match std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(user_path)
                {
                    Ok(mut file) => file
                        .write_all(b";; okuri-ari entries.\n;; okuri-nasi entries.\n")
                        .map_err(|e| e.to_string())?,
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(error) => return Err(error.to_string()),
                }
                let user = CskkDictionary::new_user_dict(user_path, "utf-8", true)
                    .map_err(|e| e.to_string())?;
                #[allow(
                    clippy::arc_with_non_send_sync,
                    reason = "CSKK dictionaries stay on the input thread"
                )]
                dictionaries.push(Arc::new(user));
            }
            #[allow(
                clippy::arc_with_non_send_sync,
                reason = "CSKK dictionaries stay on the input thread"
            )]
            dictionaries.push(Arc::new(dictionary));
            Ok(Box::into_raw(Box::new(TgskkRuntime { dictionaries })))
        })
        .unwrap_or(ptr::null_mut())
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_runtime_free(runtime: *mut TgskkRuntime) {
    if !runtime.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| unsafe { drop(Box::from_raw(runtime)) }));
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_string_free(string: *mut c_char) {
    if !string.is_null() {
        unsafe { drop(CString::from_raw(string)) };
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_runtime_save(runtime: *mut TgskkRuntime, error: *mut *mut c_char) -> u8 {
    unsafe {
        u8::from(
            report(error, || {
                let runtime = runtime.as_ref().ok_or("Runtime is NULL")?;
                for dictionary in &runtime.dictionaries {
                    dictionary.save().map_err(|e| e.to_string())?;
                }
                Ok(())
            })
            .is_some(),
        )
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_session_new(
    runtime: *const TgskkRuntime,
    error: *mut *mut c_char,
) -> *mut TgskkSession {
    unsafe {
        report(error, || {
            let runtime = runtime.as_ref().ok_or("Runtime is NULL")?;
            let input = InputSession::with_dictionaries(runtime.dictionaries.clone())
                .map_err(|e| e.to_string())?;
            Ok(Box::into_raw(Box::new(TgskkSession::new(input)?)))
        })
        .unwrap_or(ptr::null_mut())
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_session_free(session: *mut TgskkSession) {
    if !session.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| unsafe { drop(Box::from_raw(session)) }));
    }
}

unsafe fn session_call(
    session: *mut TgskkSession,
    action: impl FnOnce(&mut InputSession) -> InputOutcome,
) -> *const TgskkView {
    unsafe { session_action(session, |s| action(&mut s.input)) }
}

unsafe fn session_action(
    session: *mut TgskkSession,
    action: impl FnOnce(&mut TgskkSession) -> InputOutcome,
) -> *const TgskkView {
    let Some(session) = (unsafe { session.as_mut() }) else {
        return ptr::null();
    };
    if session.poisoned {
        return ptr::null();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        let outcome = action(session);
        session.update(outcome)?;
        if let Some(assistance) = &session.assistance {
            assistance.prefetch(&session.input);
        }
        Ok(())
    }))
    .unwrap_or_else(|_| Err("Rust panic: discard this input session".to_owned()));
    match result {
        Ok(()) => &session.view,
        Err(message) => {
            session.poisoned = true;
            session.error = diagnostic(message);
            ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_session_key(
    session: *mut TgskkSession,
    keysym: u32,
    modifiers: u32,
    release: u8,
) -> *const TgskkView {
    unsafe {
        session_call(session, |input| {
            input.process_key(&CskkKeyEvent::from_fcitx_keyevent(
                keysym,
                modifiers,
                release != 0,
            ))
        })
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_session_select(
    session: *mut TgskkSession,
    index: usize,
) -> *const TgskkView {
    unsafe { session_call(session, |input| input.confirm_candidate(index)) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_session_choose(
    session: *mut TgskkSession,
    index: usize,
) -> *const TgskkView {
    unsafe { session_call(session, |input| input.select_candidate(index)) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_session_reset(session: *mut TgskkSession) -> *const TgskkView {
    unsafe {
        session_action(session, |session| {
            if let Some(assistance) = &session.assistance {
                assistance.reset();
            }
            empty_outcome(session.input.reset())
        })
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_session_view(session: *const TgskkSession) -> *const TgskkView {
    unsafe { session.as_ref() }
        .filter(|s| !s.poisoned)
        .map_or(ptr::null(), |s| &s.view)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_session_error(session: *const TgskkSession) -> *const c_char {
    unsafe { session.as_ref() }.map_or(c"Session is NULL".as_ptr(), |s| s.error.as_ptr())
}

#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_candidate_text(
    session: *const TgskkSession,
    index: usize,
) -> *const c_char {
    unsafe { session.as_ref() }
        .filter(|s| !s.poisoned)
        .and_then(|s| s.candidates.get(index))
        .map_or(ptr::null(), |c| c.0.as_ptr())
}

#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_candidate_annotation(
    session: *const TgskkSession,
    index: usize,
) -> *const c_char {
    unsafe { session.as_ref() }
        .filter(|s| !s.poisoned)
        .and_then(|s| s.candidates.get(index))
        .and_then(|c| c.1.as_ref())
        .map_or(ptr::null(), |s| s.as_ptr())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        _directory: tempfile::TempDir,
        user: CString,
        runtime: *mut TgskkRuntime,
        session: *mut TgskkSession,
    }

    impl Fixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let user = CString::new(directory.path().join("user.utf8").to_str().unwrap()).unwrap();
            let mut error = ptr::null_mut();
            let runtime = unsafe {
                tgskk_runtime_new(
                    dictionary().as_ptr(),
                    c"utf-8".as_ptr(),
                    user.as_ptr(),
                    &mut error,
                )
            };
            assert!(!runtime.is_null(), "{}", unsafe { message(error) });
            let session = unsafe { tgskk_session_new(runtime, &mut error) };
            assert!(!session.is_null());
            assert!(error.is_null());
            Self {
                _directory: directory,
                user,
                runtime,
                session,
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            unsafe {
                tgskk_session_free(self.session);
                tgskk_runtime_free(self.runtime);
            }
        }
    }

    fn dictionary() -> CString {
        CString::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/SKK-JISYO.demo.utf8"
        ))
        .unwrap()
    }

    unsafe fn message(pointer: *const c_char) -> String {
        if pointer.is_null() {
            return String::new();
        }
        unsafe { CStr::from_ptr(pointer) }
            .to_str()
            .unwrap()
            .to_owned()
    }

    unsafe fn keys(session: *mut TgskkSession, sequence: &str) -> *const TgskkView {
        let mut result = unsafe { tgskk_session_view(session) };
        for token in sequence.split_whitespace() {
            let (symbol, modifiers) = match token {
                "space" => (32, 0),
                "Return" => (0xff0d, 0),
                "Tab" => (0xff09, 0),
                "slash" => (47, 0),
                _ if token.starts_with("C-") => (u32::from(token.as_bytes()[2]), 4),
                _ => (
                    u32::from(token.as_bytes()[0]),
                    u32::from(token.as_bytes()[0].is_ascii_uppercase()),
                ),
            };
            result = unsafe { tgskk_session_key(session, symbol, modifiers, 0) };
            assert!(!result.is_null(), "{}", unsafe {
                message(tgskk_session_error(session))
            });
        }
        result
    }

    #[test]
    fn raw_keys_candidates_selection_and_release_cross_the_abi() {
        let fixture = Fixture::new();
        unsafe {
            let view = keys(fixture.session, "K a n j i space");
            assert_eq!(message((*view).preedit), "▼漢字");
            assert_eq!((*view).candidate_count, 3);
            assert_eq!(
                message(tgskk_candidate_annotation(fixture.session, 0)),
                "文字"
            );
            let view = tgskk_session_key(fixture.session, u32::from(b'a'), 0, 1);
            assert_eq!((*view).consumed, 0);
            assert_eq!(message((*view).preedit), "▼漢字");
            let view = tgskk_session_choose(fixture.session, 1);
            assert_eq!((*view).selected, 1);
            assert_eq!(message((*view).preedit), "▼感じ");
            let view = tgskk_session_select(fixture.session, 2);
            assert_eq!(message((*view).committed), "幹事");
            assert_eq!((*view).candidate_count, 0);
            assert!(tgskk_candidate_text(fixture.session, usize::MAX).is_null());
        }
    }

    #[test]
    fn reset_discards_partial_and_nested_states_but_retains_input_mode() {
        for sequence in [
            "K a n j",
            "K a K",
            "K a n j i space",
            "T o u k y o u space a",
            "T o u k y o u space N a n i space",
            "slash a b",
            "K a n Tab",
            "q K a n j",
            "q T o u k y o u space a",
        ] {
            let fixture = Fixture::new();
            unsafe {
                keys(fixture.session, sequence);
                let view = tgskk_session_reset(fixture.session);
                assert!(!view.is_null(), "{sequence}");
                assert!(message((*view).preedit).is_empty(), "{sequence}");
                assert!(message((*view).committed).is_empty(), "{sequence}");
                assert_eq!((*view).candidate_count, 0, "{sequence}");
                let expected = if sequence.starts_with('q') {
                    "カ"
                } else {
                    "か"
                };
                let view = keys(fixture.session, "k a");
                assert_eq!(message((*view).committed), expected, "{sequence}");
            }
        }
    }

    #[test]
    fn sessions_share_learning_without_sharing_composition_and_outlive_runtime() {
        let fixture = Fixture::new();
        unsafe {
            let other = tgskk_session_new(fixture.runtime, ptr::null_mut());
            keys(fixture.session, "K a n j i space");
            assert!(message((*tgskk_session_view(other)).preedit).is_empty());
            tgskk_session_select(fixture.session, 1);
            let view = keys(other, "K a n j i space");
            assert_eq!(message((*view).preedit), "▼感じ");
            tgskk_session_reset(other);
            assert!(!tgskk_session_view(other).is_null());
            tgskk_session_free(other);
            let transient_runtime = tgskk_runtime_new(
                dictionary().as_ptr(),
                c"utf-8".as_ptr(),
                ptr::null(),
                ptr::null_mut(),
            );
            let retained = tgskk_session_new(transient_runtime, ptr::null_mut());
            tgskk_runtime_free(transient_runtime);
            assert_eq!(message((*keys(retained, "k a")).committed), "か");
            tgskk_session_free(retained);
        }
    }

    #[test]
    fn learning_survives_save_reload_and_a_missing_live_file() {
        let fixture = Fixture::new();
        unsafe {
            keys(fixture.session, "K a n j i space");
            tgskk_session_select(fixture.session, 1);
            let mut error = ptr::null_mut();
            assert_eq!(tgskk_runtime_save(fixture.runtime, &mut error), 1);
            assert!(error.is_null());
            let reopened = tgskk_runtime_new(
                dictionary().as_ptr(),
                c"utf-8".as_ptr(),
                fixture.user.as_ptr(),
                &mut error,
            );
            let session = tgskk_session_new(reopened, &mut error);
            assert_eq!(
                message((*keys(session, "K a n j i space")).preedit),
                "▼感じ"
            );
            tgskk_session_free(session);
            tgskk_runtime_free(reopened);
            keys(fixture.session, "K a n j i space");
            let selected = message(tgskk_candidate_text(fixture.session, 1));
            tgskk_session_select(fixture.session, 1);
            let backup = format!("{}.BAK", fixture.user.to_str().unwrap());
            let previous_backup = std::fs::read(&backup).unwrap();
            std::fs::remove_file(fixture.user.to_str().unwrap()).unwrap();
            assert_eq!(tgskk_runtime_save(fixture.runtime, &mut error), 1);
            assert!(error.is_null());
            assert_eq!(std::fs::read(backup).unwrap(), previous_backup);
            let runtime = tgskk_runtime_new(
                dictionary().as_ptr(),
                c"utf-8".as_ptr(),
                fixture.user.as_ptr(),
                &mut error,
            );
            let session = tgskk_session_new(runtime, &mut error);
            assert_eq!(
                message((*keys(session, "K a n j i space")).preedit),
                format!("▼{selected}")
            );
            tgskk_session_free(session);
            tgskk_runtime_free(runtime);
        }
    }

    #[test]
    fn save_failure_keeps_the_live_file_and_pending_learning_for_retry() {
        let fixture = Fixture::new();
        unsafe {
            keys(fixture.session, "K a n j i space");
            tgskk_session_select(fixture.session, 1);
            let mut error = ptr::null_mut();
            assert_eq!(tgskk_runtime_save(fixture.runtime, &mut error), 1);
            let previous = std::fs::read(fixture.user.to_str().unwrap()).unwrap();
            keys(fixture.session, "K a n j i space");
            tgskk_session_select(fixture.session, 1);
            let backup = format!("{}.BAK", fixture.user.to_str().unwrap());
            std::fs::remove_file(&backup).unwrap();
            std::fs::create_dir(&backup).unwrap();
            assert_eq!(tgskk_runtime_save(fixture.runtime, &mut error), 0);
            assert!(!message(error).is_empty());
            tgskk_string_free(error);
            assert_eq!(
                std::fs::read(fixture.user.to_str().unwrap()).unwrap(),
                previous
            );
            std::fs::remove_dir(&backup).unwrap();
            assert_eq!(tgskk_runtime_save(fixture.runtime, &mut error), 1);
            assert!(error.is_null());
            let runtime = tgskk_runtime_new(
                dictionary().as_ptr(),
                c"utf-8".as_ptr(),
                fixture.user.as_ptr(),
                &mut error,
            );
            let session = tgskk_session_new(runtime, &mut error);
            assert_eq!(
                message((*keys(session, "K a n j i space")).preedit),
                "▼漢字"
            );
            tgskk_session_free(session);
            tgskk_runtime_free(runtime);
        }
    }

    #[test]
    fn invalid_handles_arguments_and_indices_fail_without_unwinding() {
        unsafe {
            let mut error = ptr::null_mut();
            assert!(
                tgskk_runtime_new(ptr::null(), c"utf-8".as_ptr(), ptr::null(), &mut error)
                    .is_null()
            );
            assert!(!message(error).is_empty());
            tgskk_string_free(error);
            let invalid = [0xff, 0];
            assert!(
                tgskk_runtime_new(
                    invalid.as_ptr().cast(),
                    c"utf-8".as_ptr(),
                    ptr::null(),
                    &mut error
                )
                .is_null()
            );
            tgskk_string_free(error);
            assert!(tgskk_session_new(ptr::null(), &mut error).is_null());
            tgskk_string_free(error);
            assert!(tgskk_session_reset(ptr::null_mut()).is_null());
            tgskk_session_free(ptr::null_mut());
            tgskk_runtime_free(ptr::null_mut());
            tgskk_string_free(ptr::null_mut());
        }
        let fixture = Fixture::new();
        unsafe {
            keys(fixture.session, "K a n j i space");
            let view = tgskk_session_choose(fixture.session, usize::MAX);
            assert_eq!((*view).consumed, 0);
            assert_eq!(message((*view).preedit), "▼漢字");
            let view = tgskk_session_select(fixture.session, usize::MAX);
            assert_eq!((*view).consumed, 0);
            assert_eq!(message((*view).preedit), "▼漢字");
        }
    }

    #[test]
    fn panic_is_contained_and_poisoned_session_cannot_be_reused() {
        let fixture = Fixture::new();
        unsafe {
            assert!(session_call(fixture.session, |_| panic!("injected failure")).is_null());
            assert!(message(tgskk_session_error(fixture.session)).contains("Rust panic"));
            assert!(tgskk_session_key(fixture.session, 97, 0, 0).is_null());
            assert!(tgskk_session_view(fixture.session).is_null());
        }
    }
    #[test]
    fn zenz_attachment_and_context_errors_preserve_ordinary_composition() {
        let fixture = Fixture::new();
        unsafe {
            let mut error = ptr::null_mut();
            let zenz = tgskk_zenz_new(
                c"/missing/tgskk-worker".as_ptr(),
                c"/missing/model".as_ptr(),
                &mut error,
            );
            assert!(!zenz.is_null());
            assert!(error.is_null());
            let before = keys(fixture.session, "K a n j i");
            assert_eq!(message((*before).preedit), "▽かんじ");
            assert_eq!(tgskk_session_set_zenz(fixture.session, zenz), 1);
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            while tgskk_zenz_status(zenz) != 2 {
                assert!(std::time::Instant::now() < deadline);
                std::thread::yield_now();
            }
            let diagnostic = tgskk_zenz_take_error(zenz);
            assert!(message(diagnostic).contains("Cannot open Zenz model"));
            tgskk_string_free(diagnostic);
            assert!(tgskk_zenz_take_error(zenz).is_null());
            assert!(tgskk_zenz_take_error(ptr::null()).is_null());
            tgskk_zenz_free(zenz);
            assert_eq!(
                tgskk_session_context(fixture.session, c"駅で".as_ptr(), 1),
                1
            );
            assert_eq!(tgskk_session_context(fixture.session, ptr::null(), 1), 0);
            assert_eq!(
                message(tgskk_session_error(fixture.session)),
                "Required string is NULL"
            );
            assert_eq!(tgskk_session_context(fixture.session, ptr::null(), 0), 1);
            let view = keys(fixture.session, "space");
            assert_eq!(message((*view).preedit), "▼漢字");
            assert_eq!((*view).zenz_applied, 0);
            assert_eq!(tgskk_session_set_zenz(fixture.session, ptr::null()), 1);
            assert_eq!(
                message((*tgskk_session_view(fixture.session)).preedit),
                "▼漢字"
            );
            let view = keys(fixture.session, "Return");
            assert_eq!(message((*view).committed), "漢字");
        }
    }
}
