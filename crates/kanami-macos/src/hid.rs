//! HID-level mapping and lock state for Caps Lock.
//!
//! Caps Lock toggles at the HID level, where a `CGEventTap` cannot stop it. To use Caps Lock as a
//! trigger key, `hidutil` first makes the system send it as F18 (F18 has no lock function), and the
//! backend then recognizes F18 as Caps Lock again. The mapping is not persistent: it is restored on
//! exit. If kanami crashes or is killed (`kill -9`) before it can clean up, a guard process undoes
//! it: installing the mapping starts `kanami __caps-guard`, which blocks reading a pipe in its own
//! session; as soon as kanami is gone the pipe closes and the guard undoes the mapping. `kanami
//! reset` is the last resort.
//!
//! Changes in “System Settings → Keyboard → Modifier Keys” are stored per keyboard and take effect
//! before this mapping. `hidutil` cannot see them, so they are read separately from the global
//! preferences.

use std::ffi::{c_char, c_void};
use std::io::{self, Read};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;

/// The guard process's hidden subcommand; the command-line entry point calls [`guard_main`] when it receives it.
pub const GUARD_COMMAND: &str = "__caps-guard";

const HIDUTIL: &str = "/usr/bin/hidutil";
const CAPS_LOCK: u64 = 0x7_0000_0039;
const F18: u64 = 0x7_0000_006D;
const KANAMI: (u64, u64) = (CAPS_LOCK, F18);

#[derive(Debug)]
pub enum CapsError {
    Io(io::Error),
    /// Another program has already set a key mapping through hidutil; it is left alone to avoid overwriting it.
    Conflict,
}

impl std::fmt::Display for CapsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CapsError::Io(e) => f.write_str(&kanami_i18n::tr!(
                "Calling hidutil failed: {error}",
                error = e
            )),
            CapsError::Conflict => f.write_str(&kanami_i18n::tr!(
                "Another program has already set a key mapping through hidutil; to avoid overwriting it, Caps Lock rules will not take effect"
            )),
        }
    }
}

/// An installed mapping, undone by [`restore`](Self::restore).
#[derive(Debug)]
pub struct CapsMapping {
    guard: Option<Child>,
}

impl CapsMapping {
    pub fn install() -> Result<Self, CapsError> {
        // A mapping left behind by an earlier abnormal exit counts as our own
        match current().map_err(CapsError::Io)? {
            Mapping::Empty | Mapping::Kanami => {}
            Mapping::Other => return Err(CapsError::Conflict),
        }
        set(Some(KANAMI)).map_err(CapsError::Io)?;
        // The guard failing to start does not affect use; it only loses crash protection
        let guard = std::env::current_exe().ok().and_then(|exe| {
            Command::new(exe)
                .arg(GUARD_COMMAND)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .ok()
        });
        Ok(CapsMapping { guard })
    }

    pub fn restore(mut self) -> io::Result<()> {
        let result = set(None);
        if let Some(mut guard) = self.guard.take() {
            // Closing stdin tells the guard to exit; its stdout closing means it has exited, so it
            // cannot later undo a mapping from the next install
            drop(guard.stdin.take());
            if let Some(mut stdout) = guard.stdout.take() {
                let _ = io::copy(&mut stdout, &mut io::sink());
            }
            let _ = guard.wait();
        }
        result
    }
}

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    static mach_task_self_: u32;
    fn IOServiceMatching(name: *const c_char) -> *mut c_void;
    fn IOServiceGetMatchingService(main_port: u32, matching: *mut c_void) -> u32;
    fn IOServiceOpen(service: u32, task: u32, kind: u32, connect: *mut u32) -> i32;
    fn IOObjectRelease(object: u32) -> i32;
    fn IOHIDGetModifierLockState(connect: u32, selector: i32, state: *mut bool) -> i32;
    fn IOHIDSetModifierLockState(connect: u32, selector: i32, state: bool) -> i32;
}

const IOHID_PARAM_CONNECT: u32 = 1;
const IOHID_CAPS_LOCK_STATE: i32 = 1;

fn iokit_error(code: i32) -> io::Error {
    io::Error::other(format!("IOKit {code:#x}"))
}

/// Toggles Caps Lock and returns the new state. A synthesized key event only looks like Caps Lock
/// to apps and leaves the lock state alone, so this sets the state in the HID system directly.
pub fn toggle_caps_lock() -> io::Result<bool> {
    static CONNECT: OnceLock<Result<u32, i32>> = OnceLock::new();
    let connect = (*CONNECT.get_or_init(|| unsafe {
        let matching = IOServiceMatching(c"IOHIDSystem".as_ptr());
        let service = IOServiceGetMatchingService(0, matching);
        let mut connect = 0;
        let result = IOServiceOpen(service, mach_task_self_, IOHID_PARAM_CONNECT, &mut connect);
        IOObjectRelease(service);
        if result == 0 {
            Ok(connect)
        } else {
            Err(result)
        }
    }))
    .map_err(iokit_error)?;
    let mut on = false;
    unsafe {
        let result = IOHIDGetModifierLockState(connect, IOHID_CAPS_LOCK_STATE, &mut on);
        if result != 0 {
            return Err(iokit_error(result));
        }
        let result = IOHIDSetModifierLockState(connect, IOHID_CAPS_LOCK_STATE, !on);
        if result != 0 {
            return Err(iokit_error(result));
        }
    }
    Ok(!on)
}

/// Guard process entry point: detaches from kanami's process tree and undoes the mapping once the pipe closes.
pub fn guard_main() -> ! {
    unsafe extern "C" {
        fn setsid() -> i32;
        fn fork() -> i32;
    }
    // Double fork: the intermediate process exits immediately and launchd adopts the guard, so it
    // is not killed along with the whole process tree (for example when closing some terminals' tabs)
    unsafe {
        setsid();
        match fork() {
            0 => {}
            pid if pid > 0 => std::process::exit(0),
            _ => {} // fork failed: keep guarding from the original process
        }
    }
    let mut buf = [0u8; 64];
    // kanami never writes, so read blocks until the pipe closes (returns 0 or an error)
    while matches!(io::stdin().read(&mut buf), Ok(n) if n > 0) {}
    let _ = reset();
    std::process::exit(0)
}

/// `kanami reset`: undoes the mapping kanami left behind. Returns whether anything was actually undone.
pub fn reset() -> io::Result<bool> {
    match current()? {
        Mapping::Kanami => set(None).map(|()| true),
        Mapping::Empty | Mapping::Other => Ok(false),
    }
}

/// The note shown to the user about Caps Lock changes in System Settings “Modifier Keys”; `None` when there are none.
pub fn system_caps_notice() -> Option<String> {
    let output = Command::new("/usr/bin/defaults")
        .args(["-currentHost", "read", "-g"])
        .output()
        .ok()?;
    let remaps: Vec<(u64, u64)> = pairs(&String::from_utf8_lossy(&output.stdout))
        .into_iter()
        .filter(|&(src, dst)| src != dst && (src == CAPS_LOCK || dst == CAPS_LOCK))
        .collect();
    if remaps.is_empty() {
        return None;
    }
    let mut list: Vec<String> = remaps
        .iter()
        .map(|&(src, dst)| format!("{} → {}", usage_name(src), usage_name(dst)))
        .collect();
    list.dedup();
    let caps_from: Vec<String> = remaps
        .iter()
        .filter(|&&(_, dst)| dst == CAPS_LOCK)
        .map(|&(src, _)| usage_name(src))
        .collect();
    let remaps = list.join(", ");
    Some(if caps_from.is_empty() {
        kanami_i18n::tr!(
            "Note: modifier keys were changed in System Settings ({remaps}), so kanami's caps_lock rules will not apply to any key. To make them apply to Caps Lock itself, restore the default in “System Settings → Keyboard → Keyboard Shortcuts… → Modifier Keys”",
            remaps = remaps
        )
    } else {
        kanami_i18n::tr!(
            "Note: modifier keys were changed in System Settings ({remaps}), so kanami's caps_lock rules will actually apply to {keys}. To make them apply to Caps Lock itself, restore the default in “System Settings → Keyboard → Keyboard Shortcuts… → Modifier Keys”",
            remaps = remaps,
            keys = caps_from.join(", ")
        )
    })
}

fn usage_name(usage: u64) -> String {
    let name = match usage {
        CAPS_LOCK => "caps_lock",
        0x7_0000_00E0 => "left_control",
        0x7_0000_00E1 => "left_shift",
        0x7_0000_00E2 => "left_option",
        0x7_0000_00E3 => "left_command",
        0x7_0000_00E4 => "right_control",
        0x7_0000_00E5 => "right_shift",
        0x7_0000_00E6 => "right_option",
        0x7_0000_00E7 => "right_command",
        0x7_0000_0029 => "escape",
        0xFF_0000_0003 => "fn",
        _ => return kanami_i18n::tr!("other key"),
    };
    name.to_owned()
}

#[derive(Debug, PartialEq, Eq)]
enum Mapping {
    Empty,
    Kanami,
    Other,
}

fn current() -> io::Result<Mapping> {
    let output = Command::new(HIDUTIL)
        .args(["property", "--get", "UserKeyMapping"])
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    Ok(classify(&String::from_utf8_lossy(&output.stdout)))
}

/// Extracts all (source key, target key) pairs from `hidutil` / `defaults` output.
/// Both print one dictionary per mapping with fields in alphabetical order, Dst before Src, so
/// pairing them in order of appearance is enough.
fn pairs(output: &str) -> Vec<(u64, u64)> {
    let numbers = |field: &str| -> Vec<u64> {
        output
            .match_indices(field)
            .filter_map(|(i, _)| {
                let rest = output[i + field.len()..].trim_start_matches([' ', '=']);
                let end = rest
                    .find(|c: char| !c.is_ascii_digit())
                    .unwrap_or(rest.len());
                rest[..end].parse().ok()
            })
            .collect()
    };
    let src = numbers("HIDKeyboardModifierMappingSrc");
    let dst = numbers("HIDKeyboardModifierMappingDst");
    src.into_iter().zip(dst).collect()
}

/// Parses `hidutil property --get` output: one line or block per HID service, `(null)` when unset.
fn classify(output: &str) -> Mapping {
    let pairs = pairs(output);
    if pairs.is_empty() {
        return if output.contains('{') {
            Mapping::Other
        } else {
            Mapping::Empty
        };
    }
    if pairs.iter().all(|&p| p == KANAMI) {
        Mapping::Kanami
    } else {
        Mapping::Other
    }
}

fn set(mapping: Option<(u64, u64)>) -> io::Result<()> {
    let items = match mapping {
        Some((src, dst)) => format!(
            "{{\"HIDKeyboardModifierMappingSrc\":{src},\"HIDKeyboardModifierMappingDst\":{dst}}}"
        ),
        None => String::new(),
    };
    let status = Command::new(HIDUTIL)
        .args([
            "property",
            "--set",
            &format!("{{\"UserKeyMapping\":[{items}]}}"),
        ])
        .stdout(std::process::Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(kanami_i18n::tr!(
            "hidutil exited with {status}",
            status = status
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Changes the real Caps Lock state for a moment, so it only runs on request:
    /// `cargo test -p kanami-macos -- --ignored`
    #[test]
    #[ignore]
    fn toggles_caps_lock_and_back() {
        let first = toggle_caps_lock().unwrap();
        let second = toggle_caps_lock().unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn classifies_hidutil_output() {
        let empty =
            "RegistryID  Key                   Value\n10000093c   UserKeyMapping   (null)\n";
        assert_eq!(classify(empty), Mapping::Empty);

        let ours = "RegistryID  Key                   Value\n10000093c   UserKeyMapping   (\n        {\n        HIDKeyboardModifierMappingDst = 30064771181;\n        HIDKeyboardModifierMappingSrc = 30064771129;\n    }\n)\n";
        assert_eq!(classify(ours), Mapping::Kanami);

        let other = ours.replace("30064771181", "30064771113");
        assert_eq!(classify(&other), Mapping::Other);
        assert_eq!(classify("(\n    {\n    }\n)"), Mapping::Other);
    }

    #[test]
    fn pairs_system_modifier_mapping() {
        let defaults = r#"
    "com.apple.keyboard.modifiermapping.1452-544-0" =     (
                {
            HIDKeyboardModifierMappingDst = 30064771303;
            HIDKeyboardModifierMappingSrc = 30064771303;
        },
                {
            HIDKeyboardModifierMappingDst = 30064771129;
            HIDKeyboardModifierMappingSrc = 30064771296;
        },
                {
            HIDKeyboardModifierMappingDst = 30064771300;
            HIDKeyboardModifierMappingSrc = 30064771129;
        }
    );"#;
        assert_eq!(
            pairs(defaults),
            [
                (0x7_0000_00E7, 0x7_0000_00E7),
                (0x7_0000_00E0, CAPS_LOCK),
                (CAPS_LOCK, 0x7_0000_00E4)
            ]
        );
    }
}
