//! Local IPC for the kanami background service: one JSON message per line. This crate holds only the
//! address and the client; the server lives in `kanami`.
//!
//! A request is `{"cmd": "..."}`; the reply is `{"ok": true, ...}` or `{"ok": false, "error": "..."}`.
//! Commands: `status`, `devices`, `pause`, `resume`, `reload`, `quit`, `subscribe`.
//! After the `subscribe` reply, the connection keeps receiving events: `key`, `app`, `paused`, `skipped`.

use std::io::{BufRead, BufReader, Lines, Write};
use std::path::PathBuf;

use serde_json::{Value, json};

/// kanami's data directory: `~/Library/Application Support/kanami` on macOS and `%APPDATA%\kanami`
/// on Windows. It holds the service's program copy, the single-instance lock, the IPC address and the
/// config history.
pub fn data_dir() -> Result<PathBuf, String> {
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| kanami_i18n::tr!("Could not find the APPDATA directory"))?;
    #[cfg(not(windows))]
    let base = std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join("Library/Application Support"))
        .ok_or_else(|| kanami_i18n::tr!("Could not find the HOME directory"))?;
    Ok(base.join("kanami"))
}

/// The IPC address (a Unix socket).
pub fn address() -> Result<PathBuf, String> {
    Ok(data_dir()?.join("kanami.sock"))
}

#[cfg(unix)]
type Stream = std::os::unix::net::UnixStream;

#[cfg(unix)]
fn connect() -> Result<Stream, String> {
    let path = address()?;
    Stream::connect(&path).map_err(|e| {
        kanami_i18n::tr!(
            "Connecting to {path} failed ({error}); make sure kanami is running",
            path = path.display(),
            error = e
        )
    })
}

#[cfg(not(unix))]
type Stream = std::fs::File;

#[cfg(not(unix))]
fn connect() -> Result<Stream, String> {
    Err(kanami_i18n::tr!("IPC is not implemented on Windows yet"))
}

/// Sends a command and returns the reply.
pub fn request(command: &str) -> Result<Value, String> {
    let mut lines = send(command)?;
    let line = lines
        .next()
        .ok_or_else(|| kanami_i18n::tr!("kanami did not reply"))?
        .map_err(|e| e.to_string())?;
    serde_json::from_str(&line).map_err(|e| e.to_string())
}

/// Subscribes to events. The first item is the subscription reply and each later item is an event;
/// the stream ends when kanami exits.
pub fn subscribe() -> Result<impl Iterator<Item = Value>, String> {
    let lines = send("subscribe")?;
    Ok(lines
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str(&line).ok()))
}

fn send(command: &str) -> Result<Lines<BufReader<Stream>>, String> {
    let mut stream = connect()?;
    writeln!(stream, "{}", json!({ "cmd": command })).map_err(|e| e.to_string())?;
    Ok(BufReader::new(stream).lines())
}
