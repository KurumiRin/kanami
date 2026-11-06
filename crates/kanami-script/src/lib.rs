//! Luau script host: implements [`Hooks`] and forwards engine callbacks to scripts.
//!
//! Scripts run in a Luau sandbox (built-in libraries are read-only, with no file or process access)
//! and can drive the engine only through the global table `kanami`. Each callback has an execution
//! time budget and is aborted when it runs over, and the key is handled as usual.
//! See `docs/scripting.md` for the API.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;
use std::rc::Rc;
use std::time::{Duration, Instant};

use kanami_config::{Config, Script};
use kanami_core::{Api, Engine, Hooks, KeyEvent, KeyState, ModKind, ModSet};
use mlua::{Function, Lua, MultiValue, Table, Value, VmState};

/// Execution time budget for each callback.
const CALLBACK_BUDGET: Duration = Duration::from_millis(20);
/// Time budget for loading a script (running its top-level code).
const LOAD_BUDGET: Duration = Duration::from_secs(1);

pub type Log = Box<dyn Fn(String)>;

/// Loads scripts according to the config for use by `engine`; gives `None` when the config uses no scripts.
/// Checks that every function a rule `call`s is defined in a script.
pub fn hooks_for(
    config: &Config,
    engine: &Engine,
    log: Log,
) -> Result<Option<Box<dyn Hooks>>, String> {
    let functions: Vec<&str> = engine.functions().collect();
    if config.scripts.is_empty() && functions.is_empty() {
        return Ok(None);
    }
    let host = ScriptHost::load(&config.scripts, log)?;
    let missing = host.missing(functions.into_iter());
    if !missing.is_empty() {
        return Err(kanami_i18n::tr!(
            "Functions that rules `call` are not defined in any script: {names}",
            names = missing.join(", ")
        ));
    }
    Ok(Some(Box::new(host)))
}

/// State shared between the script host and the API functions.
struct Shared {
    /// Points to the current [`Api`] during a callback and is null otherwise.
    api: Cell<*mut c_void>,
    /// Handlers registered by `kanami.on_key`, called in registration order.
    on_key: RefCell<Vec<Function>>,
    timers: RefCell<HashMap<u32, Function>>,
    deadline: Cell<Instant>,
}

pub struct ScriptHost {
    lua: Lua,
    shared: Rc<Shared>,
    log: Rc<Log>,
}

impl ScriptHost {
    /// Loads scripts in order. `log` receives the scripts' `kanami.log` output and runtime errors.
    pub fn load(scripts: &[Script], log: Log) -> Result<Self, String> {
        let lua = Lua::new();
        let shared = Rc::new(Shared {
            api: Cell::new(std::ptr::null_mut()),
            on_key: RefCell::new(Vec::new()),
            timers: RefCell::new(HashMap::new()),
            deadline: Cell::new(Instant::now() + LOAD_BUDGET),
        });
        let deadline = Rc::clone(&shared);
        lua.set_interrupt(move |_| {
            if Instant::now() > deadline.deadline.get() {
                Err(mlua::Error::runtime(kanami_i18n::tr!(
                    "Script execution timed out"
                )))
            } else {
                Ok(VmState::Continue)
            }
        });
        let log = Rc::new(log);
        let init_failed = |e: mlua::Error| {
            kanami_i18n::tr!(
                "Initializing the script environment failed: {error}",
                error = e
            )
        };
        install_api(&lua, &shared, &log).map_err(init_failed)?;
        // The require provided by mlua can load modules from disk and would bypass the sandbox; scripts are all loaded through [script] files
        lua.globals()
            .raw_set("require", Value::Nil)
            .map_err(init_failed)?;
        lua.sandbox(true).map_err(|e| {
            kanami_i18n::tr!("Enabling the script sandbox failed: {error}", error = e)
        })?;
        for script in scripts {
            shared.deadline.set(Instant::now() + LOAD_BUDGET);
            lua.load(&script.code)
                .set_name(format!("={}", script.name))
                .exec()
                .map_err(|e| {
                    kanami_i18n::tr!(
                        "Loading script {name} failed: {error}",
                        name = script.name,
                        error = clean(&e)
                    )
                })?;
        }
        Ok(ScriptHost { lua, shared, log })
    }

    /// Returns the functions that rules `call` but no script defines.
    pub fn missing<'a>(&self, functions: impl Iterator<Item = &'a str>) -> Vec<String> {
        let globals = self.lua.globals();
        functions
            .filter(|name| !matches!(globals.get::<Value>(*name), Ok(Value::Function(_))))
            .map(str::to_owned)
            .collect()
    }

    /// Lets API functions reach the engine during a callback and clears it afterwards.
    fn with_api<R>(&self, api: &mut Api<'_>, f: impl FnOnce() -> mlua::Result<R>) -> Option<R> {
        self.shared.api.set((api as *mut Api<'_>).cast());
        self.shared.deadline.set(Instant::now() + CALLBACK_BUDGET);
        let result = f();
        self.shared.api.set(std::ptr::null_mut());
        match result {
            Ok(value) => Some(value),
            Err(e) => {
                (self.log)(kanami_i18n::tr!(
                    "kanami: script error: {error}",
                    error = clean(&e)
                ));
                None
            }
        }
    }
}

impl Hooks for ScriptHost {
    fn wants_keys(&self) -> bool {
        !self.shared.on_key.borrow().is_empty()
    }

    /// Down and repeat: calls handlers in turn and stops at the first that returns `true`; up: calls all of them and ignores the return values.
    fn on_key(&mut self, api: &mut Api<'_>, event: KeyEvent) -> bool {
        // Take the list out for the duration of the call (no allocation); handlers registered during the call are merged back afterwards
        let handlers = std::mem::take(&mut *self.shared.on_key.borrow_mut());
        let lua = &self.lua;
        let handled = self.with_api(api, || {
            let e = lua.create_table()?;
            e.set("key", event.key.name())?;
            e.set(
                "state",
                match event.state {
                    KeyState::Down => "down",
                    KeyState::Up => "up",
                    KeyState::Repeat => "repeat",
                },
            )?;
            e.set("mods", mods_table(lua, event.mods)?)?;
            for handler in &handlers {
                let handled = handler.call::<Option<bool>>(&e)?.unwrap_or(false);
                if handled && event.state != KeyState::Up {
                    return Ok(true);
                }
            }
            Ok(false)
        });
        let mut slot = self.shared.on_key.borrow_mut();
        let added = std::mem::replace(&mut *slot, handlers);
        slot.extend(added);
        handled.unwrap_or(false)
    }

    fn call(&mut self, api: &mut Api<'_>, function: &str) {
        let lua = &self.lua;
        self.with_api(api, || match lua.globals().get::<Value>(function)? {
            Value::Function(f) => f.call::<()>(()),
            _ => Err(mlua::Error::runtime(kanami_i18n::tr!(
                "Function {name} does not exist",
                name = function
            ))),
        });
    }

    fn timer(&mut self, api: &mut Api<'_>, id: u32) {
        let Some(callback) = self.shared.timers.borrow_mut().remove(&id) else {
            return;
        };
        self.with_api(api, || callback.call::<()>(()));
    }
}

/// Strips the redundant prefix from an mlua error, keeping only the part meant for the user.
fn clean(e: &mlua::Error) -> String {
    let text = e.to_string();
    text.strip_prefix("runtime error: ")
        .unwrap_or(&text)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn mods_table(lua: &Lua, mods: ModSet) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    for (kind, name) in [
        (ModKind::Control, "ctrl"),
        (ModKind::Option, "opt"),
        (ModKind::Shift, "shift"),
        (ModKind::Command, "cmd"),
    ] {
        let (left, right) = kind.bits();
        t.set(name, mods.intersects(left.union(right)))?;
    }
    t.set("fn", mods.contains(ModSet::FN))?;
    Ok(t)
}

/// Gets the current callback's [`Api`].
///
/// Safety: the pointer is non-null only during [`ScriptHost::with_api`] and points to an `Api` alive
/// on the caller's stack; scripts run only on the same thread and within the same call stack.
fn with<R>(shared: &Shared, f: impl FnOnce(&mut Api<'_>) -> R) -> mlua::Result<R> {
    let ptr = shared.api.get().cast::<Api<'static>>();
    if ptr.is_null() {
        return Err(mlua::Error::runtime(kanami_i18n::tr!(
            "This kanami function can only be used inside a callback (not while a script is loading)"
        )));
    }
    Ok(f(unsafe { &mut *ptr }))
}

fn install_api(lua: &Lua, shared: &Rc<Shared>, log: &Rc<Log>) -> mlua::Result<()> {
    let k = lua.create_table()?;

    let s = Rc::clone(shared);
    k.set(
        "on_key",
        lua.create_function(move |_, f: Function| {
            s.on_key.borrow_mut().push(f);
            Ok(())
        })?,
    )?;

    let s = Rc::clone(shared);
    k.set(
        "tap",
        lua.create_function(move |_, exprs: MultiValue| {
            let mut chords = Vec::new();
            for value in exprs {
                let expr = value.to_string()?;
                let chord = kanami_config::parse_chord(&expr).map_err(mlua::Error::runtime)?;
                chords.push(chord);
            }
            with(&s, |api| {
                for chord in chords {
                    if !api.tap(chord) {
                        return Err(mlua::Error::runtime(kanami_i18n::tr!(
                            "caps_lock cannot be used as output"
                        )));
                    }
                }
                Ok(())
            })?
        })?,
    )?;

    let s = Rc::clone(shared);
    k.set(
        "type",
        lua.create_function(move |_, text: String| with(&s, |api| api.type_text(&text)))?,
    )?;

    let s = Rc::clone(shared);
    k.set(
        "run",
        lua.create_function(move |_, command: String| with(&s, |api| api.run(&command)))?,
    )?;

    let s = Rc::clone(shared);
    k.set(
        "after",
        lua.create_function(move |_, (ms, f): (u64, Function)| {
            let id = with(&s, |api| api.after(ms))?;
            s.timers.borrow_mut().insert(id, f);
            Ok(id)
        })?,
    )?;

    let s = Rc::clone(shared);
    k.set(
        "cancel",
        lua.create_function(move |_, id: u32| {
            s.timers.borrow_mut().remove(&id);
            with(&s, |api| api.cancel(id))
        })?,
    )?;

    let s = Rc::clone(shared);
    k.set(
        "layer",
        lua.create_function(move |_, (name, on): (String, Option<bool>)| {
            with(&s, |api| api.set_layer(&name, on.unwrap_or(true)))?
                .then_some(())
                .ok_or_else(|| {
                    mlua::Error::runtime(kanami_i18n::tr!(
                        "Layer {name} does not exist",
                        name = name
                    ))
                })
        })?,
    )?;

    let s = Rc::clone(shared);
    k.set(
        "layer_active",
        lua.create_function(move |_, name: String| with(&s, |api| api.layer_active(&name)))?,
    )?;

    let s = Rc::clone(shared);
    k.set(
        "set",
        lua.create_function(move |_, (name, value): (String, Value)| {
            let value = match value {
                Value::Nil => None,
                other => Some(other.to_string()?),
            };
            with(&s, |api| api.set_var(&name, value.as_deref()))
        })?,
    )?;

    let s = Rc::clone(shared);
    k.set(
        "get",
        lua.create_function(move |_, name: String| {
            with(&s, |api| api.var(&name).map(str::to_owned))
        })?,
    )?;

    let s = Rc::clone(shared);
    k.set(
        "app",
        lua.create_function(move |_, ()| with(&s, |api| api.app().map(str::to_owned)))?,
    )?;

    let s = Rc::clone(shared);
    k.set(
        "input_source",
        lua.create_function(move |_, ()| with(&s, |api| api.input_source().map(str::to_owned)))?,
    )?;

    let s = Rc::clone(shared);
    k.set(
        "mods",
        lua.create_function(move |lua, ()| {
            let mods = with(&s, |api| api.mods())?;
            mods_table(lua, mods)
        })?,
    )?;

    let s = Rc::clone(shared);
    k.set(
        "now",
        lua.create_function(move |_, ()| with(&s, |api| api.now()))?,
    )?;

    let log = Rc::clone(log);
    k.set(
        "log",
        lua.create_function(move |_, values: MultiValue| {
            let parts: Vec<String> = values
                .iter()
                .map(|v| v.to_string())
                .collect::<mlua::Result<_>>()?;
            log(kanami_i18n::tr!(
                "[script] {message}",
                message = parts.join(" ")
            ));
            Ok(())
        })?,
    )?;

    lua.globals().set("kanami", k)
}

#[cfg(test)]
mod tests;
