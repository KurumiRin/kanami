//! `kanami service`: registers a LaunchAgent that starts at login and restarts after a crash.
//!
//! Installing copies the current executable to `~/Library/Application Support/kanami/`, so
//! recompiling the source does not affect the running service, and the Accessibility permission
//! is always granted to this one path.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use kanami_i18n::tr;

const LABEL: &str = "dev.kanami.agent";

fn home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| tr!("Cannot find the HOME directory"))
}

fn plist_path() -> Result<PathBuf, String> {
    Ok(home()?.join(format!("Library/LaunchAgents/{LABEL}.plist")))
}

pub fn install_dir() -> Result<PathBuf, String> {
    kanami_ipc::data_dir()
}

pub fn log_path() -> Result<PathBuf, String> {
    Ok(home()?.join("Library/Logs/kanami.log"))
}

/// The current user's graphical session domain.
fn domain() -> String {
    unsafe extern "C" {
        fn getuid() -> u32;
    }
    format!("gui/{}", unsafe { getuid() })
}

fn target() -> String {
    format!("{}/{LABEL}", domain())
}

fn launchctl(args: &[&str]) -> Result<std::process::Output, String> {
    Command::new("/bin/launchctl")
        .args(args)
        .output()
        .map_err(|e| tr!("Failed to run launchctl: {error}", error = e))
}

pub fn install(config: &Path) -> Result<ExitCode, String> {
    let config = config.canonicalize().map_err(|e| {
        tr!(
            "Cannot find the config file {path}: {error}",
            path = config.display(),
            error = e
        )
    })?;
    let dir = install_dir()?;
    fs::create_dir_all(&dir).map_err(|e| {
        tr!(
            "Failed to create {path}: {error}",
            path = dir.display(),
            error = e
        )
    })?;
    let exe = dir.join("kanami");
    let current = std::env::current_exe()
        .map_err(|e| tr!("Cannot find the current executable: {error}", error = e))?;

    // Stop the old service before replacing the executable, so a running file is never overwritten
    let plist = plist_path()?;
    let _ = launchctl(&["bootout", &target()]);
    if current != exe {
        let _ = fs::remove_file(&exe);
        fs::copy(&current, &exe).map_err(|e| {
            tr!(
                "Failed to copy the executable to {path}: {error}",
                path = exe.display(),
                error = e
            )
        })?;
    }

    let log = log_path()?;
    if let Some(parent) = plist.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            tr!(
                "Failed to create {path}: {error}",
                path = parent.display(),
                error = e
            )
        })?;
    }
    fs::write(&plist, plist_xml(&exe, &config, &log)).map_err(|e| {
        tr!(
            "Failed to write {path}: {error}",
            path = plist.display(),
            error = e
        )
    })?;

    let output = launchctl(&["bootstrap", &domain(), &plist.to_string_lossy()])?;
    if !output.status.success() {
        return Err(tr!(
            "launchctl bootstrap failed: {error}",
            error = String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    println!("{}", tr!("kanami service installed and started"));
    println!("  {}", tr!("Program: {path}", path = exe.display()));
    println!(
        "  {}",
        tr!(
            "Config: {path} (reloaded automatically when saved)",
            path = config.display()
        )
    );
    println!("  {}", tr!("Log: {path}", path = log.display()));
    println!();
    println!(
        "{}",
        tr!(
            "On first launch macOS asks for Accessibility permission. In “System Settings → Privacy & Security → Accessibility”,\nallow {path}. The service starts working automatically once it is allowed.",
            path = exe.display()
        )
    );
    println!(
        "{}",
        tr!(
            "Note: running `kanami service install` again replaces the executable, which may need to be authorized again."
        )
    );
    Ok(ExitCode::SUCCESS)
}

pub fn uninstall() -> Result<ExitCode, String> {
    let plist = plist_path()?;
    let _ = launchctl(&["bootout", &target()]);
    let existed = plist.exists();
    if existed {
        fs::remove_file(&plist).map_err(|e| {
            tr!(
                "Failed to delete {path}: {error}",
                path = plist.display(),
                error = e
            )
        })?;
    }
    let exe = install_dir()?.join("kanami");
    if exe.exists() {
        let _ = fs::remove_file(&exe);
    }
    if existed {
        println!(
            "{}",
            tr!("kanami service stopped and uninstalled (config and log are kept)")
        );
    } else {
        println!("{}", tr!("The kanami service is not installed"));
    }
    Ok(ExitCode::SUCCESS)
}

pub fn restart() -> Result<ExitCode, String> {
    let output = launchctl(&["kickstart", "-k", &target()])?;
    if !output.status.success() {
        return Err(tr!(
            "The kanami service is not installed. Run `kanami service install` first."
        ));
    }
    println!("{}", tr!("kanami service restarted"));
    Ok(ExitCode::SUCCESS)
}

pub fn status() -> Result<ExitCode, String> {
    let output = launchctl(&["print", &target()])?;
    if !output.status.success() {
        println!("{}", tr!("The kanami service is not installed"));
        return Ok(ExitCode::FAILURE);
    }
    let info = String::from_utf8_lossy(&output.stdout);
    let field = |name: &str| {
        info.lines()
            .map(str::trim)
            .find_map(|l| l.strip_prefix(name)?.strip_prefix(" = "))
            .map(str::to_owned)
    };
    let state = field("state").unwrap_or_else(|| tr!("unknown"));
    let running = state == "running";
    let shown = if running {
        tr!("running")
    } else {
        tr!("not running")
    };
    println!("{}", tr!("kanami service: {state}", state = shown));
    if let Some(pid) = field("pid") {
        println!("  {}", tr!("Process: {pid}", pid = pid));
    }
    if let Some(code) = field("last exit code").filter(|c| c != "(never exited)") {
        println!("  {}", tr!("Last exit code: {code}", code = code));
    }
    let log = log_path()?;
    println!("  {}", tr!("Log: {path}", path = log.display()));
    if let Ok(text) = fs::read_to_string(&log) {
        let lines: Vec<&str> = text.lines().collect();
        let tail = &lines[lines.len().saturating_sub(5)..];
        if !tail.is_empty() {
            println!("\n{}", tr!("Recent log:"));
            for line in tail {
                println!("  {line}");
            }
        }
    }
    Ok(if running {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn plist_xml(exe: &Path, config: &Path, log: &Path) -> String {
    let [exe, config, log] = [exe, config, log].map(|p| escape(&p.to_string_lossy()));
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe}</string>
        <string>run</string>
        <string>--service</string>
        <string>-c</string>
        <string>{config}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <!-- Restart only after an abnormal exit; a normal stop (exit code 0) does not restart -->
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>ThrottleInterval</key>
    <integer>5</integer>
    <!-- Same scheduling priority as the foreground app, to avoid key latency -->
    <key>ProcessType</key>
    <string>Interactive</string>
    <key>StandardOutPath</key>
    <string>{log}</string>
    <key>StandardErrorPath</key>
    <string>{log}</string>
</dict>
</plist>
"#
    )
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_escapes_paths() {
        let xml = plist_xml(
            Path::new("/a/kanami"),
            Path::new("/b/R&D <x>.toml"),
            Path::new("/c/kanami.log"),
        );
        assert!(xml.contains("<string>/b/R&amp;D &lt;x&gt;.toml</string>"));
        assert!(xml.contains("<string>--service</string>"));
    }
}
