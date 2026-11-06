# Architecture

## Targets

| Metric | Target |
|---|---|
| Extra latency per key press | p99 < 1 ms |
| Idle CPU | ≈ 0% (only a configuration check once per second) |
| Resident memory | < 15 MB |
| Binary size | < 5 MB (currently about 700 KB) |

## Layers

```
        Platform backend                Core                      Config / plugins
┌──────────────────┐   KeyEvent   ┌──────────────┐   Rules   ┌──────────────┐
│ kanami-macos     │ ───────────► │ kanami-core  │ ◄──────── │ kanami-config│
│ CGEventTap       │ ◄─────────── │ Engine       │           │ TOML         │
│ (later: HID)     │   KeyEvent   │ pure logic   │           │ (later: Luau)│
└──────────────────┘              └──────────────┘           └──────────────┘
```

- **Core (`kanami-core`)**: takes physical key events and time in milliseconds, and produces the key events to emit. It depends on no system API and its behavior is deterministic, so rules can have unit tests and the `[[test]]` entries in the configuration can be replayed offline.
- **Config (`kanami-config`)**: runs only at startup and on reload. It parses TOML into `Rules` and maps the engine's validation errors back to line and column numbers in the source file.
- **Platform backend**: intercepts keys, translates platform key codes into `Key`, and emits the events the engine produces.
- **Command line (`kanami`)**: wires the three together, and handles hot reload and running `run` commands in the background.

## Engine

Processing has two stages:

1. **`[remap]`**: physical key → logical key, stateless, a table lookup. The current keyboard's `[device.*.remap]` is checked first, then the general `[remap]` (skipped for keyboards with `inherit = false`).

The emergency stop comes before both stages: the engine records the physical keys held, and holding Left Control, Space and Esc together toggles a "disabled" state, during which every event is output unchanged. Rule groups and keyboard-specific rules are expanded in the config layer: each rule carries a `scope` condition (the group's and the keyboard's conditions), which must hold together with the rule's own `when`.
2. **Combos, layers, rules**: stateful. Tap/hold and simultaneous presses need to see later events before they can be decided. Events are queued during the decision and replayed in their original order afterwards.

### Decisions

At most one "decider" waits at a time:

- **Tap/hold**: looks at the events in the queue and decides by strategy. If the trigger key is released first, it is a tap. A timeout, another key going down under `eager`, or another key being fully pressed and released under `permissive` all count as hold.
- **Combo**: while later keys all belong to some candidate combo, it keeps waiting. When all keys are down and no longer candidate exists, it fires immediately. On any other event or a timeout, it looks for a combo that exactly matches the keys pressed so far, and if none exists, replays the first key as a regular key.

When the queue exceeds 32 events, a decision is forced to avoid waiting forever.

### Modifiers

The engine tracks modifiers itself and does not trust the flags reported by the system. Each held key can contribute three things:

- **Base modifiers**: physical modifiers, keys remapped to modifiers, and keys whose hold was decided as a modifier;
- **Temporarily removed**: when `cmd+k → ctrl+l` matches, the matched ⌘ is removed while K is held;
- **Temporarily added**: likewise, ⌃ is added.

Desired state = (base − removed) ∪ added. After every state change, `sync` compares the desired state with the state already emitted, and first sends the missing releases and then the missing presses. So the output of `cmd+k → ctrl+l` is "release ⌘, press ⌃, L, release ⌃, press ⌘ back".

### Hot path

Rules are compiled into key-indexed lookup tables at construction. The output buffer and event queue are preallocated and reused after clearing, so handling an event does not allocate, lock or do I/O. A `run` command only records an index in the output, and the command-line layer hands it to a background thread.

## macOS backend

A `CGEventTap` is installed at the session level (`kCGSessionEventTap`). All callbacks run on the main thread's run loop, so no locks are needed:

| Source | Role |
|---|---|
| Event tap | Key events → engine |
| Decision timer | Reset after each event according to `next_deadline`; drives timeout decisions |
| Config check timer | Once per second; reloads when the configuration changes |
| NSWorkspace notification | Frontmost app changes → `set_app` |
| TIS distributed notification | Input source switches → `set_input_source` |
| dispatch signal sources | SIGINT / SIGTERM / SIGHUP → stop the run loop and take the normal clean-up path |

### Emitting events

- When the engine output is exactly the original event (the most common pass-through), the original event is returned directly, with only the modifier flags corrected.
- Otherwise the original event is swallowed, a new event is synthesized with `CGEventCreateKeyboardEvent`, and it is posted to the session level with `CGEventPost`. Synthesized events are marked in the `kCGEventSourceUserData` field and pass straight through when they reach the event tap again.
- Mouse click, drag and scroll events also pass through the tap, and their flags are corrected to the engine's current modifier state, so remapped modifiers can be combined with the mouse. Mouse movement is not tapped, to avoid extra overhead.
- Modifiers are synthesized as `FlagsChanged` events. Arrow keys, function keys and so on carry the keypad / Fn flags that a real keyboard would, following its habits. The keyboard type of the most recent physical event is reused, which keeps characters correct under different layouts.

### Identifying keyboards

When the configuration uses `[device.*]`, the backend opens all keyboards read-only with IOHIDManager (not exclusive, needs the Input Monitoring permission), with one HID value queue per keyboard. In the event tap callback, all queues are first drained synchronously, and then events are paired with the raw records by HID key code and press/release. Raw values are written into the queue directly by the kernel, earlier than the events that reach the event tap after going through hidd and WindowServer, so the pairing does not depend on the dispatch order of notifications. When the system layer has changed the key mapping (such as modifier settings or Caps Lock → F18), raw key codes do not match; in that case, if only one keyboard has produced keys recently, it is assumed to be the source. Auto-repeat keeps the keyboard on which the key went down.

In each queued event, the engine records the bitmap of devices it matched, and when replaying after a decision, it evaluates conditions against the event's original device.

### Caps Lock

Caps Lock is toggled at the HID layer, and swallowing the event in `CGEventTap` cannot prevent it. When the configuration uses `caps_lock`:

1. At startup, `hidutil` maps Caps Lock to F18 (F18 has no locking function);
2. The backend recognizes the received F18 as Caps Lock and hands it to the engine;
3. The mapping is removed on exit.

Safeguards:

- If another program has already set a key mapping through `hidutil`, kanami does not overwrite it, reports the conflict, and passes Caps Lock through unchanged.
- Changes in "System Settings → Keyboard → Modifier Keys" are stored per keyboard in the global preferences and take effect before the `hidutil` mapping, so they are not visible in `hidutil`. kanami reads them at startup, and if Caps Lock is involved, it reports which key the Caps Lock rule actually acts on.
- On a crash or a forced kill (`kill -9`, closing a terminal tab that ends the whole process tree) there is no time to clean up. When the mapping is installed, a guard process `kanami __caps-guard` is started. It forks twice so that launchd adopts it and it is outside kanami's process tree, and blocks reading a pipe. As soon as kanami disappears, the pipe closes and the guard removes the mapping. On a normal exit, kanami removes the mapping itself, then closes the pipe and waits for the guard to exit, which ensures the guard cannot remove a mapping that is installed again later.
- Fallback: `kanami reset` removes it manually, and the next start of kanami also recognizes the mapping as its own.

### Known limitations

- Password fields turn on Secure Input, and the system then does not hand keys to the event tap.
- Synthesized events do not change the HID-layer modifier state, so a few apps that query the hardware modifier state directly may not see the modifiers kanami emits. The virtual HID device of M3 can solve this.
- Multiple keyboards cannot be told apart.

## Windows backend

`kanami-windows` intercepts keys with a low-level keyboard hook (`WH_KEYBOARD_LL`). All callbacks run in the message loop of one thread:

| Source | Role |
|---|---|
| Keyboard hook | Key events → engine |
| Thread timer (`SetTimer`) | Decision timeouts; checks the configuration and keyboard layout every second |
| `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` | Foreground window changes → program file name |
| `SetConsoleCtrlHandler` | Ctrl+C / console close → exit the message loop and send missing release events |

Differences from macOS:

- The low-level hook does not distinguish the first press from auto-repeat, so the backend records the physical key state to tell them apart.
- Modifier state is maintained by the key events themselves, so no flags need correcting on pass-through. Synthesized events are sent in one go with `SendInput`, and `dwExtraInfo` is marked to avoid processing them again.
- Swallowing Caps Lock prevents the caps toggle, so no HID-layer mapping is needed.
- Before a synthesized Win / Alt release, the unassigned key code `0xE8` is pressed once, which keeps the system from treating a lone press and release as opening the Start menu or activating the menu bar.
- Text is sent one UTF-16 unit at a time with `KEYEVENTF_UNICODE`.
- Known limitations: keys in windows of higher privilege (administrator) cannot be intercepted (UIPI); it does not work on secure desktops such as UAC and the lock screen; some games and anti-cheat software ignore injected input; `SetTimer` has a precision of about 15 ms.

The backend interface `Handler` is defined in `kanami-core::backend`. It has only types and no system calls, and the command-line layer connects both backends with the same code.

## Script plugins

The core engine does not depend on Lua. It defines only the extension interface `Hooks`:

- `on_key`: every logical key event is handed to the extension after `[remap]` and before the other rules. Down and repeat events can be swallowed by the extension, but **releases always finish through the normal path**: the extension can only observe them, which rules out stuck keys by construction.
- `call`: triggered by `call = "function_name"` in a rule.
- `timer`: a timer set by the extension with `Api::after` expires. Timers are driven by engine time like the tap/hold timeout, and `next_deadline` takes the earlier of the two, so tests can replay them deterministically.

In callbacks, the extension operates the engine through `Api`: tapping key combos (through the same modifier management), typing text, running commands, switching layers, and reading and writing variables. During a callback the engine takes the extension out of itself and puts it back afterwards, so no extra synchronization is needed.

`kanami-script` is the implementation based on Luau (mlua):

- Sandbox: built-in libraries are read-only, `require`, which could read the disk, is removed, and there is no file, network or process access.
- Time budget: Luau's interrupt callback checks a deadline, 20 ms per callback and 1 second when loading.
- API functions are registered once at load time. During a callback, `Api` is reached through a pointer valid only inside the callback, so no functions are created per key press.
- The runtime adds about 1.4 MB, and can be removed with the `script` build feature.

Text output (`kanami.type`) is sent on macOS with `CGEventKeyboardSetUnicodeString`, at most 20 UTF-16 units per event, independent of the keyboard layout and input method. Text events carry no modifiers, so they do not mix with shortcuts that are held down.

## Language

The display language is the config's `settings.language`, then the `KANAMI_LANG` environment variable, then the first supported OS language, then English. The tray relabels on reload.
