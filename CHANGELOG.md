# Changelog

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and version numbers follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- English and Simplified Chinese UI. The language comes from `settings.language` (`"en"` or `"zh-CN"`), then the `KANAMI_LANG` environment variable, then the first supported system language, then English. The tray relabels when the configuration reloads. The new `kanami-i18n` crate holds the message catalog.
- `caps_lock` can be an output, so Caps Lock and Control can be swapped in `[remap]`. On macOS it toggles the lock state through the HID system.
- Rule engine (M1):
  - Key expressions: `cmd+k`, `hyper+t`, `meh`, left/right-specific modifiers, `none`, key sequences.
  - `[remap]` supports modifiers and modifier combinations (such as `caps_lock = "hyper"`).
  - `[[rule]]`: exact matching of combos, tap/hold (three strategies: `timeout`, `permissive`, `eager`), `run` for background commands, and `when.app` / `when.app_not` for per-app rules.
  - `[layer.*]`: layers activated by holding or toggling; unmapped keys fall through to the next layer.
  - `[[combo]]`: several keys pressed together.
  - `[[test]]` and `kanami test`: write rule tests in the configuration and replay them offline.
- Config errors carry line and column numbers, and a misspelled key name gets a spelling suggestion.
- `kanami schema`: prints a JSON Schema for editor completion and validation.
- `kanami run` reloads the configuration automatically after a save, and keeps using the old configuration when the new one has errors.
- macOS backend: synthesized key and modifier events, timeout timers, frontmost app tracking, HID-level mapping for Caps Lock, and clean-up on SIGINT / SIGTERM / SIGHUP.
- `kanami reset`: removes a Caps Lock mapping left behind by an abnormal exit.
- Guard process for the Caps Lock mapping: removes the mapping automatically when kanami crashes or is force-killed.
- `kanami service install|uninstall|status|restart`: runs in the background as a LaunchAgent, starts at login and restarts after a crash. In service mode, kanami waits instead of exiting when the configuration has errors or permission has not been granted yet, and the log has timestamps.
- Only one kanami can run at a time.
- macOS: the modifier flags of mouse click, drag and scroll events are corrected to the engine state, so "a remapped modifier + click" (such as Ctrl+click when Caps Lock acts as Ctrl) works.
- `kanami monitor` also prints the bundle ID of the frontmost app.
- `kanami run --trace`: prints the input and output of every key, for troubleshooting.
- At startup, kanami checks the System Settings modifier-key changes that involve Caps Lock and reports which key the Caps Lock rule actually acts on.
- M2 script plugins:
  - `[script]` loads Luau scripts, which rules and combos invoke with `call`. Scripts run in a sandbox with a 20 ms time budget per callback.
  - Script API: `on_key`, `tap`, `type` (types arbitrary text), `run`, `after`/`cancel` timers, layers, variables, the frontmost app and the input source.
  - Bundled plugins: Leader key, tap dance and text expansion (`plugins/`).
  - Conditions `when.var` (script variables) and `when.input_source` / `when.input_source_not` (input source).
  - `[[test]]` supports configurations with scripts, `"text"` output and the `input_source` field.
  - Build feature `script` (on by default); without it the binary is about 1.3 MB.
- M3 per-keyboard rules: `[device.<name>]` (vendor_id / product_id / product name), `when.device` / `when.device_not`, the `device` field of `[[test]]`, and the `kanami devices` command. On macOS, keyboards are tracked read-only through IOHIDManager (needs the Input Monitoring permission and is turned on only when used), and `monitor` and `--trace` show which keyboard a key came from.
- M4 Windows backend (not yet verified on real hardware): low-level keyboard hook plus `SendInput`, supporting all rules, scripts, text output, and per-program and per-keyboard-layout rules. The `run`, `monitor`, `check` and `test` commands work on both platforms. CI gains Windows.
- M0: key interception based on `CGEventTap`; one-to-one remaps and disabled keys; the `run`, `monitor`, `check` and `keys` commands.
- `settings.typing_streak` (default 150 ms): during continuous typing, tap/hold keys (including layer keys) are sent as soon as they go down and combos do not wait, so typing is neither delayed nor misfired. The default comes from common values for QMK's Flow Tap and ZMK's `require-prior-idle-ms`.
- Holding a layer key longer than the decision limit and releasing it without pressing any other key still sends a tap instead of being swallowed.
- When there is no configuration file, kanami creates an empty one. All keys pass through unchanged, and no remaps or rules are preset.
- G2 visual editing in the settings window:
  - Five pages: remaps, rules, layers, combos and keyboards. Remaps and rules are edited per keyboard: choose a keyboard first, connected keyboards appear directly as tabs, and the first rule added is written to the configuration automatically.
  - Rules read like a sentence and open for editing on click. Rule groups, switches, search and drag-to-reorder are supported, and conditions can be picked from running apps.
  - The layers page is a live keyboard diagram: pressed keys light up, clicking a key edits it, and keys that are not set show ▽ for transparent.
  - Key picker: a full-size keyboard where a key is chosen by clicking or by pressing it, with the held modifiers shown live. Remapping pauses while recording, and a remap takes effect as soon as it is chosen.
  - All edits are written back to the configuration file by path, keeping comments and formatting. Edits are validated first and are not written if there are errors.
  - The keyboard diagram can correct key positions: switch between the Mac and Windows layouts with one click, or calibrate (press every key, and when the lit position is wrong, click the correct position and press again). Calibrated keys (including those that were already right) are marked green with a check, uncalibrated keys are dimmed, and calibration progress is shown. Calibration is remembered per keyboard and follows the keyboard currently being typed on.
  - When keyboard identification becomes necessary after a configuration reload or after the settings window opens, the background service starts identifying keyboards automatically, without a restart.
  - macOS layer HUD: after holding a layer key for about 0.4 s, the layer's keys appear at the bottom of the screen with pressed keys lighting up live. A quick tap does not flash it. It can be turned off on the overview page or with `settings.layer_hud`.
  - New keyboard prompt: the first time a keyboard is connected, pressing the key left of the space bar identifies whether it has a Mac or Windows layout. For a Windows layout, Alt and Win can be swapped (written as a remap specific to that keyboard). The prompt can be ignored, and nothing changes unless it is used.
  - The overview page drops "last rewrite" and gains a slider for the decision times.
- G1 background service (the first half of the GUI):
  - Menu bar icon (macOS) / tray icon (Windows): status line, pause / resume remapping, open settings, Event Viewer, open the configuration in an editor, reload the configuration, and quit. On macOS the menu items carry line icons (template images that follow light/dark mode and highlight color; since macOS 27 the system hides menu images by default, so they are explicitly requested here). The background service shows the icon by default, and `kanami run --tray` can show it too. On macOS this switches to the NSApplication event loop.
  - Local IPC (macOS: `~/Library/Application Support/kanami/kanami.sock`, connectable only by the owner; one JSON object per line): `status`, `pause`, `resume`, `reload`, `devices`, `subscribe`, `quit`. Key events are sent only when there are subscribers, over a fixed-size channel; when it is full, events are dropped and the number skipped is reported, so the key callback never waits. The Windows named pipe is not implemented yet.
  - `kanami ctl`: calls the IPC above from a terminal.
  - New backend wake-up mechanism: commands from other threads run on the main thread.
- G1 settings window (`apps/settings`, Tauri 2 + Svelte 5, started on demand and exits when the window is closed):
  - Overview: a large keycap for the last rewrite, permission status, pause remapping, launch at login, and the configuration file.
  - When the Accessibility permission is missing, the whole page becomes a permission guide that disappears after the grant (the background service accepts IPC connections while it waits for the permission).
  - Event Viewer: subscribes to the background service's key events, shows input and output as keycaps, shows only remapped keys by default, keeps up to 5000 entries in a virtual list, and supports pause, clear and search.
  - Configuration source: a CodeMirror editor with TOML highlighting, completion of key names, field names and table names from the JSON Schema, live validation, and a gutter mark showing whether each test passes. Save with ⌘S; a history of earlier versions (the latest 50, viewable and restorable) is kept before each save; external changes are reported and the two sides can be compared.
  - "Open Settings..." and "Event Viewer..." in the menu bar open it; if it is already open, only the window is brought to the front.
  - The frontmost app is shown with its localized name; the segmented control's thumb slides, pages fade in, and scroll bars are thin.
  - `kanami-ipc`: the IPC address and client are a separate crate shared by the background service, `kanami ctl` and the settings window. The script loading logic moved to `kanami-script`, so the command line and the settings window run tests with the same code.
- G0 engine additions (preparing for the GUI):
  - Emergency stop: hold Left Control + Space + Esc together to disable all remapping (keys and mouse events pass through, and Caps Lock returns to its system behavior), and press again to resume. It recognizes physical keys only and does not depend on the configuration.
  - Rule `enabled = false`; rule groups `[[group]]` (shared `when` and switch), with rules ordered by their position in the file.
  - Per-keyboard sets: `remap`, `rule` and `group` can be written under `[device.<name>]` and take effect before the general settings; with `inherit = false`, only the keyboard's own set is used.
  - New key names: numeric keypad (`keypad_0`-`keypad_9`, `keypad_period`, `keypad_plus`, `keypad_minus`, `keypad_multiply`, `keypad_divide`, `keypad_enter`, `keypad_equal`, `num_lock`), `insert`, `print_screen`, `scroll_lock`, `pause` and `menu`. macOS has no separate key codes for Print Screen / Scroll Lock / Pause.

### Changed

- The minimum supported Rust version is now 1.90 (`tray-icon` requires it).
- Pressing a layer's `hold` trigger key while a modifier is held no longer enters the layer, so system shortcuts such as ⌘Space and ⌃Space take effect immediately.

### Fixed

- macOS: key and mouse events synthesized by other programs (for example Logitech Options+ turning a mouse side button into ⌘[) keep their modifiers instead of having them erased, so the side buttons' forward and back functions work again.
- Keys synthesized by other programs (Logitech Options+, password manager autofill, text expansion tools, AutoHotkey and so on) pass through unchanged and no longer take part in remapping. Like Karabiner, kanami only remaps the physical keyboard. `--trace` and `monitor` mark them.
- macOS: after the event tap is suspended by the system, after Secure Input (password fields) was on, or after sleep, key state is reconciled with the hardware and any release that did not pass through kanami is supplied, which prevents stuck modifiers or layers.
- macOS: the same input source switch is no longer notified twice.
