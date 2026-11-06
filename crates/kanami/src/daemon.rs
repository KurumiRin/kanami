//! The logic behind `run` and `monitor`: wires the engine to the platform backend.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};
use std::time::SystemTime;
use std::{fs, thread};

use kanami_config::Config;
use kanami_core::backend::{DeviceInfo, Handler, NO_OUTPUT, Options, TapMode, Waker};
use kanami_core::{Engine, Key, KeyEvent, KeyState, ModSet, Out, Rules};
use kanami_i18n::tr;

use crate::hud::Hud;
use crate::ipc::{self, Bridge, Command as IpcCommand, KeyRecord, Record, Status};
use crate::tray::Tray;
use crate::{build, platform};

pub struct RunOptions {
    /// Print the input and output of every key.
    pub trace: bool,
    /// Run as a background service: log lines get timestamps; a bad config or missing permission
    /// makes it wait instead of exiting.
    pub service: bool,
    /// The config summary shown in the startup message.
    pub summary: String,
    /// Show the menu bar / tray icon.
    pub tray: bool,
}

/// When `config` is `Err` (service mode only), all keys pass through unchanged until the config is fixed and takes effect automatically.
pub fn run(path: &Path, config: Result<Config, String>, options: RunOptions) -> Result<(), String> {
    let _lock = platform::lock_instance()?;
    let log = spawn_printer(false, options.service);
    let say = |line: String| {
        let _ = log.send(line);
    };
    let built = config.and_then(|c| {
        let hud = Hud::new(&c.rules);
        Ok((build::engine(&c, logger(&log))?, c.watched, hud))
    });
    let mut error = None;
    let (engine, watched, hud) = match built {
        Ok(built) => built,
        Err(e) if options.service => {
            say(tr!(
                "kanami: {error}\n  This takes effect once the config is fixed; until then all keys pass through unchanged",
                error = e
            ));
            error = Some(e);
            let empty = Engine::new(&Rules::default()).map_err(|e| e.to_string())?;
            (empty, Vec::new(), Hud::new(&Rules::default()))
        }
        Err(e) => return Err(e),
    };
    let bridge = ipc::start(
        Status {
            config: fs::canonicalize(path)
                .unwrap_or_else(|_| path.to_owned())
                .display()
                .to_string(),
            summary: options.summary.clone(),
            error: error.clone(),
            ..Status::default()
        },
        log.clone(),
    );
    // IPC starts first, so the settings window can read the missing Accessibility permission while we wait for it
    platform::ensure_permission(options.service, &say)?;
    let path_shown = path.display();
    say(match (options.summary.is_empty(), options.service) {
        (true, true) => tr!("kanami: Loaded {path}", path = path_shown),
        (false, true) => tr!(
            "kanami: Loaded {path} ({summary})",
            path = path_shown,
            summary = options.summary
        ),
        (true, false) => tr!(
            "kanami: Loaded {path}; press Ctrl+C to quit",
            path = path_shown
        ),
        (false, false) => tr!(
            "kanami: Loaded {path} ({summary}); press Ctrl+C to quit",
            path = path_shown,
            summary = options.summary
        ),
    });
    let handler = Daemon {
        engine,
        path: path.to_owned(),
        stamps: stamps(path, &watched),
        watched,
        app: None,
        input_source: None,
        commands: spawn_runner(),
        released: Out::new(),
        log,
        trace: options.trace,
        summary: options.summary,
        error,
        bridge,
        seen_devices: Vec::new(),
        quit: false,
        tray: None,
        wants_tray: options.tray,
        hud,
    };
    let options = Options {
        mode: TapMode::Filter,
        poll: true,
        app: options.tray,
    };
    platform::run(options, handler).map_err(|e| e.to_string())
}

pub fn monitor() -> Result<(), String> {
    eprintln!(
        "{}",
        tr!("kanami: Listening for keys; press Ctrl+C to quit")
    );
    let options = Options {
        mode: TapMode::ListenOnly,
        poll: false,
        app: false,
    };
    let monitor = Monitor {
        log: spawn_printer(true, false),
    };
    platform::run(options, monitor).map_err(|e| e.to_string())
}

/// Forwards script logs to the log thread.
fn logger(log: &Sender<String>) -> build::Log {
    let log = log.clone();
    Box::new(move |line| {
        let _ = log.send(line);
    })
}

type Stamp = Option<(SystemTime, u64)>;

struct Daemon {
    engine: Engine,
    path: PathBuf,
    /// Modification times of the config and script files; a change triggers a reload.
    stamps: Vec<Stamp>,
    watched: Vec<PathBuf>,
    app: Option<String>,
    input_source: Option<String>,
    commands: Sender<Arc<str>>,
    /// Releases the old engine sends when reloading.
    released: Out,
    log: Sender<String>,
    /// Print the input and output of every key (`--trace`).
    trace: bool,
    /// Talks to the settings window and `kanami ctl`.
    bridge: Bridge,
    /// Keyboard IDs already reported to IPC (avoids locking on every key).
    seen_devices: Vec<u32>,
    quit: bool,
    /// Config summary and current error for the menu bar status line.
    summary: String,
    error: Option<String>,
    tray: Option<Tray>,
    wants_tray: bool,
    hud: Hud,
}

impl Daemon {
    /// Hands the commands in the engine output to a background thread; the hot path only does one channel send.
    fn dispatch_runs(&self) {
        for command in &self.engine.output().runs {
            let _ = self.commands.send(Arc::clone(command));
        }
    }

    fn log(&self, line: String) {
        let _ = self.log.send(line);
    }

    fn trace(&self, now: u64, input: Option<KeyEvent>, device: Option<&DeviceInfo>) {
        let out = self.engine.output();
        let output = if !out.is_empty() {
            format_out(out)
        } else if input.is_some_and(|e| e.state == KeyState::Repeat) {
            tr!("(repeat, ignored)")
        } else if self.engine.next_deadline().is_some() {
            tr!("(waiting to decide)")
        } else {
            tr!("(swallowed)")
        };
        let input = input.map_or_else(|| tr!("(timer)"), |e| format_events(&[e]));
        let device = device.map_or_else(String::new, |d| format!("  [{}]", d.product));
        self.log(format!("{now:>8} {input:<22} → {output}{device}"));
    }

    /// Re-reads the config and swaps in a new engine; releases from the old engine go into
    /// `released`. A bad config keeps the old one.
    fn reload(&mut self) -> bool {
        let built = kanami_config::load(&self.path)
            .map_err(|e| format!("{}:{e}", self.path.display()))
            .and_then(|config| {
                let engine = build::engine(&config, logger(&self.log))?;
                Ok((engine, config))
            });
        let (mut engine, config) = match built {
            Ok(built) => built,
            Err(e) => {
                self.log(tr!(
                    "kanami: Config has errors, keeping the previous config\n  {error}",
                    error = e
                ));
                self.error = Some(e.clone());
                self.bridge.update(|s| s.error = Some(e));
                self.refresh_tray();
                return false;
            }
        };
        crate::apply_language(&config.rules);
        let summary = crate::summary(&config);
        let (watched, rules) = (config.watched, config.rules);
        engine.set_app(self.app.as_deref());
        engine.set_input_source(self.input_source.as_deref());
        if self.engine.stopped() {
            engine.set_stopped(true);
        }
        let released = self.engine.reset();
        self.released.keys.extend_from_slice(&released.keys);
        self.engine = engine;
        self.hud.set_rules(&rules);
        self.stamps = stamps(&self.path, &watched);
        self.watched = watched;
        self.log(tr!("kanami: Config reloaded"));
        self.error = None;
        self.summary.clone_from(&summary);
        self.bridge.update(|s| {
            s.error = None;
            s.summary = summary;
        });
        self.refresh_tray();
        true
    }

    fn refresh_tray(&self) {
        if let Some(tray) = &self.tray {
            tray.update(self.engine.stopped(), &self.summary, self.error.as_deref());
        }
    }

    /// Opens the config file in the default text editor.
    fn edit_config(&self) {
        #[cfg(target_os = "macos")]
        let child = Command::new("open").arg("-t").arg(&self.path).spawn();
        #[cfg(windows)]
        let child = Command::new("notepad.exe").arg(&self.path).spawn();
        match child {
            Ok(mut child) => {
                thread::spawn(move || child.wait());
            }
            Err(e) => self.log(tr!(
                "kanami: Failed to open the config file: {error}",
                error = e
            )),
        }
    }

    /// Opens the settings window (a separate program in the same directory as kanami).
    fn open_settings(&self, page: &str) {
        let Some(app) = settings_app() else {
            self.log(tr!(
                "kanami: Cannot find the settings window program (kanami-settings); the settings window is still in development"
            ));
            return;
        };
        match Command::new(&app).args(["--page", page]).spawn() {
            Ok(mut child) => {
                thread::spawn(move || child.wait());
            }
            Err(e) => self.log(tr!(
                "kanami: Failed to open the settings window ({app}): {error}",
                app = app.display(),
                error = e
            )),
        }
    }

    /// Pauses or resumes remapping (menu, IPC, emergency stop); the engine output goes into `released`.
    fn set_paused(&mut self, paused: bool) {
        if self.engine.stopped() == paused {
            return;
        }
        let out = self.engine.set_stopped(paused);
        self.released.keys.extend_from_slice(&out.keys);
        self.paused_changed();
    }

    fn paused_changed(&self) {
        let paused = self.engine.stopped();
        self.log(if paused {
            tr!(
                "kanami: All remaps are paused; press Left Control + Space + Esc together to resume"
            )
        } else {
            tr!("kanami: Remapping resumed")
        });
        self.bridge.update(|s| s.paused = paused);
        self.bridge.send(Record::Paused(paused));
        self.refresh_tray();
    }

    /// Hands the input and output of this key to IPC when there are subscribers.
    fn publish(&mut self, now: u64, input: KeyEvent, device: Option<&DeviceInfo>) {
        if let Some(d) = device
            && !self.seen_devices.contains(&d.id)
        {
            self.seen_devices.push(d.id);
            self.bridge.note_device(d.id, &d.product);
        }
        if !self.bridge.listening() {
            return;
        }
        let out = self.engine.output();
        let mut record = KeyRecord {
            time: now,
            input,
            out: [KeyRecord::EMPTY; ipc::MAX_OUT],
            len: 0,
            more: out.keys.len() > ipc::MAX_OUT,
            text: !out.texts.is_empty(),
            run: !out.runs.is_empty(),
            waiting: out.is_empty() && self.engine.next_deadline().is_some(),
            device: device.map(|d| d.id),
        };
        for (slot, key) in record.out.iter_mut().zip(&out.keys) {
            *slot = *key;
            record.len += 1;
        }
        self.bridge.send(Record::Key(record));
    }
}

impl Daemon {
    /// While the settings window is open it also needs to know which keyboard a key came from (the keyboard diagram uses it). Without permission we skip it rather than prompt for it.
    fn bridge_wants_devices(&self) -> bool {
        #[cfg(target_os = "macos")]
        return self.bridge.listening() && kanami_macos::input_monitoring_allowed();
        #[cfg(not(target_os = "macos"))]
        false
    }
}

impl Handler for Daemon {
    fn event(&mut self, event: KeyEvent, device: Option<&DeviceInfo>, now: u64) -> &Out {
        let stopped = self.engine.stopped();
        self.engine.set_device(device);
        self.engine.process(event, now);
        if self.engine.stopped() != stopped {
            self.paused_changed();
        }
        self.dispatch_runs();
        if self.trace {
            self.trace(now, Some(event), device);
        }
        self.publish(now, event, device);
        self.hud.update(self.engine.active_layer(), now);
        match event.state {
            KeyState::Down => self.hud.press(event.key, true),
            KeyState::Up => self.hud.press(event.key, false),
            KeyState::Repeat => {}
        }
        self.engine.output()
    }

    fn tick(&mut self, now: u64) -> &Out {
        self.engine.tick(now);
        self.dispatch_runs();
        if self.trace && !self.engine.output().is_empty() {
            self.trace(now, None, None);
        }
        self.hud.update(self.engine.active_layer(), now);
        self.engine.output()
    }

    fn deadline(&self) -> Option<u64> {
        match (self.engine.next_deadline(), self.hud.deadline()) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    fn mods(&self) -> Option<ModSet> {
        // Mouse events pass through unchanged while paused too
        (!self.engine.stopped()).then(|| self.engine.mods())
    }

    fn app_changed(&mut self, bundle_id: Option<&str>) {
        if self.trace {
            let unknown = tr!("(unknown)");
            self.log(format!(
                "{:>8} {}",
                "",
                tr!("Frontmost app: {app}", app = bundle_id.unwrap_or(&unknown))
            ));
        }
        self.engine.set_app(bundle_id);
        self.app = bundle_id.map(str::to_owned);
        self.bridge.update(|s| s.app.clone_from(&self.app));
        self.bridge.send(Record::App(self.app.clone()));
    }

    fn input_source_changed(&mut self, id: Option<&str>) {
        if self.trace {
            let unknown = tr!("(unknown)");
            self.log(format!(
                "{:>8} {}",
                "",
                tr!("Input source: {source}", source = id.unwrap_or(&unknown))
            ));
        }
        self.engine.set_input_source(id);
        self.input_source = id.map(str::to_owned);
    }

    fn poll(&mut self, _now: u64) -> &Out {
        let current = stamps(&self.path, &self.watched);
        if current == self.stamps {
            return &NO_OUTPUT;
        }
        self.stamps = current;
        self.released.keys.clear();
        self.reload();
        &self.released
    }

    fn started(&mut self, waker: Waker) {
        self.bridge.set_waker(waker);
        if self.wants_tray {
            match Tray::new(self.bridge.remote()) {
                Ok(tray) => self.tray = Some(tray),
                Err(e) => self.log(tr!(
                    "kanami: Failed to create the menu bar icon: {error}",
                    error = e
                )),
            }
            self.refresh_tray();
            #[cfg(target_os = "macos")]
            self.hud.set_view(Box::new(kanami_macos::Hud::new()));
        }
    }

    fn wake(&mut self, now: u64) -> &Out {
        self.released.keys.clear();
        while let Ok(command) = self.bridge.commands.try_recv() {
            match command {
                IpcCommand::Pause => self.set_paused(true),
                IpcCommand::Resume => self.set_paused(false),
                IpcCommand::TogglePause => self.set_paused(!self.engine.stopped()),
                IpcCommand::Reload => {
                    self.reload();
                }
                IpcCommand::OpenSettings => self.open_settings("overview"),
                IpcCommand::OpenEvents => self.open_settings("events"),
                IpcCommand::EditConfig => self.edit_config(),
                IpcCommand::Quit => self.quit = true,
            }
        }
        self.hud.update(self.engine.active_layer(), now);
        &self.released
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn wants_caps_lock(&self) -> bool {
        // While paused, remove the Caps Lock mapping so it behaves as the system intends
        self.engine.uses_key(Key::CapsLock) && !self.engine.stopped()
    }

    fn wants_devices(&self) -> bool {
        self.engine.uses_devices() || self.trace || self.bridge_wants_devices()
    }

    fn tap_disabled(&mut self, reason: &'static str) {
        self.log(tr!(
            "kanami: The event tap was paused by the system because of {reason} and has been restored; keys pressed meanwhile did not go through kanami",
            reason = reason
        ));
    }

    fn foreign(&mut self, event: KeyEvent, source: u32) {
        if self.trace {
            let input = format_events(&[event]);
            let note = tr!(
                "passed through (sent by another program, process {pid})",
                pid = source
            );
            self.log(format!("{:>8} {input:<22} → {note}", ""));
        }
    }

    fn held(&self, key: Key) -> bool {
        self.engine.is_held(key)
    }

    fn lost_release(&mut self, key: Key, now: u64) -> &Out {
        self.log(tr!(
            "kanami: The release of {key} did not go through kanami (secure input, a paused event tap or sleep); it has been supplied",
            key = key.name()
        ));
        self.event(KeyEvent::new(key, KeyState::Up), None, now)
    }

    fn shutdown(&mut self) -> &Out {
        ipc::cleanup();
        self.hud.hide();
        self.engine.reset()
    }
}

/// The settings window program: `kanami-settings` in the same directory as kanami.
fn settings_app() -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "kanami-settings.exe"
    } else {
        "kanami-settings"
    };
    let path = std::env::current_exe().ok()?.with_file_name(name);
    path.is_file().then_some(path)
}

/// File modification time and size, used to detect changes (editors often write a temp file and rename it, so we do not watch file handles).
fn stamp(path: &Path) -> Stamp {
    let meta = fs::metadata(path).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

fn stamps(config: &Path, watched: &[PathBuf]) -> Vec<Stamp> {
    std::iter::once(config)
        .chain(watched.iter().map(PathBuf::as_path))
        .map(stamp)
        .collect()
}

/// `+key` is a press, `-key` a release, `~key` an auto-repeat; modifier state goes in square brackets.
fn format_events(events: &[KeyEvent]) -> String {
    let parts: Vec<String> = events
        .iter()
        .map(|e| {
            let sign = match e.state {
                KeyState::Down => '+',
                KeyState::Up => '-',
                KeyState::Repeat => '~',
            };
            if e.mods.is_empty() {
                format!("{sign}{}", e.key.name())
            } else {
                format!("{sign}{}[{}]", e.key.name(), e.mods)
            }
        })
        .collect();
    parts.join(" ")
}

/// Shows keys, text and commands in the order they are sent.
fn format_out(out: &Out) -> String {
    let mut parts = Vec::new();
    let mut texts = out.texts.iter().peekable();
    for (i, event) in out.keys.iter().enumerate() {
        while let Some((_, text)) = texts.next_if(|(at, _)| *at == i) {
            parts.push(format!("\"{text}\""));
        }
        parts.push(format_events(std::slice::from_ref(event)));
    }
    parts.extend(texts.map(|(_, text)| format!("\"{text}\"")));
    parts.extend(out.runs.iter().map(|c| format!("run `{c}`")));
    parts.join(" ")
}

/// Log output goes through a background thread, so a slow terminal cannot stall the key callback (the system pauses the event tap if the callback times out).
fn spawn_printer(stdout: bool, timestamps: bool) -> Sender<String> {
    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || {
        // Writes fail once the terminal closes: ignore the error and never panic, or the exit cleanup would not run
        for line in rx {
            let line = if timestamps {
                format!("[{}] {line}", platform::local_time())
            } else {
                line
            };
            let _ = if stdout {
                writeln!(io::stdout().lock(), "{line}")
            } else {
                writeln!(io::stderr().lock(), "{line}")
            };
        }
    });
    tx
}

fn spawn_runner() -> Sender<Arc<str>> {
    let (tx, rx) = mpsc::channel::<Arc<str>>();
    thread::spawn(move || {
        for command in rx {
            match Command::new("/bin/sh")
                .args(["-c", &command])
                .stdin(Stdio::null())
                .spawn()
            {
                // A separate thread waits for the child to finish, so no zombie is left behind
                Ok(mut child) => {
                    thread::spawn(move || child.wait());
                }
                Err(e) => {
                    let _ = writeln!(
                        io::stderr(),
                        "{}",
                        tr!(
                            "kanami: Failed to run `{command}`: {error}",
                            command = command,
                            error = e
                        )
                    );
                }
            }
        }
    });
    tx
}

struct Monitor {
    log: Sender<String>,
}

impl Monitor {
    fn print(&self, event: KeyEvent, source: &str) {
        let state = match event.state {
            KeyState::Down => "down",
            KeyState::Up => "up",
            KeyState::Repeat => "repeat",
        };
        let mods = event.mods.to_string();
        let _ = self.log.send(format!(
            "{state:<7}{:<16}{mods:<6}{source}",
            event.key.name()
        ));
    }
}

impl Handler for Monitor {
    fn event(&mut self, event: KeyEvent, device: Option<&DeviceInfo>, _now: u64) -> &Out {
        let device = device.map_or_else(String::new, |d| format!("[{}]", d.product));
        self.print(event, &device);
        &NO_OUTPUT
    }

    fn foreign(&mut self, event: KeyEvent, source: u32) {
        self.print(event, &tr!("[other program, process {pid}]", pid = source));
    }

    fn wants_devices(&self) -> bool {
        true
    }

    fn app_changed(&mut self, bundle_id: Option<&str>) {
        let unknown = tr!("(unknown)");
        let _ = self
            .log
            .send(format!("app    {}", bundle_id.unwrap_or(&unknown)));
    }

    fn input_source_changed(&mut self, id: Option<&str>) {
        let unknown = tr!("(unknown)");
        let _ = self.log.send(tr!(
            "input source {source}",
            source = id.unwrap_or(&unknown)
        ));
    }

    fn tap_disabled(&mut self, reason: &'static str) {
        let _ = self.log.send(tr!(
            "(event tap paused by the system because of {reason}, restored)",
            reason = reason
        ));
    }
}
