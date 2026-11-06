# GUI design proposal (draft)

> Status: **under review**. A clickable UI sketch is in [gui-mockup.html](gui-mockup.html) (open it in a browser).

## Principles

1. **The GUI is optional.** The command line and the configuration file remain first-class. The GUI is a visual editor for the configuration plus a status panel, and does not introduce a second configuration.
2. **The GUI writes back to the same `config.toml`.** It edits with `toml_edit`, keeping comments, order and content it does not understand (such as script source), so hand editing and GUI editing can be mixed.
3. **The GUI runs separately from remapping.** Remapping is done by the existing `kanami` background service, and the GUI is only its client. A GUI that crashes, lags or is not open does not affect remapping. With the GUI closed, only the 2 MB background service stays resident.
4. **Small.** No Electron. The target is an installer of about 10 MB (Karabiner-Elements is about 50 MB).

## Architecture

```
┌─ Settings window (Tauri, started on demand, exits when closed) ──────────────┐
│ Rules · Packs · Layers · Keyboards · Event Viewer · Config source            │
│ Frontend (system WebView: HTML/CSS + Svelte)  ⇅  Rust command layer          │
│                       reads/writes config.toml (toml_edit)                   │
└──────────────────────────────────────┬───────────────────────────────────────┘
                                       │  IPC: local socket / named pipe, JSON
┌──────────────────────────────────────┴───────────────────────────────────────┐
│ kanami background service (resident, pure Rust, launchd / scheduled task)    │
│ Engine ← hot reload ← config.toml          event stream → Event Viewer       │
│ Menu bar / tray icon (native menu)         Layer HUD (native small window)   │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Only the background service stays resident.** The remapping engine, the menu bar / tray icon and the layer HUD all live in it. All of them are Rust plus native system interfaces, never go through a browser, and normally take a few MB of memory. The settings window is a separate process: it starts when "Open Settings" is chosen from the menu bar, and the whole process exits when the window is closed, so the browser engine uses memory only while settings are being viewed. Key handling never goes through the settings window, so a slow GUI cannot affect typing.

The background service gains a local IPC interface (a Unix socket in the user directory on macOS, a named pipe on Windows), one JSON object per line:

| Request | Purpose |
|---|---|
| `status` | Running state, configuration summary, latest errors, permission state |
| `pause` / `resume` | Pause / resume remapping (menu bar switch) |
| `reload` | Reload the configuration immediately |
| `subscribe` | Subscribe to key input → output, device, frontmost app and pause state (Event Viewer). Only events after the subscription are pushed, see "Event Viewer" below. |
| `devices` | Connected keyboards |
| `show` | When the settings window is already open, bring it to the front (choosing "Open Settings" in the menu bar again); done together with the settings window |
| `quit` | Stop remapping and exit |

Validating the configuration, running tests and listing key names do not need the background service. The GUI calls existing crates such as `kanami-config` directly.

The settings window can be closed at any time. When the background service is not running (for example Kanami.app was opened directly), the window shows "Remapping is not running" and offers "Start / install as a background service".

## Technology choices

**Conclusion: the settings window uses Tauri 2 (system WebView), and the resident part is entirely Rust plus native system interfaces.**

| Part | Technology |
|---|---|
| Settings window | Tauri 2; frontend Svelte 5 (no runtime after compilation); TOML editor CodeMirror 6, connected to the existing JSON Schema for completion and validation; no large component library, styles are written by hand to match the sketch |
| Menu bar / tray icon and menu | `tray-icon` and `muda` in the background service, native system menu |
| Layer HUD | Drawn directly with system interfaces in the background service (a non-activating `NSPanel` on macOS, a topmost layered window on Windows). It must appear the moment a layer key is held and cannot wait for a browser to start. |

**Why a browser-based approach**

- **Smallest size**: the browser engine ships with the system (WKWebView on macOS, the WebView2 built into Windows 11), so the program itself is only a few MB, smaller even than self-drawing frameworks such as iced and Slint.
- **Performance is solved by architecture**: the cost of a browser approach is a startup of a few hundred milliseconds and higher memory while the window is open (over a hundred MB on Windows). The settings window starts on demand and exits when closed, and the resident part does not use a browser, so everyday performance and memory depend only on the background service.
- **Best looking**: the sketch itself is HTML/CSS and can be reproduced one to one. Chinese input methods, screen readers and code editors are all ready-made.
- **Most widely used**: among Rust desktop frameworks, developers choose Tauri the most, so answers to problems are easy to find.

**Other options considered**

| Option | Why it was not chosen |
|---|---|
| GPUI (Zed's framework) + gpui-component | Polished look, but it is developed officially only inside the Zed repository and follows Zed's changes. The "many users" are mainly Zed users, and few developers pick it as a framework. |
| iced | Pure Rust and independently maintained, but the default look is plain and few well-known apps on macOS / Windows use it |
| Slint | Not chosen |
| egui | Immediate mode, redraws whole regions when the UI changes; Chinese fonts have to be bundled |
| SwiftUI + WinUI | Neither is cross-platform, so two UIs would be needed |
| Wrapping native controls ourselves | Text layout, input methods, accessibility and rendering would all have to be built, far more work than kanami itself |
| Electron | Bundles Chromium, 100 MB or more |

**The GUI has a single code base.** The settings window is the same frontend and the same Rust command layer on macOS and Windows. Platform differences are switched only by the `mac` / `win` classes on the root element and their design variables, and no page is written for one platform alone. `tray-icon` and `muda`, used for the menu bar / tray icon, are also one cross-platform code base. Only the layer HUD needs a small separate implementation per platform.

## Visual design

kanami's theme is keys, so the whole UI has a single core visual element: the **keycap**. Rules, events, layers and packs all draw keys as keycaps, and "input keycap → output keycap" is the whole message of the product. Everything else stays quiet and follows each platform's native conventions.

The **palette** is taken from the overall mood of the two skins of the character Kanami in Strinova, and uses **one color family**: cherry pink as the accent, plum purple as the dark color, creamy white as the light color, with every neutral tinted slightly toward cherry pink. The UI does not include character artwork (copyright, and to avoid overpowering the interface).

| Name | Source | Light | Dark | Use |
|---|---|---|---|---|
| Cherry pink | Pink sweater, roses | `#D8668F` (keycap `#E07A9F` / side `#B5577A`) | Controls `#F08DB1`, keycap and primary button `#C9658B` / `#8E3F5F` | Remapped keys, switches, selected state, primary button |
| Plum purple | Dusk shadow, dark part of the plaid skirt | Modifier keycap `#5A4766` / side `#3F3049`, layer base `#3F3049` | `#3A2E44` / `#241C2B`, base `#18121C` | ⌘⌥⌃⇧, hyper, dark background |
| Creamy white | White shirt | Letter keycap `#FFFDFC` / side `#E6DCE1` | `#4B3F52` / `#33293A` | Regular keys; in a layer, the key that enters the layer is a creamy-white keycap with a cherry-pink outline |
| Pale pink | — | `#F8E6EE` | `#45283A` | Selected items, light background for emphasis areas |
| Ink | — | `#33283A` | `#F3EBF2` | Body text |
| Status | Grass, yellow dress (desaturated) | Green `#4E9A6A`, dark gold `#B9862C`, orange-red `#D2552E` | `#6FBF8B`, `#E0AE52`, `#F07A52` | Running, warning, error |

The error color leans orange to separate it from cherry pink in hue, and error messages also carry an exclamation icon, so they are not distinguished by color alone. In dark mode, keycaps and the primary button use a darkened cherry pink to keep the contrast of white text. Adjacent keycaps keep a 4px gap so they are not read as one key.

**Icons**: a custom set of line icons (SVG, 24 grid, 1.7 stroke width, round caps and joins), bundled with the frontend. No icon font, emoji or special characters are used instead. The color follows the text and turns cherry pink when selected.

- App icon: a cherry-pink keycap with a creamy-white K on its top face. The Windows title bar and tray use it directly.
- macOS menu bar: a single-color outline version of the same shape (template icon) that follows the light/dark state of the system menu bar.
- Navigation: Overview (gauge), Rules (two-way arrow), Packs (box), Layers (stacked layers), Keyboards, Event Viewer (pulse line), Config source (angle brackets).
- Permission rows: a System Settings-style colored square with a white line icon (a person for Accessibility, a keyboard for Input Monitoring).
- Each item in the menu bar drop-down also has an icon. Since macOS 27 the system hides menu item images by default (`NSMenuItem.preferredImageVisibility` defaults to "automatic"), and kanami explicitly sets it to "visible".

**Fonts**: UI text uses the system font (SF Pro / PingFang on macOS, Segoe UI Variable / Microsoft YaHei on Windows). Only the text on keycaps uses Barlow, a DIN-style typeface that looks like keycap legends, bundled with the app.

**The two platforms differ only in the shell**:

| | macOS | Windows 11 |
|---|---|---|
| Window | Red/yellow/green buttons, translucent sidebar, System Settings-style rounded grouped lists | Window buttons on the right of the title bar; navigation on a Mica background, with content in a slightly lighter "content layer" (WinUI's LayerFillColor, rounded top-left corner + thin left and top borders) holding Win11 settings cards |
| Navigation selection | Rounded highlight | Accent bar on the left |
| Controls | macOS switches and buttons | Fluent switches (hollow when off) and buttons |
| Resident entry | Menu bar drop-down (native) | Right-click menu of the tray icon (native; the tile panel in the sketch is not built) |

Template-like patterns deliberately avoided: no row of statistic cards on the overview page; the Event Viewer draws input and output as keycaps instead of a string of code; descriptions are written as complete sentences.

## Language

The UI language is the config's `settings.language`, then the `KANAMI_LANG` environment variable, then the first supported OS language, then English. The tray menu relabels when the configuration reloads.

## UI structure

### 1. Menu bar icon (Windows: tray icon)

Provided by the background service as a native system menu. It does not require opening the settings window.

Icon states: normal / paused / configuration error / missing permission. The menu contains:

- Status line: `Running · 3 rules · 1 layer`; when the configuration has errors, the first error is shown
- Enable remapping (switch, shortcut configurable)
- Open Settings... / Event Viewer...
- Reload configuration · Open the configuration file in an editor
- Launch at login (switch, equivalent to `kanami service install`)
- Quit the GUI (remapping keeps running) · Stop remapping

### 2. Settings window (left navigation)

| Page | Content |
|---|---|
| **Overview** | Permission check (Accessibility, Input Monitoring; one click opens the matching System Settings page when not granted, and the page refreshes automatically after the grant); pause remapping; launch at login; HID mode switch; current configuration file path; decision times (tap/hold, typing streak, combo) |
| **Remaps** | A two-column table of `[remap]`: from → to. Both sides are keycap buttons that open the key picker: a full-size keyboard where a key is chosen by clicking or by pressing it (held modifiers are shown live), and keys missing from the keyboard can still be clicked. A choice takes effect immediately, and the target can also be Hyper or disabled. |
| **Rules** | At the top, choose a **keyboard** first: "All keyboards" and each keyboard are tabs (connected ones get a green dot, ones that are not connected but have rules are dimmed), and each tab shows its rule count. The selected tab decides which set of rules is edited, and rules themselves no longer carry a "which keyboard" condition, see "Per-keyboard rules" below. Below is a flat rule list without folding: a group is only a small heading row (group name, shared condition such as "only in terminals", whole-group switch). Groups and rules can be reordered by dragging, are checked from top to bottom, and the first match wins. Each row reads like a sentence ("hold caps to become control") and has its own switch (corresponding to `enabled = false`). Actions: key / tap·hold / run command / call script. Conditions are picked from running apps and enabled input sources instead of typing IDs. The list is searchable and can be filtered by "All / Mine / From packs". Groups from a pack are labeled "Pack"; their rules are shown as usual but cannot be edited one by one, and the group heading has "Adjust parameters", which jumps to the Packs page. The bottom of the page lists custom scripts, with "Open in editor". |
| **Packs** | A separate page, see "Rule packs" below |
| **Layers** | Live keyboard diagram, see "Layers" below |
| **Combos** | Keys pressed together → action, with a slider for the decision window |
| **Keyboards** | Connected keyboards and keyboards that have rules; rename, choose a layout, remove (together with its rules). A new keyboard appears automatically in the tabs of the Rules page the first time it is connected, without needing a name first. |
| **Tests** | List of `[[test]]`; run all; on failure shows expected / actual; "Record this as a test" is available from the Event Viewer |
| **Config source** | In the "Advanced" group of the navigation, see "Config source" below |

Any change made in the settings window is validated with `kanami-config` first. If there are errors, nothing is written to the file, and the error is marked on the corresponding card.

### Per-keyboard rules

Rules are organized into sets per keyboard instead of choosing a keyboard in each rule (Karabiner's simple modifications also pick the device first and then remap):

- **All keyboards**: rules that apply to every keyboard. Most people use only this set.
- **Rules specific to one keyboard**: apply to that keyboard only and are checked **before** the "All keyboards" rules, so they can override the general rules. Below the specific rules there is a line "N rules for all keyboards also apply to this keyboard", with a switch. When it is off, the keyboard uses only its own rules.
- Keyboards are identified by vendor ID, product ID and product name (keyboards that borrow another vendor's ID, such as NIZ, are told apart by product name). Rules of a keyboard that is not connected are kept, and take effect automatically once it connects.
- Remaps (`[remap]`) are likewise split into "All keyboards" and per-keyboard sets. Layers are not split per keyboard for now; the "which keyboard" choice on the Layers page only decides which layout is drawn.
- When adding a pack, the "Add to" choice picks a tab, "All keyboards" by default.

Configuration file syntax (added to the engine in G0):

```toml
[device.niz]
product = "NIZ BT5.2"
inherit = true               # also use the "all keyboards" rules, default true

[device.niz.remap]
left_option = "left_command"
left_command = "left_option"

[[device.niz.rule]]
trigger = "hyper+t"
action = { run = "open -a Terminal" }
```

The existing `when.device` condition remains supported for hand-written configurations but is no longer offered in the GUI. When it appears in a configuration, the Rules page shows "Move to this keyboard's rules" on that rule, with one-click migration.

### Layers (modeled on KeyPath)

The core of the layer feature is a **live keyboard diagram**: every key shows directly what it becomes now, and clicking a key edits that key.

- **Layer tabs**: the base layer and each layer, with a dot on those that have content. Next to a tab is how it is entered ("hold space to enter", "press hyper+n to toggle"). The last tab is "New layer".
- **How keys are drawn**: a changed key is cherry pink and shows the original key and the target. A tap/hold key is drawn as a key inside a key: the main body is the hold behavior, and the small label in the corner is the tap key. A layer key is a creamy-white keycap with a cherry-pink outline reading "Hold: Nav layer". In a non-base layer, keys that are not set are drawn as **▽ transparent**, with a faint preview of what they are in the next layer.
- **Live**: pressed keys light up on the diagram, which also confirms that the right layout was chosen.
- **Key editing panel** (appears under the keyboard after a key is clicked): "j in the Nav layer"; a segmented control for the type: key, tap/hold, enter layer, run command, call script, transparent (in the base layer, "keep as is"), disabled; the target is chosen with the key picker; undo.
- **Safeguards**: when a key is set to "enter a layer", the same position in the target layer is automatically set to transparent, so it is possible to get out again. A warning is shown for a layer with no way to exit.
- **Display options**: above the diagram, choose "which keyboard, physical layout, keyboard layout"; changing them refreshes immediately. The keyboard layout (QWERTY, Dvorak, Colemak, Colemak-DH, Workman, AZERTY, QWERTZ and so on) changes only the legends on the keycaps and follows the system by default.
- **The bottom row order differs between the platforms**: control / option / command on macOS, Ctrl / Win / Alt on Windows. The same physical position corresponds to different kanami key names on the two platforms (for example Alt on Windows is `left_option`), and the diagram maps positions to the correct key names per platform.

**Layer HUD** (drawn by the background service with system interfaces; on macOS an NSPanel with CALayer; it does not accept clicks or take focus): after holding a layer key for about 0.4 seconds, a small keyboard diagram of that layer appears at the bottom of the screen (a quick tap does not flash it) and disappears on release, with pressed keys lighting up live. It can be turned off on the Overview page (`settings.layer_hud`). It is shown only in menu bar mode and is not built on Windows yet. So far it has only unit tests for the display timing and a rendering preview; it has not been run end to end with real keys.

### Keyboard layouts

The Layers page has to draw a keyboard. What it draws follows these rules:

- **What can be known automatically**: ANSI / ISO / JIS (recorded by the system; it decides the shape of Return and left Shift); the legends on the keycaps (from the current keyboard layout, such as AZERTY; Chinese input methods use the US layout); the names of modifiers (control / option / command on macOS, Ctrl / Win / Alt on Windows).
- **What cannot be known: the hardware position of each key.** The system reports only key codes, not where a key is. A Windows-layout keyboard attached to a Mac has Win as its second key (sending command) and Alt as its third (sending option), the opposite of an Apple keyboard. Keys remapped in firmware and layouts like HHKB behave the same way, and vendor IDs cannot be relied on (NIZ borrows Apple's ID). So the positions on the diagram are an assumption, and the strategy is:
  - Rules match key codes and have nothing to do with positions on the diagram, so a wrong diagram does not affect remapping. When picking a key, the key actually pressed wins, so pressing a key directly is always right.
  - The Layers page can correct the display: one click switches between "Mac layout / Windows layout", which solves the most common case. For other keys that do not match, use "Calibrate key positions": press every key on the keyboard, and each key pressed turns green on the diagram. If the lit position is wrong, click the correct position on the diagram and press that key again, and the two keys are swapped. Switching to another window exits automatically. In calibration mode, calibrated keys are green with a check and uncalibrated keys are dimmed; normally a check stays in the corner, and the progress and the keys with changed positions are shown below the diagram.
  - These settings are remembered per keyboard, affect only the display, and automatically follow the keyboard currently being typed on. The key picker uses the same result.
- **What cannot be known: the physical layout** (60% / 65% / 75% / TKL / full size). The key capabilities reported over HID are unreliable, and vendor and product IDs may be borrowed (for example NIZ borrows Apple's ID), so kanami does not guess.
- **When no layout is chosen, all keys kanami can remap are drawn** (full size: F-key row, main block, navigation block, arrow keys, numeric keypad). A small layout is not drawn by default, so that keys the user can remap are not hidden. Keys kanami does not support on the current platform (PrtSc / ScrLk / Pause on macOS, which the system treats as F13-F15; Fn and keypad `=` on Windows) are drawn as hollow gray outlines labeled "Remapping is not supported on this platform". The keypad, Insert, Menu and so on can all be remapped.
- On the Keyboards page, the user chooses a layout once per keyboard (with thumbnails), in two groups: "Standard" (100%, TKL, 75%, 65%, 60%, 40%, HHKB) and "Split and ergonomic" (Corne, Ferris Sweep, Sofle, Kinesis 360 and so on). Each layout has finer options (ISO Enter, split Backspace, split left and right Shift, bottom row style). After a layout is chosen, keys that have never been pressed on this keyboard are faded, which helps to notice a wrongly chosen layout.
- A local **KLE** or **QMK `info.json`** file can be imported to get an exact layout directly (the file is only read; kanami does not talk to the keyboard).
- The layout affects only the display. It is stored in the GUI's own settings and not written to `config.toml`.

### Rule packs

A rule pack is a set of rules written by someone else and shipped with tests (a rule group template with parameters). Community packs may number in the thousands, so they get a separate page designed like an app store: search, categories, sorting, list plus details.

**Page structure**

- Top: a search box (name, description, and the keys and apps involved are all searchable); a source switch "Added / Built-in / Community"; sorting (most downloaded, recently updated, name).
- Category tags (with counts): modifiers, layers and navigation, editor style, input methods, launching apps, text expansion, specific apps, games, keyboard models. The category is filled in at submission, and CI checks the value.
- Left: a compact list with name, badges, a one-line description, author, download count and whether it is added. It keeps loading while scrolling (a virtual list, smooth with thousands of entries).
- Right: details with author, version, license and update time; badges and test results; "What it does" (which keys it changes, which layers it adds, whether it runs commands, whether it ships a script, in which apps it does not apply); **conflicts with your existing rules** (overlapping keys, which one has priority); all rules listed in the same sentence style as the Rules page; "Add to" (all keyboards or one keyboard) and "View on GitHub".
- When a pack is selected under "Added", the details become settings: adjust parameters (for example for Caps Lock, the tap can be Esc / Backspace / Hyper / unchanged, the hold can be Control / Hyper / Shift, and the decision strategy), view the generated rules, "Convert to custom" (it becomes a regular group that can be edited freely and no longer follows pack updates), remove, and jump to the Rules page.
- Built-in packs ship with the program: Caps Lock makeover, Hyper key, arrow-key layer, home-row modifiers, number layer, Leader key, text expansion, tap dance.
- Any group on the Rules page can be "Shared as a pack": it is exported as a pack folder, and the contribution guide is opened in the browser. kanami does not store GitHub accounts or tokens.

**Community repository** `kanami-packs` (GitHub)

- **Submission**: one folder per pack, containing `pack.toml` (name, description, author, version, license, category, parameters, rules) and `README.md`, and it must include `[[test]]`. It is submitted through a pull request. Licenses are limited to permissive ones such as MIT, Apache-2.0 and CC0.
- **Automatic checks** (CI): configuration validation, all tests passing, unique name, increasing version number, valid category. It also works out which keys are changed and whether commands or scripts are used, and writes that into the index. A pack that uses `run` or a script must be reviewed by a person before merging.
- **Publishing**: after merging, CI packages each version as a GitHub Release asset (the file name contains the version) and generates the index. The download count is the Release asset download count, summarized into the index by CI on a schedule. No server of its own is needed, and no user data is collected.
- **Index**: published on GitHub Pages and gzip-compressed. An entry is about 300 bytes, so a thousand packs are about 300 KB, and a few dozen KB compressed. Beyond ten thousand, the index is split into several files by category, fetching summaries first and details on demand. Search and sorting happen locally. kanami goes online to fetch the index only when the user opens "Community" (it is cached, and an ETag decides whether it is updated), and is otherwise offline.
- **Installation**: the specified version is downloaded, its SHA-256 is checked against the index, and it is stored under `packs/` in the configuration directory. The version and hash are recorded in the configuration file. All content of the details page is shown before adding, and a pack that runs commands requires expanding and reading every command before it can be added. **No automatic updates**: when a new version exists, "Added" shows a notice, and the update is manual after reviewing the diff.
- **Badges**: built-in, reviewed, unreviewed (passed only the automatic checks), runs commands.

Syntax in the configuration file:

```toml
[[pack]]
name = "caps-lock"                 # built-in pack
params = { tap = "escape", hold = "control" }

[[pack]]
name = "emacs-motion"              # community pack
source = "community"
version = "1.3.0"
sha256 = "…"
```

Packs are expanded into regular rule groups when `kanami-config` loads them, so the engine does not need to know packs exist. "Convert to custom" writes the expanded result back to the configuration file.

### 3. Event Viewer

A live table: time, input, output, which keyboard, frontmost app. It can be paused and filtered by key / app / keyboard. Selecting a range allows "Record as a test", which generates the input / expect of a `[[test]]` automatically. It corresponds to Karabiner's EventViewer but shows the keys **before and after remapping** together. It also shows the currently active layers and script variables live, and can copy as JSON.

**Buffering**:

- The background service keeps a preallocated fixed-size ring buffer (about 4096 entries) of fixed-size records (time, input key and state, output key, device, app index). The key callback only writes one record into it: no allocation, no string formatting, no locks. With no subscribers, the cost is one atomic flag check.
- The IPC thread takes records from the buffer, converts them to JSON and sends them to the GUI. When the GUI cannot keep up, the oldest are dropped and a "skipped N" message is sent, which the GUI draws as a notice row in the table. The key callback never waits for the GUI.
- The GUI keeps at most the latest 5000 entries and shows them in a virtual list. "Pause" freezes only the display, and "Clear" clears the list in the GUI.
- The existing `--trace` formats strings in the callback and hands them to a printing thread over an unbounded channel, which is enough for command-line debugging. When the Event Viewer is implemented, it will be changed to the ring buffer above, and `--trace` will read from the buffer too.

**Privacy**: key records may contain passwords.

- They live in memory only and are never written to disk. Only keys pressed after the viewer is opened are recorded, and closing it clears them.
- The default is "only remapped keys". Regular typing appears only after switching manually to "All keys", and the bottom of the page warns about screen recording or screen sharing.
- macOS Secure Input (password fields and so on) is never handed to kanami by the system anyway.

### Config source

For people who prefer to write the configuration directly. It sits in the "Advanced" group of the navigation.

- **Takes effect on save**: validation is live while typing (line and column, spelling suggestions, one-click fix), and the configuration is applied only after saving with ⌘S / Ctrl+S. When there are errors it is not applied, and remapping keeps using the last correct configuration.
- **Completion**: field names, key names and app names are completed from the JSON Schema, with descriptions on hover.
- **Interoperates with the visual pages**: above each `[[rule]]` there is "Open in Rules page", and every rule on the Rules page can be "Viewed in source", which jumps to the matching line. Changes made in the GUI are written back with `toml_edit`, keeping comments and formatting.
- **Tests**: above each `[[test]]` the pass / fail state and "Run" are shown.
- **External changes**: when the file is changed in another editor and there are also unsaved changes here, a notice at the top offers "Compare both sides / Use the one on disk". When there are no changes here, it simply refreshes.
- **History**: every save (from any page) keeps a snapshot, the latest 50 are kept, and they can be compared and restored. The GUI rewrites the configuration file, and snapshots make a broken file recoverable.
- "Open in an external editor".

### Safety

- **Emergency stop**: the engine has a built-in fixed chord (Left Control + Space + Esc pressed together) that does not depend on the configuration, so it works even when the configuration is broken, and immediately disables all remapping.
- A warning for "keys that can no longer be produced" (for example caps was changed to esc and no other key can send caps).

### 4. Getting started (no wizard)

There is no multi-step first-run wizard: a welcome page carries no information, common presets duplicate the packs, and "Launch at login" can simply be on by default. What the user needs to do appears when it is needed:

1. **When a permission is missing**: the whole settings window shows "kanami cannot remap yet", listing Accessibility and Input Monitoring, with a check on those allowed and "Open System Settings" on those not allowed. After the grant it refreshes automatically and the page disappears; until then the navigation is grayed out and not clickable. First installation and **every upgrade** (not notarized, so the system asks for authorization again) go through here. Windows needs no authorization, so it does not appear there.
2. **When there are no rules yet**: the Rules page shows four common built-in packs (Caps Lock makeover, Hyper key, arrow-key layer, modifier swap), each with "Add", and below them "New rule" and "Find more in Packs".
3. **When a new keyboard is connected for the first time**: a notice appears at the top of the settings window and can be ignored. After "Start identifying", pressing the key right next to the left of the space bar on that keyboard decides the layout: sending command means a Mac layout, sending option means a Windows layout, and the diagram is drawn accordingly. It does not rely on the legends printed on the keycaps and does not guess from the vendor ID. For a Windows layout, "Swap" can be chosen to write Alt and Win as remaps specific to that keyboard; in other cases the user is directed to key position calibration. Without clicking, kanami does not listen to keys and changes nothing. Apple's own keyboards get no notice.
4. **Confirming it works**: the keyboard diagram on the Layers page lights up pressed keys live, and the Event Viewer shows what each key became.

**No configuration at the start**: on first run an empty configuration file is created and all keys pass through unchanged. Remaps, rules, layers and combos are all added by the user; only parameters that do not affect existing input, such as decision times, have defaults.

## HID mode (optional)

Turning on the switch on the Overview page starts a guide, see [hid-mode.md](hid-mode.md):

1. Explains the benefit (works in password fields, games and virtual machines) and the cost (a driver must be installed, it runs with administrator privileges, and it cannot be used together with Karabiner-Elements)
2. Downloads and installs the driver: an official installer of a pinned version, with progress shown and the hash verified
3. Opens "System Settings → Privacy & Security", the user clicks "Allow", and the GUI continues automatically once it detects that the driver is active
4. Installs the administrator background service (the system shows a password prompt)
5. Detects that the virtual keyboard is ready; afterwards it can be turned off or uninstalled at any time

## Industry survey conclusions

The survey covered software remapping tools (Karabiner-Elements, PowerToys Keyboard Manager, BetterTouchTool, Keyboard Maestro, Input Remapper, KeyPath) and firmware configuration tools (VIA, Vial, QMK Configurator, ZMK Studio, Wootility, Keychron Launcher, Logitech G HUB, Razer Synapse):

- None of the software remapping tools has visual layer editing, and most do not draw a keyboard. KeyPath (a macOS GUI for kanata) is the exception and is the main reference for the layer part of this proposal.
- Firmware tools are the best at drawing keyboards and editing layers, but they know the keyboard layout in advance. When VIA meets an unknown keyboard, it asks the user to import a definition, which is its most complained-about point. kanami always offers a generic layout as a fallback.
- Borrowed: rule rows as sentences with separate switches, both recording and clicking to pick, conditions set on groups, rule packs, ▽ transparent keys and inherited display, safeguards for layer keys, live key highlighting, a permission checklist, emergency stop.
- Avoided: complex rules that can only be written as JSON, making the user type process names, and listing keys the user's keyboard does not have without any notice.

## Future feature: reading and writing keyboard firmware (not planned yet)

For keyboards that support VIA / Vial (and later ZMK Studio), kanami can read and write the key mappings in the keyboard firmware directly, forming two remapping layers: a "firmware layer + software layer". Rules placed in firmware also work in password fields, games, the BIOS and on other computers, without HID mode.

- Reading: the definition of a Vial keyboard is stored in the firmware, so the exact layout can be read directly. A VIA keyboard needs a lookup in VIA's definition library by vendor and product ID (online; the license must be confirmed).
- Writing: edit key mappings, layers and tap/hold (QMK's mod-tap / layer-tap) in firmware; Vial also has combos and tap dance (the number is limited by the keyboard's storage). Before writing, kanami backs up automatically, shows a diff first and allows one-click restore. Firmware mappings are also written to the configuration file (`[firmware.<keyboard name>]`) and can be managed with git.
- A rule can be marked "writable to the keyboard". Rules that use per-app behavior, scripts or text output stay in the software layer. When both layers set the same key, conflicts are checked, and the Event Viewer shows "physical key → sent by firmware → after kanami" separately.
- The scope is limited to key mappings; lighting, knobs and flashing firmware are not included. Searching the QMK keyboard library online by model also belongs here.
- Prerequisite: real VIA / Vial keyboards are needed for development and testing.

## Packaging

- macOS: `Kanami.app` contains the background service (`kanami`, which is also the command-line program) and the settings window program (Tauri). In the permissions list it appears as Kanami with an icon, which is easier to find than a bare binary. The command line can be installed through Homebrew or linked into PATH from the GUI with "Install command-line tools".
- Windows: the installer contains `kanami.exe` and the settings window program. WebView2 comes with Windows 11 and needs no separate download. Launch at login is implemented with a scheduled task.
- The consequences of having no developer account (not notarized, re-authorization after upgrades) are stated plainly on the missing-permission page and in the release notes.

## Phases

| Phase | Content |
|---|---|
| **G0 engine additions** (done) | Emergency stop; rule `enabled`; rule groups; per-keyboard rules and remaps (`rule`, `group`, `remap`, `inherit` under `[device.<name>]`); added key names for the keypad, Insert, Print Screen, Scroll Lock, Pause, Menu and so on |
| **G1 minimum usable** (done; the Windows named pipe is still to do, so on Windows the settings window cannot connect to the background service yet) | Background service IPC; menu bar / tray icon and status in the background service; settings window started and exited on demand; pause/resume; whole-window missing-permission page; launch at login; config source (completion, test marks, history, external change notice); Event Viewer (ring buffer, remapped keys only) |
| **G2 visual editing** (in progress) | Done: rules, remaps, layers (live keyboard diagram, key editing, macOS layer HUD), combos, keyboards page; key picker; empty states. New keyboard notice is done. To do: layout templates and KLE / QMK file import; migrating `when.device` to keyboard-specific rules |
| **G3 packs and tests** | `[[pack]]` loading and expansion; Packs page (added, built-in, parameter settings, convert to custom); share as a pack; test panel; recording tests from the Event Viewer |
| **G3.5 community packs** | `kanami-packs` repository, contribution guide, CI checks, Release publishing and index; "Community" on the Packs page (search, categories, sorting, conflict check), hash verification, update notices (after the repository is built) |
| **G4 HID mode** | Driver installation guide, administrator service, HID backend |
| **Later** | Keyboard firmware read/write (VIA / Vial / ZMK), online search of the QMK keyboard library by model |
