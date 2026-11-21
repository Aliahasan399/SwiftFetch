//! DPAPI-backed protection for saved request headers at rest.
//!
//! Windows only. `headers_json` may contain `Cookie` / `Authorization` values,
//! so when the user opts in we protect it with the OS credential facility
//! (`CryptProtectData` / `CryptUnprotectData`) using an application-specific
//! entropy blob. `CryptProtectMemory` is deliberately NOT used: its scope is
//! the current process and it cannot be read back by a later process.
//!
//! Two properties are non-negotiable here:
//!
//! 1. **No app-held key.** The key is owned and protected by Windows (DPAPI),
//!    scoped to the current user account on this machine. We pass
//!    `CRYPTPROTECT_UI_FORBIDDEN` and no `CRYPTPROTECT_LOCAL_MACHINE` /
//!    roaming flag, so protection is OS-managed, local, and never prompts.
//! 2. **Honest degradation.** On non-Windows hosts no encryption exists.
//!    `encrypt` fails and `decrypt` fails for envelopes; we never write a
//!    fake/masked value and claim it is protected.
//!
//! Wire format of an encrypted cell (a plain SQLite TEXT value):
//!
//! ```text
//! sfenc1:<base64( [version_byte=0x01] || <dpapi blob> )>
//! ```
//!
//! The DPAPI blob already carries an integrity MAC, so a tampered or
//! wrong-key blob is rejected by `CryptUnprotectData` itself.

use std::fmt;

/// Prefix that makes an encrypted cell recognizable in the database.
pub const ENVELOPE_PREFIX: &str = "sfenc1:";

/// Version of the payload layout after the prefix.
const FORMAT_VERSION: u8 = 0x01;

/// Application-specific entropy. Stable across builds so a value written by
/// one version is readable by the next. This is *not* a secret key — DPAPI
/// owns the key; entropy only separates our blobs from other DPAPI data
/// protected under the same user account.
const ENTROPY: &[u8] = b"SwiftFetch/headers/v1/local-state";

/// Explicit scope: OS-managed, current-user, machine-bound, no UI prompt.
/// (CRYPTPROTECT_UI_FORBIDDEN; no LOCAL_MACHINE, no roaming.)
#[cfg(windows)]
const DPAPI_SCOPE_FLAGS: u32 = 0x0000_0001;

/// Why a secret operation could not be completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretError {
    /// No OS-protected store is available on this platform.
    Unavailable,
    /// `CryptProtectData` refused to protect the bytes.
    ProtectFailed,
    /// The cell is not a well-formed envelope of this format.
    Malformed,
    /// `CryptUnprotectData` refused: wrong key, tampered blob, or wrong scope.
    DecryptFailed,
}

impl fmt::Display for SecretError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            SecretError::Unavailable => "OS-protected secret storage is unavailable",
            SecretError::ProtectFailed => "Could not protect secret data",
            SecretError::Malformed => "Malformed protected secret envelope",
            SecretError::DecryptFailed => "Could not decrypt protected secret data",
        };
        f.write_str(text)
    }
}

impl std::error::Error for SecretError {}

/// Whether OS-backed secret storage is available on this platform.
pub fn is_available() -> bool {
    cfg!(windows)
}

/// True when `value` uses our envelope format. Legacy plaintext never does.
pub fn is_envelope(value: &str) -> bool {
    value.starts_with(ENVELOPE_PREFIX)
}

/// Protect UTF-8 bytes and return a storable envelope string.
///
/// Fails on non-Windows hosts and on any DPAPI error; callers must treat an
/// error as "encryption did not happen" rather than storing a fake value.
pub fn encrypt(plaintext: &[u8]) -> Result<String, SecretError> {
    if !is_available() {
        return Err(SecretError::Unavailable);
    }
    let blob = dpapi_protect(plaintext).map_err(|_| SecretError::ProtectFailed)?;
    let mut payload = Vec::with_capacity(1 + blob.len());
    payload.push(FORMAT_VERSION);
    payload.extend_from_slice(&blob);
    Ok(format!("{ENVELOPE_PREFIX}{}", base64_encode(&payload)))
}

/// Reverse `encrypt`. Plaintext (legacy rows) is returned unchanged.
///
/// A recognizable but undecryptable envelope is an error — never silently
/// reinterpreted as plaintext headers. No panic on any malformed input.
pub fn decrypt(value: &str) -> Result<String, SecretError> {
    if !is_envelope(value) {
        // Legacy plaintext. No prefix, no encryption to undo.
        return Ok(value.to_string());
    }
    if !is_available() {
        return Err(SecretError::Unavailable);
    }
    let encoded = &value[ENVELOPE_PREFIX.len()..];
    let payload = base64_decode(encoded).map_err(|_| SecretError::Malformed)?;
    if payload.first() != Some(&FORMAT_VERSION) {
        return Err(SecretError::Malformed);
    }
    let plaintext = dpapi_unprotect(&payload[1..]).map_err(|_| SecretError::DecryptFailed)?;
    String::from_utf8(plaintext).map_err(|_| SecretError::Malformed)
}

// ---------------------------------------------------------------------------
// Windows DPAPI FFI
// ---------------------------------------------------------------------------

#[cfg(windows)]
#[repr(C)]
struct DataBlob {
    cb_data: u32,
    pb_data: *mut u8,
}

#[cfg(windows)]
#[link(name = "crypt32")]
unsafe extern "system" {
    fn CryptProtectData(
        p_data_in: *mut DataBlob,
        sz_data_descr: *const u16,
        p_optional_entropy: *mut DataBlob,
        pv_reserved: *mut core::ffi::c_void,
        p_prompt_struct: *const core::ffi::c_void,
        dw_flags: u32,
        p_data_out: *mut DataBlob,
    ) -> i32;

    fn CryptUnprotectData(
        p_data_in: *mut DataBlob,
        ppsz_data_descr: *mut *mut u16,
        p_optional_entropy: *mut DataBlob,
        pv_reserved: *mut core::ffi::c_void,
        p_prompt_struct: *const core::ffi::c_void,
        dw_flags: u32,
        p_data_out: *mut DataBlob,
    ) -> i32;
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn LocalFree(h_mem: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
}

#[cfg(windows)]
fn dpapi_protect(plaintext: &[u8]) -> Result<Vec<u8>, SecretError> {
    let mut input = DataBlob {
        cb_data: plaintext.len() as u32,
        pb_data: plaintext.as_ptr() as *mut u8,
    };
    let mut entropy = DataBlob {
        cb_data: ENTROPY.len() as u32,
        pb_data: ENTROPY.as_ptr() as *mut u8,
    };
    let mut output = DataBlob { cb_data: 0, pb_data: std::ptr::null_mut() };
    // SAFETY: all pointers are valid for the duration of the call; output is
    // owned by DPAPI and released with LocalFree below.
    let ok = unsafe {
        CryptProtectData(
            &mut input,
            std::ptr::null(),
            &mut entropy,
            std::ptr::null_mut(),
            std::ptr::null(),
            DPAPI_SCOPE_FLAGS,
            &mut output,
        )
    };
    if ok == 0 || output.pb_data.is_null() {
        return Err(SecretError::ProtectFailed);
    }
    // SAFETY: DPAPI allocated output.pb_data with LocalAlloc and reports its
    // length in output.cb_data; copy before freeing.
    let bytes = unsafe {
        let slice = std::slice::from_raw_parts(output.pb_data, output.cb_data as usize);
        let owned = slice.to_vec();
        LocalFree(output.pb_data as *mut core::ffi::c_void);
        owned
    };
    Ok(bytes)
}

#[cfg(windows)]
fn dpapi_unprotect(blob: &[u8]) -> Result<Vec<u8>, SecretError> {
    let mut input = DataBlob {
        cb_data: blob.len() as u32,
        pb_data: blob.as_ptr() as *mut u8,
    };
    let mut entropy = DataBlob {
        cb_data: ENTROPY.len() as u32,
        pb_data: ENTROPY.as_ptr() as *mut u8,
    };
    let mut output = DataBlob { cb_data: 0, pb_data: std::ptr::null_mut() };
    // SAFETY: see dpapi_protect. A description string is not requested.
    let ok = unsafe {
        CryptUnprotectData(
            &mut input,
            std::ptr::null_mut(),
            &mut entropy,
            std::ptr::null_mut(),
            std::ptr::null(),
            DPAPI_SCOPE_FLAGS,
            &mut output,
        )
    };
    if ok == 0 || output.pb_data.is_null() {
        return Err(SecretError::DecryptFailed);
    }
    // SAFETY: copy DPAPI's output before freeing it with LocalFree.
    let bytes = unsafe {
        let slice = std::slice::from_raw_parts(output.pb_data, output.cb_data as usize);
        let owned = slice.to_vec();
        LocalFree(output.pb_data as *mut core::ffi::c_void);
        owned
    };
    Ok(bytes)
}

#[cfg(not(windows))]
fn dpapi_protect(_plaintext: &[u8]) -> Result<Vec<u8>, SecretError> {
    Err(SecretError::Unavailable)
}

#[cfg(not(windows))]
fn dpapi_unprotect(_blob: &[u8]) -> Result<Vec<u8>, SecretError> {
    Err(SecretError::Unavailable)
}

// ---------------------------------------------------------------------------
// Minimal, self-contained standard base64 (no non-dev dependency needed)
// ---------------------------------------------------------------------------

const B64_ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encode bytes as standard padded base64.
pub(crate) fn base64_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 3 <= input.len() {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8) | input[i + 2] as u32;
        out.push(B64_ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(B64_ALPHABET[((n >> 12) & 63) as usize] as char);
        out.push(B64_ALPHABET[((n >> 6) & 63) as usize] as char);
        out.push(B64_ALPHABET[(n & 63) as usize] as char);
        i += 3;
    }
    match input.len() - i {
        1 => {
            let n = (input[i] as u32) << 16;
            out.push(B64_ALPHABET[((n >> 18) & 63) as usize] as char);
            out.push(B64_ALPHABET[((n >> 12) & 63) as usize] as char);
            out.push_str("==");
        }
        2 => {
            let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8);
            out.push(B64_ALPHABET[((n >> 18) & 63) as usize] as char);
            out.push(B64_ALPHABET[((n >> 12) & 63) as usize] as char);
            out.push(B64_ALPHABET[((n >> 6) & 63) as usize] as char);
            out.push('=');
        }
        _ => {}
    }
    out
}

fn b64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// Decode standard padded base64. Rejects bad alphabet, bad padding and sizes.
pub(crate) fn base64_decode(input: &str) -> Result<Vec<u8>, ()> {
    let bytes = input.as_bytes();
    if bytes.len() % 4 != 0 {
        return Err(());
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks_exact(4) {
        let padding = chunk.iter().rev().take_while(|&&c| c == b'=').count();
        if padding > 2 {
            return Err(());
        }
        let mut value = 0u32;
        for &byte in &chunk[..4 - padding] {
            value = (value << 6) | b64_value(byte).ok_or(())? as u32;
        }
        value <<= 6 * padding;
        out.push((value >> 16) as u8);
        if padding < 2 {
            out.push((value >> 8) as u8);
        }
        if padding < 1 {
            out.push(value as u8);
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Unit tests (platform-independent parts)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trips_all_lengths() {
        for len in 0..40 {
            let input: Vec<u8> = (0..len).map(|i| (i as u8).wrapping_mul(37).wrapping_add(11)).collect();
            let encoded = base64_encode(&input);
            assert_eq!(base64_decode(&encoded).unwrap(), input, "length {len}");
        }
    }

    #[test]
    fn base64_rejects_malformed_input() {
        assert!(base64_decode("A").is_err());
        assert!(base64_decode("A===").is_err());
        assert!(base64_decode("AB=C").is_err());
        assert!(base64_decode("A!BC").is_err());
    }

    #[test]
    fn envelope_detection_and_legacy_plaintext() {
        assert!(!is_envelope("[]"));
        assert!(!is_envelope(r#"[{"name":"Cookie","value":"x"}]"#));
        assert!(is_envelope("sfenc1:QUJD"));
        // Legacy plaintext always round-trips as itself without any OS call.
        let legacy = r#"[{"name":"Authorization","value":"Bearer x"}]"#;
        assert_eq!(decrypt(legacy).unwrap(), legacy);
    }
}
