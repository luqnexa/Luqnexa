// SPDX-License-Identifier: AGPL-3.0-only
//! Bounded, opt-in C ABI for the Rust int8 model; no paths or network access.
use crate::{INTERNAL_ERROR, INVALID_ARGUMENT, INVALID_HANDLE, boundary, input};
use luqnexa_ml::{MODEL_BYTES, Model};
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Default)]
struct Models {
    next: u64,
    entries: Vec<(u64, Arc<Model>)>,
}
static MODELS: OnceLock<Mutex<Models>> = OnceLock::new();
fn registry() -> &'static Mutex<Models> {
    MODELS.get_or_init(|| Mutex::new(Models::default()))
}

/// Load at most four models concurrently. Model bytes are copied, not retained.
/// The host must validate the source and signature before loading bytes.
/// # Safety
/// `bytes` reads `length` bytes; `out_handle` points to a writable u64.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn luqnexa_model_create(
    bytes: *const u8,
    length: usize,
    out_handle: *mut u64,
) -> i32 {
    boundary(|| {
        if out_handle.is_null() {
            return Err(INVALID_ARGUMENT);
        }
        unsafe { out_handle.write(0) };
        if bytes.is_null() || length != MODEL_BYTES {
            return Err(INVALID_ARGUMENT);
        }
        let mut models = registry().lock().map_err(|_| INTERNAL_ERROR)?;
        if models.entries.len() == 4 {
            return Err(INVALID_ARGUMENT);
        }
        let model = Model::from_bytes(unsafe { std::slice::from_raw_parts(bytes, length) })
            .map_err(|_| INVALID_ARGUMENT)?;
        let handle = models.next.checked_add(1).ok_or(INTERNAL_ERROR)?;
        models.next = handle;
        models.entries.push((handle, Arc::new(model)));
        unsafe { out_handle.write(handle) };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn luqnexa_model_destroy(handle: u64) -> i32 {
    boundary(|| {
        let mut models = registry().lock().map_err(|_| INTERNAL_ERROR)?;
        let index = models
            .entries
            .iter()
            .position(|(id, _)| *id == handle)
            .ok_or(INVALID_HANDLE)?;
        models.entries.swap_remove(index);
        Ok(())
    })
}

/// Score one existing candidate, never generate, alter, or commit text.
/// Input spans are UTF-8. Reading <=256 bytes, context <=4096 bytes,
/// surface <=384 bytes; model uses 64/last48/96 Unicode scalar values.
/// Failed calls leave `out_score` unchanged. No heap allocation after loading.
/// # Safety
/// Input spans must be readable and out_score writable and nonoverlapping.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn luqnexa_model_score(
    handle: u64,
    reading: *const u8,
    reading_length: usize,
    context: *const u8,
    context_length: usize,
    surface: *const u8,
    surface_length: usize,
    out_score: *mut f32,
) -> i32 {
    boundary(|| {
        if out_score.is_null() {
            return Err(INVALID_ARGUMENT);
        }
        let model = registry()
            .lock()
            .map_err(|_| INTERNAL_ERROR)?
            .entries
            .iter()
            .find(|(id, _)| *id == handle)
            .map(|(_, m)| Arc::clone(m))
            .ok_or(INVALID_HANDLE)?;
        let reading = unsafe { input(reading, reading_length, 256)? };
        let context = unsafe { input(context, context_length, 4096)? };
        let surface = unsafe { input(surface, surface_length, 384)? };
        let prepared = model.prepare(reading, context);
        let score = model.score(&prepared, surface);
        unsafe { out_score.write(score) };
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{INPUT_LIMIT, INVALID_UTF8, OK};
    #[test]
    fn c_model_lifecycle_utf8_bounds_and_scores_match_rust() {
        let model = luqnexa_ml::Trainer::new(1337).quantize();
        let bytes = model.to_bytes();
        let mut handle = 0;
        assert_eq!(
            unsafe { luqnexa_model_create(bytes.as_ptr(), bytes.len(), &mut handle) },
            OK
        );
        let reading = "pythondeaiwotsukuru";
        let context = "プログラミング";
        let surface = "PythonでAIを作る";
        let mut score = 99.0;
        assert_eq!(
            unsafe {
                luqnexa_model_score(
                    handle,
                    reading.as_ptr(),
                    reading.len(),
                    context.as_ptr(),
                    context.len(),
                    surface.as_ptr(),
                    surface.len(),
                    &mut score,
                )
            },
            OK
        );
        assert_eq!(
            score,
            model.score(&model.prepare(reading, context), surface)
        );
        assert_eq!(
            unsafe {
                luqnexa_model_score(
                    handle,
                    [255].as_ptr(),
                    1,
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    0,
                    &mut score,
                )
            },
            INVALID_UTF8
        );
        let unchanged = score;
        assert_eq!(
            unsafe {
                luqnexa_model_score(
                    handle,
                    reading.as_ptr(),
                    257,
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    0,
                    &mut score,
                )
            },
            INPUT_LIMIT
        );
        assert_eq!(score, unchanged);
        assert_eq!(luqnexa_model_destroy(handle), OK);
        assert_eq!(luqnexa_model_destroy(handle), INVALID_HANDLE);
        assert_eq!(
            unsafe {
                luqnexa_model_score(
                    handle,
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    0,
                    &mut score,
                )
            },
            INVALID_HANDLE
        );
        assert_eq!(
            unsafe { luqnexa_model_create(std::ptr::null(), bytes.len(), &mut handle) },
            INVALID_ARGUMENT
        );
        assert_eq!(handle, 0);
        let mut corrupt = bytes.clone();
        corrupt[0] ^= 1;
        assert_eq!(
            unsafe { luqnexa_model_create(corrupt.as_ptr(), corrupt.len(), &mut handle) },
            INVALID_ARGUMENT
        );
        assert_eq!(handle, 0);
        let mut loaded = [0; 4];
        for id in &mut loaded {
            assert_eq!(
                unsafe { luqnexa_model_create(bytes.as_ptr(), bytes.len(), id) },
                OK
            );
        }
        assert_eq!(
            unsafe { luqnexa_model_create(bytes.as_ptr(), bytes.len(), &mut handle) },
            INVALID_ARGUMENT
        );
        assert_eq!(handle, 0);
        for id in loaded {
            assert_eq!(luqnexa_model_destroy(id), OK);
        }
    }
}
