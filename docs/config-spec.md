# Configuration reference

> Applies to: M2. Fields marked "not yet supported" are rejected as unknown fields.
>
> [简体中文](zh-CN/config-spec.md)

This document defines the M1 configuration syntax. Design principles:

- **Common needs fit on one line.** Remaps, tap/hold, the Hyper key and layers need no nesting.
- **Predictable behavior.** Match order, priority and timing follow explicit rules.
- **Verifiable.** A configuration can be fully validated before startup, and rules can have tests.

## Overview

```toml
#:schema ./kanami.schema.json   # editor completion and validation, see section 9

[settings]          # global parameters
[remap]             # stage 1: physical key -> logical key
[[rule]]            # stage 2: combos, tap/hold, conditional rules
[layer.<name>]      # layers
[[combo]]           # several keys pressed together
[device.<name>]     # keyboards, used by when.device
[script]            # Luau script plugin, see scripting.md
[[test]]            # rule tests, run by `kanami test`
```

Every section is optional. An empty file is a valid configuration.

---

## 1. Key expressions

Every place that takes a key uses the same string syntax: `modifier+modifier+key`.

| Syntax | Meaning |
|---|---|
| `"a"`, `"escape"`, `"caps_lock"` | A single key. Key names are listed by `kanami keys`. |
| `"keypad_1"`, `"keypad_enter"`, `"num_lock"` | Numeric keypad (`keypad_0`-`keypad_9`, `keypad_period`, `keypad_plus`, `keypad_minus`, `keypad_multiply`, `keypad_divide`, `keypad_enter`, `keypad_equal`) |
| `"insert"`, `"print_screen"`, `"scroll_lock"`, `"pause"`, `"menu"` | PC keyboard keys. On macOS, Insert is the Help key, and Print Screen / Scroll Lock / Pause are treated by the system as F13-F15 and cannot be remapped on their own. |
| `"cmd+k"`, `"ctrl+shift+tab"` | Combo |
| `"hyper"` | All four modifiers ⌃⌥⇧⌘ pressed together |
| `"meh"` | Three modifiers ⌃⌥⇧ |
| `"hyper+t"` | Hyper plus T |
| `"none"` | Sends nothing (disables the key) |

Modifiers in a combo can use short names:

| Name | When matching | When sending |
|---|---|---|
| `ctrl` / `control` | Either left or right | Left |
| `opt` / `option` / `alt` | Either left or right | Left |
| `shift` | Either left or right | Left |
| `cmd` / `command` | Either left or right | Left |
| `fn` | Fn | Fn |
| `left_cmd`, `right_opt`, etc. | Only the given side | The given side |

To send a sequence of keys, use an array. The keys are sent in order:

```toml
to = ["cmd+a", "cmd+c"]
```

### Modifier matching

- **A bare key matches any modifiers**, and the existing modifiers are kept. With `a → b`, ⇧A becomes ⇧B.
- **A key with modifiers matches exactly.** `from = "cmd+k"` matches K with only ⌘ held; ⌘⇧K does not match.
- On a match, the modifiers written in `from` are **replaced** by the modifiers in `to`. `cmd+k → ctrl+l` sends ⌃L, not ⌘⌃L.

---

## 2. `[settings]`

```toml
[settings]
tap_timeout = 200     # milliseconds. Time limit for the tap/hold decision
combo_timeout = 50    # milliseconds. Window for deciding simultaneous presses
typing_streak = 150   # milliseconds. Typing-streak window; 0 turns it off
layer_hud = true      # after holding a layer key for about 0.4 s, show that layer's keys at the bottom of the screen (display only)
language = "en"       # "en" or "zh-CN". When omitted, kanami follows the system language (display only)
```

`typing_streak`: when less than this time has passed since the previous non-modifier key press, kanami treats the input as continuous typing. Tap/hold keys (including the `hold` key of a layer) are then treated as taps and sent immediately, and combos stop waiting for a decision. With a layer on the space bar or modifiers on letter keys, typing is neither delayed nor misfired. The cost is that after typing, a short pause is needed before the key can be held.

The three defaults come from common settings in similar tools. `tap_timeout` of 200 is the default in QMK and ZMK. `typing_streak` of 150 is a common value for QMK's Flow Tap and ZMK's `require-prior-idle-ms`; increase it for slower typing (rule of thumb: about 10500 ÷ words per minute). `combo_timeout` of 50 is Karabiner's simultaneous-press window.

`language` only changes the language of the text kanami displays. It does not affect key matching or any other behavior.

---

## 3. `[remap]`: stage-1 remaps

Used to fix the keyboard layout itself, for example swapping keys or turning Caps Lock into Ctrl. It runs first, and every later rule sees the remapped keys.

```toml
[remap]
caps_lock = "left_control"     # modifiers are supported since M1
right_command = "right_option"
grave = "escape"
# caps_lock = "hyper"          # or make it Hyper (a key can be written only once)
f1 = "none"
```

Rules:

- The left side must be a single key. The right side can be a single key, a modifier combination (such as `hyper`) or `none`.
- Conditions are not supported; a remap is always active. For per-app behavior, use `[[rule]]`.
- Modifiers are kept as described above: with `a = "b"`, ⇧A → ⇧B.
- `caps_lock` works as an output and toggles Caps Lock when pressed, like the real key. To swap Caps Lock and left Control, write both directions:

```toml
[remap]
caps_lock = "left_control"
left_control = "caps_lock"
```

---

## 4. `[[rule]]`: rules

Rules are matched in the order they appear in the file, and **the first matching rule wins**. Rules inside a group take part in the ordering at their position in the file.

Every rule accepts `enabled = false` to turn it off temporarily (the syntax is still checked).

### 4.1 Plain mapping

```toml
[[rule]]
from = "cmd+k"
to = "ctrl+l"
when.app = ["com.apple.Terminal", "com.googlecode.iterm2"]
```

### 4.2 Tap / hold

```toml
[[rule]]
from = "caps_lock"
tap = "escape"
hold = "left_control"
```

Only one of `to`, `run`, `call` and `tap`/`hold` can be used. `hold` can be a modifier, a modifier combination (such as `"hyper"`) or a regular key. Without `tap`, a tap sends `from` itself (except `caps_lock`, whose tap sends nothing).

The `from` of a tap/hold rule must be a single key without modifiers.

#### Decision strategy

The tap/hold distinction is the part of these tools that affects typing feel the most. `strategy` decides what happens when another key is pressed while the key is held:

```toml
[[rule]]
from = "caps_lock"
tap = "escape"
hold = "left_control"
strategy = "permissive"   # default
```

| Strategy | Decided as "hold" when | Suited for |
|---|---|---|
| `timeout` | The key is held longer than `tap_timeout` | Keys that are rarely used together with other keys |
| `permissive` (default) | The timeout passes, **or** another key is pressed and released completely before this key is released | Most cases; resists misfires during fast typing |
| `eager` | The timeout passes, **or** another key is pressed at all | Modifiers that need the fastest response, such as Caps → Ctrl |

Example: during fast typing the fingers overlap, giving the sequence `Space↓ B↓ Space↑ B↑`. With `permissive`, Space is released first, so it is decided as a tap and the output is "space b". With `eager`, it is wrongly decided as the layer key plus B. This is why `permissive` is the default.

A tap/hold decision delays the key's output until it is released. This is inherent to every tap/hold implementation and affects only keys with `tap`/`hold` configured. To have them sent immediately while typing, set [`typing_streak`](#2-settings).

### 4.3 Running a command

```toml
[[rule]]
from = "hyper+t"
run = "open -a Terminal"
```

`run` executes asynchronously on a background thread and does not block key handling. The trigger key is swallowed.

### 4.4 Calling a script

```toml
[[rule]]
from = "right_option"
call = "leader"
```

`call` invokes a global function defined in `[script]`, and the trigger key is swallowed. `[[combo]]` also accepts `call`. For writing scripts, see [scripting.md](scripting.md).

### 4.5 Conditions: `when`

| Field | Meaning |
|---|---|
| `app` | The frontmost app's bundle ID is in the list |
| `app_not` | The frontmost app is not in the list |
| `input_source` | The current input source ID is in the list, such as `com.apple.keylayout.ABC` or `com.apple.inputmethod.SCIM.ITABC` |
| `input_source_not` | The current input source is not in the list |
| `var` | A script variable equals the given value, such as `when.var = { mode = "vim" }`. The value can be a string, boolean or integer, and is compared as a string. |
| `device` | The key comes from one of the keyboards named in `[device.*]`, see "External keyboards" in section 6 |
| `device_not` | The key does not come from these keyboards |

Multiple fields in one `when` are combined with AND. The IDs of the frontmost app and the input source can be looked up with `kanami monitor`.

On Windows, `app` takes the program file name in lowercase, such as `code.exe`, and `input_source` takes the keyboard layout ID (8 hex digits), such as `08040804`.

### 4.6 Rule groups: `[[group]]`

A group shares conditions and an on/off switch among several rules. A rule is active only when both the group's `when` and the rule's own `when` match; `enabled = false` turns off the whole group.

```toml
[[group]]
name = "In terminals"           # group name shown in the UI
when.app = ["com.apple.Terminal", "com.googlecode.iterm2"]

[[group.rule]]
from = "cmd+k"
to = "ctrl+l"

[[group.rule]]
from = "cmd+d"
to = "ctrl+d"
```

---

## 5. `[layer.<name>]`: layers

```toml
[layer.nav]
hold = "space"          # hold Space to enter the layer; a tap is still Space
map = { h = "left", j = "down", k = "up", l = "right", u = "cmd+left", o = "cmd+right" }

[layer.num]
toggle = "hyper+n"      # press once to enter, press again to leave
map = { j = "1", k = "2", l = "3" }
```

Rules:

- Choose one activation method: `hold` (active while held; a tap sends the original key; `strategy` can be set) or `toggle` (on/off switch).
- If any modifier is held when the `hold` trigger key goes down, the layer is not entered and the trigger is treated as a regular key. This keeps ⌘Space (Spotlight) and ⌃Space (switch input source) working.
- The tap output of a `hold` layer is the trigger key itself by default. Set `tap = "..."` to change it.
- If a `hold` key is held longer than `tap_timeout` but released without any other key being pressed, it is still sent as a tap and not swallowed.
- When several layers are active, **the most recently activated layer has priority**.
- Keys not mapped in a layer **fall through** to the next layer and finally to the regular rules.
- Values in `map` use the same syntax as `to`: a combo, an array or `none`.

---

## 6. `[[combo]]`: simultaneous presses

```toml
[[combo]]
keys = ["j", "k"]
to = "escape"

[[combo]]
keys = ["s", "d", "f"]
run = "open -a Safari"
timeout = 30            # optional, overrides settings.combo_timeout
```

Rules:

- The combo fires when all keys in `keys` are pressed within the `combo_timeout` window, in any order. It sends one tap, and the releases of those keys are swallowed afterwards.
- When several combos share their first keys (such as `j k` and `j k l`), kanami waits until the window ends or the longer combo is complete, then decides.
- `keys` cannot contain modifiers and needs at least two keys.
- Cost: after a key that belongs to a combo goes down, its output waits for the window to end, adding up to `combo_timeout` of delay. Keys outside combos are not affected. When `typing_streak` is set, keys in a typing streak do not wait and are treated as regular keys.

### External keyboards

`[device.<name>]` describes a keyboard; every field written must match. Remaps, rules and groups specific to that keyboard are written directly under it:

```toml
[device.niz]
product = "NIZ"          # product name contains this text (case-insensitive)
# vendor_id = 0x05ac
# product_id = 0x0220
# inherit = false        # use only the remaps and rules here, not those for all keyboards (default true)

# This keyboard only: swap ⌥ and ⌘
[device.niz.remap]
left_option = "left_command"
left_command = "left_option"

[[device.niz.rule]]
from = "right_option"
to = "hyper"

[[device.niz.group]]
name = "In terminals"
when.app = ["com.apple.Terminal"]

[[device.niz.group.rule]]
from = "cmd+k"
to = "ctrl+l"
```

- A keyboard-specific remap is looked up before `[remap]`, and keyboard-specific rules are matched before the top-level `[[rule]]`, so they can override the general settings.
- With `inherit = false`, the keyboard no longer uses `[remap]` or the top-level rules (layers and `[[combo]]` are still shared).
- Rules and groups under `[device.*]` cannot also contain `when.device`.
- A keyboard name can also be written in the `when.device` / `when.device_not` of any rule. The rule then applies to those keyboards only, at its original position in the rule list.

`kanami devices` lists the ID and product name of connected keyboards. Some third-party keyboards report Apple's IDs; use `product` to tell them apart.

Identifying keyboards needs the Input Monitoring permission (macOS) and is turned on only when the configuration uses `[device.*]`. Windows does not support per-keyboard rules yet.

---

## 7. Match order

```
physical key
  │
  ▼
[device.*.remap]      stage 1: remaps specific to the current keyboard
[remap]               remaps for all keyboards (skipped for keyboards with inherit = false)
  │
  ▼
[[combo]]             simultaneous-press decision
  │
  ▼
active layers         starting from the most recently activated; unmapped keys fall through
  │
  ▼
layer trigger keys    hold / toggle of [layer.*]
  │
  ▼
[[device.*.rule]]     rules specific to the current keyboard (including groups), in file order
[[rule]]              rules for all keyboards (including groups), in file order; the first match wins
  │
  ▼
pass through unchanged
```

---

## 8. `[[test]]`: rule tests

The kanami core engine is deterministic, so key sequences can be replayed offline and the output checked against expectations. `kanami test` runs every test in the configuration and needs no system permissions.

```toml
[[test]]
name = "caps tap is esc"
input = "caps_lock"
expect = "escape"

[[test]]
name = "caps hold is ctrl"
input = "+caps_lock c -caps_lock"
expect = "ctrl+c"

[[test]]
name = "space layer: fast typing does not misfire"
input = "+space +b -space -b"
expect = "space b"

[[test]]
name = "long-pressing space enters the nav layer"
input = "+space @300 j -space"
expect = "down"
```

Sequence syntax:

| Syntax | Meaning |
|---|---|
| `a` | Press and release A |
| `+a` / `-a` | Press A / release A |
| `ctrl+c` | Press ⌃, tap C, release ⌃ |
| `@300` | Wait 300 milliseconds |
| `"hello"` | Text typed by a script with `kanami.type` (only in `expect`) |

`expect` describes the output with the same syntax (waits are not allowed). In `+x` / `-x`, x can also be a modifier name that stands for a single key, such as `ctrl` or `cmd`, which means the left-hand key.

At the end of a test, pending tap/hold decisions, combos and script timers are resolved as if they had timed out. Optional fields:

| Field | Meaning |
|---|---|
| `name` | Test name, shown on failure |
| `app` | Simulated frontmost app bundle ID, for testing `when.app` conditions |
| `input_source` | Simulated input source ID, for testing `when.input_source` conditions |
| `device` | Which keyboard from `[device.*]` the simulated keys come from |

Each test starts from a fresh state, and the script is reloaded. `run` actions do not appear in test output.

---

## 9. Error messages and hot reload

- Every error carries a line and column. A misspelled key name gets the closest suggestion, for example `config.toml:3:7: Unknown key name "esacpe"; the closest name is "escape"`.
- `kanami schema` prints a JSON Schema. Save it next to the configuration file and put `#:schema ./kanami.schema.json` on the first line of the configuration. Editors that support Taplo (such as Even Better TOML in VS Code) then complete key names and validate as you type:

  ```bash
  kanami schema > ~/.config/kanami/kanami.schema.json
  ```

- `kanami run` checks the configuration file once per second and reloads it after a save. If the new configuration has errors, kanami **keeps using the old configuration** and reports the error location.
- If any key is held during a reload, kanami first sends the missing release events and then switches rules, which avoids stuck modifiers.

## 10. Limits

| Item | Limit |
|---|---|
| Layers | 32 |
| `[[combo]]` | 64 |
| Key expressions in one array action | 32 |

---

## 11. Emergency stop

Hold **Left Control + Space + Esc** together to disable all remapping immediately: every key and mouse event passes through unchanged, and Caps Lock returns to its system behavior. Press the same keys again to resume. This chord is built into the engine and is not affected by the configuration, so it works even when the configuration is broken. It recognizes physical keys only; a Caps Lock remapped to Control does not count.

## 12. Script plugins

Needs that declarative configuration cannot cover are implemented with Luau scripts, see [scripting.md](scripting.md). Tap dance, the Leader key and text expansion ship as bundled plugins in the `plugins/` directory.

## 13. Not yet supported

- Remapping mouse buttons and the scroll wheel

For implementation details, see [architecture.md](architecture.md).
