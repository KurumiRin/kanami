//! Platform differences: backend entry point, permissions, single-instance lock, local time.

#[cfg(target_os = "macos")]
pub use kanami_macos::run;
#[cfg(windows)]
pub use kanami_windows::run;

#[cfg(target_os = "macos")]
pub use mac::{ensure_permission, local_time, lock_instance};

#[cfg(windows)]
pub use kanami_windows::{local_time, lock_instance};

/// The Windows low-level keyboard hook needs no extra permission.
#[cfg(windows)]
pub fn ensure_permission(_service: bool, _say: &dyn Fn(String)) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "macos")]
mod mac {
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::os::fd::AsRawFd;
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use kanami_core::backend::TapMode;
    use kanami_i18n::tr;

    /// Checks the Accessibility permission. In service mode, without it we wait for authorization instead of exiting (or launchd would restart us over and over).
    pub fn ensure_permission(service: bool, say: &dyn Fn(String)) -> Result<(), String> {
        if kanami_macos::accessibility_trusted(true) {
            return Ok(());
        }
        if !service {
            return Err(kanami_macos::TapError::CreateFailed(TapMode::Filter).to_string());
        }
        let exe =
            std::env::current_exe().map_or_else(|_| "kanami".into(), |p| p.display().to_string());
        say(tr!(
            "kanami: Waiting for Accessibility permission. Allow {path} in “System Settings → Privacy & Security → Accessibility”",
            path = exe
        ));
        while !kanami_macos::accessibility_trusted(false) {
            thread::sleep(Duration::from_secs(2));
        }
        say(tr!("kanami: Accessibility permission granted"));
        Ok(())
    }

    /// Only one kanami may remap at a time, or every key would be handled twice. The returned file must be kept for as long as it runs.
    pub fn lock_instance() -> Result<fs::File, String> {
        unsafe extern "C" {
            fn flock(fd: i32, operation: i32) -> i32;
        }
        const LOCK_EX: i32 = 2;
        const LOCK_NB: i32 = 4;
        let dir = crate::service::install_dir()?;
        fs::create_dir_all(&dir).map_err(|e| {
            tr!(
                "Failed to create {path}: {error}",
                path = dir.display(),
                error = e
            )
        })?;
        let path = dir.join("kanami.lock");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| {
                tr!(
                    "Failed to open {path}: {error}",
                    path = path.display(),
                    error = e
                )
            })?;
        if unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) } != 0 {
            let pid = fs::read_to_string(&path).unwrap_or_default();
            return Err(tr!(
                "Another kanami is already running (process {pid}). If it is the background service, check it with `kanami service status`",
                pid = pid.trim()
            ));
        }
        let _ = file.set_len(0);
        let _ = write!(file, "{}", std::process::id());
        Ok(file)
    }

    /// Local time, such as `2026-11-05 14:40:12`.
    pub fn local_time() -> String {
        #[repr(C)]
        struct Tm {
            sec: i32,
            min: i32,
            hour: i32,
            mday: i32,
            mon: i32,
            year: i32,
            wday: i32,
            yday: i32,
            isdst: i32,
            gmtoff: std::ffi::c_long,
            zone: *const std::ffi::c_char,
        }
        unsafe extern "C" {
            fn localtime_r(time: *const i64, result: *mut Tm) -> *mut Tm;
        }
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64);
        let mut tm = std::mem::MaybeUninit::<Tm>::zeroed();
        if unsafe { localtime_r(&secs, tm.as_mut_ptr()) }.is_null() {
            return secs.to_string();
        }
        let tm = unsafe { tm.assume_init() };
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            tm.year + 1900,
            tm.mon + 1,
            tm.mday,
            tm.hour,
            tm.min,
            tm.sec
        )
    }
}
