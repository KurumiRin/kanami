use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use kanami_config::Config;
use kanami_core::{Key, Rules};
use kanami_i18n::tr;

mod build;
#[cfg(any(target_os = "macos", windows))]
mod daemon;
#[cfg(any(target_os = "macos", windows))]
mod hud;
#[cfg(any(target_os = "macos", windows))]
mod ipc;
#[cfg(any(target_os = "macos", windows))]
mod platform;
#[cfg(target_os = "macos")]
mod service;
#[cfg(any(target_os = "macos", windows))]
mod tray;

fn usage() -> String {
    tr!(
        "\
kanami — a small, fast, programmable keyboard remapper

Usage:
  kanami run     [-c <config>] [--trace] [--tray]
                                   Remap keys using the config (runs in the foreground, Ctrl+C quits, saving the config reloads it)
                                   --trace prints the input and output of every key, for troubleshooting
                                   --tray  shows the menu bar / tray icon (shown by default for the background service)
  kanami ctl     status|pause|resume|reload|devices|events|quit
                                   Control a running kanami; events keeps printing the input and output of every key
  kanami monitor                   Listen only; print key events and the frontmost app
  kanami check   [-c <config>]     Validate the config
  kanami test    [-c <config>]     Run the [[test]] cases in the config
  kanami keys                      List key names and how to write modifiers
  kanami devices                   List connected keyboards, for use in [device.*]
  kanami schema                    Print the JSON Schema of the config file
  kanami reset                     Remove the Caps Lock mapping kanami left behind (use after an abnormal exit)
  kanami service install [-c <config>]
                                   Install as a background service: starts at login, restarts after a crash
  kanami service uninstall         Stop and uninstall the background service
  kanami service status            Show the background service status and recent log
  kanami service restart           Restart the background service

The config file defaults to $KANAMI_CONFIG, then ~/.config/kanami/config.toml (Windows: %APPDATA%\\kanami\\config.toml)"
    )
}

fn main() -> ExitCode {
    kanami_i18n::set(kanami_i18n::system());
    let args: Vec<String> = env::args().skip(1).collect();
    match cli(&args) {
        Ok(code) => code,
        Err(msg) => {
            eprintln!("kanami: {msg}");
            ExitCode::FAILURE
        }
    }
}

fn cli(args: &[String]) -> Result<ExitCode, String> {
    let Some((command, rest)) = args.split_first() else {
        println!("{}", usage());
        return Ok(ExitCode::SUCCESS);
    };
    match command.as_str() {
        #[cfg(target_os = "macos")]
        kanami_macos::GUARD_COMMAND => kanami_macos::caps_guard_main(),
        "run" => {
            let flag = |name: &str| rest.iter().any(|a| a == name);
            let (trace, service, tray) = (flag("--trace"), flag("--service"), flag("--tray"));
            let rest: Vec<String> = rest
                .iter()
                .filter(|a| !matches!(a.as_str(), "--trace" | "--service" | "--tray"))
                .cloned()
                .collect();
            if rest.is_empty() {
                ensure_default_config()?;
            }
            run(&config_path(&rest)?, trace, service, service || tray)
        }
        "ctl" => ctl(rest),
        "service" => service_command(rest),
        "monitor" => no_args(rest).and_then(|()| monitor()),
        "check" => check(&config_path(rest)?),
        "test" => test(&config_path(rest)?),
        "keys" => no_args(rest).map(|()| keys()),
        "devices" => no_args(rest).and_then(|()| devices()),
        "schema" => no_args(rest).map(|()| {
            print!("{}", kanami_config::schema());
            ExitCode::SUCCESS
        }),
        "reset" => no_args(rest).and_then(|()| reset()),
        "-h" | "--help" | "help" => {
            println!("{}", usage());
            Ok(ExitCode::SUCCESS)
        }
        "-V" | "--version" => {
            println!("kanami {}", env!("CARGO_PKG_VERSION"));
            Ok(ExitCode::SUCCESS)
        }
        other => Err(tr!(
            "Unknown command \"{command}\". Run `kanami help` for usage.",
            command = other
        )),
    }
}

fn no_args(args: &[String]) -> Result<(), String> {
    match args.first() {
        None => Ok(()),
        Some(arg) => Err(tr!("Unexpected argument \"{arg}\"", arg = arg)),
    }
}

fn config_path(args: &[String]) -> Result<PathBuf, String> {
    match args {
        [] => default_config_path(),
        [flag, path] if flag == "-c" || flag == "--config" => Ok(PathBuf::from(path)),
        [flag] if flag == "-c" || flag == "--config" => {
            Err(tr!("{flag} needs a config file path after it", flag = flag))
        }
        [arg, ..] => Err(tr!("Unrecognized argument \"{arg}\"", arg = arg)),
    }
}

fn default_config_path() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("KANAMI_CONFIG") {
        return Ok(path.into());
    }
    // Windows: %APPDATA%\kanami\config.toml
    if cfg!(windows)
        && let Some(appdata) = env::var_os("APPDATA")
    {
        return Ok(PathBuf::from(appdata).join("kanami").join("config.toml"));
    }
    let home = env::var_os("HOME")
        .ok_or_else(|| tr!("Cannot find the HOME directory. Specify a config file with -c."))?;
    Ok(PathBuf::from(home).join(".config/kanami/config.toml"))
}

/// Creates an empty config when the default location has none: no remaps, rules are up to the user.
fn ensure_default_config() -> Result<(), String> {
    let path = default_config_path()?;
    if path.exists() {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| {
            tr!(
                "Failed to create {path}: {error}",
                path = dir.display(),
                error = e
            )
        })?;
    }
    fs::write(&path, "").map_err(|e| {
        tr!(
            "Failed to create {path}: {error}",
            path = path.display(),
            error = e
        )
    })
}

/// Loads and validates the config, then switches to its language. Errors start with the file
/// path so editors can jump to them.
fn load(path: &Path) -> Result<Config, String> {
    let config = kanami_config::load(path).map_err(|e| match e {
        kanami_config::ConfigError::Io(..) => e.to_string(),
        _ => format!("{}:{e}", path.display()),
    })?;
    apply_language(&config.rules);
    Ok(config)
}

/// Uses the language from the config, or the system language when it sets none.
fn apply_language(rules: &Rules) {
    kanami_i18n::set(rules.settings.language.unwrap_or_else(kanami_i18n::system));
}

fn summary(config: &Config) -> String {
    let r = &config.rules;
    let mut s = tr!(
        "remaps: {remaps}, rules: {rules}, layers: {layers}, combos: {combos}",
        remaps = r.remaps.len(),
        rules = r.rules.len(),
        layers = r.layers.len(),
        combos = r.combos.len()
    );
    if !config.scripts.is_empty() {
        s.push_str(&tr!(", scripts: {scripts}", scripts = config.scripts.len()));
    }
    s
}

/// Script logs go straight to the terminal (for `check` and `test`).
fn print_log() -> build::Log {
    Box::new(|line| eprintln!("{line}"))
}

fn check(path: &Path) -> Result<ExitCode, String> {
    let config = load(path)?;
    // Load the scripts once to confirm they run and every function used by call exists
    build::engine(&config, print_log())?;
    println!(
        "{}",
        tr!(
            "{path}: Config is valid, {summary}",
            path = path.display(),
            summary = summary(&config)
        )
    );
    Ok(ExitCode::SUCCESS)
}

fn test(path: &Path) -> Result<ExitCode, String> {
    let config = load(path)?;
    if config.tests.is_empty() {
        println!(
            "{}",
            tr!("{path} contains no [[test]] cases", path = path.display())
        );
        return Ok(ExitCode::SUCCESS);
    }
    let mut failed = 0;
    for case in &config.tests {
        match case.run(&config.rules, build::test_hooks(&config, print_log())?) {
            Ok(()) => println!("✓ {}", case.name),
            Err(m) => {
                failed += 1;
                println!(
                    "{}",
                    tr!("✗ {name} (line {line})", name = case.name, line = case.line)
                );
                println!("    {}", tr!("Expected: {expected}", expected = m.expected));
                println!("    {}", tr!("Actual: {actual}", actual = m.actual));
            }
        }
    }
    let passed = config.tests.len() - failed;
    println!(
        "\n{}",
        tr!(
            "{passed} passed, {failed} failed",
            passed = passed,
            failed = failed
        )
    );
    Ok(if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn keys() -> ExitCode {
    println!("{}", tr!("Key names:"));
    for key in Key::ALL {
        println!("  {}", key.name());
    }
    println!("\n{}", tr!("Modifier names for key chords:"));
    for name in kanami_config::modifier_names() {
        println!("  {name}");
    }
    ExitCode::SUCCESS
}

#[cfg(any(target_os = "macos", windows))]
fn run(path: &Path, trace: bool, service: bool, tray: bool) -> Result<ExitCode, String> {
    let mut summary_line = String::new();
    let config = match load(path) {
        Ok(config) => {
            summary_line = summary(&config);
            Ok(config)
        }
        // In service mode a bad config does not exit, or launchd would restart us over and over
        Err(e) if service => Err(e),
        Err(e) => return Err(e),
    };
    let options = daemon::RunOptions {
        trace,
        service,
        summary: summary_line,
        tray,
    };
    daemon::run(path, config, options)?;
    Ok(ExitCode::SUCCESS)
}

#[cfg(target_os = "macos")]
fn service_command(args: &[String]) -> Result<ExitCode, String> {
    match args.split_first() {
        Some((sub, rest)) if sub == "install" => {
            if rest.is_empty() {
                ensure_default_config()?;
            }
            let path = config_path(rest)?;
            let config = load(&path)?;
            println!(
                "{}",
                tr!("Config is valid: {summary}", summary = summary(&config))
            );
            service::install(&path)
        }
        Some((sub, rest)) if sub == "uninstall" => {
            no_args(rest).and_then(|()| service::uninstall())
        }
        Some((sub, rest)) if sub == "status" => no_args(rest).and_then(|()| service::status()),
        Some((sub, rest)) if sub == "restart" => no_args(rest).and_then(|()| service::restart()),
        Some((sub, _)) => Err(tr!(
            "Unknown service subcommand \"{sub}\". Available: install, uninstall, status, restart.",
            sub = sub
        )),
        None => Err(tr!(
            "Usage: kanami service install|uninstall|status|restart"
        )),
    }
}

#[cfg(any(target_os = "macos", windows))]
fn monitor() -> Result<ExitCode, String> {
    daemon::monitor()?;
    Ok(ExitCode::SUCCESS)
}

#[cfg(target_os = "macos")]
fn reset() -> Result<ExitCode, String> {
    match kanami_macos::reset_caps_lock() {
        Ok(true) => println!("{}", tr!("Caps Lock mapping removed")),
        Ok(false) => println!("{}", tr!("No mapping to remove")),
        Err(e) => return Err(tr!("Failed to run hidutil: {error}", error = e)),
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(not(target_os = "macos"))]
fn unsupported(what: &str) -> Result<ExitCode, String> {
    Err(tr!(
        "{what} is only supported on macOS for now",
        what = what
    ))
}

#[cfg(not(any(target_os = "macos", windows)))]
fn run(_path: &Path, _trace: bool, _service: bool, _tray: bool) -> Result<ExitCode, String> {
    unsupported(&tr!("Key remapping"))
}

#[cfg(any(target_os = "macos", windows))]
fn ctl(args: &[String]) -> Result<ExitCode, String> {
    let command = match args {
        [c] if matches!(
            c.as_str(),
            "status" | "pause" | "resume" | "reload" | "devices" | "quit"
        ) =>
        {
            c.as_str()
        }
        [c] if c == "events" => "subscribe",
        _ => {
            return Err(tr!(
                "Usage: kanami ctl status|pause|resume|reload|devices|events|quit"
            ));
        }
    };
    ipc::request(command)?;
    Ok(ExitCode::SUCCESS)
}

#[cfg(not(any(target_os = "macos", windows)))]
fn ctl(_args: &[String]) -> Result<ExitCode, String> {
    unsupported("ctl")
}

#[cfg(not(any(target_os = "macos", windows)))]
fn monitor() -> Result<ExitCode, String> {
    unsupported(&tr!("Monitoring"))
}

#[cfg(target_os = "macos")]
fn devices() -> Result<ExitCode, String> {
    let devices = kanami_macos::list_devices();
    if devices.is_empty() {
        println!("{}", tr!("No keyboards found"));
        return Ok(ExitCode::SUCCESS);
    }
    println!("vendor_id  product_id  {}", tr!("Product name"));
    for d in &devices {
        println!(
            "{:#06x}     {:#06x}      {}",
            d.vendor_id, d.product_id, d.product
        );
    }
    let d = &devices[0];
    println!(
        "\n{}\n",
        tr!(
            "Write it like this in the config; remaps and rules for this keyboard go under it ([device.my_keyboard.remap], [[device.my_keyboard.rule]]):"
        )
    );
    println!(
        "{}",
        tr!(
            "# Every field you write must match; keyboards with the same IDs can be told apart with product = \"part of the product name\""
        )
    );
    println!("[device.my_keyboard]");
    println!("vendor_id = {:#06x}", d.vendor_id);
    println!("product_id = {:#06x}", d.product_id);
    Ok(ExitCode::SUCCESS)
}

#[cfg(not(target_os = "macos"))]
fn devices() -> Result<ExitCode, String> {
    unsupported(&tr!("Per-keyboard settings"))
}

#[cfg(not(target_os = "macos"))]
fn service_command(_args: &[String]) -> Result<ExitCode, String> {
    unsupported(&tr!("The background service"))
}

#[cfg(not(target_os = "macos"))]
fn reset() -> Result<ExitCode, String> {
    unsupported("reset")
}
