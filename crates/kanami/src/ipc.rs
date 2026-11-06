//! Local IPC between the settings window, `kanami ctl` and the background service: one JSON
//! object per line.
//!
//! Threads:
//! - the listener thread accepts connections, one thread per connection reads requests and
//!   sends replies;
//! - the event thread turns key records from the main thread into JSON and sends them to every
//!   subscriber.
//!
//! The main thread only does two things: with subscribers, it `try_send`s key records into a
//! fixed-size channel (when full they are dropped and counted, never waited for); when woken, it
//! runs the commands in the command channel. The status snapshot sits behind a mutex, so reading
//! it does not disturb the main thread.
//!
//! On macOS this is a Unix socket in the user's directory (only the owner can connect). The
//! Windows named pipe is not implemented yet (it must be limited to the owner, to be done once it
//! can be tested on a real Windows machine), so some code there is unused for now.
#![cfg_attr(windows, allow(dead_code, unused_imports))]

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};

use kanami_core::backend::Waker;
use kanami_core::{Key, KeyEvent, KeyState};
use kanami_i18n::tr;
use serde_json::{Value, json};

/// Capacity of the event channel.
const CAPACITY: usize = 4096;
/// The most output keys one key record carries; beyond that only `more` is set.
pub const MAX_OUT: usize = 8;

/// Commands for the main thread to run (from IPC or the menu bar menu).
#[derive(Clone, Copy, Debug)]
pub enum Command {
    Pause,
    Resume,
    TogglePause,
    Reload,
    OpenSettings,
    OpenEvents,
    EditConfig,
    Quit,
}

/// The input and output of one key. Fixed-size and copyable, so sending it through the channel allocates nothing.
#[derive(Clone, Copy)]
pub struct KeyRecord {
    pub time: u64,
    pub input: KeyEvent,
    pub out: [KeyEvent; MAX_OUT],
    pub len: u8,
    pub more: bool,
    pub text: bool,
    pub run: bool,
    /// Still waiting for a tap / hold or combo decision.
    pub waiting: bool,
    pub device: Option<u32>,
}

impl KeyRecord {
    pub const EMPTY: KeyEvent = KeyEvent::new(Key::A, KeyState::Up);
}

pub enum Record {
    Key(KeyRecord),
    App(Option<String>),
    Paused(bool),
}

/// Status snapshot, updated by the main thread when something changes.
#[derive(Default)]
pub struct Status {
    pub paused: bool,
    pub config: String,
    pub summary: String,
    /// The current config error (while the service keeps the old config or passes keys through).
    pub error: Option<String>,
    /// Keyboards seen: (backend ID, product name).
    pub devices: Vec<(u32, String)>,
    /// The current frontmost app.
    pub app: Option<String>,
}

/// The part shared between threads.
struct Shared {
    commands: Mutex<Sender<Command>>,
    waker: Mutex<Option<Waker>>,
    status: Mutex<Status>,
    subscribers: AtomicUsize,
    dropped: AtomicU64,
    #[cfg(unix)]
    streams: Mutex<Vec<(u64, std::os::unix::net::UnixStream)>>,
}

/// The end held by the main thread.
pub struct Bridge {
    pub commands: Receiver<Command>,
    events: SyncSender<Record>,
    shared: Arc<Shared>,
}

impl Bridge {
    /// Key records are only worth building when there are subscribers.
    pub fn listening(&self) -> bool {
        self.shared.subscribers.load(Ordering::Relaxed) > 0
    }

    /// Never waits: when the channel is full the record is dropped and the drop is counted.
    pub fn send(&self, record: Record) {
        if !self.listening() {
            return;
        }
        if let Err(TrySendError::Full(_)) = self.events.try_send(record) {
            self.shared.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// An end that can send commands to the main thread from any thread (used by menu callbacks).
    pub fn remote(&self) -> Remote {
        Remote(Arc::clone(&self.shared))
    }

    pub fn set_waker(&self, waker: Waker) {
        if let Ok(mut slot) = self.shared.waker.lock() {
            *slot = Some(waker);
        }
    }

    pub fn update(&self, f: impl FnOnce(&mut Status)) {
        if let Ok(mut status) = self.shared.status.lock() {
            f(&mut status);
        }
    }

    /// Has this keyboard been seen? If not, record it (the lock is only taken for writing when a new keyboard appears).
    pub fn note_device(&self, id: u32, product: &str) {
        if let Ok(mut status) = self.shared.status.lock()
            && !status.devices.iter().any(|(i, _)| *i == id)
        {
            status.devices.push((id, product.to_owned()));
        }
    }
}

#[derive(Clone)]
pub struct Remote(Arc<Shared>);

impl Remote {
    pub fn send(&self, command: Command) -> bool {
        dispatch(&self.0, command)
    }
}

/// Hands a command to the main thread and wakes it. Returns false while the service is quitting.
fn dispatch(shared: &Shared, command: Command) -> bool {
    let sent = shared
        .commands
        .lock()
        .is_ok_and(|tx| tx.send(command).is_ok());
    if let Ok(waker) = shared.waker.lock()
        && let Some(waker) = waker.as_ref()
    {
        waker.wake();
    }
    sent
}

#[cfg(unix)]
use kanami_ipc::address;

/// Removes the socket file on exit.
pub fn cleanup() {
    #[cfg(unix)]
    if let Ok(path) = address() {
        let _ = std::fs::remove_file(path);
    }
}

/// Starts the listener and event threads. If listening fails it only logs, and the service keeps running.
pub fn start(status: Status, log: Sender<String>) -> Bridge {
    let (command_tx, commands) = mpsc::channel();
    let (events, event_rx) = mpsc::sync_channel(CAPACITY);
    let shared = Arc::new(Shared {
        commands: Mutex::new(command_tx),
        waker: Mutex::new(None),
        status: Mutex::new(status),
        subscribers: AtomicUsize::new(0),
        dropped: AtomicU64::new(0),
        #[cfg(unix)]
        streams: Mutex::new(Vec::new()),
    });
    #[cfg(unix)]
    unix::spawn(Arc::clone(&shared), event_rx, log);
    #[cfg(not(unix))]
    {
        drop(event_rx);
        let _ = log.send(tr!(
            "kanami: IPC on Windows (settings window communication) is not implemented yet"
        ));
    }
    Bridge {
        commands,
        events,
        shared,
    }
}

/// Handles one request; returns the reply and whether to start subscribing to events.
fn handle(shared: &Shared, line: &str) -> (Value, bool) {
    let request: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => {
            return (
                json!({ "ok": false, "error": tr!("Invalid JSON: {error}", error = e) }),
                false,
            );
        }
    };
    let command = match request["cmd"].as_str() {
        Some("status") => return (json!({ "ok": true, "status": status_json(shared) }), false),
        Some("devices") => return (json!({ "ok": true, "devices": devices(shared) }), false),
        Some("subscribe") => return (json!({ "ok": true }), true),
        Some("pause") => Command::Pause,
        Some("resume") => Command::Resume,
        Some("reload") => Command::Reload,
        Some("quit") => Command::Quit,
        Some(other) => {
            return (
                json!({ "ok": false, "error": tr!("Unknown command {command}", command = other) }),
                false,
            );
        }
        None => {
            return (
                json!({ "ok": false, "error": tr!("Missing cmd field") }),
                false,
            );
        }
    };
    if dispatch(shared, command) {
        (json!({ "ok": true }), false)
    } else {
        (
            json!({ "ok": false, "error": tr!("The service is quitting") }),
            false,
        )
    }
}

/// The keyboards connected right now.
#[cfg(target_os = "macos")]
fn devices(_shared: &Shared) -> Vec<Value> {
    kanami_macos::list_devices()
        .iter()
        .map(|d| json!({ "vendor_id": d.vendor_id, "product_id": d.product_id, "product": d.product }))
        .collect()
}

/// Windows cannot enumerate them, so this lists the keyboards that have sent keys.
#[cfg(not(target_os = "macos"))]
fn devices(shared: &Shared) -> Vec<Value> {
    shared.status.lock().map_or_else(
        |_| Vec::new(),
        |s| {
            s.devices
                .iter()
                .map(|(_, product)| json!({ "product": product }))
                .collect()
        },
    )
}

fn status_json(shared: &Shared) -> Value {
    let Ok(s) = shared.status.lock() else {
        return Value::Null;
    };
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "pid": std::process::id(),
        "paused": s.paused,
        "config": s.config,
        "summary": s.summary,
        "error": s.error,
        "app": s.app,
        "permissions": permissions(),
    })
}

#[cfg(target_os = "macos")]
fn permissions() -> Value {
    json!({
        "accessibility": kanami_macos::accessibility_trusted(false),
        "input_monitoring": kanami_macos::input_monitoring_allowed(),
    })
}

#[cfg(not(target_os = "macos"))]
fn permissions() -> Value {
    json!({})
}

fn event_json(record: &Record, shared: &Shared) -> Value {
    let key = |e: &KeyEvent| {
        let state = match e.state {
            KeyState::Down => "down",
            KeyState::Up => "up",
            KeyState::Repeat => "repeat",
        };
        json!({ "key": e.key.name(), "state": state, "mods": e.mods.to_string() })
    };
    match record {
        Record::Key(r) => {
            let device = r.device.and_then(|id| {
                let status = shared.status.lock().ok()?;
                status
                    .devices
                    .iter()
                    .find(|(i, _)| *i == id)
                    .map(|(_, p)| p.clone())
            });
            let output: Vec<Value> = r.out[..r.len as usize].iter().map(key).collect();
            json!({
                "type": "key",
                "time": r.time,
                "input": key(&r.input),
                "output": output,
                "more": r.more,
                "text": r.text,
                "run": r.run,
                "waiting": r.waiting,
                "device": device,
            })
        }
        Record::App(app) => json!({ "type": "app", "app": app }),
        Record::Paused(paused) => json!({ "type": "paused", "paused": paused }),
    }
}

#[cfg(unix)]
mod unix {
    use std::fs;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::sync::Arc;
    use std::sync::atomic::Ordering;
    use std::sync::mpsc::{Receiver, Sender};
    use std::thread;
    use std::time::Duration;

    use kanami_i18n::tr;

    use super::{Record, Shared, address, event_json, handle};

    pub(super) fn spawn(shared: Arc<Shared>, events: Receiver<Record>, log: Sender<String>) {
        let listener = match bind() {
            Ok(listener) => listener,
            Err(e) => {
                let _ = log.send(tr!(
                    "kanami: IPC listening failed ({error}); the settings window will not be able to connect",
                    error = e
                ));
                return;
            }
        };
        let pump = Arc::clone(&shared);
        thread::spawn(move || pump_events(&pump, &events));
        thread::spawn(move || {
            let mut next_id = 0;
            for stream in listener.incoming().flatten() {
                next_id += 1;
                let (shared, id) = (Arc::clone(&shared), next_id);
                thread::spawn(move || serve(&shared, stream, id));
            }
        });
    }

    fn bind() -> Result<UnixListener, String> {
        let path = address()?;
        // We already hold the single-instance lock, so an old socket must be left over from an abnormal exit
        let _ = fs::remove_file(&path);
        let listener = UnixListener::bind(&path).map_err(|e| e.to_string())?;
        // Only the owner can connect: key records may contain passwords
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
        Ok(listener)
    }

    fn serve(shared: &Shared, stream: UnixStream, id: u64) {
        let Ok(reader) = stream.try_clone() else {
            return;
        };
        let mut writer = &stream;
        let mut subscribed = false;
        for line in BufReader::new(reader).lines() {
            let Ok(line) = line else { break };
            if line.trim().is_empty() {
                continue;
            }
            let (reply, subscribe) = handle(shared, &line);
            if subscribe && !subscribed {
                // Reply before joining the subscriber list, so the reply is the first line this connection receives
                if writeln!(writer, "{reply}").is_err() {
                    break;
                }
                if let Ok(clone) = stream.try_clone()
                    && let Ok(mut streams) = shared.streams.lock()
                {
                    let _ = clone.set_write_timeout(Some(Duration::from_secs(1)));
                    streams.push((id, clone));
                    shared.subscribers.store(streams.len(), Ordering::Relaxed);
                    subscribed = true;
                }
                continue;
            }
            if writeln!(writer, "{reply}").is_err() {
                break;
            }
        }
        if subscribed && let Ok(mut streams) = shared.streams.lock() {
            streams.retain(|(i, _)| *i != id);
            shared.subscribers.store(streams.len(), Ordering::Relaxed);
        }
    }

    fn pump_events(shared: &Shared, events: &Receiver<Record>) {
        for record in events {
            let dropped = shared.dropped.swap(0, Ordering::Relaxed);
            if dropped > 0 {
                broadcast(
                    shared,
                    &format!("{{\"type\":\"skipped\",\"count\":{dropped}}}"),
                );
            }
            broadcast(shared, &event_json(&record, shared).to_string());
        }
    }

    /// Subscribers that cannot be written to (window closed, stuck for over a second) are removed.
    fn broadcast(shared: &Shared, line: &str) {
        let Ok(mut streams) = shared.streams.lock() else {
            return;
        };
        streams.retain_mut(|(_, stream)| writeln!(stream, "{line}").is_ok());
        shared.subscribers.store(streams.len(), Ordering::Relaxed);
    }
}

/// `kanami ctl`: sends one command and prints the reply; with `subscribe` it keeps printing events.
pub fn request(command: &str) -> Result<(), String> {
    if command == "subscribe" {
        for event in kanami_ipc::subscribe()? {
            println!("{event}");
        }
    } else {
        println!("{}", kanami_ipc::request(command)?);
    }
    Ok(())
}
