//! The kanami settings window: started on demand, and quits when the window is closed.
//!
//! Remapping is done by the background service; this only reads status, sends commands and
//! subscribes to key events over IPC. Config validation and tests call `kanami-config` directly,
//! the same code as the command-line `kanami check` / `kanami test`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod edit;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::UNIX_EPOCH;

use kanami_config::ConfigError;
use kanami_core::Engine;
use kanami_i18n::{Lang, tr};
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

/// How many history versions to keep at most.
const HISTORY: usize = 50;

/// Startup argument `--page <page>`: opening from the menu bar's "Event Viewer…" goes straight to that page.
struct InitialPage(String);

fn page_from(args: &[String]) -> Option<String> {
    let i = args.iter().position(|a| a == "--page")?;
    args.get(i + 1).cloned()
}

#[tauri::command]
fn platform() -> &'static str {
    if cfg!(windows) { "win" } else { "mac" }
}

#[tauri::command]
fn initial_page(page: tauri::State<InitialPage>) -> String {
    page.0.clone()
}

/// The system language as a tag such as `en` or `zh-CN`.
#[tauri::command]
fn system_language() -> String {
    kanami_i18n::system().tag().to_owned()
}

/// Switches the language of this process and the window title. Unknown tags keep the current language.
#[tauri::command]
fn set_language(app: AppHandle, tag: String) {
    if let Some(lang) = Lang::parse(&tag) {
        kanami_i18n::set(lang);
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_title(&tr!("kanami Settings"));
    }
}

/// The background service status; an error when it is not running.
#[tauri::command]
fn status() -> Result<Value, String> {
    let reply = kanami_ipc::request("status")?;
    Ok(reply["status"].clone())
}

/// Pause, resume, reload, quit.
#[tauri::command]
fn control(command: &str) -> Result<(), String> {
    if !matches!(command, "pause" | "resume" | "reload" | "quit") {
        return Err(tr!("Unsupported command {command}", command = command));
    }
    let reply = kanami_ipc::request(command)?;
    match reply["error"].as_str() {
        Some(error) => Err(error.to_owned()),
        None => Ok(()),
    }
}

/// Set while this window has paused remapping for the key picker.
static RECORDING: AtomicBool = AtomicBool::new(false);

/// While the key picker is open: pauses remapping so the keys pressed are the keyboard's own, and
/// removes the menu so combinations such as ⌘Q and ⌘W can be recorded. Resumes when it closes.
/// Returns whether the background service is running.
#[tauri::command]
fn recording(app: AppHandle, on: bool) -> Result<bool, String> {
    let connected = kanami_ipc::request("status").ok();
    if on {
        let _ = app.remove_menu();
        if let Some(reply) = &connected
            && reply["status"]["paused"] == false
            && kanami_ipc::request("pause").is_ok()
        {
            RECORDING.store(true, Ordering::SeqCst);
        }
    } else {
        stop_recording();
        let menu = tauri::menu::Menu::default(&app).map_err(|e| e.to_string())?;
        app.set_menu(menu).map_err(|e| e.to_string())?;
    }
    Ok(connected.is_some())
}

fn stop_recording() {
    if RECORDING.swap(false, Ordering::SeqCst) {
        let _ = kanami_ipc::request("resume");
    }
}

static SUBSCRIBED: AtomicBool = AtomicBool::new(false);

/// Subscribes to key events and turns them into the frontend's `kanami-event` events; sends `kanami-disconnected` when the background service quits.
#[tauri::command]
fn subscribe(app: AppHandle) -> Result<(), String> {
    if SUBSCRIBED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let events = match kanami_ipc::subscribe() {
        Ok(events) => events,
        Err(e) => {
            SUBSCRIBED.store(false, Ordering::SeqCst);
            return Err(e);
        }
    };
    std::thread::spawn(move || {
        // The first line is the reply to the subscription
        for event in events.skip(1) {
            let _ = app.emit("kanami-event", event);
        }
        SUBSCRIBED.store(false, Ordering::SeqCst);
        let _ = app.emit("kanami-disconnected", ());
    });
    Ok(())
}

/// The config file in use: the background service decides when it is running, otherwise the default location.
#[tauri::command]
fn config_path() -> Result<String, String> {
    if let Ok(status) = kanami_ipc::request("status")
        && let Some(path) = status["status"]["config"].as_str()
    {
        return Ok(path.to_owned());
    }
    Ok(default_config()?.display().to_string())
}

/// Same as the command line: `$KANAMI_CONFIG`, then `~/.config/kanami/config.toml`
/// (Windows: `%APPDATA%\kanami\config.toml`).
fn default_config() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("KANAMI_CONFIG") {
        return Ok(PathBuf::from(path));
    }
    if cfg!(windows) {
        return Ok(kanami_ipc::data_dir()?.join("config.toml"));
    }
    let home = std::env::var_os("HOME").ok_or_else(|| tr!("Cannot find the HOME directory"))?;
    Ok(PathBuf::from(home).join(".config/kanami/config.toml"))
}

#[derive(Serialize)]
struct ConfigFile {
    text: String,
    /// Modification time (milliseconds); 0 when the file does not exist.
    modified: u64,
}

#[tauri::command]
fn read_config(path: &str) -> Result<ConfigFile, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => {
            return Err(tr!(
                "Failed to read {path}: {error}",
                path = path,
                error = e
            ));
        }
    };
    Ok(ConfigFile {
        text,
        modified: modified(Path::new(path)),
    })
}

/// The file modification time (milliseconds), used to detect outside edits.
#[tauri::command]
fn config_modified(path: &str) -> u64 {
    modified(Path::new(path))
}

fn modified(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as u64)
}

#[derive(Serialize)]
struct Problem {
    line: usize,
    column: usize,
    message: String,
}

#[derive(Serialize)]
struct TestResult {
    name: String,
    line: usize,
    passed: bool,
    expected: String,
    actual: String,
}

#[derive(Serialize)]
struct Check {
    problem: Option<Problem>,
    summary: Option<String>,
    tests: Vec<TestResult>,
}

/// Validates config text and runs its `[[test]]` cases (without writing the file).
#[tauri::command]
fn check_config(path: &str, text: &str) -> Check {
    let base = Path::new(path).parent();
    let config = match kanami_config::parse_in(text, base) {
        Ok(config) => config,
        Err(e) => {
            return Check {
                problem: Some(problem(text, e)),
                summary: None,
                tests: Vec::new(),
            };
        }
    };
    let rules = &config.rules;
    let summary = tr!(
        "remaps: {remaps}, rules: {rules}, layers: {layers}, combos: {combos}",
        remaps = rules.remaps.len(),
        rules = rules.rules.len(),
        layers = rules.layers.len(),
        combos = rules.combos.len()
    );
    let mut tests = Vec::new();
    for case in &config.tests {
        let hooks = Engine::new(rules)
            .map_err(|e| e.to_string())
            .and_then(|engine| kanami_script::hooks_for(&config, &engine, Box::new(|_| {})));
        let hooks = match hooks {
            Ok(hooks) => hooks,
            Err(message) => {
                return Check {
                    problem: Some(Problem {
                        line: 1,
                        column: 1,
                        message,
                    }),
                    summary: Some(summary),
                    tests,
                };
            }
        };
        let (passed, expected, actual) = match case.run(rules, hooks) {
            Ok(()) => (true, String::new(), String::new()),
            Err(m) => (false, m.expected, m.actual),
        };
        tests.push(TestResult {
            name: case.name.clone(),
            line: case.line,
            passed,
            expected,
            actual,
        });
    }
    Check {
        problem: None,
        summary: Some(summary),
        tests,
    }
}

fn problem(text: &str, error: ConfigError) -> Problem {
    match error {
        ConfigError::Invalid {
            line,
            column,
            message,
        } => Problem {
            line,
            column,
            message,
        },
        ConfigError::Toml(e) => {
            let offset = e.span().map_or(0, |s| s.start).min(text.len());
            let before = &text[..offset];
            Problem {
                line: before.matches('\n').count() + 1,
                column: before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1,
                message: e.message().to_owned(),
            }
        }
        ConfigError::Io(_, e) => Problem {
            line: 1,
            column: 1,
            message: e.to_string(),
        },
    }
}

/// Saves the config: refuses with `"conflict"` when the file was changed elsewhere (its
/// modification time is not `expected`). The old content goes into the history before writing.
/// Returns the new modification time.
#[tauri::command]
fn save_config(path: &str, text: &str, expected: u64) -> Result<u64, String> {
    let file = Path::new(path);
    if modified(file) != expected {
        return Err("conflict".to_owned());
    }
    if let Ok(old) = fs::read_to_string(file)
        && old != text
    {
        snapshot(&old)?;
    }
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).map_err(|e| {
            tr!(
                "Failed to create {path}: {error}",
                path = dir.display(),
                error = e
            )
        })?;
    }
    fs::write(file, text)
        .map_err(|e| tr!("Failed to save {path}: {error}", path = path, error = e))?;
    Ok(modified(file))
}

/// The config file structure (for the visual pages) and its modification time.
#[derive(Serialize)]
struct Model {
    model: Value,
    modified: u64,
}

#[tauri::command]
fn config_model(path: &str) -> Result<Model, String> {
    let file = read_config(path)?;
    Ok(Model {
        model: edit::model(&file.text)?,
        modified: file.modified,
    })
}

/// An edit from the visual pages: rewrites the config file by path, keeping comments and
/// formatting. If the edited config has errors it is not written and the error is returned; when
/// the file was changed elsewhere it returns `"conflict"`.
#[tauri::command]
fn edit_config(path: &str, ops: Vec<edit::Op>, expected: u64) -> Result<Model, String> {
    let old = read_config(path)?;
    if old.modified != expected {
        return Err("conflict".to_owned());
    }
    let text = edit::apply(&old.text, &ops)?;
    let base = Path::new(path).parent();
    let config = kanami_config::parse_in(&text, base).map_err(|e| {
        let p = problem(&text, e);
        tr!(
            "{message} (line {line})",
            message = p.message,
            line = p.line
        )
    })?;
    Engine::new(&config.rules).map_err(|e| e.to_string())?;
    let modified = save_config(path, &text, expected)?;
    Ok(Model {
        model: edit::model(&text)?,
        modified,
    })
}

#[derive(Serialize)]
struct RunningApp {
    id: String,
    name: String,
}

/// The keyboards connected now (enumerated by the background service).
#[tauri::command]
fn devices() -> Result<Value, String> {
    let response = kanami_ipc::request("devices")?;
    Ok(response["devices"].clone())
}

/// Regular apps that are running (with windows, in the Dock), for picking in a rule's "only in these apps".
#[tauri::command]
fn running_apps() -> Vec<RunningApp> {
    #[cfg(target_os = "macos")]
    {
        let Ok(out) = Command::new("lsappinfo").arg("list").output() else {
            return Vec::new();
        };
        let text = String::from_utf8_lossy(&out.stdout);
        let mut apps = Vec::new();
        // One block per process: `bundleID="…"`, and `type="Foreground"` means a regular app
        for block in text.split("\n ").filter(|b| b.contains("ASN:")) {
            let field = |name: &str| {
                let start = block.find(name)? + name.len();
                let rest = &block[start..];
                Some(rest[..rest.find('"')?].to_owned())
            };
            if field("type=\"").as_deref() != Some("Foreground") {
                continue;
            }
            if let Some(id) = field("bundleID=\"")
                && !apps.iter().any(|a: &RunningApp| a.id == id)
            {
                apps.push(RunningApp {
                    name: app_name(&id),
                    id,
                });
            }
        }
        apps.sort_by_key(|a| a.name.to_lowercase());
        apps
    }
    #[cfg(not(target_os = "macos"))]
    Vec::new()
}

fn history_dir() -> Result<PathBuf, String> {
    Ok(kanami_ipc::data_dir()?.join("history"))
}

/// Stores a history version, keeping only the latest [`HISTORY`]. The file name is the save time (milliseconds).
fn snapshot(text: &str) -> Result<(), String> {
    let dir = history_dir()?;
    fs::create_dir_all(&dir).map_err(|e| {
        tr!(
            "Failed to create {path}: {error}",
            path = dir.display(),
            error = e
        )
    })?;
    let now = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    fs::write(dir.join(format!("{now}.toml")), text).map_err(|e| e.to_string())?;
    let mut ids = history_ids(&dir);
    while ids.len() > HISTORY {
        let oldest = ids.remove(0);
        let _ = fs::remove_file(dir.join(format!("{oldest}.toml")));
    }
    Ok(())
}

/// The history version IDs (save times), oldest to newest.
fn history_ids(dir: &Path) -> Vec<u64> {
    let mut ids: Vec<u64> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.strip_suffix(".toml")?.parse().ok())
        .collect();
    ids.sort_unstable();
    ids
}

/// The history versions, newest to oldest.
#[tauri::command]
fn history() -> Result<Vec<u64>, String> {
    let mut ids = history_ids(&history_dir()?);
    ids.reverse();
    Ok(ids)
}

#[tauri::command]
fn history_read(id: u64) -> Result<String, String> {
    fs::read_to_string(history_dir()?.join(format!("{id}.toml"))).map_err(|e| e.to_string())
}

/// The display name of an app (such as `com.tencent.xinWeChat` → "WeChat"). On macOS, Spotlight finds
/// the app by Bundle ID and its file name is used; if it is not found, the last segment of the ID
/// is used. On Windows it already is the program file name.
#[tauri::command]
fn app_name(id: &str) -> String {
    static CACHE: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);
    if let Ok(cache) = CACHE.lock()
        && let Some(name) = cache.as_ref().and_then(|c| c.get(id))
    {
        return name.clone();
    }
    let fallback = id.rsplit('.').next().unwrap_or(id).to_owned();
    #[cfg(target_os = "macos")]
    let name = Command::new("mdfind")
        .arg(format!(
            "kMDItemCFBundleIdentifier == '{}'",
            id.replace('\'', "")
        ))
        .output()
        .ok()
        .and_then(|out| {
            let text = String::from_utf8(out.stdout).ok()?;
            let path = text.lines().find(|l| l.ends_with(".app"))?.to_owned();
            // The localized display name ("WeChat"), or the file name when unavailable
            let shown = Command::new("mdls")
                .args(["-raw", "-name", "kMDItemDisplayName", &path])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|n| n.trim().trim_end_matches(".app").to_owned())
                .filter(|n| !n.is_empty() && n != "(null)");
            shown.or_else(|| Path::new(&path).file_stem()?.to_str().map(str::to_owned))
        })
        .unwrap_or(fallback);
    #[cfg(not(target_os = "macos"))]
    let name = fallback;
    if let Ok(mut cache) = CACHE.lock() {
        cache
            .get_or_insert_with(HashMap::new)
            .insert(id.to_owned(), name.clone());
    }
    name
}

#[tauri::command]
fn schema() -> String {
    kanami_config::schema()
}

/// Opens in the default text editor.
#[tauri::command]
fn open_in_editor(path: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let child = Command::new("open").arg("-t").arg(path).spawn();
    #[cfg(windows)]
    let child = Command::new("notepad.exe").arg(path).spawn();
    child
        .map(drop)
        .map_err(|e| tr!("Failed to open the editor: {error}", error = e))
}

/// Opens the matching permission page in System Settings.
#[tauri::command]
fn open_permission(kind: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let pane = match kind {
            "accessibility" => "Privacy_Accessibility",
            "input_monitoring" => "Privacy_ListenEvent",
            other => return Err(tr!("Unknown permission {permission}", permission = other)),
        };
        let url = format!("x-apple.systempreferences:com.apple.preference.security?{pane}");
        Command::new("open")
            .arg(url)
            .spawn()
            .map(drop)
            .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = kind;
        Ok(())
    }
}

/// The `kanami` command-line program: the one beside the settings window, or the one installed as the background service.
fn kanami_cli() -> Result<PathBuf, String> {
    let name = if cfg!(windows) {
        "kanami.exe"
    } else {
        "kanami"
    };
    let beside = std::env::current_exe()
        .ok()
        .map(|exe| exe.with_file_name(name));
    let installed = kanami_ipc::data_dir().ok().map(|dir| dir.join(name));
    beside
        .into_iter()
        .chain(installed)
        .find(|p| p.is_file())
        .ok_or_else(|| tr!("Cannot find the kanami command-line program"))
}

/// Whether it is installed as a background service that starts at login.
#[tauri::command]
fn login_item() -> bool {
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").is_some_and(|home| {
            PathBuf::from(home)
                .join("Library/LaunchAgents/dev.kanami.agent.plist")
                .is_file()
        })
    }
    #[cfg(not(target_os = "macos"))]
    false
}

/// On: installs the background service (which also starts remapping); off: uninstalls it (which also stops remapping).
#[tauri::command]
fn set_login_item(enabled: bool, config: &str) -> Result<(), String> {
    let cli = kanami_cli()?;
    let mut command = Command::new(cli);
    if enabled {
        // Create an empty config when there is none yet: no remaps
        let file = Path::new(config);
        if !file.exists() {
            if let Some(dir) = file.parent() {
                fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            fs::write(file, "").map_err(|e| e.to_string())?;
        }
        command.args(["service", "install", "-c", config]);
    } else {
        command.args(["service", "uninstall"]);
    }
    let output = command.output().map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

fn main() {
    kanami_i18n::set(kanami_i18n::system());
    let args: Vec<String> = std::env::args().collect();
    let page = page_from(&args).unwrap_or_else(|| "overview".to_owned());
    tauri::Builder::default()
        // When already open, opening again (clicking "Open Settings" in the menu bar) only brings the window to the front
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
            if let Some(page) = page_from(&args) {
                let _ = app.emit("kanami-page", page);
            }
        }))
        .manage(InitialPage(page))
        // Closing the window while the key picker is open must also resume remapping
        .on_window_event(|_, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                stop_recording();
            }
        })
        .invoke_handler(tauri::generate_handler![
            platform,
            initial_page,
            system_language,
            set_language,
            status,
            control,
            subscribe,
            config_path,
            read_config,
            config_modified,
            check_config,
            save_config,
            config_model,
            edit_config,
            running_apps,
            devices,
            recording,
            history,
            history_read,
            schema,
            app_name,
            open_in_editor,
            open_permission,
            login_item,
            set_login_item,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start the settings window");
}
