use super::{TgskkRuntime, report};
use std::ffi::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use tgskk_core::input::{CskkDictionary, DictionaryUpdates, SaveVersion};

enum Command {
    Save(Vec<Option<DictionaryUpdates>>),
    Stop,
}
struct Completed {
    saved: Vec<(usize, SaveVersion)>,
    errors: Vec<String>,
}
pub(super) struct TgskkSaver {
    dictionaries: Vec<Arc<CskkDictionary>>,
    commands: mpsc::SyncSender<Command>,
    completed: mpsc::Receiver<Completed>,
    worker: Option<JoinHandle<()>>,
    pending: bool,
}
impl TgskkSaver {
    fn tick(&mut self) -> Result<(), String> {
        if self.pending {
            match self.completed.try_recv() {
                Ok(result) => {
                    self.pending = false;
                    for (index, version) in result.saved {
                        self.dictionaries[index]
                            .acknowledge_save(version)
                            .map_err(|e| e.to_string())?;
                    }
                    if !result.errors.is_empty() {
                        return Err(result.errors.join("; "));
                    }
                }
                Err(mpsc::TryRecvError::Empty) => return Ok(()),
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err("Dictionary writer stopped".into());
                }
            }
        }
        let updates: Vec<_> = self
            .dictionaries
            .iter()
            .map(|dict| dict.take_save_updates().map_err(|e| e.to_string()))
            .collect::<Result<_, _>>()?;
        if updates.iter().any(Option::is_some) {
            self.commands
                .try_send(Command::Save(updates))
                .map_err(|_| "Dictionary writer unavailable")?;
            self.pending = true;
        }
        Ok(())
    }
}
impl Drop for TgskkSaver {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        for dictionary in &self.dictionaries {
            dictionary.stop_save_stream();
        }
    }
}
#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_runtime_saver_new(
    runtime: *const TgskkRuntime,
    error: *mut *mut c_char,
) -> *mut TgskkSaver {
    unsafe {
        report(error, || {
            let runtime = runtime.as_ref().ok_or("Runtime is NULL")?;
            let mut dictionaries: Vec<Arc<CskkDictionary>> = Vec::new();
            let mut seeds = Vec::new();
            for dictionary in &runtime.dictionaries {
                match dictionary.start_save_stream() {
                    Ok(Some(seed)) => {
                        dictionaries.push(dictionary.clone());
                        seeds.push(seed);
                    }
                    Ok(None) => {}
                    Err(error) => {
                        for dictionary in &dictionaries {
                            dictionary.stop_save_stream();
                        }
                        return Err(error.to_string());
                    }
                }
            }
            let (commands, receive) = mpsc::sync_channel(1);
            let (send, completed) = mpsc::sync_channel(1);
            let worker = std::thread::Builder::new()
                .name("tgskk-dictionary-writer".into())
                .spawn(move || {
                    let mut writers: Vec<_> = seeds.into_iter().map(|seed| seed.writer()).collect();
                    while let Ok(command) = receive.recv() {
                        let Command::Save(updates) = command else {
                            break;
                        };
                        let mut result = Completed {
                            saved: Vec::new(),
                            errors: Vec::new(),
                        };
                        for (index, (writer, updates)) in
                            writers.iter_mut().zip(updates).enumerate()
                        {
                            if let Some(updates) = updates {
                                let version = updates.version();
                                match writer.apply_and_save(updates) {
                                    Ok(()) => result.saved.push((index, version)),
                                    Err(error) => result.errors.push(error.to_string()),
                                }
                            }
                        }
                        if send.send(result).is_err() {
                            break;
                        }
                    }
                });
            let worker = match worker {
                Ok(worker) => worker,
                Err(error) => {
                    for dictionary in &dictionaries {
                        dictionary.stop_save_stream();
                    }
                    return Err(error.to_string());
                }
            };
            Ok(Box::into_raw(Box::new(TgskkSaver {
                dictionaries,
                commands,
                completed,
                worker: Some(worker),
                pending: false,
            })))
        })
    }
    .unwrap_or(ptr::null_mut())
}
#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_saver_tick(saver: *mut TgskkSaver, error: *mut *mut c_char) -> u8 {
    unsafe { u8::from(report(error, || saver.as_mut().ok_or("Saver is NULL")?.tick()).is_some()) }
}
#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_saver_idle(saver: *const TgskkSaver) -> u8 {
    unsafe { saver.as_ref() }.map_or(0, |s| {
        u8::from(
            !s.pending
                && s.dictionaries
                    .iter()
                    .all(|dict| matches!(dict.has_unsaved_changes(), Ok(false))),
        )
    })
}
#[unsafe(no_mangle)]
unsafe extern "C" fn tgskk_saver_free(saver: *mut TgskkSaver) {
    if !saver.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| unsafe { drop(Box::from_raw(saver)) }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use std::ffi::CString;
    use std::time::{Duration, Instant};
    fn key(session: *mut TgskkSession, text: &str) {
        for c in text.chars() {
            let modifiers = if c.is_ascii_uppercase() { 1 } else { 0 };
            assert!(!unsafe { tgskk_session_key(session, c as u32, modifiers, 0) }.is_null());
        }
    }
    fn finish(saver: *mut TgskkSaver) {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let mut error = ptr::null_mut();
            assert_eq!(unsafe { tgskk_saver_tick(saver, &mut error) }, 1);
            assert!(error.is_null());
            if unsafe { tgskk_saver_idle(saver) } != 0 {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
    }
    #[test]
    fn asynchronous_save_keeps_new_learning_and_retries_disk_errors() {
        let directory = tempfile::tempdir().unwrap();
        let user = directory.path().join("user");
        let user_arg = CString::new(user.to_str().unwrap()).unwrap();
        let dictionary = CString::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/SKK-JISYO.demo.utf8"
        ))
        .unwrap();
        let encoding = CString::new("utf-8").unwrap();
        let mut error = ptr::null_mut();
        let runtime = unsafe {
            tgskk_runtime_new(
                dictionary.as_ptr(),
                encoding.as_ptr(),
                user_arg.as_ptr(),
                &mut error,
            )
        };
        assert!(!runtime.is_null());
        let saver = unsafe { tgskk_runtime_saver_new(runtime, &mut error) };
        assert!(!saver.is_null());
        let session = unsafe { tgskk_session_new(runtime, &mut error) };
        finish(saver);
        key(session, "Kanji ");
        unsafe {
            tgskk_session_key(session, 0xff0d, 0, 0);
        }
        assert_eq!(unsafe { tgskk_saver_idle(saver) }, 0);
        assert_eq!(unsafe { tgskk_saver_tick(saver, &mut error) }, 1);
        // Learning after the submitted batch must survive its acknowledgement.
        key(session, "Kisha ");
        unsafe {
            tgskk_session_key(session, 0xff0d, 0, 0);
        }
        finish(saver);
        let saved = std::fs::read_to_string(&user).unwrap();
        assert!(saved.contains("かんじ /漢字;文字/"), "{saved}");
        assert!(saved.contains("きしゃ /記者/"), "{saved}");
        // A directory at the backup path makes the atomic save fail.
        let backup = user.with_file_name("user.BAK");
        std::fs::remove_file(&backup).unwrap();
        std::fs::create_dir(&backup).unwrap();
        key(session, "Kisha  ");
        unsafe {
            tgskk_session_key(session, 0xff0d, 0, 0);
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let success = unsafe { tgskk_saver_tick(saver, &mut error) };
            if success == 0 {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(!error.is_null());
        unsafe {
            tgskk_string_free(error);
        }
        assert_eq!(std::fs::read_to_string(&user).unwrap(), saved);
        std::fs::remove_dir(&backup).unwrap();
        finish(saver);
        assert!(
            std::fs::read_to_string(&user)
                .unwrap()
                .contains("きしゃ /汽車/記者/")
        );
        unsafe {
            tgskk_runtime_free(runtime);
            tgskk_session_free(session);
            tgskk_saver_free(saver);
        }
    }
}
