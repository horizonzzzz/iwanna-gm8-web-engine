use std::sync::{Mutex, OnceLock};

use crate::{WebInputState, WebRuntimeHost};
use crate::result_store::{
    last_result_len, read_utf8_from_ptr, store_error_result, store_json_result,
};

fn runtime_host() -> &'static Mutex<WebRuntimeHost> {
    static RUNTIME: OnceLock<Mutex<WebRuntimeHost>> = OnceLock::new();
    RUNTIME.get_or_init(|| Mutex::new(WebRuntimeHost::new()))
}

#[no_mangle]
pub extern "C" fn iwm_alloc(len: usize) -> *mut u8 {
    let mut bytes = Vec::<u8>::with_capacity(len);
    let pointer = bytes.as_mut_ptr();
    std::mem::forget(bytes);
    pointer
}

#[no_mangle]
pub extern "C" fn iwm_free(pointer: *mut u8, len: usize) {
    if pointer.is_null() {
        return;
    }

    unsafe {
        let _ = Vec::from_raw_parts(pointer, 0, len);
    }
}

#[no_mangle]
pub extern "C" fn iwm_last_result_len() -> usize {
    last_result_len()
}

#[no_mangle]
pub extern "C" fn iwm_boot_json(pointer: *const u8, len: usize) -> usize {
    let package_json = match read_utf8_from_ptr(pointer, len) {
        Ok(value) => value,
        Err(error) => return store_error_result(error),
    };

    let mut host = runtime_host().lock().expect("runtime host mutex poisoned");
    match host.boot_from_json(&package_json) {
        Ok(snapshot) => store_json_result(&snapshot),
        Err(error) => store_error_result(error),
    }
}

#[no_mangle]
pub extern "C" fn iwm_set_input_json(pointer: *const u8, len: usize) -> usize {
    let input_json = match read_utf8_from_ptr(pointer, len) {
        Ok(value) => value,
        Err(error) => return store_error_result(error),
    };

    let input = match serde_json::from_str::<WebInputState>(&input_json) {
        Ok(value) => value,
        Err(error) => return store_error_result(error.to_string()),
    };

    let mut host = runtime_host().lock().expect("runtime host mutex poisoned");
    host.set_input(input);
    match host.snapshot() {
        Some(snapshot) => store_json_result(&snapshot),
        None => store_error_result("runtime core is not booted".into()),
    }
}

#[no_mangle]
pub extern "C" fn iwm_tick(frames: u32) -> usize {
    let mut host = runtime_host().lock().expect("runtime host mutex poisoned");
    match host.tick(frames) {
        Ok(snapshot) => store_json_result(&snapshot),
        Err(error) => store_error_result(error),
    }
}

#[no_mangle]
pub extern "C" fn iwm_reset() -> usize {
    let mut host = runtime_host().lock().expect("runtime host mutex poisoned");
    match host.reset() {
        Ok(snapshot) => store_json_result(&snapshot),
        Err(error) => store_error_result(error),
    }
}

#[no_mangle]
pub extern "C" fn iwm_select_room(room_id: u32) -> usize {
    let mut host = runtime_host().lock().expect("runtime host mutex poisoned");
    match host.select_room(room_id as usize) {
        Ok(snapshot) => store_json_result(&snapshot),
        Err(error) => store_error_result(error),
    }
}

#[no_mangle]
pub extern "C" fn iwm_snapshot_json() -> usize {
    let host = runtime_host().lock().expect("runtime host mutex poisoned");
    match host.snapshot() {
        Some(snapshot) => store_json_result(&snapshot),
        None => store_error_result("runtime core is not booted".into()),
    }
}

#[no_mangle]
pub extern "C" fn iwm_frame_json() -> usize {
    let host = runtime_host().lock().expect("runtime host mutex poisoned");
    match host.frame_snapshot() {
        Ok(frame) => store_json_result(&frame),
        Err(error) => store_error_result(error),
    }
}

#[no_mangle]
pub extern "C" fn iwm_diagnostics_json() -> usize {
    let host = runtime_host().lock().expect("runtime host mutex poisoned");
    store_json_result(&host.diagnostics())
}
