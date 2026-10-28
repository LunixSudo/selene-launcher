//! Windows turns Mica and acrylic into a flat colour while the window is inactive, for example
//! while a file dialog is open over it. The window keeps telling Windows it is active, so the
//! effect stays. Focus itself is unaffected.
//!
//! This is the only place with `unsafe`: subclassing a window is a plain Win32 call.
#![allow(unsafe_code)]

use tauri::Window;

#[cfg(windows)]
pub fn keep_backdrop(window: &Window) {
    use windows_sys::Win32::UI::Shell::SetWindowSubclass;

    const SUBCLASS_ID: usize = 1;
    if let Ok(hwnd) = window.hwnd() {
        // SAFETY: the handle belongs to a live window of this process, and the callback has
        // the signature Windows expects. Setting the same id again only replaces it.
        unsafe {
            SetWindowSubclass(hwnd.0, Some(always_active), SUBCLASS_ID, 0);
        }
    }
}

#[cfg(not(windows))]
pub fn keep_backdrop(_window: &Window) {}

#[cfg(windows)]
unsafe extern "system" fn always_active(
    hwnd: windows_sys::Win32::Foundation::HWND,
    message: u32,
    wparam: windows_sys::Win32::Foundation::WPARAM,
    lparam: windows_sys::Win32::Foundation::LPARAM,
    _id: usize,
    _data: usize,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::UI::Shell::DefSubclassProc;
    use windows_sys::Win32::UI::WindowsAndMessaging::WM_NCACTIVATE;

    let wparam = if message == WM_NCACTIVATE { 1 } else { wparam };
    // SAFETY: passes Windows' own arguments on to the next handler, as subclasses must.
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}
