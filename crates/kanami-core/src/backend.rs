//! The interface between platform backends and the layers above. Type definitions only, with no system calls; each platform backend implements `run(options, handler)`.

use std::sync::Arc;

use crate::{Key, KeyEvent, KeyState, ModSet, Out};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TapMode {
    /// May swallow or rewrite events.
    Filter,
    /// Read-only listening; the handler's output is ignored.
    ListenOnly,
}

#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub mode: TapMode,
    /// Calls [`Handler::poll`] once a second (to check for config changes).
    pub poll: bool,
    /// Runs as a resident GUI app, which the menu bar / tray icon needs (on macOS this uses the NSApplication event loop instead).
    pub app: bool,
}

/// Lets other threads wake the backend's thread, after which the backend calls [`Handler::wake`] on that thread.
#[derive(Clone)]
pub struct Waker(Arc<dyn Fn() + Send + Sync>);

impl Waker {
    pub fn new(wake: impl Fn() + Send + Sync + 'static) -> Self {
        Waker(Arc::new(wake))
    }

    pub fn wake(&self) {
        (self.0)()
    }
}

/// Returned when there is no output.
pub static NO_OUTPUT: Out = Out::new();

/// The physical keyboard that produced a key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceInfo {
    /// An ID assigned by the backend; it stays the same while the device is connected.
    pub id: u32,
    pub vendor_id: u32,
    pub product_id: u32,
    /// The product name, such as `Magic Keyboard`.
    pub product: String,
}

/// A key on the layer HUD. Position and width are in key widths, and `y` is the row (counting from the top).
#[derive(Clone, Debug, PartialEq)]
pub struct HudKey {
    pub key: Key,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub label: String,
    pub style: HudStyle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudStyle {
    /// Not changed in this layer; falls through to the layer below.
    Plain,
    /// Changed in this layer; `label` is its output now.
    Mapped,
    /// The key that enters this layer.
    Trigger,
}

/// Backend callbacks. All methods are called on the same thread; the returned keys and text are posted by the backend (the caller handles `runs` itself).
pub trait Handler {
    /// Hot path: must be fast and must not block. `now` is the time in milliseconds.
    /// `device` is the keyboard that produced this key, or `None` if the backend cannot tell.
    fn event(&mut self, event: KeyEvent, device: Option<&DeviceInfo>, now: u64) -> &Out;

    fn tick(&mut self, _now: u64) -> &Out {
        &NO_OUTPUT
    }

    /// The time at which [`tick`](Self::tick) next needs to be called.
    fn deadline(&self) -> Option<u64> {
        None
    }

    /// The modifiers that should currently be down. When this returns `Some`, the backend uses it to correct the modifier flags of mouse events,
    /// so that a remapped modifier plus a click works (such as Ctrl+click when Caps Lock acts as Ctrl).
    fn mods(&self) -> Option<ModSet> {
        None
    }

    /// The frontmost app changed. On macOS this is the Bundle ID; on Windows it is the executable file name (such as `notepad.exe`).
    fn app_changed(&mut self, _app: Option<&str>) {}

    /// The input method changed. On macOS this is the input source ID; on Windows it is the keyboard layout (such as `00000804`).
    fn input_source_changed(&mut self, _id: Option<&str>) {}

    fn poll(&mut self, _now: u64) -> &Out {
        &NO_OUTPUT
    }

    /// Whether to identify which keyboard a key came from (needs extra permission, so only enabled when used).
    fn wants_devices(&self) -> bool {
        false
    }

    /// Whether Caps Lock should be handled as an ordinary key.
    fn wants_caps_lock(&self) -> bool {
        false
    }

    /// The system paused event listening (callback too slow, etc.) and the backend resumed it immediately. Events during the pause do not pass through kanami.
    fn tap_disabled(&mut self, _reason: &'static str) {}

    /// A key synthesized by another program (such as Logitech Options+ or a password manager's autofill). The backend passes it through unchanged without giving it to the engine,
    /// since like Karabiner it only remaps the physical keyboard; it is used for display only here. `source` is the PID that posted it, or 0 if unknown.
    fn foreign(&mut self, _event: KeyEvent, _source: u32) {}

    /// Whether the engine thinks this physical key is still down. When idle, the backend checks against the hardware state to find releases that bypassed kanami.
    fn held(&self, _key: Key) -> bool {
        false
    }

    /// This key has actually been released, but the release did not pass through kanami (secure input, paused listening, sleep, etc.); deliver a release now.
    fn lost_release(&mut self, key: Key, now: u64) -> &Out {
        let event = KeyEvent {
            key,
            state: KeyState::Up,
            mods: ModSet::empty(),
        };
        self.event(event, None, now)
    }

    /// Called once when the backend is ready, before it enters the event loop (on the backend thread; in `app` mode the menu bar icon can be created here).
    fn started(&mut self, _waker: Waker) {}

    /// A thread called [`Waker::wake`].
    fn wake(&mut self, _now: u64) -> &Out {
        &NO_OUTPUT
    }

    /// When this returns true the backend leaves the event loop ([`shutdown`](Self::shutdown) is still called afterwards).
    fn should_quit(&self) -> bool {
        false
    }

    /// Called before exit; returns the release events that still need to be sent.
    fn shutdown(&mut self) -> &Out {
        &NO_OUTPUT
    }
}
