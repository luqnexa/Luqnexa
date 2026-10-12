// SPDX-License-Identifier: AGPL-3.0-only
//! C ABI v1: opaque integer handles, caller-owned buffers, explicit key events.
//! All pointers must designate valid memory for their declared lengths. Handles
//! are validated; fabricated pointers cannot be validated by an in-process ABI.
use nextime_core::{Engine, Key, Session, SessionError};
use std::{
    collections::BTreeMap,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Mutex, OnceLock},
};

pub const OK: i32 = 0;
pub const INVALID_ARGUMENT: i32 = 1;
pub const INVALID_HANDLE: i32 = 2;
pub const INVALID_UTF8: i32 = 3;
pub const BUFFER_TOO_SMALL: i32 = 4;
pub const INPUT_LIMIT: i32 = 5;
pub const DICTIONARY_ERROR: i32 = 6;
pub const INTERNAL_ERROR: i32 = 7;

#[derive(Debug)]
struct State {
    session: Session,
    committed: String,
}
#[derive(Default)]
struct Registry {
    next: u64,
    sessions: BTreeMap<u64, Arc<Mutex<State>>>,
}
static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
fn registry() -> &'static Mutex<Registry> {
    REGISTRY.get_or_init(|| Mutex::new(Registry::default()))
}

fn boundary(f: impl FnOnce() -> Result<(), i32>) -> i32 {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => OK,
        Ok(Err(code)) => code,
        Err(_) => INTERNAL_ERROR,
    }
}
fn state(handle: u64) -> Result<Arc<Mutex<State>>, i32> {
    registry()
        .lock()
        .map_err(|_| INTERNAL_ERROR)?
        .sessions
        .get(&handle)
        .cloned()
        .ok_or(INVALID_HANDLE)
}
fn session_error(error: SessionError) -> i32 {
    match error {
        SessionError::Conversion(nextime_core::ConvertError::InputTooLong) => INPUT_LIMIT,
        SessionError::Conversion(nextime_core::ConvertError::Dictionary(_)) => DICTIONARY_ERROR,
        _ => INVALID_ARGUMENT,
    }
}

unsafe fn input<'a>(bytes: *const u8, length: usize, max: usize) -> Result<&'a str, i32> {
    if length > max {
        return Err(INPUT_LIMIT);
    }
    if length == 0 {
        return Ok("");
    }
    if bytes.is_null() {
        return Err(INVALID_ARGUMENT);
    }
    // SAFETY: the caller guarantees a readable span; bounds are checked above.
    std::str::from_utf8(unsafe { std::slice::from_raw_parts(bytes, length) })
        .map_err(|_| INVALID_UTF8)
}

#[unsafe(no_mangle)]
pub extern "C" fn nextime_abi_version() -> u32 {
    1
}

/// Create with the bundled dictionary (path_length=0), or a UTF-8 NUD path.
/// # Safety
/// `out_handle` is writable; `path` is readable for `path_length` bytes when nonzero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nextime_session_create(
    path: *const u8,
    path_length: usize,
    limit: u32,
    out_handle: *mut u64,
) -> i32 {
    boundary(|| {
        if out_handle.is_null() {
            return Err(INVALID_ARGUMENT);
        }
        // SAFETY: caller provides a writable u64.
        unsafe {
            out_handle.write(0);
        }
        if !(1..=32).contains(&limit) {
            return Err(INVALID_ARGUMENT);
        }
        let path = unsafe { input(path, path_length, 4096)? };
        if path.contains('\0') {
            return Err(INVALID_ARGUMENT);
        }
        let engine = if path.is_empty() {
            Engine::bundled()
        } else {
            Engine::new(nextime_core::WithCommonPredictions::new(
                nexc_lite::FileDictionary::open_with_cache(path, 32)
                    .map_err(|_| DICTIONARY_ERROR)?,
            ))
        };
        let session = Session::new(engine, limit as usize).map_err(session_error)?;
        let mut registry = registry().lock().map_err(|_| INTERNAL_ERROR)?;
        let handle = registry.next.checked_add(1).ok_or(INTERNAL_ERROR)?;
        registry.next = handle;
        registry.sessions.insert(
            handle,
            Arc::new(Mutex::new(State {
                session,
                committed: String::new(),
            })),
        );
        unsafe {
            out_handle.write(handle);
        }
        Ok(())
    })
}

/// Invalid and already destroyed handles return INVALID_HANDLE.
#[unsafe(no_mangle)]
pub extern "C" fn nextime_session_destroy(handle: u64) -> i32 {
    boundary(|| {
        registry()
            .lock()
            .map_err(|_| INTERNAL_ERROR)?
            .sessions
            .remove(&handle)
            .map(|_| ())
            .ok_or(INVALID_HANDLE)
    })
}

/// Keys: 1=text, 2=backspace, 3=convert, 4=next, 5=previous, 6=enter, 7=cancel.
/// Flags: bit0=handled, bit1=committed. Failed events leave composition unchanged.
/// # Safety
/// `text` is readable for `length` bytes when nonzero; `flags` is a writable u32.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nextime_session_event(
    handle: u64,
    key: u32,
    text: *const u8,
    length: usize,
    flags: *mut u32,
) -> i32 {
    boundary(|| {
        if flags.is_null() {
            return Err(INVALID_ARGUMENT);
        }
        unsafe {
            flags.write(0);
        }
        if key != 1 && length != 0 {
            return Err(INVALID_ARGUMENT);
        }
        let key = match key {
            1 => Key::Text(unsafe { input(text, length, 512)? }),
            2 => Key::Backspace,
            3 => Key::Convert,
            4 => Key::Next,
            5 => Key::Previous,
            6 => Key::Enter,
            7 => Key::Cancel,
            8 => Key::Literal,
            9 => Key::ToggleMode,
            11 => Key::ToggleAi,
            12 => Key::ClearContext,
            13 => Key::ClearLearning,
            14 => Key::ToggleTraining,
            15 => Key::PollAi,
            10 => Key::Space,
            _ => return Err(INVALID_ARGUMENT),
        };
        let state = state(handle)?;
        let mut state = state.lock().map_err(|_| INTERNAL_ERROR)?;
        let outcome = state.session.event(key).map_err(session_error)?;
        let bits = u32::from(outcome.handled) | if outcome.committed.is_some() { 2 } else { 0 };
        state.committed = outcome.committed.unwrap_or_default();
        unsafe {
            flags.write(bits);
        }
        Ok(())
    })
}

/// # Safety
/// `count` and `selected` are writable u32 pointers. selected=u32::MAX when absent.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nextime_session_candidates(
    handle: u64,
    count: *mut u32,
    selected: *mut u32,
) -> i32 {
    boundary(|| {
        if count.is_null() || selected.is_null() {
            return Err(INVALID_ARGUMENT);
        }
        unsafe {
            count.write(0);
            selected.write(u32::MAX);
        }
        let state = state(handle)?;
        let state = state.lock().map_err(|_| INTERNAL_ERROR)?;
        unsafe {
            count.write(state.session.candidates().len() as u32);
            selected.write(
                state
                    .session
                    .selected_index()
                    .map_or(u32::MAX, |i| i as u32),
            );
        }
        Ok(())
    })
}

/// Copy UTF-8 with trailing NUL. kinds: 1=raw, 2=preedit, 3=display, 4=candidate,
/// 5=last committed text. Index applies only to kind4. required includes NUL.
/// Query size with buffer=NULL/capacity=0; returns BUFFER_TOO_SMALL.
/// # Safety
/// `required` is writable; non-null buffer is writable for capacity bytes.
/// Output pointers must not overlap each other or internal/input memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nextime_session_text(
    handle: u64,
    kind: u32,
    index: u32,
    buffer: *mut u8,
    capacity: usize,
    required: *mut usize,
) -> i32 {
    boundary(|| {
        if required.is_null() || (buffer.is_null() && capacity != 0) {
            return Err(INVALID_ARGUMENT);
        }
        unsafe {
            required.write(0);
        }
        let state = state(handle)?;
        let state = state.lock().map_err(|_| INTERNAL_ERROR)?;
        let text = match kind {
            1 => state.session.raw(),
            2 => state.session.preedit(),
            3 => state.session.display(),
            4 => {
                &state
                    .session
                    .candidates()
                    .get(index as usize)
                    .ok_or(INVALID_ARGUMENT)?
                    .surface
            }
            5 => &state.committed,
            9 => state.session.local_ai_status(),
            7 => {
                if state.session.ai_enabled() {
                    "AI:on"
                } else {
                    "AI:off"
                }
            }
            8 => {
                if state.session.text_learning_enabled() && state.session.ai_enabled() {
                    "学習:on"
                } else {
                    "学習:off"
                }
            }
            6 => {
                if state.session.english_mode() {
                    "English"
                } else {
                    "日本語"
                }
            }
            _ => return Err(INVALID_ARGUMENT),
        };
        let needed = text.len() + 1;
        unsafe {
            required.write(needed);
        }
        if capacity < needed {
            return Err(BUFFER_TOO_SMALL);
        }
        // SAFETY: sufficient caller-owned non-null buffer was checked above.
        unsafe {
            std::ptr::copy_nonoverlapping(text.as_ptr(), buffer, text.len());
            buffer.add(text.len()).write(0);
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    unsafe fn text(handle: u64, kind: u32) -> String {
        let mut required = 0;
        assert_eq!(
            unsafe {
                nextime_session_text(handle, kind, 0, std::ptr::null_mut(), 0, &mut required)
            },
            BUFFER_TOO_SMALL
        );
        let mut bytes = vec![0; required];
        assert_eq!(
            unsafe {
                nextime_session_text(
                    handle,
                    kind,
                    0,
                    bytes.as_mut_ptr(),
                    bytes.len(),
                    &mut required,
                )
            },
            OK
        );
        bytes.pop();
        String::from_utf8(bytes).unwrap()
    }
    #[test]
    fn abi_lifecycle_buffers_and_explicit_commit() {
        // SAFETY: test pointers reference live, sufficiently sized stack/Vec storage.
        unsafe {
            let mut handle = 0;
            assert_eq!(
                nextime_session_create(std::ptr::null(), 0, 8, &mut handle),
                OK
            );
            let mut flags = 0;
            assert_eq!(
                nextime_session_event(handle, 1, b"hashi".as_ptr(), 5, &mut flags),
                OK
            );
            assert_eq!(flags, 1);
            assert_eq!(text(handle, 2), "はし");
            assert_eq!(
                nextime_session_event(handle, 3, std::ptr::null(), 0, &mut flags),
                OK
            );
            assert_eq!(text(handle, 3), "橋");
            assert_eq!(text(handle, 5), "");
            let (mut count, mut selected) = (0, 0);
            assert_eq!(
                nextime_session_candidates(handle, &mut count, &mut selected),
                OK
            );
            assert_eq!((count, selected), (4, 0));
            assert_eq!(
                nextime_session_event(handle, 4, std::ptr::null(), 0, &mut flags),
                OK
            );
            assert_eq!(
                nextime_session_event(handle, 6, std::ptr::null(), 0, &mut flags),
                OK
            );
            assert_eq!(flags, 3);
            assert_eq!(text(handle, 5), "箸");
            let mut bytes = [77; 2];
            let mut required = 0;
            assert_eq!(
                nextime_session_text(handle, 5, 0, bytes.as_mut_ptr(), 2, &mut required),
                BUFFER_TOO_SMALL
            );
            assert_eq!(bytes, [77; 2]);
            assert_eq!(required, "箸".len() + 1);
            assert_eq!(nextime_session_destroy(handle), OK);
            assert_eq!(nextime_session_destroy(handle), INVALID_HANDLE);
            assert_eq!(
                nextime_session_event(handle, 7, std::ptr::null(), 0, &mut flags),
                INVALID_HANDLE
            );
        }
    }
    #[test]
    fn bad_arguments_utf8_limits_and_handles() {
        unsafe {
            let mut handle = 0;
            assert_eq!(
                nextime_session_create(std::ptr::null(), 0, 0, &mut handle),
                INVALID_ARGUMENT
            );
            assert_eq!(handle, 0);
            assert_eq!(
                nextime_session_create(std::ptr::null(), 0, 8, std::ptr::null_mut()),
                INVALID_ARGUMENT
            );
            assert_eq!(
                nextime_session_create(std::ptr::null(), 0, 8, &mut handle),
                OK
            );
            let mut flags = 0;
            assert_eq!(
                nextime_session_event(handle, 1, [255].as_ptr(), 1, &mut flags),
                INVALID_UTF8
            );
            assert_eq!(
                nextime_session_event(handle, 1, std::ptr::null(), 1, &mut flags),
                INVALID_ARGUMENT
            );
            assert_eq!(
                nextime_session_event(handle, 1, std::ptr::null(), 513, &mut flags),
                INPUT_LIMIT
            );
            assert_eq!(
                nextime_session_event(handle, 123, std::ptr::null(), 0, &mut flags),
                INVALID_ARGUMENT
            );
            assert_eq!(text(handle, 1), "");
            assert_eq!(nextime_session_destroy(handle), OK);
            assert_eq!(boundary(|| panic!("test boundary")), INTERNAL_ERROR);
        }
    }

    #[test]
    fn ffi_opens_compressed_dictionary() {
        let path =
            std::env::temp_dir().join(format!("nextime-ffi-test-{}.nud", std::process::id()));
        nexc_max::build_dictionary(&path, &nextime_core::Dictionary::bundled(), 1).unwrap();
        let path_text = path.to_str().unwrap();
        unsafe {
            let mut handle = 0;
            let mut flags = 0;
            assert_eq!(
                nextime_session_create(path_text.as_ptr(), path_text.len(), 8, &mut handle),
                OK
            );
            assert_eq!(
                nextime_session_event(handle, 1, b"nihongo".as_ptr(), 7, &mut flags),
                OK
            );
            assert_eq!(
                nextime_session_event(handle, 3, std::ptr::null(), 0, &mut flags),
                OK
            );
            assert_eq!(text(handle, 3), "日本語");
            assert_eq!(nextime_session_destroy(handle), OK);
        }
        std::fs::remove_file(path).unwrap();
    }
}
