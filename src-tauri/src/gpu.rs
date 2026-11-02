//! On a laptop with two graphics cards Windows tends to run Java on the slower built-in one.
//! The choice it offers in Settings › Display › Graphics is a per-program registry value,
//! which the launcher sets for the Java it installed.
#![allow(unsafe_code)]

use std::path::Path;

/// Turns the preference on or off for `exe`. A registry that refuses only costs speed, so
/// the game starts either way.
pub fn set_high_performance(exe: &Path, enabled: bool) {
    if let Err(err) = write_preference(exe, enabled) {
        log::warn!(
            "could not set the graphics card for {}: {err}",
            exe.display()
        );
    }
}

#[cfg(windows)]
fn write_preference(exe: &Path, enabled: bool) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows_sys::Win32::System::Registry::{
        HKEY_CURRENT_USER, REG_SZ, RegDeleteKeyValueW, RegSetKeyValueW,
    };

    fn wide(text: impl IntoIterator<Item = u16>) -> Vec<u16> {
        text.into_iter().chain(std::iter::once(0)).collect()
    }

    let key = wide("Software\\Microsoft\\DirectX\\UserGpuPreferences".encode_utf16());
    let name = wide(exe.as_os_str().encode_wide());
    let status = if enabled {
        let value = wide("GpuPreference=2;".encode_utf16());
        let bytes = u32::try_from(value.len() * size_of::<u16>()).unwrap_or(u32::MAX);
        // SAFETY: every pointer is to a NUL-terminated buffer that outlives the call, and
        // `bytes` is the length of `value`.
        unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                REG_SZ,
                value.as_ptr().cast(),
                bytes,
            )
        }
    } else {
        // SAFETY: as above.
        unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr()) }
    };
    match status {
        ERROR_SUCCESS | ERROR_FILE_NOT_FOUND => Ok(()),
        code => Err(std::io::Error::from_raw_os_error(
            i32::try_from(code).unwrap_or(i32::MAX),
        )),
    }
}

#[cfg(not(windows))]
#[expect(clippy::unnecessary_wraps, reason = "matches the Windows version")]
fn write_preference(_exe: &Path, _enabled: bool) -> std::io::Result<()> {
    Ok(())
}
