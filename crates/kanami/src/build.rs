//! Builds the engine from the config: compiles the rules and loads the scripts. Shared by `run`, `check` and `test`.

use kanami_config::Config;
use kanami_core::{Engine, Hooks};

pub type Log = Box<dyn Fn(String)>;

pub fn engine(config: &Config, log: Log) -> Result<Engine, String> {
    let mut engine = Engine::new(&config.rules).map_err(|e| e.to_string())?;
    if let Some(hooks) = hooks_for(config, &engine, log)? {
        engine.set_hooks(hooks);
    }
    Ok(engine)
}

/// For `[[test]]`: each test gets a fresh script host, so they do not affect each other.
pub fn test_hooks(config: &Config, log: Log) -> Result<Option<Box<dyn Hooks>>, String> {
    let engine = Engine::new(&config.rules).map_err(|e| e.to_string())?;
    hooks_for(config, &engine, log)
}

#[cfg(feature = "script")]
use kanami_script::hooks_for;

#[cfg(not(feature = "script"))]
use kanami_i18n::tr;

#[cfg(not(feature = "script"))]
fn hooks_for(
    config: &Config,
    engine: &Engine,
    _log: Log,
) -> Result<Option<Box<dyn Hooks>>, String> {
    if config.scripts.is_empty() && engine.functions().next().is_none() {
        Ok(None)
    } else {
        Err(tr!(
            "This kanami was built without the script feature, so [script] and call are unavailable"
        ))
    }
}
