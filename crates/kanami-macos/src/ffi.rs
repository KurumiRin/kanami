//! Declarations for the CoreGraphics / CoreFoundation / ApplicationServices / libdispatch / ObjC runtime APIs in use.

#![allow(non_upper_case_globals, non_snake_case)]

use std::ffi::{c_char, c_int, c_void};

pub type CFTypeRef = *const c_void;
pub type CFMachPortRef = *mut c_void;
pub type CFRunLoopRef = *mut c_void;
pub type CFRunLoopSourceRef = *mut c_void;
pub type CFRunLoopTimerRef = *mut c_void;
pub type CFStringRef = *const c_void;
pub type CFDictionaryRef = *const c_void;
pub type CFIndex = isize;
pub type CFAbsoluteTime = f64;
pub type CGEventRef = *mut c_void;
pub type CGEventTapProxy = *mut c_void;
pub type CGEventType = u32;
pub type CGEventFlags = u64;

pub type CGEventTapCallBack = unsafe extern "C" fn(
    proxy: CGEventTapProxy,
    ty: CGEventType,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef;

pub type CFRunLoopTimerCallBack = unsafe extern "C" fn(timer: CFRunLoopTimerRef, info: *mut c_void);

#[repr(C)]
pub struct CFRunLoopTimerContext {
    pub version: CFIndex,
    pub info: *mut c_void,
    pub retain: *const c_void,
    pub release: *const c_void,
    pub copy_description: *const c_void,
}

// CGEventTapLocation / CGEventTapPlacement / CGEventTapOptions
pub const kCGSessionEventTap: u32 = 1;
pub const kCGHeadInsertEventTap: u32 = 0;
pub const kCGEventTapOptionDefault: u32 = 0;
pub const kCGEventTapOptionListenOnly: u32 = 1;

// CGEventType
pub const kCGEventLeftMouseDown: CGEventType = 1;
pub const kCGEventLeftMouseUp: CGEventType = 2;
pub const kCGEventRightMouseDown: CGEventType = 3;
pub const kCGEventRightMouseUp: CGEventType = 4;
pub const kCGEventLeftMouseDragged: CGEventType = 6;
pub const kCGEventRightMouseDragged: CGEventType = 7;
pub const kCGEventKeyDown: CGEventType = 10;
pub const kCGEventKeyUp: CGEventType = 11;
pub const kCGEventFlagsChanged: CGEventType = 12;
pub const kCGEventScrollWheel: CGEventType = 22;
pub const kCGEventOtherMouseDown: CGEventType = 25;
pub const kCGEventOtherMouseUp: CGEventType = 26;
pub const kCGEventOtherMouseDragged: CGEventType = 27;
pub const kCGEventTapDisabledByTimeout: CGEventType = 0xFFFF_FFFE;
pub const kCGEventTapDisabledByUserInput: CGEventType = 0xFFFF_FFFF;

// CGEventField
pub const kCGKeyboardEventAutorepeat: u32 = 8;
pub const kCGKeyboardEventKeycode: u32 = 9;
pub const kCGKeyboardEventKeyboardType: u32 = 10;
pub const kCGEventSourceUnixProcessID: u32 = 41;
pub const kCGEventSourceUserData: u32 = 42;

// CGEventSourceStateID
pub const kCGEventSourceStateHIDSystemState: i32 = 1;

// CGEventFlags (the device-independent part)
pub const kCGEventFlagMaskAlphaShift: CGEventFlags = 0x0001_0000;
pub const kCGEventFlagMaskShift: CGEventFlags = 0x0002_0000;
pub const kCGEventFlagMaskControl: CGEventFlags = 0x0004_0000;
pub const kCGEventFlagMaskAlternate: CGEventFlags = 0x0008_0000;
pub const kCGEventFlagMaskCommand: CGEventFlags = 0x0010_0000;
pub const kCGEventFlagMaskNumericPad: CGEventFlags = 0x0020_0000;
pub const kCGEventFlagMaskSecondaryFn: CGEventFlags = 0x0080_0000;
pub const kCGEventFlagMaskNonCoalesced: CGEventFlags = 0x0000_0100;

// IOKit NX_DEVICE*KEYMASK: device-dependent bits that tell left and right modifiers apart
pub const NX_DEVICELCTLKEYMASK: CGEventFlags = 0x0000_0001;
pub const NX_DEVICELSHIFTKEYMASK: CGEventFlags = 0x0000_0002;
pub const NX_DEVICERSHIFTKEYMASK: CGEventFlags = 0x0000_0004;
pub const NX_DEVICELCMDKEYMASK: CGEventFlags = 0x0000_0008;
pub const NX_DEVICERCMDKEYMASK: CGEventFlags = 0x0000_0010;
pub const NX_DEVICELALTKEYMASK: CGEventFlags = 0x0000_0020;
pub const NX_DEVICERALTKEYMASK: CGEventFlags = 0x0000_0040;
pub const NX_DEVICERCTLKEYMASK: CGEventFlags = 0x0000_2000;

pub const fn event_mask(ty: CGEventType) -> u64 {
    1 << ty
}

/// A version 0 run loop source: once marked by `CFRunLoopSourceSignal`, `perform` is called on its owning thread.
#[repr(C)]
pub struct CFRunLoopSourceContext {
    pub version: CFIndex,
    pub info: *mut c_void,
    pub retain: *const c_void,
    pub release: *const c_void,
    pub copy_description: *const c_void,
    pub equal: *const c_void,
    pub hash: *const c_void,
    pub schedule: *const c_void,
    pub cancel: *const c_void,
    pub perform: Option<unsafe extern "C" fn(info: *mut c_void)>,
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    pub static kCFRunLoopCommonModes: CFStringRef;
    pub static kCFBooleanTrue: CFTypeRef;
    pub static kCFTypeDictionaryKeyCallBacks: c_void;
    pub static kCFTypeDictionaryValueCallBacks: c_void;

    pub fn CFRelease(cf: CFTypeRef);
    pub fn CFMachPortCreateRunLoopSource(
        allocator: CFTypeRef,
        port: CFMachPortRef,
        order: CFIndex,
    ) -> CFRunLoopSourceRef;
    pub fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    pub fn CFRunLoopGetMain() -> CFRunLoopRef;
    pub fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);
    pub fn CFRunLoopRun();
    pub fn CFRunLoopStop(rl: CFRunLoopRef);
    pub fn CFRunLoopWakeUp(rl: CFRunLoopRef);
    pub fn CFRunLoopSourceCreate(
        allocator: CFTypeRef,
        order: CFIndex,
        context: *mut CFRunLoopSourceContext,
    ) -> CFRunLoopSourceRef;
    pub fn CFRunLoopSourceSignal(source: CFRunLoopSourceRef);
    pub fn CFRunLoopSourceInvalidate(source: CFRunLoopSourceRef);
    pub fn CFAbsoluteTimeGetCurrent() -> CFAbsoluteTime;
    pub fn CFRunLoopTimerCreate(
        allocator: CFTypeRef,
        fire_date: CFAbsoluteTime,
        interval: f64,
        flags: u64,
        order: CFIndex,
        callout: CFRunLoopTimerCallBack,
        context: *mut CFRunLoopTimerContext,
    ) -> CFRunLoopTimerRef;
    pub fn CFRunLoopAddTimer(rl: CFRunLoopRef, timer: CFRunLoopTimerRef, mode: CFStringRef);
    pub fn CFRunLoopTimerSetNextFireDate(timer: CFRunLoopTimerRef, fire_date: CFAbsoluteTime);
    pub fn CFRunLoopTimerInvalidate(timer: CFRunLoopTimerRef);
    pub fn CFDictionaryCreate(
        allocator: CFTypeRef,
        keys: *const CFTypeRef,
        values: *const CFTypeRef,
        num_values: CFIndex,
        key_callbacks: *const c_void,
        value_callbacks: *const c_void,
    ) -> CFDictionaryRef;
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    pub static kAXTrustedCheckOptionPrompt: CFStringRef;

    pub fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;

    pub fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: CGEventTapCallBack,
        user_info: *mut c_void,
    ) -> CFMachPortRef;
    pub fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    pub fn CGEventCreateKeyboardEvent(
        source: CFTypeRef,
        keycode: u16,
        key_down: bool,
    ) -> CGEventRef;
    pub fn CGEventPost(tap: u32, event: CGEventRef);
    pub fn CGEventSetType(event: CGEventRef, ty: CGEventType);
    pub fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    pub fn CGEventSetIntegerValueField(event: CGEventRef, field: u32, value: i64);
    pub fn CGEventGetFlags(event: CGEventRef) -> CGEventFlags;
    pub fn CGEventSetFlags(event: CGEventRef, flags: CGEventFlags);
    pub fn CGEventKeyboardSetUnicodeString(event: CGEventRef, length: usize, string: *const u16);
    pub fn CGEventSourceKeyState(state: i32, key: u16) -> bool;
}

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    pub static kTISPropertyInputSourceID: CFStringRef;
    pub static kTISNotifySelectedKeyboardInputSourceChanged: CFStringRef;

    pub fn TISCopyCurrentKeyboardInputSource() -> CFTypeRef;
    pub fn TISGetInputSourceProperty(source: CFTypeRef, key: CFStringRef) -> CFTypeRef;
    /// Returns Carbon's `Boolean` (one byte).
    pub fn IsSecureEventInputEnabled() -> u8;
}

pub type CFNotificationCallback = unsafe extern "C" fn(
    center: *mut c_void,
    observer: *mut c_void,
    name: CFStringRef,
    object: *const c_void,
    user_info: CFDictionaryRef,
);
pub const CFNotificationSuspensionBehaviorDeliverImmediately: CFIndex = 4;
pub const kCFStringEncodingUTF8: u32 = 0x0800_0100;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    pub fn CFNotificationCenterGetDistributedCenter() -> *mut c_void;
    pub fn CFNotificationCenterAddObserver(
        center: *mut c_void,
        observer: *const c_void,
        callback: CFNotificationCallback,
        name: CFStringRef,
        object: *const c_void,
        suspension_behavior: CFIndex,
    );
    pub fn CFStringGetCString(
        string: CFStringRef,
        buffer: *mut c_char,
        size: CFIndex,
        encoding: u32,
    ) -> bool;
}

// libdispatch is part of libSystem and links automatically
unsafe extern "C" {
    pub static _dispatch_main_q: c_void;
    pub static _dispatch_source_type_signal: c_void;

    pub fn dispatch_source_create(
        ty: *const c_void,
        handle: usize,
        mask: usize,
        queue: *const c_void,
    ) -> *mut c_void;
    pub fn dispatch_source_set_event_handler_f(
        source: *mut c_void,
        handler: extern "C" fn(*mut c_void),
    );
    pub fn dispatch_resume(object: *mut c_void);
    pub fn signal(signum: c_int, handler: usize) -> usize;
}

pub type Id = *mut c_void;
pub type Sel = *mut c_void;

#[link(name = "objc")]
unsafe extern "C" {
    pub fn objc_getClass(name: *const c_char) -> Id;
    pub fn sel_registerName(name: *const c_char) -> Sel;
    pub fn objc_msgSend();
    pub fn objc_allocateClassPair(superclass: Id, name: *const c_char, extra: usize) -> Id;
    pub fn objc_registerClassPair(class: Id);
    pub fn class_addMethod(class: Id, sel: Sel, imp: *const c_void, types: *const c_char) -> bool;
    pub fn objc_autoreleasePoolPush() -> *mut c_void;
    pub fn objc_autoreleasePoolPop(pool: *mut c_void);
}

#[cfg(not(target_arch = "aarch64"))]
#[link(name = "objc")]
unsafe extern "C" {
    pub fn objc_msgSend_stret();
}

// The layer popup is drawn with CALayer.
#[link(name = "QuartzCore", kind = "framework")]
unsafe extern "C" {}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    pub fn CGColorCreateSRGB(r: f64, g: f64, b: f64, a: f64) -> *mut c_void;
}

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {
    pub static NSWorkspaceDidActivateApplicationNotification: Id;
    pub static NSWorkspaceApplicationKey: Id;
}
