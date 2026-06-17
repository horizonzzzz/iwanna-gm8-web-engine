use std::sync::{Mutex, OnceLock};

use iwm_runtime_core::RuntimeValue;
use serde_json::Value;

use crate::result_store::{
    last_result_len, read_utf8_from_ptr, store_error_result, store_json_result,
};
use crate::{WebInputState, WebRuntimeHost};

fn parse_web_input_state(pointer: *const u8, len: usize) -> Result<WebInputState, String> {
    let input_json = read_utf8_from_ptr(pointer, len)?;
    serde_json::from_str::<WebInputState>(&input_json).map_err(|error| error.to_string())
}

fn parse_global_overrides(
    pointer: *const u8,
    len: usize,
) -> Result<Vec<(String, RuntimeValue)>, String> {
    let globals_json = read_utf8_from_ptr(pointer, len)?;
    let parsed = serde_json::from_str::<std::collections::HashMap<String, Value>>(&globals_json)
        .map_err(|error| error.to_string())?;
    parsed
        .into_iter()
        .map(|(key, value)| {
            let runtime_value = match value {
                Value::Number(number) => number
                    .as_f64()
                    .map(RuntimeValue::Number)
                    .ok_or_else(|| format!("global override {key} is not a finite number"))?,
                Value::Bool(value) => RuntimeValue::Bool(value),
                Value::String(value) => RuntimeValue::Text(value),
                _ => {
                    return Err(format!(
                        "global override {key} must be a number, boolean, or string"
                    ))
                }
            };
            Ok((key, runtime_value))
        })
        .collect()
}

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
    let input = match parse_web_input_state(pointer, len) {
        Ok(value) => value,
        Err(error) => return store_error_result(error),
    };

    let mut host = runtime_host().lock().expect("runtime host mutex poisoned");
    host.set_input(input);
    match host.snapshot() {
        Some(snapshot) => store_json_result(&snapshot),
        None => store_error_result("runtime core is not booted".into()),
    }
}

#[no_mangle]
pub extern "C" fn iwm_step_json(pointer: *const u8, len: usize) -> usize {
    let input = match parse_web_input_state(pointer, len) {
        Ok(value) => value,
        Err(error) => return store_error_result(error),
    };

    let mut host = runtime_host().lock().expect("runtime host mutex poisoned");
    match host.step(input) {
        Ok(result) => store_json_result(&result),
        Err(error) => store_error_result(error),
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
pub extern "C" fn iwm_set_globals_json(pointer: *const u8, len: usize) -> usize {
    let globals = match parse_global_overrides(pointer, len) {
        Ok(value) => value,
        Err(error) => return store_error_result(error),
    };

    let mut host = runtime_host().lock().expect("runtime host mutex poisoned");
    let mut snapshot = None;
    for (key, value) in globals {
        match host.set_global(key, value) {
            Ok(next_snapshot) => snapshot = Some(next_snapshot),
            Err(error) => return store_error_result(error),
        }
    }
    match snapshot.or_else(|| host.snapshot()) {
        Some(snapshot) => store_json_result(&snapshot),
        None => store_error_result("runtime core is not booted".into()),
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
