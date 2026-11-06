//! Single-instance lock and local time.

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, SYSTEMTIME,
};
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::System::Threading::CreateMutexW;

/// While it is held, no other kanami process can start.
pub struct InstanceLock(HANDLE);

impl Drop for InstanceLock {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

/// Only one kanami may remap at a time, otherwise every key would be processed twice.
pub fn lock_instance() -> Result<InstanceLock, String> {
    let name: Vec<u16> = "Local\\kanami-instance\0".encode_utf16().collect();
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        return Err(kanami_i18n::tr!("Creating the single-instance lock failed"));
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe { CloseHandle(handle) };
        return Err(kanami_i18n::tr!("Another kanami is already running"));
    }
    Ok(InstanceLock(handle))
}

/// Local time, such as `2026-11-05 14:40:12`.
pub fn local_time() -> String {
    let mut t: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut t) };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond
    )
}
