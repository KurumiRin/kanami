//! Current input method: reads the TIS input source ID and subscribes to switch notifications.

use std::ffi::{CStr, c_char, c_void};

use crate::ffi::*;

pub type Callback = unsafe fn(info: *mut c_void, id: Option<&str>);

/// The ID of the current keyboard input source, such as `com.apple.keylayout.ABC` or `com.apple.inputmethod.SCIM.ITABC`.
pub fn current() -> Option<String> {
    unsafe {
        let source = TISCopyCurrentKeyboardInputSource();
        if source.is_null() {
            return None;
        }
        let id = TISGetInputSourceProperty(source, kTISPropertyInputSourceID);
        let result = cf_string(id);
        CFRelease(source);
        result
    }
}

unsafe fn cf_string(string: CFStringRef) -> Option<String> {
    if string.is_null() {
        return None;
    }
    let mut buf = [0 as c_char; 256];
    let ok = unsafe {
        CFStringGetCString(
            string,
            buf.as_mut_ptr(),
            buf.len() as CFIndex,
            kCFStringEncodingUTF8,
        )
    };
    ok.then(|| {
        unsafe { CStr::from_ptr(buf.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    })
}

static mut CALLBACK: Option<(Callback, *mut c_void)> = None;

unsafe extern "C" fn changed(
    _center: *mut c_void,
    _observer: *mut c_void,
    _name: CFStringRef,
    _object: *const c_void,
    _user_info: CFDictionaryRef,
) {
    // Notifications are dispatched on the main thread run loop
    if let Some((callback, info)) = unsafe { CALLBACK } {
        unsafe { callback(info, current().as_deref()) };
    }
}

/// Starts observing input method switches.
///
/// # Safety
/// `info` must stay valid until the process exits and be used only on the main thread.
pub unsafe fn observe(info: *mut c_void, callback: Callback) {
    unsafe {
        CALLBACK = Some((callback, info));
        CFNotificationCenterAddObserver(
            CFNotificationCenterGetDistributedCenter(),
            info,
            changed,
            kTISNotifySelectedKeyboardInputSourceChanged,
            std::ptr::null(),
            CFNotificationSuspensionBehaviorDeliverImmediately,
        );
    }
}
