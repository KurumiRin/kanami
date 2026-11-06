//! Layer popup: a small keyboard diagram at the bottom of the screen. It is drawn directly with an
//! NSPanel plus CALayer, and takes no clicks and does not steal focus.
//!
//! It can be used only on the main thread, and needs an NSApplication (menu bar mode).

use std::ffi::{CStr, CString, c_void};
use std::mem::transmute;
use std::ptr;

use kanami_core::Key;
use kanami_core::backend::{HudKey, HudStyle};

use crate::ffi::*;

/// Height of a key and width of a 1u key (in points).
const UNIT_W: f64 = 30.0;
const UNIT_H: f64 = 28.0;
const GAP: f64 = 3.0;
const PADDING: f64 = 12.0;
const HEADER: f64 = 24.0;
/// Distance from the bottom edge of the screen's usable area.
const MARGIN: f64 = 28.0;

#[repr(C)]
#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// Calls an Objective-C method: `msg!(return_type; receiver, c"selector", argument => type, ...)`.
macro_rules! msg {
    ($ret:ty; $recv:expr, $sel:expr $(, $arg:expr => $ty:ty)*) => {{
        let f: unsafe extern "C" fn(Id, Sel $(, $ty)*) -> $ret =
            transmute(objc_msgSend as unsafe extern "C" fn());
        f($recv, sel($sel) $(, $arg)*)
    }};
}

unsafe fn sel(name: &CStr) -> Sel {
    unsafe { sel_registerName(name.as_ptr()) }
}

unsafe fn class(name: &CStr) -> Id {
    unsafe { objc_getClass(name.as_ptr()) }
}

unsafe fn nsstring(text: &str) -> Id {
    let text = CString::new(text.replace('\0', "")).unwrap_or_default();
    unsafe {
        msg!(Id; class(c"NSString"), c"stringWithUTF8String:", text.as_ptr() => *const std::ffi::c_char)
    }
}

fn color(r: f64, g: f64, b: f64, a: f64) -> *mut c_void {
    unsafe { CGColorCreateSRGB(r, g, b, a) }
}

/// The screen's usable area (excluding the Dock and the menu bar).
unsafe fn visible_frame(screen: Id) -> Rect {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        msg!(Rect; screen, c"visibleFrame")
    }
    #[cfg(not(target_arch = "aarch64"))]
    unsafe {
        let mut rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        };
        let f: unsafe extern "C" fn(*mut Rect, Id, Sel) =
            transmute(objc_msgSend_stret as unsafe extern "C" fn());
        f(&mut rect, screen, sel(c"visibleFrame"));
        rect
    }
}

struct TextStyle {
    size: f64,
    color: *mut c_void,
    center: bool,
    scale: f64,
}

struct Palette {
    panel: *mut c_void,
    plain: *mut c_void,
    mapped: *mut c_void,
    trigger: *mut c_void,
    pressed: *mut c_void,
    ink: *mut c_void,
    dim: *mut c_void,
    dark: *mut c_void,
    accent: *mut c_void,
}

pub struct Hud {
    panel: Id,
    root: Id,
    palette: Palette,
    /// Each key's layer and its normal fill color; the color changes while the key is pressed.
    keys: Vec<(Key, Id, *mut c_void)>,
}

impl Default for Hud {
    fn default() -> Self {
        Hud {
            panel: ptr::null_mut(),
            root: ptr::null_mut(),
            palette: Palette {
                panel: color(0.141, 0.110, 0.169, 0.94),
                plain: color(0.353, 0.278, 0.400, 1.0),
                mapped: color(0.878, 0.478, 0.624, 1.0),
                trigger: color(1.0, 0.992, 0.988, 1.0),
                pressed: color(0.980, 0.420, 0.660, 1.0),
                ink: color(1.0, 1.0, 1.0, 1.0),
                dim: color(0.804, 0.749, 0.827, 0.75),
                dark: color(0.290, 0.227, 0.314, 1.0),
                accent: color(0.847, 0.400, 0.561, 1.0),
            },
            keys: Vec::new(),
        }
    }
}

impl Hud {
    pub fn new() -> Self {
        Self::default()
    }

    unsafe fn create_panel(&mut self) {
        /// NSWindowStyleMaskBorderless | NSWindowStyleMaskNonactivatingPanel.
        const STYLE: u64 = 1 << 7;
        const BUFFERED: u64 = 2;
        /// NSStatusWindowLevel.
        const LEVEL: i64 = 25;
        /// Shown on all desktops, excluded from window cycling, and visible over full-screen apps.
        const BEHAVIOR: u64 = (1 << 0) | (1 << 4) | (1 << 6) | (1 << 8);
        unsafe {
            let rect = Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
            };
            let alloc = msg!(Id; class(c"NSPanel"), c"alloc");
            let panel = msg!(Id; alloc, c"initWithContentRect:styleMask:backing:defer:",
                rect => Rect, STYLE => u64, BUFFERED => u64, false => bool);
            msg!(Id; panel, c"retain");
            msg!((); panel, c"setReleasedWhenClosed:", false => bool);
            msg!((); panel, c"setLevel:", LEVEL => i64);
            msg!((); panel, c"setOpaque:", false => bool);
            let clear = msg!(Id; class(c"NSColor"), c"clearColor");
            msg!((); panel, c"setBackgroundColor:", clear => Id);
            msg!((); panel, c"setHasShadow:", true => bool);
            msg!((); panel, c"setIgnoresMouseEvents:", true => bool);
            msg!((); panel, c"setHidesOnDeactivate:", false => bool);
            msg!((); panel, c"setCollectionBehavior:", BEHAVIOR => u64);
            let view = msg!(Id; panel, c"contentView");
            msg!((); view, c"setWantsLayer:", true => bool);
            let root = msg!(Id; view, c"layer");
            msg!((); root, c"setCornerRadius:", 14.0 => f64);
            msg!((); root, c"setMasksToBounds:", true => bool);
            msg!((); root, c"setBackgroundColor:", self.palette.panel => *mut c_void);
            self.panel = panel;
            self.root = root;
        }
    }

    unsafe fn text(&self, parent: Id, frame: Rect, text: &str, style: TextStyle) {
        let TextStyle {
            size,
            color,
            center,
            scale,
        } = style;
        unsafe {
            let layer = msg!(Id; class(c"CATextLayer"), c"layer");
            msg!((); layer, c"setFrame:", frame => Rect);
            msg!((); layer, c"setString:", nsstring(text) => Id);
            let font = msg!(Id; class(c"NSFont"), c"boldSystemFontOfSize:", size => f64);
            msg!((); layer, c"setFont:", font => Id);
            msg!((); layer, c"setFontSize:", size => f64);
            msg!((); layer, c"setForegroundColor:", color => *mut c_void);
            msg!((); layer, c"setContentsScale:", scale => f64);
            msg!((); layer, c"setTruncationMode:", nsstring("end") => Id);
            if center {
                msg!((); layer, c"setAlignmentMode:", nsstring("center") => Id);
            }
            msg!((); parent, c"addSublayer:", layer => Id);
        }
    }

    /// Shows the key positions of a layer.
    pub fn show(&mut self, title: &str, keys: &[HudKey]) {
        unsafe {
            let pool = objc_autoreleasePoolPush();
            if self.panel.is_null() {
                self.create_panel();
            }
            let columns = keys.iter().map(|k| k.x + k.w).fold(0.0, f64::max);
            let rows = keys.iter().map(|k| k.y + 1.0).fold(0.0, f64::max);
            let width = PADDING * 2.0 + columns * (UNIT_W + GAP) - GAP;
            let height = PADDING * 2.0 + HEADER + rows * (UNIT_H + GAP) - GAP;

            let screen = msg!(Id; class(c"NSScreen"), c"mainScreen");
            let area = visible_frame(screen);
            let scale = msg!(f64; screen, c"backingScaleFactor");
            let frame = Rect {
                x: area.x + (area.w - width) / 2.0,
                y: area.y + MARGIN,
                w: width,
                h: height,
            };

            let transaction = class(c"CATransaction");
            msg!((); transaction, c"begin");
            msg!((); transaction, c"setDisableActions:", true => bool);
            msg!((); self.panel, c"setFrame:display:", frame => Rect, true => bool);
            msg!((); self.root, c"setSublayers:", ptr::null_mut::<c_void>() => *mut c_void);
            self.keys.clear();

            let title_frame = Rect {
                x: PADDING + 2.0,
                y: height - PADDING - 17.0,
                w: width - PADDING * 2.0,
                h: 17.0,
            };
            self.text(
                self.root,
                title_frame,
                title,
                TextStyle {
                    size: 12.0,
                    color: self.palette.ink,
                    center: false,
                    scale,
                },
            );

            for key in keys {
                let w = key.w * (UNIT_W + GAP) - GAP;
                let x = PADDING + key.x * (UNIT_W + GAP);
                let y = height - PADDING - HEADER - (key.y + 1.0) * (UNIT_H + GAP) + GAP;
                let layer = msg!(Id; class(c"CALayer"), c"layer");
                msg!((); layer, c"setFrame:", Rect { x, y, w, h: UNIT_H } => Rect);
                msg!((); layer, c"setCornerRadius:", 5.0 => f64);
                let (fill, ink) = match key.style {
                    HudStyle::Plain => (self.palette.plain, self.palette.dim),
                    HudStyle::Mapped => (self.palette.mapped, self.palette.ink),
                    HudStyle::Trigger => (self.palette.trigger, self.palette.dark),
                };
                msg!((); layer, c"setBackgroundColor:", fill => *mut c_void);
                if matches!(key.style, HudStyle::Trigger) {
                    msg!((); layer, c"setBorderWidth:", 2.0 => f64);
                    msg!((); layer, c"setBorderColor:", self.palette.accent => *mut c_void);
                }
                let size = if key.label.chars().count() > 5 {
                    8.5
                } else {
                    10.5
                };
                let text = Rect {
                    x: 1.0,
                    y: (UNIT_H - size - 4.0) / 2.0,
                    w: w - 2.0,
                    h: size + 4.0,
                };
                self.text(
                    layer,
                    text,
                    &key.label,
                    TextStyle {
                        size,
                        color: ink,
                        center: true,
                        scale,
                    },
                );
                msg!((); self.root, c"addSublayer:", layer => Id);
                self.keys.push((key.key, layer, fill));
            }
            msg!((); transaction, c"commit");
            msg!((); self.panel, c"orderFrontRegardless");
            objc_autoreleasePoolPop(pool);
        }
    }

    /// Lights up live while a key is pressed.
    pub fn press(&mut self, key: Key, down: bool) {
        let Some(&(_, layer, fill)) = self.keys.iter().find(|(k, ..)| *k == key) else {
            return;
        };
        unsafe {
            let transaction = class(c"CATransaction");
            msg!((); transaction, c"begin");
            msg!((); transaction, c"setDisableActions:", true => bool);
            let color = if down { self.palette.pressed } else { fill };
            msg!((); layer, c"setBackgroundColor:", color => *mut c_void);
            msg!((); transaction, c"commit");
        }
    }

    pub fn hide(&mut self) {
        if self.panel.is_null() {
            return;
        }
        unsafe {
            msg!((); self.panel, c"orderOut:", ptr::null_mut::<c_void>() => *mut c_void);
        }
    }
}
