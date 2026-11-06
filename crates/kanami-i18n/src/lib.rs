//! Localized user-facing messages.
//!
//! English source strings double as catalog keys, so the code stays readable and a missing
//! translation falls back to English. Add a language by adding a catalog module and a [`Lang`]
//! variant; `tests/catalog.rs` checks that every `tr!` message has a translation.
//!
//! The language defaults to English until a binary calls [`set`], which keeps tests independent of
//! the machine they run on.

use std::fmt::{Display, Write};
use std::sync::atomic::{AtomicU8, Ordering};

pub mod zh_cn;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Lang {
    #[default]
    En,
    ZhCn,
}

impl Lang {
    /// Parses a BCP 47 tag such as `en-US`, `zh-Hans-CN` or `zh_CN.UTF-8`. Other languages give `None`.
    pub fn parse(tag: &str) -> Option<Lang> {
        let primary = tag.split(['-', '_', '.']).next()?;
        match primary.to_ascii_lowercase().as_str() {
            "en" => Some(Lang::En),
            "zh" => Some(Lang::ZhCn),
            _ => None,
        }
    }

    /// The tag used in config files and by the settings window.
    pub fn tag(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::ZhCn => "zh-CN",
        }
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(Lang::En as u8);

pub fn set(lang: Lang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

pub fn current() -> Lang {
    if CURRENT.load(Ordering::Relaxed) == Lang::ZhCn as u8 {
        Lang::ZhCn
    } else {
        Lang::En
    }
}

/// The language to use without an explicit choice: `KANAMI_LANG`, then the first system language
/// kanami has a translation for, then English.
pub fn system() -> Lang {
    std::env::var("KANAMI_LANG")
        .ok()
        .and_then(|tag| Lang::parse(&tag))
        .or_else(|| first_supported(platform::preferred_languages()))
        .unwrap_or_default()
}

fn first_supported(tags: Vec<String>) -> Option<Lang> {
    tags.iter().find_map(|tag| Lang::parse(tag))
}

/// The translation of `key` in the current language; `key` itself when there is none.
pub fn text(key: &'static str) -> &'static str {
    match current() {
        Lang::En => key,
        Lang::ZhCn => zh_cn::ENTRIES
            .iter()
            .find(|(en, _)| *en == key)
            .map_or(key, |(_, zh)| zh),
    }
}

/// Translates `key` and fills in its `{name}` placeholders. Used through [`tr!`].
pub fn render(key: &'static str, args: &[(&str, &dyn Display)]) -> String {
    let mut rest = text(key);
    let mut out = String::with_capacity(rest.len());
    while let Some(start) = rest.find('{') {
        let (before, tail) = rest.split_at(start);
        out.push_str(before);
        let arg = tail[1..].find('}').and_then(|end| {
            let name = &tail[1..=end];
            let (_, value) = args.iter().find(|(n, _)| *n == name)?;
            Some((*value, end + 2))
        });
        match arg {
            Some((value, used)) => {
                let _ = write!(out, "{value}");
                rest = &tail[used..];
            }
            // A literal brace, such as in a TOML example.
            None => {
                out.push('{');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Formats a translated message: `tr!("Failed to read {path}: {error}", path = p.display(), error = e)`.
///
/// The message must be a string literal so `tests/catalog.rs` can find it.
#[macro_export]
macro_rules! tr {
    ($key:literal $(, $name:ident = $value:expr)* $(,)?) => {
        $crate::render($key, &[$((stringify!($name), &$value as &dyn ::std::fmt::Display)),*])
    };
}

#[cfg(target_os = "macos")]
mod platform {
    use std::ffi::{CStr, c_char, c_void};

    type CFIndex = isize;
    const UTF8: u32 = 0x0800_0100;

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFLocaleCopyPreferredLanguages() -> *const c_void;
        fn CFArrayGetCount(array: *const c_void) -> CFIndex;
        fn CFArrayGetValueAtIndex(array: *const c_void, index: CFIndex) -> *const c_void;
        fn CFStringGetCString(
            string: *const c_void,
            buffer: *mut c_char,
            size: CFIndex,
            encoding: u32,
        ) -> u8;
        fn CFRelease(object: *const c_void);
    }

    pub fn preferred_languages() -> Vec<String> {
        let mut tags = Vec::new();
        // SAFETY: the array follows the Create rule and is released below; its items are borrowed
        // CFStrings read while the array is alive.
        unsafe {
            let array = CFLocaleCopyPreferredLanguages();
            if array.is_null() {
                return tags;
            }
            for i in 0..CFArrayGetCount(array) {
                let mut buffer = [0 as c_char; 64];
                let item = CFArrayGetValueAtIndex(array, i);
                if CFStringGetCString(item, buffer.as_mut_ptr(), 64, UTF8) != 0 {
                    tags.push(
                        CStr::from_ptr(buffer.as_ptr())
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
            }
            CFRelease(array);
        }
        tags
    }
}

#[cfg(windows)]
mod platform {
    const MUI_LANGUAGE_NAME: u32 = 0x8;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetUserPreferredUILanguages(
            flags: u32,
            count: *mut u32,
            buffer: *mut u16,
            size: *mut u32,
        ) -> i32;
    }

    pub fn preferred_languages() -> Vec<String> {
        let (mut count, mut size) = (0, 0);
        // SAFETY: the first call only reports the size; the second writes at most `size` units.
        let names = unsafe {
            if GetUserPreferredUILanguages(
                MUI_LANGUAGE_NAME,
                &mut count,
                std::ptr::null_mut(),
                &mut size,
            ) == 0
            {
                return Vec::new();
            }
            let mut buffer = vec![0u16; size as usize];
            if GetUserPreferredUILanguages(
                MUI_LANGUAGE_NAME,
                &mut count,
                buffer.as_mut_ptr(),
                &mut size,
            ) == 0
            {
                return Vec::new();
            }
            buffer
        };
        names
            .split(|&unit| unit == 0)
            .filter(|name| !name.is_empty())
            .map(String::from_utf16_lossy)
            .collect()
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
mod platform {
    pub fn preferred_languages() -> Vec<String> {
        ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .filter_map(|name| std::env::var(name).ok())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_language_tags() {
        assert_eq!(Lang::parse("en-US"), Some(Lang::En));
        assert_eq!(Lang::parse("zh-Hans-CN"), Some(Lang::ZhCn));
        assert_eq!(Lang::parse("zh_CN.UTF-8"), Some(Lang::ZhCn));
        assert_eq!(Lang::parse("ja-JP"), None);
    }

    #[test]
    fn picks_the_first_language_with_a_translation() {
        let tags = |list: &[&str]| list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert_eq!(
            first_supported(tags(&["ja", "zh-Hans", "en"])),
            Some(Lang::ZhCn)
        );
        assert_eq!(first_supported(tags(&["en-GB", "zh-Hans"])), Some(Lang::En));
        assert_eq!(first_supported(tags(&["ja"])), None);
    }

    #[test]
    fn fills_placeholders_and_keeps_literal_braces() {
        assert_eq!(render("{a} and {b}", &[("a", &1), ("b", &"x")]), "1 and x");
        assert_eq!(render("{ h = {a} }", &[("a", &1)]), "{ h = 1 }");
        assert_eq!(render("{unknown}", &[]), "{unknown}");
    }
}
