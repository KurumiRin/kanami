//! Frontmost app tracking: subscribes to NSWorkspace app activation notifications.
//!
//! Notifications are dispatched on the main thread run loop, the same thread as the event tap, so no
//! locking is needed.

use std::ffi::{CStr, c_char, c_void};
use std::mem::transmute;
use std::ptr;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

use crate::ffi::*;

/// The function called when a notification arrives and its argument (used from a single thread; the
/// atomic types are only there to satisfy `static`).
static INFO: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static CALLBACK: AtomicUsize = AtomicUsize::new(0);

pub type Callback = unsafe fn(info: *mut c_void, bundle_id: Option<&str>);

unsafe fn sel(name: &CStr) -> Sel {
    unsafe { sel_registerName(name.as_ptr()) }
}

unsafe fn send0(obj: Id, name: &CStr) -> Id {
    if obj.is_null() {
        return ptr::null_mut();
    }
    let f: unsafe extern "C" fn(Id, Sel) -> Id =
        unsafe { transmute(objc_msgSend as unsafe extern "C" fn()) };
    unsafe { f(obj, sel(name)) }
}

unsafe fn send1(obj: Id, name: &CStr, arg: Id) -> Id {
    if obj.is_null() {
        return ptr::null_mut();
    }
    let f: unsafe extern "C" fn(Id, Sel, Id) -> Id =
        unsafe { transmute(objc_msgSend as unsafe extern "C" fn()) };
    unsafe { f(obj, sel(name), arg) }
}

unsafe fn bundle_id(app: Id) -> Option<String> {
    let string = unsafe { send0(app, c"bundleIdentifier") };
    if string.is_null() {
        return None;
    }
    let f: unsafe extern "C" fn(Id, Sel) -> *const c_char =
        unsafe { transmute(objc_msgSend as unsafe extern "C" fn()) };
    let utf8 = unsafe { f(string, sel(c"UTF8String")) };
    if utf8.is_null() {
        return None;
    }
    Some(
        unsafe { CStr::from_ptr(utf8) }
            .to_string_lossy()
            .into_owned(),
    )
}

unsafe fn workspace() -> Id {
    unsafe { send0(objc_getClass(c"NSWorkspace".as_ptr()), c"sharedWorkspace") }
}

pub fn frontmost() -> Option<String> {
    unsafe {
        let pool = objc_autoreleasePoolPush();
        let id = bundle_id(send0(workspace(), c"frontmostApplication"));
        objc_autoreleasePoolPop(pool);
        id
    }
}

unsafe extern "C" fn app_activated(_this: Id, _cmd: Sel, notification: Id) {
    unsafe {
        let pool = objc_autoreleasePoolPush();
        let info = send0(notification, c"userInfo");
        let app = send1(info, c"objectForKey:", NSWorkspaceApplicationKey);
        let id = bundle_id(app);
        let callback = CALLBACK.load(Ordering::Relaxed);
        if callback != 0 {
            let callback: Callback = transmute(callback);
            callback(INFO.load(Ordering::Relaxed), id.as_deref());
        }
        objc_autoreleasePoolPop(pool);
    }
}

/// Starts observing. From then on every frontmost app change calls `callback(info, bundle_id)`.
///
/// # Safety
/// `info` must stay valid until the process exits and be used only on the main thread.
pub unsafe fn observe(info: *mut c_void, callback: Callback) {
    INFO.store(info, Ordering::Relaxed);
    CALLBACK.store(callback as usize, Ordering::Relaxed);
    unsafe {
        let name = c"KanamiAppObserver";
        let mut class = objc_getClass(name.as_ptr());
        if class.is_null() {
            class = objc_allocateClassPair(objc_getClass(c"NSObject".as_ptr()), name.as_ptr(), 0);
            let imp = app_activated as unsafe extern "C" fn(Id, Sel, Id);
            class_addMethod(
                class,
                sel(c"appActivated:"),
                imp as *const c_void,
                c"v@:@".as_ptr(),
            );
            objc_registerClassPair(class);
        }
        let observer = send0(send0(class, c"alloc"), c"init");
        let center = send0(workspace(), c"notificationCenter");
        let add: unsafe extern "C" fn(Id, Sel, Id, Sel, Id, Id) =
            transmute(objc_msgSend as unsafe extern "C" fn());
        add(
            center,
            sel(c"addObserver:selector:name:object:"),
            observer,
            sel(c"appActivated:"),
            NSWorkspaceDidActivateApplicationNotification,
            ptr::null_mut(),
        );
    }
}
