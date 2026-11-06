//! Windows backend: intercepts keys with a low-level keyboard hook (`WH_KEYBOARD_LL`), hands them to
//! the [`Handler`], then passes them through, swallows them or emits new events with `SendInput`.
//!
//! All callbacks (hook, timers, foreground window changes) run in the message loop of one thread, so no locking is needed.

#![cfg(windows)]

mod app;
mod hook;
mod keycode;
mod sys;

pub use hook::{RunError, run};
pub use kanami_core::backend::{Handler, NO_OUTPUT, Options, TapMode};
pub use sys::{InstanceLock, local_time, lock_instance};
