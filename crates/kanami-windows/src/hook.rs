use std::ffi::c_void;
use std::fmt;
use std::ptr;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};
use std::time::Instant;

use kanami_core::backend::{Handler, Options, TapMode, Waker};
use kanami_core::{Key, KeyEvent, KeyState, ModSet, Out};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, MAPVK_VK_TO_VSC, MapVirtualKeyW, SendInput,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, EVENT_SYSTEM_FOREGROUND, GetMessageW, HC_ACTION, HHOOK,
    KBDLLHOOKSTRUCT, KillTimer, LLKHF_EXTENDED, LLKHF_INJECTED, MSG, PostThreadMessageW, SetTimer,
    SetWindowsHookExW, UnhookWindowsHookEx, WH_KEYBOARD_LL, WINEVENT_OUTOFCONTEXT, WM_APP,
    WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN, WM_TIMER,
};

use crate::{app, keycode};

/// Marks events we sent so they are not processed again ("kanami").
const MARKER: usize = 0x6B61_6E61_6D69;
/// An unassigned virtual key code. Tapping it before a synthesized Win / Alt release stops the
/// system from treating "pressed and released alone" as opening the Start menu or activating the
/// menu bar (AutoHotkey does the same).
const MASK_KEY: u16 = 0xE8;
/// Thread message that wakes the message loop.
const WM_WAKE: u32 = WM_APP + 1;

#[derive(Debug)]
pub enum RunError {
    HookFailed,
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::HookFailed => {
                f.write_str(&kanami_i18n::tr!("Installing the keyboard hook failed"))
            }
        }
    }
}

impl std::error::Error for RunError {}

/// The hook callback has no user-data parameter, so a global pointer finds the context (used only on the message loop thread).
static CONTEXT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static THREAD: AtomicU32 = AtomicU32::new(0);

struct Context<H> {
    handler: H,
    mode: TapMode,
    start: Instant,
    /// Keys physically held down, used to tell a first press from auto-repeat (the low-level hook does not).
    pressed: [bool; Key::COUNT],
    deadline_timer: usize,
    poll_timer: usize,
    layout: Option<String>,
    inputs: Vec<INPUT>,
}

impl<H: Handler> Context<H> {
    fn now(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    fn reschedule(&mut self, now: u64) {
        match self.handler.deadline() {
            Some(deadline) => {
                let delay = deadline.saturating_sub(now).clamp(1, u32::MAX as u64) as u32;
                // With a null hwnd, passing an existing ID replaces that timer and passing 0 creates a new one
                self.deadline_timer =
                    unsafe { SetTimer(ptr::null_mut(), self.deadline_timer, delay, None) };
            }
            None if self.deadline_timer != 0 => {
                unsafe { KillTimer(ptr::null_mut(), self.deadline_timer) };
                self.deadline_timer = 0;
            }
            None => {}
        }
    }

    fn check_layout(&mut self) {
        let layout = app::foreground_layout();
        if layout != self.layout {
            self.handler.input_source_changed(layout.as_deref());
            self.layout = layout;
        }
    }
}

/// Installs the keyboard hook on the current thread and enters the message loop until Ctrl+C / console close.
/// Before exiting it sends the missing release events.
pub fn run<H: Handler>(options: Options, handler: H) -> Result<(), RunError> {
    let ctx = Box::into_raw(Box::new(Context {
        handler,
        mode: options.mode,
        start: Instant::now(),
        pressed: [false; Key::COUNT],
        deadline_timer: 0,
        poll_timer: 0,
        layout: None,
        inputs: Vec::with_capacity(64),
    }));
    CONTEXT.store(ctx.cast(), Ordering::Relaxed);
    THREAD.store(unsafe { GetCurrentThreadId() }, Ordering::Relaxed);
    unsafe {
        let module = GetModuleHandleW(ptr::null());
        let hook: HHOOK = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc::<H>), module, 0);
        if hook.is_null() {
            drop(Box::from_raw(ctx));
            return Err(RunError::HookFailed);
        }
        let foreground = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            ptr::null_mut(),
            Some(foreground_proc::<H>),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );
        SetConsoleCtrlHandler(Some(on_ctrl), 1);

        (*ctx).handler.app_changed(app::foreground_app().as_deref());
        (*ctx).check_layout();
        if options.poll {
            (*ctx).poll_timer = SetTimer(ptr::null_mut(), 0, 1000, None);
        }
        // Other threads (IPC, tray menu) post a thread message, and the message loop calls Handler::wake
        let thread = THREAD.load(Ordering::Relaxed);
        (*ctx).handler.started(Waker::new(move || {
            PostThreadMessageW(thread, WM_WAKE, 0, 0);
        }));

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            if msg.message == WM_WAKE {
                let c = &mut *ctx;
                let now = c.now();
                let out = c.handler.wake(now);
                send(&mut c.inputs, out);
                c.reschedule(now);
                if c.handler.should_quit() {
                    break;
                }
                continue;
            }
            if msg.message == WM_TIMER {
                let c = &mut *ctx;
                let now = c.now();
                if msg.wParam == c.poll_timer {
                    c.check_layout();
                    let out = c.handler.poll(now);
                    send(&mut c.inputs, out);
                } else if msg.wParam == c.deadline_timer {
                    let out = c.handler.tick(now);
                    send(&mut c.inputs, out);
                }
                c.reschedule(now);
                continue;
            }
            DispatchMessageW(&msg);
        }

        UnhookWindowsHookEx(hook);
        if !foreground.is_null() {
            UnhookWinEvent(foreground);
        }
        let c = &mut *ctx;
        let out = c.handler.shutdown();
        send(&mut c.inputs, out);
        // The hook may still reference the global pointer; the context is freed when the process exits
    }
    Ok(())
}

unsafe extern "system" fn on_ctrl(_ctrl_type: u32) -> i32 {
    unsafe { PostThreadMessageW(THREAD.load(Ordering::Relaxed), WM_QUIT, 0, 0) };
    1
}

unsafe extern "system" fn hook_proc<H: Handler>(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let pass = || unsafe { CallNextHookEx(ptr::null_mut(), code, wparam, lparam) };
    if code != HC_ACTION as i32 {
        return pass();
    }
    let info = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
    if info.dwExtraInfo == MARKER {
        return pass();
    }
    let Some(key) = keycode::to_key(info.vkCode, info.flags & LLKHF_EXTENDED != 0) else {
        return pass();
    };
    let ctx = unsafe { &mut *CONTEXT.load(Ordering::Relaxed).cast::<Context<H>>() };
    let down = matches!(wparam as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
    // Keys injected by other programs (AutoHotkey, Logitech software, password manager autofill and so on) pass through unchanged and take no part in remapping
    if info.flags & LLKHF_INJECTED != 0 {
        let state = if down { KeyState::Down } else { KeyState::Up };
        ctx.handler.foreign(KeyEvent::new(key, state), 0);
        return pass();
    }
    let state = match (down, ctx.pressed[key as usize]) {
        (true, true) => KeyState::Repeat,
        (true, false) => KeyState::Down,
        (false, _) => KeyState::Up,
    };
    ctx.pressed[key as usize] = down;

    // The modifiers on the input event are only for display (monitor); the engine tracks modifier state itself
    let mods = Key::ALL
        .iter()
        .filter(|&&k| ctx.pressed[k as usize])
        .filter_map(|&k| ModSet::of_key(k))
        .fold(ModSet::empty(), ModSet::union);
    let now = ctx.now();
    // The low-level hook has no device information, so per-keyboard rules are not supported yet
    let out = ctx.handler.event(KeyEvent { key, state, mods }, None, now);
    let result = if ctx.mode == TapMode::ListenOnly {
        None
    } else if let [only] = out.keys.as_slice()
        && out.texts.is_empty()
        && only.key == key
        && only.state == state
    {
        // The most common case: pass through unchanged (Windows maintains modifier state from the key events themselves, so no correction is needed)
        None
    } else {
        send(&mut ctx.inputs, out);
        Some(1)
    };
    ctx.reschedule(now);
    result.unwrap_or_else(pass)
}

unsafe extern "system" fn foreground_proc<H: Handler>(
    _hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    _object: i32,
    _child: i32,
    _thread: u32,
    _time: u32,
) {
    let ctx = unsafe { &mut *CONTEXT.load(Ordering::Relaxed).cast::<Context<H>>() };
    ctx.handler.app_changed(app::exe_name(hwnd).as_deref());
    ctx.check_layout();
}

fn key_input(vk: u16, scan: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: MARKER,
            },
        },
    }
}

/// Hands keys and text (inserted at the position it recorded) to `SendInput` in order and in one call, so other input cannot interrupt them.
fn send(inputs: &mut Vec<INPUT>, out: &Out) {
    inputs.clear();
    let mut texts = out.texts.iter().peekable();
    for (i, event) in out.keys.iter().enumerate() {
        while let Some((_, text)) = texts.next_if(|(at, _)| *at == i) {
            push_text(inputs, text);
        }
        push_key(inputs, *event);
    }
    for (_, text) in texts {
        push_text(inputs, text);
    }
    if !inputs.is_empty() {
        unsafe {
            SendInput(
                inputs.len() as u32,
                inputs.as_ptr(),
                size_of::<INPUT>() as i32,
            )
        };
    }
}

fn push_key(inputs: &mut Vec<INPUT>, event: KeyEvent) {
    let Some(vk) = keycode::from_key(event.key) else {
        return;
    };
    let up = event.state == KeyState::Up;
    if up
        && matches!(
            event.key,
            Key::LeftCommand | Key::RightCommand | Key::LeftOption | Key::RightOption
        )
    {
        inputs.push(key_input(MASK_KEY, 0, 0));
        inputs.push(key_input(MASK_KEY, 0, KEYEVENTF_KEYUP));
    }
    let mut flags = 0;
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    if keycode::is_extended(event.key) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    let scan = unsafe { MapVirtualKeyW(vk.into(), MAPVK_VK_TO_VSC) } as u16;
    inputs.push(key_input(vk, scan, flags));
}

/// Types text as Unicode: one press and one release per UTF-16 unit, independent of keyboard layout and input method.
fn push_text(inputs: &mut Vec<INPUT>, text: &str) {
    for unit in text.encode_utf16() {
        inputs.push(key_input(0, unit, KEYEVENTF_UNICODE));
        inputs.push(key_input(0, unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
    }
}
