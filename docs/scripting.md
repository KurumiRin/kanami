# Script plugins

> [简体中文](zh-CN/scripting.md)

Needs that declarative configuration cannot cover are written as [Luau](https://luau.org) scripts. Luau is Roblox's open-source Lua dialect. Its syntax is largely the same as Lua 5.1, with additions such as type annotations and `+=`.

kanami ships three plugins that can be used directly and also serve as references for writing scripts:

| Plugin | Function |
|---|---|
| [`plugins/leader.luau`](../plugins/leader.luau) | Leader key: press the Leader, then a sequence of keys to trigger an action |
| [`plugins/tap_dance.luau`](../plugins/tap_dance.luau) | Tap dance: pressing the same key a different number of times triggers different actions |
| [`plugins/expand.luau`](../plugins/expand.luau) | Text expansion: typing an abbreviation replaces it with the full text |

A complete runnable example is in [`examples/scripting.toml`](../examples/scripting.toml). Run its tests with `kanami test -c examples/scripting.toml`.

## Loading scripts

```toml
[script]
files = ["plugins/leader.luau", "~/my.luau"]   # relative paths are relative to the directory of the config file
source = '''
leader_map({ ["g h"] = function() kanami.run("open https://github.com") end })
'''
```

Scripts run in order: `files` first, then `source`. Like the configuration file, script files reload automatically after a save.

## Calling from the configuration

`call` invokes a global function defined in a script. It works in `[[rule]]` and `[[combo]]`:

```toml
[[rule]]
from = "right_option"
call = "leader"

[[combo]]
keys = ["j", "k"]
call = "toggle_vim_mode"
```

`kanami check` verifies that every function named by `call` is defined.

## Controlling rules with variables

Variables set by a script with `kanami.set` can be used as rule conditions:

```toml
[[rule]]
from = "h"
to = "left"
when.var = { mode = "vim" }
```

```lua
function toggle_vim_mode()
    kanami.set("mode", kanami.get("mode") == "vim" and nil or "vim")
end
```

Variable values are always compared as strings. A variable that is not set counts as an empty string.

## API

All functions live in the global table `kanami`. Except for `on_key`, they can be called only inside callbacks; calling them while the script loads is an error.

### Listening for keys

```lua
kanami.on_key(function(e)
    -- e.key: key name, same as in the config, such as "a" or "left_command" (the key after [remap])
    -- e.state: "down", "up", "repeat"
    -- e.mods: { ctrl, opt, shift, cmd, fn }, the modifiers held at that moment
    return false   -- returning true means handled; kanami does not process this key any further
end)
```

- Every key event is passed to `on_key` after `[remap]` and before the other rules.
- Several handlers can be registered, and they are called in registration order. For down and repeat events, the first handler that returns `true` stops the chain.
- **Release events always finish through the normal path.** The return value is ignored and every handler receives them. A script bug cannot leave a key stuck.
- For a key whose down event returned `true`, the following repeats and the release are swallowed.

### Output

| Function | Description |
|---|---|
| `kanami.tap(expr, ...)` | Taps one or more key expressions in order, using the same syntax as the config: `kanami.tap("cmd+c", "cmd+v")` |
| `kanami.type(text)` | Types a piece of text. Any Unicode works, regardless of keyboard layout and input source. |
| `kanami.run(command)` | Runs a shell command in the background |

### Timers

| Function | Description |
|---|---|
| `kanami.after(ms, fn)` | Calls `fn` after `ms` milliseconds and returns a timer ID |
| `kanami.cancel(id)` | Cancels a timer |

Timers are driven by kanami's engine time, so `[[test]]` can replay them deterministically.

### Layers and variables

| Function | Description |
|---|---|
| `kanami.layer(name, on)` | Turns a layer on or off. `on` defaults to `true`. It is an error if the layer does not exist. |
| `kanami.layer_active(name)` | Whether the layer is active |
| `kanami.set(name, value)` | Sets a variable; a `nil` value deletes it |
| `kanami.get(name)` | Reads a variable; `nil` if it is not set |

### Environment

| Function | Description |
|---|---|
| `kanami.app()` | Bundle ID of the frontmost app |
| `kanami.input_source()` | Current input source ID, such as `com.apple.keylayout.ABC` |
| `kanami.mods()` | Modifiers currently held, in the same format as `e.mods` |
| `kanami.now()` | Engine time in milliseconds |
| `kanami.log(...)` | Writes a log line: printed to the terminal when run in a terminal, written to the log file by the background service |

`kanami monitor` shows the frontmost app and input source IDs.

## Safety and performance

- **Sandbox**: Luau sandbox mode, with read-only built-in libraries. There is no file, network or process access, and `require` is removed. Commands can only be run through `kanami.run`.
- **Time budget**: each callback may run at most 20 ms, and loading at most 1 second. A script that exceeds it is aborted and logged, and the key is handled as if there were no script.
- **Errors do not affect typing**: a runtime error in a script is written to the log, and the key is handled as if there were no script.
- **Size**: the script runtime adds about 1.4 MB. `cargo build --release --no-default-features` builds a version without scripting (about 1.3 MB).

## Testing configurations with scripts

`[[test]]` works as usual. In expected output, double quotes mark text typed by `kanami.type`, and the `input_source` field simulates an input source:

```toml
[[test]]
name = "typing ;sig expands to a signature"
input = "semicolon s i g"
expect = 'semicolon s i backspace backspace backspace "Best regards"'

[[test]]
name = "no expansion under a Chinese input source"
input_source = "com.apple.inputmethod.SCIM.ITABC"
input = "semicolon s i g"
expect = "semicolon s i g"
```
