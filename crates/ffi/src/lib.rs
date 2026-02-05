// this_file: crates/ffi/src/lib.rs

//! C-compatible FFI bindings for Vexy SVGO
//! This module provides a C-compatible API for the Vexy SVGO SVG optimizer,
//! allowing integration with C, C++, and other languages that can call C functions.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;
use vexy_svgo_core::{Config, optimize_with_config};

/// Error codes for FFI operations
#[repr(C)]
pub enum VexyErrorCode {
    Success = 0,
    InvalidInput = 1,
    ParseError = 2,
    OptimizationError = 3,
    MemoryError = 4,
    ConfigError = 5,
}

/// FFI-compatible optimization result
#[repr(C)]
pub struct VexyResult {
    pub error_code: VexyErrorCode,
    pub data: *mut c_char,
    pub data_length: usize,
    pub original_size: usize,
    pub optimized_size: usize,
    pub error_message: *mut c_char,
}

impl Default for VexyResult {
    fn default() -> Self {
        Self {
            error_code: VexyErrorCode::Success,
            data: ptr::null_mut(),
            data_length: 0,
            original_size: 0,
            optimized_size: 0,
            error_message: ptr::null_mut(),
        }
    }
}



/// Create a new FFI result with error
fn create_error_result(code: VexyErrorCode, message: &str) -> VexyResult {
    let error_message = match CString::new(message) {
        Ok(s) => s.into_raw(),
        Err(_) => ptr::null_mut(),
    };

    VexyResult {
        error_code: code,
        data: ptr::null_mut(),
        data_length: 0,
        original_size: 0,
        optimized_size: 0,
        error_message,
    }
}

/// Create a success result with optimized data
fn create_success_result(data: String, original_size: usize) -> VexyResult {
    let optimized_size = data.len();
    match CString::new(data) {
        Ok(c_string) => {
            let raw_ptr = c_string.into_raw();
            VexyResult {
                error_code: VexyErrorCode::Success,
                data: raw_ptr,
                data_length: optimized_size,
                original_size,
                optimized_size,
                error_message: ptr::null_mut(),
            }
        }
        Err(_) => create_error_result(
            VexyErrorCode::MemoryError,
            "Failed to convert result to C string",
        ),
    }
}

/// Optimize SVG with default configuration
///
/// # Safety
///
/// The `svg_input` pointer must be valid and point to a null-terminated string.
/// The caller is responsible for freeing the returned result using `vexy_svgo_free_result`.
#[no_mangle]
pub unsafe extern "C" fn vexy_svgo_optimize_default(svg_input: *const c_char) -> VexyResult {
    if svg_input.is_null() {
        return create_error_result(VexyErrorCode::InvalidInput, "Input SVG is null");
    }

    let svg_str = match CStr::from_ptr(svg_input).to_str() {
        Ok(s) => s,
        Err(_) => {
            return create_error_result(
                VexyErrorCode::InvalidInput,
                "Invalid UTF-8 in input SVG",
            )
        }
    };

    let original_size = svg_str.len();
    let config = Config::with_default_preset();

    match optimize_with_config(svg_str, config) {
        Ok(result) => create_success_result(result.data, original_size),
        Err(e) => create_error_result(VexyErrorCode::OptimizationError, &e.to_string()),
    }
}

/// Optimize SVG with JSON configuration
///
/// # Safety
///
/// Both `svg_input` and `config_json` pointers must be valid and point to null-terminated strings.
/// The caller is responsible for freeing the returned result using `vexy_svgo_free_result`.
#[no_mangle]
pub unsafe extern "C" fn vexy_svgo_optimize_with_config(
    svg_input: *const c_char,
    config_json: *const c_char,
) -> VexyResult {
    if svg_input.is_null() {
        return create_error_result(VexyErrorCode::InvalidInput, "Input SVG is null");
    }

    let svg_str = match CStr::from_ptr(svg_input).to_str() {
        Ok(s) => s,
        Err(_) => {
            return create_error_result(
                VexyErrorCode::InvalidInput,
                "Invalid UTF-8 in input SVG",
            )
        }
    };

    let original_size = svg_str.len();

    // Parse configuration
    let config = if config_json.is_null() {
        Config::with_default_preset()
    } else {
        let config_str = match CStr::from_ptr(config_json).to_str() {
            Ok(s) => s,
            Err(_) => {
                return create_error_result(
                    VexyErrorCode::InvalidInput,
                    "Invalid UTF-8 in config JSON",
                )
            }
        };

        match serde_json::from_str::<Config>(config_str) {
            Ok(c) => c,
            Err(e) => {
                return create_error_result(
                    VexyErrorCode::ConfigError,
                    &format!("Invalid config JSON: {e}"),
                )
            }
        }
    };

    match optimize_with_config(svg_str, config) {
        Ok(result) => create_success_result(result.data, original_size),
        Err(e) => create_error_result(VexyErrorCode::OptimizationError, &e.to_string()),
    }
}

/// Get the version of the Vexy SVGO library
///
/// # Safety
///
/// The caller is responsible for freeing the returned string using `vexy_svgo_free_string`.
#[no_mangle]
pub unsafe extern "C" fn vexy_svgo_get_version() -> *mut c_char {
    let version = env!("CARGO_PKG_VERSION");
    match CString::new(version) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Get default configuration as JSON string
///
/// # Safety
///
/// The caller is responsible for freeing the returned string using `vexy_svgo_free_string`.
#[no_mangle]
pub unsafe extern "C" fn vexy_svgo_get_default_config() -> *mut c_char {
    let config = Config::with_default_preset();
    match serde_json::to_string_pretty(&config) {
        Ok(json) => match CString::new(json) {
            Ok(c_string) => c_string.into_raw(),
            Err(_) => ptr::null_mut(),
        },
        Err(_) => ptr::null_mut(),
    }
}

/// Free the memory allocated for an VexySvgoResult
///
/// # Safety
///
/// This function must only be called with results returned from Vexy SVGO functions.
/// The result should not be used after calling this function.
#[no_mangle]
pub unsafe extern "C" fn vexy_svgo_free_result(result: VexyResult) {
    if !result.data.is_null() {
        let _ = CString::from_raw(result.data);
    }
    if !result.error_message.is_null() {
        let _ = CString::from_raw(result.error_message);
    }
}

/// Free a string returned by Vexy SVGO functions
///
/// # Safety
///
/// This function must only be called with strings returned from Vexy SVGO functions.
/// The string should not be used after calling this function.
#[no_mangle]
pub unsafe extern "C" fn vexy_svgo_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = CString::from_raw(s);
    }
}

/// Check if a string pointer is valid (for debugging)
///
/// # Safety
///
/// The `s` pointer must be either null or a valid pointer to a null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn vexy_svgo_check_string(s: *const c_char) -> c_int {
    if s.is_null() {
        return 0;
    }
    match CStr::from_ptr(s).to_str() {
        Ok(_) => 1,
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_ffi_optimize_default() {
        let svg = CString::new("<svg><circle r=\"10\"/></svg>").unwrap();
        let result = unsafe { vexy_svgo_optimize_default(svg.as_ptr()) };

        assert!(matches!(result.error_code, VexyErrorCode::Success));
        assert!(!result.data.is_null());
        assert!(result.data_length > 0);

        unsafe {
            vexy_svgo_free_result(result);
        }
    }

    #[test]
    fn test_ffi_get_version() {
        let version_ptr = unsafe { vexy_svgo_get_version() };
        assert!(!version_ptr.is_null());

        let version = unsafe { CStr::from_ptr(version_ptr) };
        assert!(!version.to_string_lossy().is_empty());

        unsafe {
            vexy_svgo_free_string(version_ptr);
        }
    }

    #[test]
    fn test_ffi_invalid_input() {
        let result = unsafe { vexy_svgo_optimize_default(ptr::null()) };
        assert!(matches!(result.error_code, VexyErrorCode::InvalidInput));
        assert!(!result.error_message.is_null());

        unsafe {
            vexy_svgo_free_result(result);
        }
    }
}
