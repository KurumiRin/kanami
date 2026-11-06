<div align="center">

# kanami

**小而快、可编程的键盘改键工具，支持 macOS 和 Windows**

[English](README.md) · 简体中文

</div>

> [!WARNING]
> kanami 处于早期开发阶段。macOS 可以日常使用；Windows 后端已经实现，但还没有在真机上验证。配置格式和 API 都可能变化。

kanami 拦截按键事件，按你的规则放行、改写或吞掉。初始不带任何改键：在你添加之前，所有按键都和平时一样。

- **小**：一个原生后台进程，不装驱动。设置窗口是单独的程序，只在打开时运行。
- **快**：规则在启动时编译成查找表，按键路径上不分配内存、不加锁。
- **可编程**：日常规则写声明式配置，其余的写 [Luau](https://luau.org) 插件，沙箱运行、有时间预算。
- **可测试**：引擎是纯逻辑。配置里可以直接写测试，用 `kanami test` 离线回放。
- **安全**：同时按住左 Control + 空格 + Esc 立即停用全部改键，再按一次恢复；配置写坏了也能用。
- **多语言**：支持英文和简体中文，跟随系统语言。

## 安装

目前从源码构建，要求 Rust 1.90 及以上。预编译版本和 Homebrew tap 随首个版本发布。

```bash
git clone https://github.com/KurumiRin/kanami.git
cd kanami
cargo build --release
```

生成的程序是 `target/release/kanami`。加 `--no-default-features` 可以去掉脚本功能，体积更小。

构建设置窗口见 [apps/settings/README.md](apps/settings/README.md)（英文）。

## 快速开始

```bash
kanami run
```

第一次运行时 macOS 会请求**辅助功能**权限。权限要授予启动 kanami 的 App（比如 Terminal、iTerm2），授权后重启该 App。按 `Ctrl+C` 退出，所有改键立即失效。

kanami 会创建一份空配置 `~/.config/kanami/config.toml`，保存后自动重载。[config.example.toml](config.example.toml) 里有带说明的示例，可以按需复制。

> [!NOTE]
> macOS 按程序签名识别权限，而 kanami 只有临时签名。每次升级后，都要在“辅助功能”里移除旧条目再添加新的。

## 后台运行

```bash
kanami service install     # 登录后自动启动，崩溃后自动拉起
kanami service status      # 状态和最近的日志
kanami service restart
kanami service uninstall
```

`install` 会把程序复制到 `~/Library/Application Support/kanami/`。第一次启动时，在“系统设置 → 隐私与安全性 → 辅助功能”里允许这份程序。日志在 `~/Library/Logs/kanami.log`。

配置有误时服务不会退出，而是原样放行所有按键，直到配置改好。同一时间只能运行一个 kanami，在终端里调试前请先卸载服务。

## 设置窗口

从菜单栏图标可以打开设置窗口，管理改键、规则、图层、组合键和按键盘区分的设置。它编辑的是同一份配置文件，并保留你写的注释。窗口里还有事件查看器（显示每个按键的输入和输出）和配置源码编辑器（带补全和历史版本）。

键盘上报的是键码，不是物理位置，所以键盘图可能和你的硬件不一致。规则始终按键码生效，直接按键来选择总是对的。至于键盘图本身，kanami 发现新键盘时会询问，也可以按键盘单独校准。

## 配置

配置文件是 `~/.config/kanami/config.toml`，也可以用 `KANAMI_CONFIG` 或 `-c <路径>` 指定。

```toml
[remap]
right_option = "hyper"          # 右 ⌥ 当 Hyper（⌃⌥⇧⌘）

[[rule]]                        # Caps Lock：单按 Esc，按住 Ctrl
from = "caps_lock"
tap = "escape"
hold = "left_control"

[[rule]]                        # Hyper+T 打开终端
from = "hyper+t"
run = "open -a Terminal"

[[rule]]                        # 只在终端里：⌘K → ⌃L
from = "cmd+k"
to = "ctrl+l"
when.app = ["com.apple.Terminal"]

[layer.nav]                     # 按住右 ⌘：HJKL 变方向键
hold = "right_command"
map = { h = "left", j = "down", k = "up", l = "right" }

[[combo]]                       # 同时按 D F 当 Esc
keys = ["d", "f"]
to = "escape"

[[test]]                        # 由 kanami test 检查
input = "+caps_lock c -caps_lock"
expect = "ctrl+c"
```

完整语法见 [docs/zh-CN/config-spec.md](docs/zh-CN/config-spec.md)。

在支持 Taplo 的编辑器里（如 VS Code 的 Even Better TOML），可以生成 Schema 并在配置第一行引用，获得补全和实时校验：

```bash
kanami schema > ~/.config/kanami/kanami.schema.json
```

```toml
#:schema ./kanami.schema.json
```

## 脚本插件

声明式规则做不到的，用 Luau 脚本写：

```toml
[script]
files = ["plugins/leader.luau"]
source = '''
leader_map({
    ["g h"] = function() kanami.run("open https://github.com") end,
    ["s"] = function() kanami.type("此致敬礼") end,
})
'''

[[rule]]
from = "right_option"
call = "leader"
```

[plugins/](plugins/) 里有 Leader 键、tap dance 和文本展开。API 见 [docs/zh-CN/scripting.md](docs/zh-CN/scripting.md)，可运行的示例见 [examples/scripting.toml](examples/scripting.toml)。

## 命令

| 命令 | 说明 |
|---|---|
| `kanami run [-c <配置>] [--tray] [--trace]` | 前台按配置改键。`--tray` 显示菜单栏图标，`--trace` 打印每个按键的输入和输出 |
| `kanami ctl status\|pause\|resume\|reload\|devices\|events\|quit` | 控制正在运行的 kanami |
| `kanami monitor` | 只监听，打印按键事件和前台 App 的 Bundle ID，需要“输入监控”权限 |
| `kanami check [-c <配置>]` | 校验配置 |
| `kanami test [-c <配置>]` | 运行配置里的 `[[test]]` |
| `kanami keys` | 列出键名和修饰键写法 |
| `kanami devices` | 列出已连接的键盘，用于按键盘区分规则 |
| `kanami schema` | 输出配置的 JSON Schema |
| `kanami reset` | 撤销 Caps Lock 映射，用于 kanami 被强杀后 Caps Lock 失灵的情况 |
| `kanami service install\|uninstall\|status\|restart` | 管理后台服务 |

## 语言

kanami 跟随系统语言：取首选语言列表里第一个有翻译的语言，都没有时用英文。也可以自己选择：在设置窗口的“概览 → 语言”里切换，或者写在配置里：

```toml
[settings]
language = "zh-CN"              # 或 "en"
```

## Windows

用法与 macOS 相同，有以下区别：

- 配置文件在 `%APPDATA%\kanami\config.toml`。
- 不需要授权，但拦截不到以管理员身份运行的窗口里的按键，除非 kanami 也以管理员身份运行。
- `when.app` 写小写的程序文件名，如 `["code.exe", "windowsterminal.exe"]`；`when.input_source` 写键盘布局编号，如 `"08040804"`（微软拼音）。两者都可以用 `kanami monitor` 查看。
- 没有 Fn 键，`service` 和 `reset` 只在 macOS 上提供。
- 命名管道还没有实现，设置窗口暂时连不上后台服务。图层浮窗只在 macOS 上提供。

## 已知限制

- `CGEventTap` 收不到密码框（Secure Input）和锁屏界面的按键，游戏和虚拟机里改键也不生效。计划用可选的 HID 模式解决。
- 用到 `caps_lock` 时，kanami 会临时在系统里把 Caps Lock 映射成 F18，退出时恢复。kanami 崩溃或被强杀时由守护进程恢复；如果 Caps Lock 仍然失灵，运行 `kanami reset`。
- 如果在“系统设置 → 键盘 → 修饰键”里改过 Caps Lock，系统的改动先生效，kanami 的 Caps Lock 规则会作用在被改成 Caps Lock 的那个键上。kanami 启动时会提示。

## 路线图

- [x] 事件拦截、单按/按住、Hyper 键、图层、组合键、按 App 生效、运行命令、热重载、规则测试
- [x] Luau 插件、Leader 键、tap dance、文本展开、按输入法生效
- [x] 按键盘区分规则（`[device.*]`）
- [x] Windows 后端（待真机验证）
- [x] 菜单栏图标、设置窗口、事件查看器、新键盘提示、图层浮窗
- [ ] 键盘配列模板和 KLE / QMK 导入
- [ ] 规则包和测试面板
- [ ] 可选的 HID 模式，借助 Karabiner 开源虚拟键盘驱动

设计说明见 [docs/architecture.md](docs/architecture.md) 和 [docs/design/gui.md](docs/design/gui.md)（英文）。

## 参与贡献

见 [CONTRIBUTING.md](CONTRIBUTING.md)（英文）。

## 许可证

可在 [Apache License 2.0](LICENSE-APACHE) 和 [MIT License](LICENSE-MIT) 中任选其一。除非你明确声明，否则你提交的贡献都按上述双许可证发布，不附加其他条款。
