# 脚本插件

> English: [../scripting.md](../scripting.md)

声明式配置覆盖不了的需求，用 [Luau](https://luau.org) 脚本来写。Luau 是 Roblox 开源的 Lua 方言，语法和 Lua 5.1 基本相同，额外有类型标注、`+=` 等语法。

kanami 附带三个插件，既能直接用，也是写脚本的参考：

| 插件 | 功能 |
|---|---|
| [`plugins/leader.luau`](../../plugins/leader.luau) | Leader 键：先按 Leader，再依次按几个键触发动作 |
| [`plugins/tap_dance.luau`](../../plugins/tap_dance.luau) | Tap dance：同一个键连按不同次数，触发不同动作 |
| [`plugins/expand.luau`](../../plugins/expand.luau) | 文本展开：输入缩写后自动替换成完整文字 |

完整的可运行示例见 [`examples/scripting.toml`](../../examples/scripting.toml)，用 `kanami test -c examples/scripting.toml` 运行其中的测试。

## 加载脚本

```toml
[script]
files = ["plugins/leader.luau", "~/my.luau"]   # 相对路径相对于配置文件所在目录
source = '''
leader_map({ ["g h"] = function() kanami.run("open https://github.com") end })
'''
```

按顺序执行：先 `files`，后 `source`。脚本文件和配置文件一样，保存后自动重载。

## 从配置里调用

`call` 调用脚本里的全局函数，可以用在 `[[rule]]` 和 `[[combo]]` 里：

```toml
[[rule]]
from = "right_option"
call = "leader"

[[combo]]
keys = ["j", "k"]
call = "toggle_vim_mode"
```

`kanami check` 会检查 `call` 的函数是否都已定义。

## 用变量控制规则

脚本用 `kanami.set` 设置的变量可以作为规则条件：

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

变量值一律按字符串比较；没设置的变量视为空字符串。

## API

所有函数都在全局表 `kanami` 里。除 `on_key` 外，都只能在回调里调用（加载脚本时调用会报错）。

### 监听按键

```lua
kanami.on_key(function(e)
    -- e.key：键名，与配置里相同，如 "a"、"left_command"（[remap] 之后的键）
    -- e.state："down"、"up"、"repeat"
    -- e.mods：{ ctrl, opt, shift, cmd, fn }，当时按着的修饰键
    return false   -- 返回 true 表示已处理，kanami 不再处理这个键
end)
```

- 每个按键事件在 `[remap]` 之后、其他规则之前交给 `on_key`。
- 可以注册多个，按注册顺序调用。按下和重复事件遇到第一个返回 `true` 的就停止。
- **松开事件总是按正常流程收尾**，返回值被忽略，所有处理函数都会收到。脚本不会因为写错而让某个键卡住。
- 按下时返回 `true` 的键，之后的重复和松开都会被吞掉。

### 输出

| 函数 | 说明 |
|---|---|
| `kanami.tap(expr, ...)` | 依次单按一个或多个按键表达式，写法与配置相同：`kanami.tap("cmd+c", "cmd+v")` |
| `kanami.type(text)` | 输入一段文字，任意 Unicode，不受键盘布局和输入法影响 |
| `kanami.run(command)` | 在后台执行 shell 命令 |

### 定时器

| 函数 | 说明 |
|---|---|
| `kanami.after(ms, fn)` | `ms` 毫秒后调用 `fn`，返回定时器编号 |
| `kanami.cancel(id)` | 取消定时器 |

定时器由 kanami 的引擎时间驱动，所以在 `[[test]]` 里也能确定地回放。

### 图层与变量

| 函数 | 说明 |
|---|---|
| `kanami.layer(name, on)` | 开关图层，`on` 省略时为 `true`；图层不存在时报错 |
| `kanami.layer_active(name)` | 图层是否激活 |
| `kanami.set(name, value)` | 设置变量，`value` 为 `nil` 时删除 |
| `kanami.get(name)` | 读取变量，未设置时为 `nil` |

### 环境

| 函数 | 说明 |
|---|---|
| `kanami.app()` | 前台 App 的 Bundle ID |
| `kanami.input_source()` | 当前输入法 ID，如 `com.apple.keylayout.ABC` |
| `kanami.mods()` | 当前按着的修饰键，格式同 `e.mods` |
| `kanami.now()` | 引擎时间（毫秒） |
| `kanami.log(...)` | 写日志：终端运行时打印到终端，后台服务写到日志文件 |

用 `kanami monitor` 可以看到前台 App 和输入法的 ID。

## 安全与性能

- **沙箱**：Luau 沙箱模式，内置库只读；没有文件、网络和进程访问，`require` 也被移除。需要执行命令时只能用 `kanami.run`。
- **时间预算**：每次回调最多执行 20 毫秒，加载时最多 1 秒，超时会被中止并写入日志，按键按原样处理。
- **出错不影响按键**：脚本运行时出错会写入日志，这次按键按没有脚本的情况处理。
- **体积**：脚本运行时约增加 1.4 MB。用 `cargo build --release --no-default-features` 可以编译不带脚本的版本（约 1.3 MB）。

## 测试带脚本的配置

`[[test]]` 照常可用。期望输出里用双引号表示 `kanami.type` 输入的文字，`input_source` 字段可以模拟输入法：

```toml
[[test]]
name = "输入 ;sig 展开成签名"
input = "semicolon s i g"
expect = 'semicolon s i backspace backspace backspace "此致敬礼"'

[[test]]
name = "中文输入法下不展开"
input_source = "com.apple.inputmethod.SCIM.ITABC"
input = "semicolon s i g"
expect = "semicolon s i g"
```
