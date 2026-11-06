use crate::{Api, KeyEvent};

/// The interface for extensions such as scripts. All callbacks run synchronously on the key-handling thread and must be fast.
pub trait Hooks {
    /// Whether [`on_key`](Self::on_key) is needed. The engine skips the callback when it is not.
    fn wants_keys(&self) -> bool {
        false
    }

    /// Every logical key event (after `[remap]`, before other rules) is offered here first.
    /// Returning `true` for a press or repeat means it is handled and the engine does not process it; a release always finishes the normal way and the return value is ignored.
    fn on_key(&mut self, _api: &mut Api<'_>, _event: KeyEvent) -> bool {
        false
    }

    /// A rule's `call = "function_name"`.
    fn call(&mut self, api: &mut Api<'_>, function: &str);

    /// A timer set by [`Api::after`] has expired.
    fn timer(&mut self, api: &mut Api<'_>, id: u32);
}
