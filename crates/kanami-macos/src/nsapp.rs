//! The menu bar icon needs the NSApplication event loop: with CFRunLoop alone, nobody dispatches clicks on the menu bar icon.
//!
//! `[NSApp run]` also drives the main thread's CFRunLoop, so the event tap and timers keep working.

use std::ffi::CStr;
use std::mem::transmute;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::ffi::*;

/// Running under `[NSApp run]`.
static RUNNING: AtomicBool = AtomicBool::new(false);

/// NSApplicationActivationPolicyAccessory: not shown in the Dock and has no main menu.
const ACCESSORY: i64 = 1;
/// NSEventTypeApplicationDefined.
const APPLICATION_DEFINED: u64 = 15;

#[repr(C)]
#[derive(Clone, Copy)]
struct NSPoint {
    x: f64,
    y: f64,
}

unsafe fn sel(name: &CStr) -> Sel {
    unsafe { sel_registerName(name.as_ptr()) }
}

unsafe fn app() -> Id {
    let f: unsafe extern "C" fn(Id, Sel) -> Id =
        unsafe { transmute(objc_msgSend as unsafe extern "C" fn()) };
    unsafe {
        f(
            objc_getClass(c"NSApplication".as_ptr()),
            sel(c"sharedApplication"),
        )
    }
}

/// Creates NSApplication and makes it a background accessory app. Must be called on the main thread, before the menu bar icon is created.
pub fn init() {
    unsafe {
        let app = app();
        let set_policy: unsafe extern "C" fn(Id, Sel, i64) -> bool =
            transmute(objc_msgSend as unsafe extern "C" fn());
        set_policy(app, sel(c"setActivationPolicy:"), ACCESSORY);
        let finish: unsafe extern "C" fn(Id, Sel) =
            transmute(objc_msgSend as unsafe extern "C" fn());
        finish(app, sel(c"finishLaunching"));
    }
}

/// Runs the event loop until [`stop`].
pub fn run() {
    RUNNING.store(true, Ordering::SeqCst);
    unsafe {
        let run: unsafe extern "C" fn(Id, Sel) = transmute(objc_msgSend as unsafe extern "C" fn());
        run(app(), sel(c"run"));
    }
    RUNNING.store(false, Ordering::SeqCst);
}

/// Stops the main thread's event loop (`[NSApp run]` or `CFRunLoopRun`). Call only on the main thread.
pub fn stop() {
    unsafe {
        if !RUNNING.load(Ordering::SeqCst) {
            CFRunLoopStop(CFRunLoopGetMain());
            return;
        }
        let app = app();
        let stop: unsafe extern "C" fn(Id, Sel, Id) =
            transmute(objc_msgSend as unsafe extern "C" fn());
        stop(app, sel(c"stop:"), ptr::null_mut());
        // stop: only takes effect after the next event is handled, so post an empty event to make the loop return right away
        let make: unsafe extern "C" fn(
            Id,
            Sel,
            u64,
            NSPoint,
            u64,
            f64,
            i64,
            Id,
            i16,
            i64,
            i64,
        ) -> Id = transmute(objc_msgSend as unsafe extern "C" fn());
        let event = make(
            objc_getClass(c"NSEvent".as_ptr()),
            sel(c"otherEventWithType:location:modifierFlags:timestamp:windowNumber:context:subtype:data1:data2:"),
            APPLICATION_DEFINED,
            NSPoint { x: 0.0, y: 0.0 },
            0,
            0.0,
            0,
            ptr::null_mut(),
            0,
            0,
            0,
        );
        if !event.is_null() {
            let post: unsafe extern "C" fn(Id, Sel, Id, bool) =
                transmute(objc_msgSend as unsafe extern "C" fn());
            post(app, sel(c"postEvent:atStart:"), event, true);
        }
    }
}

/// Since macOS 27 the system hides menu item images by default: sets every item in `menu` (an NSMenu) to always show its image.
/// Older systems lack this property, so it is skipped there.
pub fn show_menu_images(menu: *mut std::ffi::c_void) {
    /// NSMenuItemImageVisibilityVisible.
    const VISIBLE: i64 = 1;
    if menu.is_null() {
        return;
    }
    unsafe {
        let get: unsafe extern "C" fn(Id, Sel) -> Id =
            transmute(objc_msgSend as unsafe extern "C" fn());
        let count: unsafe extern "C" fn(Id, Sel) -> usize =
            transmute(objc_msgSend as unsafe extern "C" fn());
        let at: unsafe extern "C" fn(Id, Sel, usize) -> Id =
            transmute(objc_msgSend as unsafe extern "C" fn());
        let responds: unsafe extern "C" fn(Id, Sel, Sel) -> bool =
            transmute(objc_msgSend as unsafe extern "C" fn());
        let set: unsafe extern "C" fn(Id, Sel, i64) =
            transmute(objc_msgSend as unsafe extern "C" fn());

        let items = get(menu.cast(), sel(c"itemArray"));
        let visibility = sel(c"setPreferredImageVisibility:");
        for i in 0..count(items, sel(c"count")) {
            let item = at(items, sel(c"objectAtIndex:"), i);
            if responds(item, sel(c"respondsToSelector:"), visibility) {
                set(item, visibility, VISIBLE);
            }
        }
    }
}
