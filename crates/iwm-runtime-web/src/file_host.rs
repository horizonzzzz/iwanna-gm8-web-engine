use std::path::Path;

use iwm_runtime_host::RuntimeHostError;

#[cfg(target_arch = "wasm32")]
pub(crate) fn read_file(path: &Path) -> Result<Option<Vec<u8>>, RuntimeHostError> {
    let path = path_to_string(path)?;
    let required_len =
        unsafe { iwm_host_read_file(path.as_ptr(), path.len(), std::ptr::null_mut(), 0) };
    if required_len < 0 {
        return Ok(None);
    }

    let mut bytes = vec![0u8; required_len as usize];
    let written =
        unsafe { iwm_host_read_file(path.as_ptr(), path.len(), bytes.as_mut_ptr(), bytes.len()) };
    if written < 0 {
        return Ok(None);
    }
    bytes.truncate(written as usize);
    Ok(Some(bytes))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn read_file(_path: &Path) -> Result<Option<Vec<u8>>, RuntimeHostError> {
    Ok(None)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn write_file(path: &Path, bytes: &[u8]) -> Result<bool, RuntimeHostError> {
    let path = path_to_string(path)?;
    let result =
        unsafe { iwm_host_write_file(path.as_ptr(), path.len(), bytes.as_ptr(), bytes.len()) };
    Ok(result != 0)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn write_file(_path: &Path, _bytes: &[u8]) -> Result<bool, RuntimeHostError> {
    Ok(false)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn remove_file(path: &Path) -> Result<bool, RuntimeHostError> {
    let path = path_to_string(path)?;
    let result = unsafe { iwm_host_remove_file(path.as_ptr(), path.len()) };
    Ok(result != 0)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn remove_file(_path: &Path) -> Result<bool, RuntimeHostError> {
    Ok(false)
}

#[cfg(target_arch = "wasm32")]
fn path_to_string(path: &Path) -> Result<String, RuntimeHostError> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| RuntimeHostError::invalid_input("runtime file path is not valid utf-8"))
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
extern "C" {
    fn iwm_host_read_file(
        path_ptr: *const u8,
        path_len: usize,
        out_ptr: *mut u8,
        out_len: usize,
    ) -> isize;
    fn iwm_host_write_file(
        path_ptr: *const u8,
        path_len: usize,
        bytes_ptr: *const u8,
        bytes_len: usize,
    ) -> i32;
    fn iwm_host_remove_file(path_ptr: *const u8, path_len: usize) -> i32;
}
