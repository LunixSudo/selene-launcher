//! Switches a running game to fullscreen once it has loaded. Starting it fullscreen makes
//! Windows report it as not responding while the loading screen holds the window.
//!
//! Pressing F11 is what the game itself offers for this, so the window gets that key press.
//! Windows has no other way to send it than a plain Win32 call, hence the `unsafe`.
#![allow(unsafe_code)]

#[cfg(windows)]
pub fn press_fullscreen_key(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F11;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, WM_KEYDOWN, WM_KEYUP,
    };

    struct Search {
        pid: u32,
        windows: Vec<HWND>,
    }

    unsafe extern "system" fn collect(hwnd: HWND, data: LPARAM) -> i32 {
        // SAFETY: `data` is the `Search` passed to `EnumWindows` below, alive for the whole
        // enumeration, and nothing else touches it meanwhile.
        let search = unsafe { &mut *(data as *mut Search) };
        let mut owner = 0;
        // SAFETY: Windows hands over a valid handle, and `owner` outlives the call.
        unsafe { GetWindowThreadProcessId(hwnd, &raw mut owner) };
        // SAFETY: as above.
        if owner == search.pid && unsafe { IsWindowVisible(hwnd) } != 0 {
            search.windows.push(hwnd);
        }
        1
    }

    const F11_SCAN_CODE: isize = 0x57;
    let mut search = Search {
        pid,
        windows: Vec::new(),
    };
    // SAFETY: the callback only reads its handle and writes to `search`, which lives until the
    // call returns.
    unsafe { EnumWindows(Some(collect), &raw mut search as LPARAM) };
    let down: LPARAM = (F11_SCAN_CODE << 16) | 1;
    let up: LPARAM = down | (0b11 << 30);
    for &hwnd in &search.windows {
        // SAFETY: posting to a window of another process only queues a message for it.
        unsafe {
            PostMessageW(hwnd, WM_KEYDOWN, VK_F11 as WPARAM, down);
            PostMessageW(hwnd, WM_KEYUP, VK_F11 as WPARAM, up);
        }
    }
    !search.windows.is_empty()
}

#[cfg(not(windows))]
pub fn press_fullscreen_key(_pid: u32) -> bool {
    false
}

/// Whether a line from the game's log means it is about to show its menu.
pub fn is_loaded_marker(line: &str) -> bool {
    line.contains("Sound engine started")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spots_the_end_of_loading() {
        assert!(is_loaded_marker(
            "[12:01:02] [Render thread/INFO]: Sound engine started"
        ));
        assert!(is_loaded_marker(
            "[12:01:02] [Sound Library Loader/INFO]: Sound engine started"
        ));
        assert!(!is_loaded_marker(
            "[12:01:00] [main/INFO]: Loading tweak class"
        ));
    }
}
