//! kanami core: the platform-independent key event model and rule engine.
//!
//! The engine is pure logic: key events and time in, key events out, with no system API dependency,
//! so it can be unit tested directly and can replay key sequences offline to validate a config.

pub mod backend;
mod engine;
mod event;
mod hooks;
mod key;
mod mods;
mod rules;

pub use engine::{Api, Engine, Out};
pub use event::{KeyEvent, KeyState};
pub use hooks::Hooks;
pub use key::Key;
pub use mods::{ModKind, ModPattern, ModSet, Side};
pub use rules::{
    Action, Activation, Chord, Combo, Condition, DeviceSpec, Layer, Location, Remap, RemapTarget,
    Rule, RuleError, RuleErrorKind, RuleKind, Rules, Settings, Strategy, TapHold, Trigger,
};
