//! Identifies which keyboard a key came from: IOHIDManager opens all keyboards read-only (not
//! exclusively), with one HID value queue per keyboard.
//!
//! The event tap callback first drains every queue synchronously, then pairs entries by HID key code
//! and press/release, without relying on notification dispatch order: raw HID values are written to
//! the queue by the kernel, ahead of the events that reach the event tap via hidd and WindowServer.
//! Requires the Input Monitoring permission.

use std::collections::VecDeque;
use std::ffi::{CString, c_void};
use std::ptr;
use std::time::{Duration, Instant};

use kanami_core::backend::DeviceInfo;
use kanami_core::{Key, KeyState};

use crate::ffi::*;

type IOHIDManagerRef = *mut c_void;
type IOHIDDeviceRef = *mut c_void;
type IOHIDQueueRef = *mut c_void;
type IOHIDValueRef = *mut c_void;
type IOHIDElementRef = *mut c_void;
type CFSetRef = *const c_void;
type CFArrayRef = *const c_void;
type DeviceCallback = unsafe extern "C" fn(
    context: *mut c_void,
    result: i32,
    sender: *mut c_void,
    device: IOHIDDeviceRef,
);

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOHIDManagerCreate(allocator: CFTypeRef, options: u32) -> IOHIDManagerRef;
    fn IOHIDManagerSetDeviceMatching(manager: IOHIDManagerRef, matching: CFDictionaryRef);
    fn IOHIDManagerOpen(manager: IOHIDManagerRef, options: u32) -> i32;
    fn IOHIDCheckAccess(request: u32) -> u32;
    fn IOHIDManagerCopyDevices(manager: IOHIDManagerRef) -> CFSetRef;
    fn IOHIDManagerRegisterDeviceMatchingCallback(
        manager: IOHIDManagerRef,
        callback: DeviceCallback,
        context: *mut c_void,
    );
    fn IOHIDManagerRegisterDeviceRemovalCallback(
        manager: IOHIDManagerRef,
        callback: DeviceCallback,
        context: *mut c_void,
    );
    fn IOHIDManagerScheduleWithRunLoop(
        manager: IOHIDManagerRef,
        run_loop: CFRunLoopRef,
        mode: CFStringRef,
    );
    fn IOHIDDeviceGetProperty(device: IOHIDDeviceRef, key: CFStringRef) -> CFTypeRef;
    fn IOHIDDeviceCopyMatchingElements(
        device: IOHIDDeviceRef,
        matching: CFDictionaryRef,
        options: u32,
    ) -> CFArrayRef;
    fn IOHIDQueueCreate(
        allocator: CFTypeRef,
        device: IOHIDDeviceRef,
        depth: CFIndex,
        options: u32,
    ) -> IOHIDQueueRef;
    fn IOHIDQueueAddElement(queue: IOHIDQueueRef, element: IOHIDElementRef);
    fn IOHIDQueueStart(queue: IOHIDQueueRef);
    fn IOHIDQueueStop(queue: IOHIDQueueRef);
    fn IOHIDQueueCopyNextValueWithTimeout(queue: IOHIDQueueRef, timeout: f64) -> IOHIDValueRef;
    fn IOHIDValueGetElement(value: IOHIDValueRef) -> IOHIDElementRef;
    fn IOHIDValueGetIntegerValue(value: IOHIDValueRef) -> CFIndex;
    fn IOHIDElementGetUsagePage(element: IOHIDElementRef) -> u32;
    fn IOHIDElementGetUsage(element: IOHIDElementRef) -> u32;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFSetGetCount(set: CFSetRef) -> CFIndex;
    fn CFSetGetValues(set: CFSetRef, values: *mut *const c_void);
    fn CFArrayGetCount(array: CFArrayRef) -> CFIndex;
    fn CFArrayGetValueAtIndex(array: CFArrayRef, index: CFIndex) -> *const c_void;
    fn CFNumberCreate(allocator: CFTypeRef, ty: CFIndex, value: *const c_void) -> CFTypeRef;
    fn CFNumberGetValue(number: CFTypeRef, ty: CFIndex, value: *mut c_void) -> bool;
    fn CFStringCreateWithCString(
        allocator: CFTypeRef,
        string: *const i8,
        encoding: u32,
    ) -> CFStringRef;
    fn CFGetTypeID(cf: CFTypeRef) -> usize;
    fn CFNumberGetTypeID() -> usize;
    fn CFStringGetTypeID() -> usize;
}

const KCF_NUMBER_SINT32: CFIndex = 3;
const KEYBOARD_PAGE: u32 = 0x07;
/// Raw records are kept only for this long.
const RECORD_TTL: Duration = Duration::from_secs(2);
const MAX_RECORDS: usize = 128;

fn cf_string(s: &str) -> CFStringRef {
    let c = CString::new(s).expect("constant string");
    unsafe { CFStringCreateWithCString(ptr::null(), c.as_ptr(), kCFStringEncodingUTF8) }
}

fn cf_number(n: i32) -> CFTypeRef {
    unsafe { CFNumberCreate(ptr::null(), KCF_NUMBER_SINT32, (&n as *const i32).cast()) }
}

/// A matching dictionary of the form `{key: number, ...}`.
fn matching(pairs: &[(&str, i32)]) -> CFDictionaryRef {
    let keys: Vec<CFTypeRef> = pairs.iter().map(|(k, _)| cf_string(k)).collect();
    let values: Vec<CFTypeRef> = pairs.iter().map(|&(_, v)| cf_number(v)).collect();
    let dict = unsafe {
        CFDictionaryCreate(
            ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            pairs.len() as CFIndex,
            &raw const kCFTypeDictionaryKeyCallBacks,
            &raw const kCFTypeDictionaryValueCallBacks,
        )
    };
    for cf in keys.into_iter().chain(values) {
        unsafe { CFRelease(cf) };
    }
    dict
}

unsafe fn number_property(device: IOHIDDeviceRef, key: &str) -> u32 {
    let name = cf_string(key);
    let value = unsafe { IOHIDDeviceGetProperty(device, name) };
    unsafe { CFRelease(name) };
    let mut n: i32 = 0;
    if !value.is_null() && unsafe { CFGetTypeID(value) == CFNumberGetTypeID() } {
        unsafe { CFNumberGetValue(value, KCF_NUMBER_SINT32, (&mut n as *mut i32).cast()) };
    }
    n as u32
}

unsafe fn string_property(device: IOHIDDeviceRef, key: &str) -> String {
    let name = cf_string(key);
    let value = unsafe { IOHIDDeviceGetProperty(device, name) };
    unsafe { CFRelease(name) };
    if value.is_null() || unsafe { CFGetTypeID(value) != CFStringGetTypeID() } {
        return String::new();
    }
    let mut buf = [0i8; 256];
    let ok = unsafe {
        CFStringGetCString(
            value,
            buf.as_mut_ptr(),
            buf.len() as CFIndex,
            kCFStringEncodingUTF8,
        )
    };
    if !ok {
        return String::new();
    }
    unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }
        .to_string_lossy()
        .into_owned()
}

unsafe fn info(device: IOHIDDeviceRef, id: u32) -> DeviceInfo {
    unsafe {
        DeviceInfo {
            id,
            vendor_id: number_property(device, "VendorID"),
            product_id: number_property(device, "ProductID"),
            product: string_property(device, "Product"),
        }
    }
}

fn keyboard_manager() -> IOHIDManagerRef {
    unsafe {
        let manager = IOHIDManagerCreate(ptr::null(), 0);
        // Generic Desktop / Keyboard
        let dict = matching(&[("DeviceUsagePage", 1), ("DeviceUsage", 6)]);
        IOHIDManagerSetDeviceMatching(manager, dict);
        CFRelease(dict);
        manager
    }
}

/// Lists the connected keyboards (used by `kanami devices`); needs no permission.
pub fn list() -> Vec<DeviceInfo> {
    unsafe {
        let manager = keyboard_manager();
        let set = IOHIDManagerCopyDevices(manager);
        let mut devices = Vec::new();
        if !set.is_null() {
            let count = CFSetGetCount(set) as usize;
            let mut refs = vec![ptr::null(); count];
            CFSetGetValues(set, refs.as_mut_ptr());
            for (i, &device) in refs.iter().enumerate() {
                devices.push(info(device as IOHIDDeviceRef, i as u32 + 1));
            }
            CFRelease(set);
        }
        CFRelease(manager);
        devices.sort_by(|a, b| a.product.cmp(&b.product));
        devices.dedup_by(|a, b| {
            a.vendor_id == b.vendor_id && a.product_id == b.product_id && a.product == b.product
        });
        devices
    }
}

struct Tracked {
    device: IOHIDDeviceRef,
    queue: IOHIDQueueRef,
    info: DeviceInfo,
}

/// One raw HID record: which keyboard, which key code, pressed or released.
struct Record {
    device: u32,
    usage: u16,
    pressed: bool,
    at: Instant,
}

pub struct Tracker {
    manager: IOHIDManagerRef,
    devices: Vec<Tracked>,
    records: VecDeque<Record>,
    /// Which keyboard each key was last pressed on; auto-repeat reuses it.
    last_down: [Option<u32>; Key::COUNT],
    next_id: u32,
}

impl Tracker {
    /// Starts tracking. Gives `None` without the Input Monitoring permission.
    ///
    /// # Safety
    /// The returned `Box` address is registered with IOKit callbacks, so it must stay valid until the
    /// process exits and be used only on the main thread.
    pub unsafe fn start() -> Option<Box<Tracker>> {
        let manager = keyboard_manager();
        let mut tracker = Box::new(Tracker {
            manager,
            devices: Vec::new(),
            records: VecDeque::with_capacity(MAX_RECORDS),
            last_down: [None; Key::COUNT],
            next_id: 1,
        });
        unsafe {
            let ctx: *mut Tracker = &mut *tracker;
            IOHIDManagerRegisterDeviceMatchingCallback(manager, device_added, ctx.cast());
            IOHIDManagerRegisterDeviceRemovalCallback(manager, device_removed, ctx.cast());
            IOHIDManagerScheduleWithRunLoop(manager, CFRunLoopGetCurrent(), kCFRunLoopCommonModes);
            if IOHIDManagerOpen(manager, 0) != 0 {
                return None;
            }
        }
        Some(tracker)
    }

    /// Which keyboard this key came from. Gives `None` if it cannot be determined.
    pub fn device_for(&mut self, key: Key, state: KeyState) -> Option<&DeviceInfo> {
        self.drain();
        let id = match state {
            KeyState::Repeat => self.last_down[key as usize],
            KeyState::Down | KeyState::Up => {
                let pressed = state == KeyState::Down;
                let exact = key.hid_usage().and_then(|usage| {
                    self.records
                        .iter()
                        .position(|r| r.usage == usage && r.pressed == pressed)
                });
                match exact {
                    Some(i) => self.records.remove(i).map(|r| r.device),
                    // When the system layer has remapped keys the raw key code will not match; if only one keyboard was active recently, assume it
                    None => self.only_active_device(),
                }
            }
        };
        if state == KeyState::Down {
            self.last_down[key as usize] = id;
        }
        let id = id?;
        self.devices
            .iter()
            .find(|d| d.info.id == id)
            .map(|d| &d.info)
    }

    fn only_active_device(&self) -> Option<u32> {
        let first = self.records.back()?.device;
        self.records
            .iter()
            .all(|r| r.device == first)
            .then_some(first)
    }

    fn drain(&mut self) {
        let now = Instant::now();
        for tracked in &self.devices {
            loop {
                let value = unsafe { IOHIDQueueCopyNextValueWithTimeout(tracked.queue, 0.0) };
                if value.is_null() {
                    break;
                }
                unsafe {
                    let element = IOHIDValueGetElement(value);
                    let page = IOHIDElementGetUsagePage(element);
                    let usage = IOHIDElementGetUsage(element);
                    if page == KEYBOARD_PAGE && (0x04..=0xE7).contains(&usage) {
                        self.records.push_back(Record {
                            device: tracked.info.id,
                            usage: usage as u16,
                            pressed: IOHIDValueGetIntegerValue(value) != 0,
                            at: now,
                        });
                    }
                    CFRelease(value);
                }
            }
        }
        while self
            .records
            .front()
            .is_some_and(|r| now.duration_since(r.at) > RECORD_TTL)
            || self.records.len() > MAX_RECORDS
        {
            self.records.pop_front();
        }
    }

    unsafe fn add(&mut self, device: IOHIDDeviceRef) {
        if self.devices.iter().any(|d| d.device == device) {
            return;
        }
        unsafe {
            let queue = IOHIDQueueCreate(ptr::null(), device, 64, 0);
            if queue.is_null() {
                return;
            }
            let dict = matching(&[("UsagePage", KEYBOARD_PAGE as i32)]);
            let elements = IOHIDDeviceCopyMatchingElements(device, dict, 0);
            CFRelease(dict);
            if !elements.is_null() {
                for i in 0..CFArrayGetCount(elements) {
                    IOHIDQueueAddElement(
                        queue,
                        CFArrayGetValueAtIndex(elements, i) as IOHIDElementRef,
                    );
                }
                CFRelease(elements);
            }
            IOHIDQueueStart(queue);
            let id = self.next_id;
            self.next_id += 1;
            self.devices.push(Tracked {
                device,
                queue,
                info: info(device, id),
            });
        }
    }

    unsafe fn remove(&mut self, device: IOHIDDeviceRef) {
        if let Some(i) = self.devices.iter().position(|d| d.device == device) {
            let tracked = self.devices.remove(i);
            unsafe {
                IOHIDQueueStop(tracked.queue);
                CFRelease(tracked.queue);
            }
        }
    }
}

unsafe extern "C" fn device_added(
    context: *mut c_void,
    _result: i32,
    _sender: *mut c_void,
    device: IOHIDDeviceRef,
) {
    unsafe { (*context.cast::<Tracker>()).add(device) };
}

unsafe extern "C" fn device_removed(
    context: *mut c_void,
    _result: i32,
    _sender: *mut c_void,
    device: IOHIDDeviceRef,
) {
    unsafe { (*context.cast::<Tracker>()).remove(device) };
}

impl Drop for Tracker {
    fn drop(&mut self) {
        for tracked in self.devices.drain(..) {
            unsafe {
                IOHIDQueueStop(tracked.queue);
                CFRelease(tracked.queue);
            }
        }
        unsafe { CFRelease(self.manager) };
    }
}

/// Whether the Input Monitoring permission is already granted (without showing a prompt).
pub fn input_monitoring_allowed() -> bool {
    const LISTEN_EVENT: u32 = 1;
    const GRANTED: u32 = 0;
    unsafe { IOHIDCheckAccess(LISTEN_EVENT) == GRANTED }
}
