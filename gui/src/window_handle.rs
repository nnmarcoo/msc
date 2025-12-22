#[cfg(target_os = "windows")]
pub fn get_hwnd() -> Option<*mut std::ffi::c_void> {
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetForegroundWindow};
    use windows::core::PCWSTR;

    unsafe {
        let window_title: Vec<u16> = "msc\0".encode_utf16().collect();
        let hwnd = FindWindowW(PCWSTR::null(), PCWSTR::from_raw(window_title.as_ptr()));

        if let Ok(hwnd) = hwnd {
            if !hwnd.0.is_null() {
                return Some(hwnd.0 as *mut std::ffi::c_void);
            }
        }

        let hwnd = GetForegroundWindow();
        if !hwnd.0.is_null() {
            Some(hwnd.0 as *mut std::ffi::c_void)
        } else {
            None
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn get_hwnd() -> Option<*mut std::ffi::c_void> {
    None
}
