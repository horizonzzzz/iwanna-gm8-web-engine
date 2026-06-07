use std::sync::{Mutex, OnceLock};

use serde::Serialize;

fn last_result_bytes() -> &'static Mutex<Vec<u8>> {
    static LAST_RESULT: OnceLock<Mutex<Vec<u8>>> = OnceLock::new();
    LAST_RESULT.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn store_result(result: String) -> usize {
    let mut bytes = last_result_bytes()
        .lock()
        .expect("last result mutex poisoned");
    *bytes = result.into_bytes();
    bytes.as_ptr() as usize
}

pub fn store_json_result<T: Serialize>(value: &T) -> usize {
    store_result(serde_json::to_string(value).unwrap_or_else(|error| {
        format!(r#"{{"error":"failed to encode bridge result: {}"}}"#, error)
    }))
}

pub fn store_error_result(message: String) -> usize {
    store_result(format!(
        r#"{{"error":"{}"}}"#,
        message.replace('\\', "\\\\").replace('"', "\\\"")
    ))
}

pub fn read_utf8_from_ptr(pointer: *const u8, len: usize) -> Result<String, String> {
    if pointer.is_null() {
        return Err("received null pointer for JSON payload".into());
    }

    let bytes = unsafe { std::slice::from_raw_parts(pointer, len) };
    std::str::from_utf8(bytes)
        .map(|text| text.to_owned())
        .map_err(|error| error.to_string())
}

pub fn last_result_len() -> usize {
    last_result_bytes()
        .lock()
        .expect("last result mutex poisoned")
        .len()
}
