//! macOS backend: intercepts keys at the session level with a CGEventTap, hands them to the
//! [`Handler`], then passes them through, rewrites them or synthesizes new events.
//!
//! It writes the FFI directly instead of depending on a wrapper crate: few APIs are used, so this
//! keeps the binary smallest and the behavior fully under our control.

#![cfg(target_os = "macos")]

mod app;
mod devices;
mod ffi;
mod hid;
mod hud;
mod input;
mod keycode;
mod nsapp;
mod signal;
mod tap;

pub use devices::{input_monitoring_allowed, list as list_devices};
pub use hid::{GUARD_COMMAND, guard_main as caps_guard_main, reset as reset_caps_lock};
pub use hud::Hud;
pub use kanami_core::backend::{Handler, NO_OUTPUT, Options, TapMode, Waker};
pub use nsapp::show_menu_images;
pub use tap::{TapError, accessibility_trusted, run};
