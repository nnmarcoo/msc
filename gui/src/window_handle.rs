#[cfg(target_os = "windows")]
pub fn get_hwnd() -> Option<*mut std::ffi::c_void> {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    unsafe {
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
