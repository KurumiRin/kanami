//! Foreground app (executable file name) and keyboard layout.

use windows_sys::Win32::Foundation::{CloseHandle, HWND};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout;
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

/// File name of the program that owns the window, lowercased, such as `notepad.exe`.
pub fn exe_name(hwnd: HWND) -> Option<String> {
    if hwnd.is_null() {
        return None;
    }
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if process.is_null() {
        return None;
    }
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    let ok = unsafe {
        QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len)
    };
    unsafe { CloseHandle(process) };
    if ok == 0 {
        return None;
    }
    let path = String::from_utf16_lossy(&buf[..len as usize]);
    let name = path.rsplit('\\').next().unwrap_or(&path);
    Some(name.to_lowercase())
}

pub fn foreground_app() -> Option<String> {
    exe_name(unsafe { GetForegroundWindow() })
}

/// The foreground window's keyboard layout (HKL) as 8 hex digits, such as `08040804` (Chinese - Microsoft Pinyin).
pub fn foreground_layout() -> Option<String> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_null() {
        return None;
    }
    let thread = unsafe { GetWindowThreadProcessId(hwnd, std::ptr::null_mut()) };
    let hkl = unsafe { GetKeyboardLayout(thread) };
    Some(format!("{:08X}", hkl as usize as u32))
}
