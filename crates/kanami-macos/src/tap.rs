use std::ffi::c_void;
use std::fmt;
use std::ptr;
use std::time::{Instant, SystemTime};

use kanami_core::backend::{Handler, Options, TapMode, Waker};
use kanami_core::{Key, KeyEvent, KeyState, ModSet, Out};

use crate::devices::Tracker;
use crate::ffi::*;
use crate::hid::CapsMapping;
use crate::{app, input, keycode};

#[derive(Debug)]
pub enum TapError {
    /// Usually a missing permission: it must be granted to the app that launched this process (the terminal).
    CreateFailed(TapMode),
}

impl fmt::Display for TapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let permission = match self {
            TapError::CreateFailed(TapMode::Filter) => kanami_i18n::tr!("Accessibility"),
            TapError::CreateFailed(TapMode::ListenOnly) => kanami_i18n::tr!("Input Monitoring"),
        };
        f.write_str(&kanami_i18n::tr!(
            "Could not create the event tap. In “System Settings → Privacy & Security → {permission}”, allow the terminal app that runs kanami, then restart that terminal",
            permission = permission
        ))
    }
}

impl std::error::Error for TapError {}

/// Checks whether this process has the Accessibility permission; when `prompt` is true and it lacks it, the system authorization prompt is shown.
pub fn accessibility_trusted(prompt: bool) -> bool {
    if !prompt {
        return unsafe { AXIsProcessTrustedWithOptions(ptr::null()) };
    }
    unsafe {
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let options = CFDictionaryCreate(
            ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            &raw const kCFTypeDictionaryKeyCallBacks,
            &raw const kCFTypeDictionaryValueCallBacks,
        );
        let trusted = AXIsProcessTrustedWithOptions(options);
        CFRelease(options);
        trusted
    }
}

/// Marks events we synthesized so they are not processed again ("kanami").
const MARKER: i64 = 0x6B61_6E61_6D69;
/// For each modifier: (device-dependent bit that tells left from right, device-independent bit). Fn has only one bit.
const FLAG_BITS: [(ModSet, CGEventFlags, CGEventFlags); 9] = [
    (
        ModSet::LEFT_CONTROL,
        NX_DEVICELCTLKEYMASK,
        kCGEventFlagMaskControl,
    ),
    (
        ModSet::LEFT_OPTION,
        NX_DEVICELALTKEYMASK,
        kCGEventFlagMaskAlternate,
    ),
    (
        ModSet::LEFT_SHIFT,
        NX_DEVICELSHIFTKEYMASK,
        kCGEventFlagMaskShift,
    ),
    (
        ModSet::LEFT_COMMAND,
        NX_DEVICELCMDKEYMASK,
        kCGEventFlagMaskCommand,
    ),
    (
        ModSet::RIGHT_CONTROL,
        NX_DEVICERCTLKEYMASK,
        kCGEventFlagMaskControl,
    ),
    (
        ModSet::RIGHT_OPTION,
        NX_DEVICERALTKEYMASK,
        kCGEventFlagMaskAlternate,
    ),
    (
        ModSet::RIGHT_SHIFT,
        NX_DEVICERSHIFTKEYMASK,
        kCGEventFlagMaskShift,
    ),
    (
        ModSet::RIGHT_COMMAND,
        NX_DEVICERCMDKEYMASK,
        kCGEventFlagMaskCommand,
    ),
    (ModSet::FN, kCGEventFlagMaskSecondaryFn, 0),
];
/// Modifier flag bits managed by the engine; the other bits (keypad, Fn and so on) keep their original values when passed through.
const MANAGED_FLAGS: CGEventFlags = {
    let mut flags = 0;
    let mut i = 0;
    while i < 8 {
        flags |= FLAG_BITS[i].1 | FLAG_BITS[i].2;
        i += 1;
    }
    flags
};
/// A far-future time, meaning the timer is not due to fire.
const NEVER: CFAbsoluteTime = 1.0e12;

struct Context<H> {
    handler: H,
    mode: TapMode,
    port: CFMachPortRef,
    timer: CFRunLoopTimerRef,
    start: Instant,
    /// Keyboard type and Caps Lock state of the latest physical event, reused for synthesized events.
    keyboard_type: i64,
    alpha_shift: CGEventFlags,
    /// Modifiers currently down according to the modifier events that actually went through the tap.
    held: ModSet,
    /// Time of the latest physical key given to the engine.
    last_input: u64,
    /// Some releases may have bypassed the tap (tap paused, secure input was on, system slept) and need checking against the hardware state.
    suspect: bool,
    /// (monotonic time, wall-clock time) at the last poll. Monotonic time stops during sleep, so a growing gap between the two means the system slept.
    last_poll: (u64, SystemTime),
    /// The input method last reported to the handler.
    input_source: Option<String>,
    caps: Option<CapsMapping>,
    devices: Option<Box<Tracker>>,
    /// Keyboard identification was tried and failed; do not retry.
    devices_failed: bool,
}

impl<H: Handler> Context<H> {
    /// Starts keyboard identification when it is needed to know which keyboard a key came from (tried only once).
    unsafe fn start_devices(&mut self) {
        if self.devices.is_some() || self.devices_failed || !self.handler.wants_devices() {
            return;
        }
        self.devices = unsafe { Tracker::start() };
        if self.devices.is_none() {
            self.devices_failed = true;
            warn(&kanami_i18n::tr!(
                "Cannot tell which keyboard a key came from: in “System Settings → Privacy & Security → Input Monitoring”, allow the app that runs kanami, then restart kanami"
            ));
        }
    }

    fn now(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    /// During secure input, a paused tap or sleep, release events do not pass through kanami and the
    /// engine thinks the key is still down. Only after one of these has happened, and once idle for a
    /// while (so no events are still in flight), it checks against the hardware state and supplies
    /// the missing releases; it does not check otherwise, to avoid misjudging keys that really are held.
    fn release_lost(&mut self, now: u64) {
        const IDLE: u64 = 300;
        let wall = SystemTime::now();
        let (last, last_wall) = std::mem::replace(&mut self.last_poll, (now, wall));
        let elapsed = wall
            .duration_since(last_wall)
            .unwrap_or_default()
            .as_millis() as u64;
        if elapsed > now.saturating_sub(last) + 2000 {
            self.suspect = true;
        }
        if unsafe { IsSecureEventInputEnabled() } != 0 {
            // Keys do not pass through the tap during secure input; check once it ends
            self.suspect = true;
            return;
        }
        if !self.suspect
            || self.mode != TapMode::Filter
            || now.saturating_sub(self.last_input) < IDLE
        {
            return;
        }
        self.suspect = false;
        for &key in Key::ALL {
            if !self.handler.held(key) {
                continue;
            }
            // With the HID mapping, Caps Lock has already become F18 at the hardware level
            let code = match key {
                Key::CapsLock if self.caps.is_some() => keycode::from_key(Key::F18),
                key => keycode::from_key(key),
            };
            let Some(code) = code else { continue };
            if unsafe { CGEventSourceKeyState(kCGEventSourceStateHIDSystemState, code) } {
                continue;
            }
            if let Some(m) = ModSet::of_key(key) {
                self.held = self.held.difference(m);
            }
            let (keyboard_type, alpha_shift) = (self.keyboard_type, self.alpha_shift);
            post_out(
                self.handler.lost_release(key, now),
                keyboard_type,
                alpha_shift,
            );
        }
    }

    fn reschedule(&self, now: u64) {
        let fire = match self.handler.deadline() {
            Some(deadline) => {
                let delay = deadline.saturating_sub(now) as f64 / 1000.0;
                unsafe { CFAbsoluteTimeGetCurrent() + delay }
            }
            None => NEVER,
        };
        unsafe { CFRunLoopTimerSetNextFireDate(self.timer, fire) };
    }

    fn sync_caps(&mut self) {
        if self.mode != TapMode::Filter {
            return;
        }
        match (self.handler.wants_caps_lock(), self.caps.is_some()) {
            (true, false) => match CapsMapping::install() {
                Ok(mapping) => {
                    self.caps = Some(mapping);
                    if let Some(notice) = crate::hid::system_caps_notice() {
                        warn(&notice);
                    }
                }
                Err(e) => warn(&e.to_string()),
            },
            (false, true) => self.restore_caps(),
            _ => {}
        }
    }

    fn restore_caps(&mut self) {
        if let Some(mapping) = self.caps.take()
            && let Err(e) = mapping.restore()
        {
            warn(&kanami_i18n::tr!(
                "Restoring Caps Lock failed ({error}); run `kanami reset`",
                error = e
            ));
        }
    }
}

/// Installs the event tap on the main thread and enters the run loop until SIGINT / SIGTERM / SIGHUP arrives.
/// Before exiting it sends the missing release events and undoes the Caps Lock mapping.
pub fn run<H: Handler>(options: Options, handler: H) -> Result<(), TapError> {
    let tap_options = match options.mode {
        TapMode::Filter => kCGEventTapOptionDefault,
        TapMode::ListenOnly => kCGEventTapOptionListenOnly,
    };
    let keys =
        event_mask(kCGEventKeyDown) | event_mask(kCGEventKeyUp) | event_mask(kCGEventFlagsChanged);
    // Clicks, drags and scrolling mean different things with modifiers (⌘-click, ⇧-scroll, ...), so their flags need correcting; mouse movement does not
    let mouse = [
        kCGEventLeftMouseDown,
        kCGEventLeftMouseUp,
        kCGEventRightMouseDown,
        kCGEventRightMouseUp,
        kCGEventLeftMouseDragged,
        kCGEventRightMouseDragged,
        kCGEventScrollWheel,
        kCGEventOtherMouseDown,
        kCGEventOtherMouseUp,
        kCGEventOtherMouseDragged,
    ]
    .into_iter()
    .fold(0, |mask, ty| mask | event_mask(ty));
    let mask = match options.mode {
        TapMode::Filter => keys | mouse,
        TapMode::ListenOnly => keys,
    };

    let ctx = Box::into_raw(Box::new(Context {
        handler,
        mode: options.mode,
        port: ptr::null_mut(),
        timer: ptr::null_mut(),
        start: Instant::now(),
        keyboard_type: 0,
        alpha_shift: 0,
        held: ModSet::empty(),
        last_input: 0,
        suspect: false,
        last_poll: (0, SystemTime::now()),
        input_source: None,
        caps: None,
        devices: None,
        devices_failed: false,
    }));
    unsafe {
        let port = CGEventTapCreate(
            kCGSessionEventTap,
            kCGHeadInsertEventTap,
            tap_options,
            mask,
            tap_callback::<H>,
            ctx.cast(),
        );
        if port.is_null() {
            drop(Box::from_raw(ctx));
            return Err(TapError::CreateFailed(options.mode));
        }
        (*ctx).port = port;
        let run_loop = CFRunLoopGetCurrent();
        let source = CFMachPortCreateRunLoopSource(ptr::null(), port, 0);
        CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);

        let mut timer_ctx = CFRunLoopTimerContext {
            version: 0,
            info: ctx.cast(),
            retain: ptr::null(),
            release: ptr::null(),
            copy_description: ptr::null(),
        };
        // The interval is set very large; reschedule sets each next fire time
        let timer = CFRunLoopTimerCreate(
            ptr::null(),
            NEVER,
            NEVER,
            0,
            0,
            deadline_callback::<H>,
            &mut timer_ctx,
        );
        CFRunLoopAddTimer(run_loop, timer, kCFRunLoopCommonModes);
        (*ctx).timer = timer;

        let poll_timer = options.poll.then(|| {
            let t = CFRunLoopTimerCreate(
                ptr::null(),
                CFAbsoluteTimeGetCurrent() + 1.0,
                1.0,
                0,
                0,
                poll_callback::<H>,
                &mut timer_ctx,
            );
            CFRunLoopAddTimer(run_loop, t, kCFRunLoopCommonModes);
            t
        });

        (*ctx).handler.app_changed(app::frontmost().as_deref());
        app::observe(ctx.cast(), app_callback::<H>);
        input_callback::<H>(ctx.cast(), input::current().as_deref());
        input::observe(ctx.cast(), input_callback::<H>);
        (*ctx).sync_caps();
        (*ctx).start_devices();
        crate::signal::stop_on_signals();

        // Other threads (IPC, menu) wake the main thread through this source, which calls Handler::wake on the main thread
        let mut wake_ctx = CFRunLoopSourceContext {
            version: 0,
            info: ctx.cast(),
            retain: ptr::null(),
            release: ptr::null(),
            copy_description: ptr::null(),
            equal: ptr::null(),
            hash: ptr::null(),
            schedule: ptr::null(),
            cancel: ptr::null(),
            perform: Some(wake_callback::<H>),
        };
        let wake_source = CFRunLoopSourceCreate(ptr::null(), 0, &mut wake_ctx);
        CFRunLoopAddSource(run_loop, wake_source, kCFRunLoopCommonModes);
        let target = SendPtr(wake_source.cast(), run_loop.cast());
        let waker = Waker::new(move || target.wake());

        if options.app {
            crate::nsapp::init();
        }
        (*ctx).handler.started(waker);

        CGEventTapEnable(port, true);
        if options.app {
            crate::nsapp::run();
        } else {
            CFRunLoopRun();
        }

        CGEventTapEnable(port, false);
        // Other threads may still hold the waker: invalidate the source without releasing it, so later wakes do nothing
        CFRunLoopSourceInvalidate(wake_source);
        let (keyboard_type, alpha_shift) = ((*ctx).keyboard_type, (*ctx).alpha_shift);
        post_out((*ctx).handler.shutdown(), keyboard_type, alpha_shift);
        (*ctx).restore_caps();
        for t in [Some(timer), poll_timer].into_iter().flatten() {
            CFRunLoopTimerInvalidate(t);
            CFRelease(t);
        }
        CFRelease(source);
        CFRelease(port);
        // Observers still hold the pointer; Context is freed when the process exits
    }
    Ok(())
}

// The patterns use FFI constants that keep Apple's original names
#[allow(non_upper_case_globals)]
unsafe extern "C" fn tap_callback<H: Handler>(
    _proxy: CGEventTapProxy,
    ty: CGEventType,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef {
    let ctx = unsafe { &mut *user_info.cast::<Context<H>>() };
    match ty {
        kCGEventTapDisabledByTimeout | kCGEventTapDisabledByUserInput => {
            unsafe { CGEventTapEnable(ctx.port, true) };
            ctx.suspect = true;
            let reason = if ty == kCGEventTapDisabledByTimeout {
                kanami_i18n::text("timeout")
            } else {
                kanami_i18n::text("user input")
            };
            ctx.handler.tap_disabled(reason);
            return event;
        }
        kCGEventKeyDown | kCGEventKeyUp | kCGEventFlagsChanged => {}
        _ => {
            // Mouse events: the system fills the flags from the physical modifiers; change them to the engine's current modifier state
            if let Some(mods) = ctx.handler.mods() {
                let flags = unsafe { CGEventGetFlags(event) };
                let fixed = (flags & !MANAGED_FLAGS)
                    | flags_from_mods(mods)
                    | foreign_flags(flags, ctx.held);
                if fixed != flags {
                    unsafe { CGEventSetFlags(event, fixed) };
                }
            }
            return event;
        }
    }
    if unsafe { CGEventGetIntegerValueField(event, kCGEventSourceUserData) } == MARKER {
        return event;
    }

    let code = unsafe { CGEventGetIntegerValueField(event, kCGKeyboardEventKeycode) } as u16;
    let key = if ctx.caps.is_some() && Some(code) == keycode::from_key(Key::F18) {
        Key::CapsLock
    } else {
        match keycode::to_key(code) {
            Some(key) => key,
            None => return event,
        }
    };
    // Without the HID mapping, Caps Lock sends one event only when its lock state toggles; giving it to the engine would make it look held down forever
    if Some(code) == keycode::from_key(Key::CapsLock) && ctx.mode == TapMode::Filter {
        return event;
    }
    let flags = unsafe { CGEventGetFlags(event) };
    let state = match ty {
        kCGEventKeyDown
            if unsafe { CGEventGetIntegerValueField(event, kCGKeyboardEventAutorepeat) } != 0 =>
        {
            KeyState::Repeat
        }
        kCGEventKeyDown => KeyState::Down,
        kCGEventKeyUp => KeyState::Up,
        _ => modifier_state(key, flags),
    };
    // Events from the physical keyboard come from the system (PID 0); those synthesized by other programs pass through unchanged and take no part in remapping
    let source = unsafe { CGEventGetIntegerValueField(event, kCGEventSourceUnixProcessID) };
    if source != 0 {
        let input = KeyEvent {
            key,
            state,
            mods: mods_from_flags(flags),
        };
        ctx.handler.foreign(input, source as u32);
        return event;
    }
    ctx.keyboard_type = unsafe { CGEventGetIntegerValueField(event, kCGKeyboardEventKeyboardType) };
    ctx.alpha_shift = flags & kCGEventFlagMaskAlphaShift;
    if let Some(m) = ModSet::of_key(key) {
        match state {
            KeyState::Down => ctx.held = ctx.held.union(m),
            KeyState::Up => ctx.held = ctx.held.difference(m),
            KeyState::Repeat => {}
        }
    }
    let input = KeyEvent {
        key,
        state,
        mods: mods_from_flags(flags),
    };

    let now = ctx.now();
    ctx.last_input = now;
    let (keyboard_type, alpha_shift) = (ctx.keyboard_type, ctx.alpha_shift);
    // If keyboard identification only becomes needed after a config reload or opening the settings window, it starts here
    unsafe { ctx.start_devices() };
    let device = ctx.devices.as_mut().and_then(|t| t.device_for(key, state));
    let out = ctx.handler.event(input, device, now);
    let result = if ctx.mode == TapMode::ListenOnly {
        event
    } else if let [only] = out.keys.as_slice()
        && out.texts.is_empty()
        && only.key == key
        && only.state == state
    {
        // The most common case: pass through unchanged, only correcting the modifier flag bits
        let fixed =
            (flags & !MANAGED_FLAGS) | flags_from_mods(only.mods) | foreign_flags(flags, ctx.held);
        unsafe { CGEventSetFlags(event, fixed) };
        event
    } else {
        post_out(out, keyboard_type, alpha_shift);
        ptr::null_mut()
    };
    ctx.reschedule(now);
    result
}

/// Passes CoreFoundation object pointers across threads. Signaling and waking a run loop source is thread-safe.
#[derive(Clone, Copy)]
struct SendPtr(*mut c_void, *mut c_void);

unsafe impl Send for SendPtr {}
unsafe impl Sync for SendPtr {}

impl SendPtr {
    /// Signals the wake source and wakes the run loop it belongs to.
    fn wake(self) {
        unsafe {
            CFRunLoopSourceSignal(self.0.cast());
            CFRunLoopWakeUp(self.1.cast());
        }
    }
}

unsafe extern "C" fn wake_callback<H: Handler>(info: *mut c_void) {
    let ctx = unsafe { &mut *info.cast::<Context<H>>() };
    let now = ctx.now();
    let (keyboard_type, alpha_shift) = (ctx.keyboard_type, ctx.alpha_shift);
    post_out(ctx.handler.wake(now), keyboard_type, alpha_shift);
    // Pausing / resuming changes whether the Caps Lock mapping is needed; sync right away instead of waiting for the next poll
    ctx.sync_caps();
    ctx.reschedule(now);
    if ctx.handler.should_quit() {
        crate::nsapp::stop();
    }
}

unsafe extern "C" fn deadline_callback<H: Handler>(_timer: CFRunLoopTimerRef, info: *mut c_void) {
    let ctx = unsafe { &mut *info.cast::<Context<H>>() };
    let now = ctx.now();
    let (keyboard_type, alpha_shift) = (ctx.keyboard_type, ctx.alpha_shift);
    post_out(ctx.handler.tick(now), keyboard_type, alpha_shift);
    ctx.reschedule(now);
}

unsafe extern "C" fn poll_callback<H: Handler>(_timer: CFRunLoopTimerRef, info: *mut c_void) {
    let ctx = unsafe { &mut *info.cast::<Context<H>>() };
    let now = ctx.now();
    let (keyboard_type, alpha_shift) = (ctx.keyboard_type, ctx.alpha_shift);
    post_out(ctx.handler.poll(now), keyboard_type, alpha_shift);
    ctx.release_lost(now);
    ctx.sync_caps();
    ctx.reschedule(now);
}

unsafe fn app_callback<H: Handler>(info: *mut c_void, bundle_id: Option<&str>) {
    let ctx = unsafe { &mut *info.cast::<Context<H>>() };
    ctx.handler.app_changed(bundle_id);
}

unsafe fn input_callback<H: Handler>(info: *mut c_void, id: Option<&str>) {
    let ctx = unsafe { &mut *info.cast::<Context<H>>() };
    // A single input method switch often sends several notifications
    if ctx.input_source.as_deref() == id {
        return;
    }
    ctx.input_source = id.map(str::to_owned);
    ctx.handler.input_source_changed(id);
}

/// Posts keys and text in order (text goes at the position it recorded).
fn post_out(out: &Out, keyboard_type: i64, alpha_shift: CGEventFlags) {
    let mut texts = out.texts.iter().peekable();
    for (i, &event) in out.keys.iter().enumerate() {
        while let Some((_, text)) = texts.next_if(|(at, _)| *at == i) {
            post_text(text, keyboard_type);
        }
        post(event, keyboard_type, alpha_shift);
    }
    for (_, text) in texts {
        post_text(text, keyboard_type);
    }
}

/// Types text with Unicode string events, independent of keyboard layout and input method.
/// Each event carries at most 20 UTF-16 units, so long text is sent in chunks.
fn post_text(text: &str, keyboard_type: i64) {
    const CHUNK: usize = 20;
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut start = 0;
    while start < units.len() {
        let mut end = (start + CHUNK).min(units.len());
        // Do not split a surrogate pair across two events
        if end < units.len() && (0xD800..0xDC00).contains(&units[end - 1]) {
            end -= 1;
        }
        let chunk = &units[start..end];
        for down in [true, false] {
            unsafe {
                let ev = CGEventCreateKeyboardEvent(ptr::null(), 0, down);
                if ev.is_null() {
                    return;
                }
                // No modifiers, so it does not mix with shortcuts that are held down
                CGEventSetFlags(ev, kCGEventFlagMaskNonCoalesced);
                CGEventKeyboardSetUnicodeString(ev, chunk.len(), chunk.as_ptr());
                CGEventSetIntegerValueField(ev, kCGEventSourceUserData, MARKER);
                CGEventSetIntegerValueField(ev, kCGKeyboardEventKeyboardType, keyboard_type);
                CGEventPost(kCGSessionEventTap, ev);
                CFRelease(ev);
            }
        }
        start = end;
    }
}

/// Prints a notice. The terminal may already be closed, so write failures are ignored and must not panic.
fn warn(message: &str) {
    use std::io::Write;
    let _ = writeln!(std::io::stderr(), "kanami: {message}");
}

fn post(event: KeyEvent, keyboard_type: i64, alpha_shift: CGEventFlags) {
    let Some(code) = keycode::from_key(event.key) else {
        warn(&kanami_i18n::tr!(
            "macOS has no {key} key; ignored",
            key = event.key.name()
        ));
        return;
    };
    let mut flags = flags_from_mods(event.mods) | alpha_shift | kCGEventFlagMaskNonCoalesced;
    // Match a real keyboard: arrow keys carry the keypad and Fn flags, function and navigation keys carry the Fn flag
    flags |= match event.key {
        Key::Left | Key::Right | Key::Up | Key::Down => {
            kCGEventFlagMaskNumericPad | kCGEventFlagMaskSecondaryFn
        }
        key if key.is_keypad() => kCGEventFlagMaskNumericPad,
        Key::Home | Key::End | Key::PageUp | Key::PageDown | Key::Delete | Key::Insert => {
            kCGEventFlagMaskSecondaryFn
        }
        key if (Key::F1 as u8..=Key::F20 as u8).contains(&(key as u8)) => {
            kCGEventFlagMaskSecondaryFn
        }
        _ => 0,
    };
    unsafe {
        let ev = CGEventCreateKeyboardEvent(ptr::null(), code, event.state != KeyState::Up);
        if ev.is_null() {
            return;
        }
        if ModSet::of_key(event.key).is_some() {
            CGEventSetType(ev, kCGEventFlagsChanged);
        }
        CGEventSetFlags(ev, flags);
        CGEventSetIntegerValueField(ev, kCGEventSourceUserData, MARKER);
        CGEventSetIntegerValueField(ev, kCGKeyboardEventKeyboardType, keyboard_type);
        if event.state == KeyState::Repeat {
            CGEventSetIntegerValueField(ev, kCGKeyboardEventAutorepeat, 1);
        }
        CGEventPost(kCGSessionEventTap, ev);
        CFRelease(ev);
    }
}

/// A flags event does not say pressed or released itself; it is judged from the flag bit for that key.
/// Caps Lock is a locking key, so its flag bit shows the lock state.
fn modifier_state(key: Key, flags: CGEventFlags) -> KeyState {
    let mask = match key {
        Key::CapsLock => kCGEventFlagMaskAlphaShift,
        key => match FLAG_BITS
            .iter()
            .find(|(m, _, _)| Some(*m) == ModSet::of_key(key))
        {
            Some(&(_, device, _)) => device,
            None => return KeyState::Down,
        },
    };
    if flags & mask != 0 {
        KeyState::Down
    } else {
        KeyState::Up
    }
}

/// Modifier flag bits not caused by a held modifier. Events synthesized by other programs often
/// carry only flags without sending modifier events (such as Logitech Options+ turning a mouse side
/// button into ⌘[); those bits must be kept as they are and not overwritten by the engine's modifier state.
fn foreign_flags(flags: CGEventFlags, held: ModSet) -> CGEventFlags {
    flags & MANAGED_FLAGS & !flags_from_mods(held)
}

fn mods_from_flags(flags: CGEventFlags) -> ModSet {
    FLAG_BITS
        .iter()
        .filter(|&&(_, device, _)| flags & device != 0)
        .fold(ModSet::empty(), |acc, &(m, _, _)| acc.union(m))
}

fn flags_from_mods(mods: ModSet) -> CGEventFlags {
    FLAG_BITS
        .iter()
        .filter(|&&(m, _, _)| mods.contains(m))
        .fold(0, |acc, &(_, device, generic)| acc | device | generic)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifier_state_uses_device_bits() {
        let flags = kCGEventFlagMaskCommand | NX_DEVICERCMDKEYMASK;
        assert_eq!(modifier_state(Key::RightCommand, flags), KeyState::Down);
        assert_eq!(modifier_state(Key::LeftCommand, flags), KeyState::Up);
    }

    #[test]
    fn foreign_flags_keep_synthesized_modifiers() {
        // Synthesized ⌘[: only the device-independent ⌘ bit, with no modifier held
        assert_eq!(
            foreign_flags(kCGEventFlagMaskCommand, ModSet::empty()),
            kCGEventFlagMaskCommand
        );
        // Left ⌘ really held: left to the engine
        let physical = kCGEventFlagMaskCommand | NX_DEVICELCMDKEYMASK;
        assert_eq!(foreign_flags(physical, ModSet::LEFT_COMMAND), 0);
        // Left ⌥ held (possibly remapped to another key) and the synthesized event also carries ⇧: only ⇧ is kept
        let flags = kCGEventFlagMaskAlternate | NX_DEVICELALTKEYMASK | kCGEventFlagMaskShift;
        assert_eq!(
            foreign_flags(flags, ModSet::LEFT_OPTION),
            kCGEventFlagMaskShift
        );
    }

    #[test]
    fn flags_round_trip() {
        let mods = ModSet::LEFT_SHIFT
            .union(ModSet::RIGHT_COMMAND)
            .union(ModSet::FN);
        assert_eq!(mods_from_flags(flags_from_mods(mods)), mods);
        assert_eq!(
            flags_from_mods(ModSet::RIGHT_OPTION),
            kCGEventFlagMaskAlternate | NX_DEVICERALTKEYMASK
        );
    }
}
