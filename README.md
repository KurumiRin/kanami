<div align="center">

# kanami

**A small, fast, programmable keyboard remapper for macOS and Windows**

English · [简体中文](README.zh-CN.md)

</div>

> [!WARNING]
> kanami is in early development. macOS is ready for daily use. The Windows backend is implemented but has not been verified on real hardware. The config format and APIs may change.

kanami intercepts key events and passes, rewrites or drops them according to your rules. It starts with no remaps: every key behaves as usual until you add one.

- **Small**: one native background process with no drivers. The settings window is a separate program that only runs while it's open.
- **Fast**: rules compile into lookup tables at startup, and the key path never allocates or locks.
- **Programmable**: declarative config for everyday rules, and sandboxed [Luau](https://luau.org) plugins with a time budget for everything else.
- **Testable**: the engine is pure logic. Write tests in your config and replay them offline with `kanami test`.
- **Safe**: hold left Control + Space + Esc to disable all remapping instantly, and again to resume. It works even with a broken config.
- **Multilingual**: English and Simplified Chinese, following your system language.

## Install

kanami builds from source for now and needs Rust 1.90 or later. Prebuilt archives and a Homebrew tap arrive with the first release.

```bash
git clone https://github.com/KurumiRin/kanami.git
cd kanami
cargo build --release
```

The program is `target/release/kanami`. Pass `--no-default-features` to leave out scripting and get a smaller binary.

To build the settings window, see [apps/settings/README.md](apps/settings/README.md).

## Quick start

```bash
kanami run
```

macOS asks for **Accessibility** permission on first run. Grant it to the app that launches kanami, such as Terminal or iTerm2, and restart that app. Press `Ctrl+C` to quit, and all remaps stop immediately.

kanami creates an empty config at `~/.config/kanami/config.toml` and reloads it whenever you save. [config.example.toml](config.example.toml) has commented examples to copy from.

> [!NOTE]
> macOS tracks permissions by code signature, and kanami only has an ad hoc signature. After each upgrade, remove the old entry from Accessibility and add the new one.

## Run in the background

```bash
kanami service install     # start at login, restart after a crash
kanami service status      # state and recent logs
kanami service restart
kanami service uninstall
```

`install` copies the program to `~/Library/Application Support/kanami/`. Allow that copy under System Settings → Privacy & Security → Accessibility on first start. Logs go to `~/Library/Logs/kanami.log`.

If the config has an error, the service keeps running and passes every key through until the config is fixed. Only one kanami runs at a time, so uninstall the service before debugging in a terminal.

## Settings window

The menu bar icon opens a settings window for remaps, rules, layers, combos and per-keyboard setups. It edits the same config file and preserves your comments. The window also includes an event viewer, which shows the input and output of every key, and a config source editor with completion and version history.

Keyboards report key codes, not physical positions, so the keyboard map can differ from your hardware. Rules always use key codes, and pressing a key to pick it is always right. For the map itself, kanami asks when it sees a new keyboard and offers per-keyboard calibration.

## Configuration

The config file is `~/.config/kanami/config.toml`. Use `KANAMI_CONFIG` or `-c <path>` to choose another one.

```toml
[remap]
right_option = "hyper"          # right ⌥ becomes Hyper (⌃⌥⇧⌘)

[[rule]]                        # Caps Lock: Esc on tap, Ctrl on hold
from = "caps_lock"
tap = "escape"
hold = "left_control"

[[rule]]                        # Hyper+T opens Terminal
from = "hyper+t"
run = "open -a Terminal"

[[rule]]                        # Terminal only: ⌘K becomes ⌃L
from = "cmd+k"
to = "ctrl+l"
when.app = ["com.apple.Terminal"]

[layer.nav]                     # hold right ⌘: HJKL become arrow keys
hold = "right_command"
map = { h = "left", j = "down", k = "up", l = "right" }

[[combo]]                       # press D and F together for Esc
keys = ["d", "f"]
to = "escape"

[[test]]                        # checked by `kanami test`
input = "+caps_lock c -caps_lock"
expect = "ctrl+c"
```

The full syntax is in [docs/config-spec.md](docs/config-spec.md).

For completion and live validation in editors that support Taplo, such as VS Code with Even Better TOML, generate the schema and reference it on the first line of your config:

```bash
kanami schema > ~/.config/kanami/kanami.schema.json
```

```toml
#:schema ./kanami.schema.json
```

## Script plugins

Luau scripts cover what declarative rules can't:

```toml
[script]
files = ["plugins/leader.luau"]
source = '''
leader_map({
    ["g h"] = function() kanami.run("open https://github.com") end,
    ["s"] = function() kanami.type("Best regards") end,
})
'''

[[rule]]
from = "right_option"
call = "leader"
```

[plugins/](plugins/) includes a leader key, tap dance and text expansion. See [docs/scripting.md](docs/scripting.md) for the API and [examples/scripting.toml](examples/scripting.toml) for a working example.

## Commands

| Command | Description |
|---|---|
| `kanami run [-c <config>] [--tray] [--trace]` | Apply the config in the foreground. `--tray` shows the menu bar icon, and `--trace` prints each key's input and output |
| `kanami ctl status\|pause\|resume\|reload\|devices\|events\|quit` | Control a running kanami |
| `kanami monitor` | Listen only, printing key events and the frontmost app's bundle ID. Needs Input Monitoring |
| `kanami check [-c <config>]` | Validate a config |
| `kanami test [-c <config>]` | Run a config's `[[test]]` cases |
| `kanami keys` | List key names and modifier aliases |
| `kanami devices` | List connected keyboards for per-keyboard rules |
| `kanami schema` | Print the config's JSON Schema |
| `kanami reset` | Undo the Caps Lock mapping if kanami was killed and Caps Lock stopped working |
| `kanami service install\|uninstall\|status\|restart` | Manage the background service |

## Language

kanami follows your system language: the first one in your preferred list that it has a translation for, otherwise English. To choose one yourself, use Overview → Language in the settings window, or set it in the config:

```toml
[settings]
language = "zh-CN"              # or "en"
```

## Windows

Usage matches macOS, with these differences:

- The config file is `%APPDATA%\kanami\config.toml`.
- No permission is needed, but kanami can't intercept keys in windows running as administrator unless it runs as administrator too.
- `when.app` takes lowercase program names such as `["code.exe", "windowsterminal.exe"]`, and `when.input_source` takes a keyboard layout ID such as `"08040804"` (Microsoft Pinyin). `kanami monitor` shows both.
- There is no Fn key, and `service` and `reset` are macOS only.
- The settings window can't connect to the background service yet, because the named pipe isn't implemented. The layer HUD is macOS only.

## Known limitations

- `CGEventTap` doesn't see keys in password fields (Secure Input) or on the lock screen, and remaps don't apply in games and virtual machines. An optional HID mode is planned for these cases.
- Rules on `caps_lock` temporarily map Caps Lock to F18 in the system and restore it on exit. A watchdog restores it if kanami crashes or is killed. If Caps Lock still doesn't work, run `kanami reset`.
- If Caps Lock was changed under System Settings → Keyboard → Modifier Keys, the system's change applies first, so kanami's Caps Lock rules act on whichever key now produces Caps Lock. kanami warns about this at startup.

## Roadmap

- [x] Event tap, tap/hold, Hyper key, layers, combos, per-app rules, shell commands, hot reload, rule tests
- [x] Luau plugins, leader key, tap dance, text expansion, input source conditions
- [x] Per-keyboard rules (`[device.*]`)
- [x] Windows backend, pending real-hardware testing
- [x] Menu bar icon, settings window, event viewer, new-keyboard prompt, layer HUD
- [ ] Keyboard layout templates and KLE / QMK import
- [ ] Rule packs and a test panel
- [ ] Optional HID mode, using Karabiner's open-source virtual keyboard driver

The design is described in [docs/architecture.md](docs/architecture.md) and [docs/design/gui.md](docs/design/gui.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT), at your option. Unless you explicitly state otherwise, any contribution you submit is dual licensed as above, without additional terms.
