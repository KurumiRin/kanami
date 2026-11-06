//! The menu bar (macOS) / tray (Windows) icon and menu. Provided by the background service as a
//! native system menu, not through the settings window.
//!
//! It must be created and updated on the backend thread (macOS requires the main thread). When a
//! menu item is clicked, the callback hands the command to [`Remote`](crate::ipc::Remote), and the
//! main thread runs it.
//!
//! Icons are drawn in code with no image files: line icons are described on a 24 × 24 grid, the
//! same icon set as the design mockups. On macOS they are template images, which the system tints
//! to match the menu's light or dark appearance and highlighting; since macOS 27 the system hides
//! menu item images by default, so we explicitly ask for them to be shown. Windows tray menus
//! traditionally have no icons.

use tray_icon::menu::{IconMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use kanami_i18n::tr;

use crate::ipc::{Command, Remote};

pub struct Tray {
    _icon: TrayIcon,
    status: MenuItem,
    pause: IconMenuItem,
    settings: IconMenuItem,
    events: IconMenuItem,
    edit: IconMenuItem,
    reload: IconMenuItem,
    quit: IconMenuItem,
}

impl Tray {
    pub fn new(remote: Remote) -> Result<Tray, String> {
        let status = MenuItem::new("kanami", false, None);
        // The labels are filled in by `update`, which also relabels them when the language changes.
        let item = |glyph: &[Shape]| {
            let item = IconMenuItem::new("", true, None, None);
            set_glyph(&item, glyph);
            item
        };
        let pause = item(PAUSE);
        let settings = item(SETTINGS);
        let events = item(EVENTS);
        let edit = item(EXTERNAL);
        let reload = item(RELOAD);
        let quit = item(POWER);
        let menu = Menu::new();
        menu.append_items(&[
            &status,
            &PredefinedMenuItem::separator(),
            &pause,
            &PredefinedMenuItem::separator(),
            &settings,
            &events,
            &edit,
            &reload,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .map_err(|e| e.to_string())?;
        #[cfg(target_os = "macos")]
        {
            use tray_icon::menu::ContextMenu;
            kanami_macos::show_menu_images(menu.ns_menu());
        }

        let actions = [
            (pause.id().clone(), Command::TogglePause),
            (settings.id().clone(), Command::OpenSettings),
            (events.id().clone(), Command::OpenEvents),
            (edit.id().clone(), Command::EditConfig),
            (reload.id().clone(), Command::Reload),
            (quit.id().clone(), Command::Quit),
        ];
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if let Some((_, command)) = actions.iter().find(|(id, _)| *id == event.id) {
                remote.send(*command);
            }
        }));

        let builder = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("kanami");
        #[cfg(target_os = "macos")]
        let builder = builder.with_icon_templated(tray_icon()?);
        #[cfg(not(target_os = "macos"))]
        let builder = builder.with_icon(tray_icon()?);
        let icon = builder.build().map_err(|e| e.to_string())?;
        let tray = Tray {
            _icon: icon,
            status,
            pause,
            settings,
            events,
            edit,
            reload,
            quit,
        };
        tray.update(false, "", None);
        Ok(tray)
    }

    /// Updates the status line and "Pause Remapping / Resume Remapping", and relabels every menu
    /// item in the current language.
    pub fn update(&self, paused: bool, summary: &str, error: Option<&str>) {
        let line = match (paused, error) {
            (true, _) => tr!("Remapping paused"),
            (false, Some(_)) => tr!("Config has errors, using the previous config"),
            (false, None) if summary.is_empty() => tr!("Remapping"),
            (false, None) => tr!("Remapping: {summary}", summary = summary),
        };
        self.status.set_text(line);
        if paused {
            self.pause.set_text(tr!("Resume Remapping"));
            set_glyph(&self.pause, PLAY);
        } else {
            self.pause.set_text(tr!("Pause Remapping"));
            set_glyph(&self.pause, PAUSE);
        }
        self.settings.set_text(tr!("Open Settings…"));
        self.events.set_text(tr!("Event Viewer…"));
        self.edit.set_text(tr!("Open Config in Editor"));
        self.reload.set_text(tr!("Reload Config"));
        self.quit.set_text(tr!("Stop Remapping and Quit"));
    }
}

#[cfg(target_os = "macos")]
fn set_glyph(item: &IconMenuItem, glyph: &[Shape]) {
    // A 15pt canvas (30 pixels on Retina) covering only the middle 20 grid cells: the line size stays the same, and the spacing to the text is close to SF Symbols
    const SIZE: u32 = 30;
    let icon = tray_icon::menu::Icon::from_rgba(stroke(glyph, SIZE), SIZE, SIZE).ok();
    item.set_icon_templated(icon);
}

#[cfg(not(target_os = "macos"))]
fn set_glyph(_item: &IconMenuItem, _glyph: &[Shape]) {}

/// One stroke on the 24 × 24 grid. (Windows tray menus draw no icons, so it is unused there.)
#[derive(Clone, Copy)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
enum Shape {
    Line((f64, f64), (f64, f64)),
    /// An arc: center, radius, start angle and swept angle (degrees; 0 is right, 90 is down, clockwise).
    Arc((f64, f64), f64, f64, f64),
}

use Shape::{Arc, Line};

const PAUSE: &[Shape] = &[
    Line((9.0, 6.0), (9.0, 18.0)),
    Line((15.0, 6.0), (15.0, 18.0)),
];
const PLAY: &[Shape] = &[
    Line((8.0, 5.5), (8.0, 18.5)),
    Line((8.0, 18.5), (18.0, 12.0)),
    Line((18.0, 12.0), (8.0, 5.5)),
];
const SETTINGS: &[Shape] = &[
    Line((4.0, 7.0), (13.0, 7.0)),
    Line((17.0, 7.0), (20.0, 7.0)),
    Line((4.0, 17.0), (7.0, 17.0)),
    Line((11.0, 17.0), (20.0, 17.0)),
    Arc((15.0, 7.0), 2.0, 0.0, 360.0),
    Arc((9.0, 17.0), 2.0, 0.0, 360.0),
];
const EVENTS: &[Shape] = &[
    Line((3.0, 12.0), (6.5, 12.0)),
    Line((6.5, 12.0), (9.0, 6.0)),
    Line((9.0, 6.0), (13.5, 18.0)),
    Line((13.5, 18.0), (16.0, 12.0)),
    Line((16.0, 12.0), (21.0, 12.0)),
];
const EXTERNAL: &[Shape] = &[
    Line((14.0, 4.0), (20.0, 4.0)),
    Line((20.0, 4.0), (20.0, 10.0)),
    Line((20.0, 4.0), (11.0, 13.0)),
    Line((18.0, 14.0), (18.0, 18.0)),
    Arc((16.0, 18.0), 2.0, 0.0, 90.0),
    Line((16.0, 20.0), (6.0, 20.0)),
    Arc((6.0, 18.0), 2.0, 90.0, 90.0),
    Line((4.0, 18.0), (4.0, 8.0)),
    Arc((6.0, 8.0), 2.0, 180.0, 90.0),
    Line((6.0, 6.0), (10.0, 6.0)),
];
const RELOAD: &[Shape] = &[
    Arc((12.0, 12.0), 7.0, 330.0, 300.0),
    Line((10.6, 2.9), (12.8, 5.0)),
    Line((12.8, 5.0), (10.6, 7.1)),
];
const POWER: &[Shape] = &[
    Line((12.0, 3.5), (12.0, 11.0)),
    Arc((12.0, 13.0), 7.5, 300.0, 300.0),
];

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
/// Draws a line icon as a `size × size` template image (black, with the shape given by
/// transparency), 1.7 cells wide with round caps. The canvas covers grid cells 2–22; the margin
/// around it is not drawn.
fn stroke(shapes: &[Shape], size: u32) -> Vec<u8> {
    const WIDTH: f64 = 1.7;
    const FROM: f64 = 2.0;
    let scale = size as f64 / 20.0;
    raster(size, |x, y| {
        let (x, y) = (x / scale + FROM, y / scale + FROM);
        let near = shapes
            .iter()
            .any(|shape| distance(*shape, x, y) <= WIDTH / 2.0);
        if near { 1.0 } else { 0.0 }
    })
    .into_iter()
    .flat_map(|coverage| [0, 0, 0, (coverage * 255.0).round() as u8])
    .collect()
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn distance(shape: Shape, x: f64, y: f64) -> f64 {
    match shape {
        Line(a, b) => segment(x, y, a, b),
        Arc((cx, cy), r, start, sweep) => {
            let (dx, dy) = (x - cx, y - cy);
            let angle = dy.atan2(dx).to_degrees();
            if (angle - start).rem_euclid(360.0) <= sweep {
                return ((dx * dx + dy * dy).sqrt() - r).abs();
            }
            let end = |deg: f64| {
                let t = deg.to_radians();
                (cx + r * t.cos(), cy + r * t.sin())
            };
            let ((ax, ay), (bx, by)) = (end(start), end(start + sweep));
            ((x - ax).hypot(y - ay)).min((x - bx).hypot(y - by))
        }
    }
}

/// Samples each pixel 4 × 4 times; `inside` returns how much the sample point (in pixel coordinates) is covered.
fn raster(size: u32, inside: impl Fn(f64, f64) -> f64) -> Vec<f64> {
    const N: u32 = 4;
    let mut out = Vec::with_capacity((size * size) as usize);
    for y in 0..size {
        for x in 0..size {
            let mut sum = 0.0;
            for sy in 0..N {
                for sx in 0..N {
                    let px = x as f64 + (sx as f64 + 0.5) / N as f64;
                    let py = y as f64 + (sy as f64 + 0.5) / N as f64;
                    sum += inside(px, py);
                }
            }
            out.push(sum / (N * N) as f64);
        }
    }
    out
}

/// The menu bar / tray icon: a keycap with a K. On macOS it is a monochrome outline (template image); on Windows a cherry-pink keycap with a creamy-white K.
fn tray_icon() -> Result<Icon, String> {
    // The keycap takes up about the middle 3/4 of the canvas (coordinates use the canvas side as 1)
    const HALF: f64 = 0.36;
    const RADIUS: f64 = 0.11;
    const K: [((f64, f64), (f64, f64)); 3] = [
        ((0.40, 0.30), (0.40, 0.70)),
        ((0.62, 0.30), (0.42, 0.50)),
        ((0.45, 0.48), (0.63, 0.70)),
    ];
    let in_k =
        |x: f64, y: f64, width: f64| K.iter().any(|&(a, b)| segment(x, y, a, b) <= width / 2.0);
    let cap = |x: f64, y: f64| rounded_rect(x - 0.5, y - 0.5, HALF, RADIUS);

    #[cfg(target_os = "macos")]
    let (size, rgba) = {
        // Menu bar icons are at most 22pt tall: draw 44 pixels, which makes the keycap look about 16pt
        const SIZE: u32 = 44;
        const WIDTH: f64 = 0.075;
        let s = SIZE as f64;
        let rgba: Vec<u8> = raster(SIZE, |x, y| {
            let (x, y) = (x / s, y / s);
            if cap(x, y).abs() <= WIDTH / 2.0 || in_k(x, y, WIDTH) {
                1.0
            } else {
                0.0
            }
        })
        .into_iter()
        .flat_map(|c| [0, 0, 0, (c * 255.0).round() as u8])
        .collect();
        (SIZE, rgba)
    };

    #[cfg(not(target_os = "macos"))]
    let (size, rgba) = {
        const SIZE: u32 = 32;
        const WIDTH: f64 = 0.09;
        let s = SIZE as f64;
        let body = raster(
            SIZE,
            |x, y| if cap(x / s, y / s) <= 0.0 { 1.0 } else { 0.0 },
        );
        let legend = raster(SIZE, |x, y| {
            let (x, y) = (x / s, y / s);
            if cap(x, y) <= 0.0 && in_k(x, y, WIDTH) {
                1.0
            } else {
                0.0
            }
        });
        // A creamy-white #FFFDFC K on cherry pink #D8668F
        let rgba: Vec<u8> = body
            .iter()
            .zip(&legend)
            .flat_map(|(&b, &l)| {
                let t = l / b.max(1e-9);
                let mix = |a: f64, c: f64| (a + (c - a) * t).round() as u8;
                [
                    mix(216.0, 255.0),
                    mix(102.0, 253.0),
                    mix(143.0, 252.0),
                    (b * 255.0).round() as u8,
                ]
            })
            .collect();
        (SIZE, rgba)
    };

    Icon::from_rgba(rgba, size, size).map_err(|e| e.to_string())
}

/// Signed distance to the edge of a rounded rectangle (negative inside), with the rectangle centered at the origin.
fn rounded_rect(x: f64, y: f64, half: f64, radius: f64) -> f64 {
    let qx = x.abs() - (half - radius);
    let qy = y.abs() - (half - radius);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    outside + qx.max(qy).min(0.0) - radius
}

/// Distance from a point to a segment.
fn segment(px: f64, py: f64, (ax, ay): (f64, f64), (bx, by): (f64, f64)) -> f64 {
    let (dx, dy) = (bx - ax, by - ay);
    let t = (((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    ((px - ax - t * dx).powi(2) + (py - ay - t * dy).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_rect_distance_sign() {
        assert!(rounded_rect(0.0, 0.0, 0.36, 0.11) < 0.0);
        assert!(rounded_rect(0.5, 0.0, 0.36, 0.11) > 0.0);
        assert!(rounded_rect(0.36, 0.0, 0.36, 0.11).abs() < 1e-9);
    }

    #[test]
    fn arcs_respect_their_sweep() {
        // The power icon arc leaves a gap at the top
        let arc = Arc((12.0, 13.0), 7.5, 300.0, 300.0);
        assert!(distance(arc, 12.0, 20.5) < 0.01, "the bottom is on the arc");
        assert!(distance(arc, 12.0, 5.5) > 2.0, "the top is the gap");
    }

    #[test]
    fn menu_glyph_is_template() {
        // 30 pixels cover grid cells 2–22: the vertical line at grid x = 9 is at pixel 10.5
        let size = 30;
        let rgba = stroke(PAUSE, size);
        assert_eq!(rgba.len(), (size * size * 4) as usize);
        let alpha = |x: u32, y: u32| rgba[((y * size + x) * 4 + 3) as usize];
        assert_eq!(alpha(0, 0), 0, "corners are transparent");
        assert_eq!(
            alpha(15, 15),
            0,
            "transparent between the two vertical lines"
        );
        assert!(alpha(10, 15) > 200, "the left vertical line");
        assert!(
            rgba.chunks(4).all(|p| p[..3] == [0, 0, 0]),
            "only transparency is used"
        );
    }
}
