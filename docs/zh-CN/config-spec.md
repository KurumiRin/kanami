# 配置规范

> English: [../config-spec.md](../config-spec.md)

> 适用版本：M2。标注“尚未支持”的字段会被当作未知字段报错。

本文定义 M1 的配置语法。设计原则：

- **常见需求一行写完**：改键、单按/按住、Hyper 键、图层都不需要嵌套。
- **行为可预测**：匹配顺序、优先级和时序都有明确规则，不靠猜。
- **可验证**：配置可以在启动前完整校验，规则可以写测试。

## 总览

```toml
#:schema ./kanami.schema.json   # 编辑器补全与校验，见第 9 节

[settings]          # 全局参数
[remap]             # 第一阶段：物理键 → 逻辑键
[[rule]]            # 第二阶段：组合键、单按/按住、条件规则
[layer.<名字>]      # 图层
[[combo]]           # 同时按下多个键
[device.<名字>]     # 键盘，用于 when.device
[script]            # Luau 脚本插件，见 scripting.md
[[test]]            # 规则测试，`kanami test` 运行
```

所有部分都是可选的，空文件也是合法配置。

---

## 1. 按键表达式

所有出现按键的地方都用同一种字符串写法：`修饰键+修饰键+键`。

| 写法 | 含义 |
|---|---|
| `"a"`、`"escape"`、`"caps_lock"` | 单个键，键名见 `kanami keys` |
| `"keypad_1"`、`"keypad_enter"`、`"num_lock"` | 小键盘（`keypad_0`–`keypad_9`、`keypad_period`、`keypad_plus`、`keypad_minus`、`keypad_multiply`、`keypad_divide`、`keypad_enter`、`keypad_equal`） |
| `"insert"`、`"print_screen"`、`"scroll_lock"`、`"pause"`、`"menu"` | PC 键盘的功能键；macOS 上 Insert 是 Help 键，Print Screen / Scroll Lock / Pause 会被系统当成 F13–F15，不能单独改 |
| `"cmd+k"`、`"ctrl+shift+tab"` | 组合键 |
| `"hyper"` | ⌃⌥⇧⌘ 四个修饰键同时按下 |
| `"meh"` | ⌃⌥⇧ 三个修饰键 |
| `"hyper+t"` | Hyper 加 T |
| `"none"` | 什么都不发（禁用） |

组合键里的修饰键可以用简写：

| 简写 | 匹配时 | 发出时 |
|---|---|---|
| `ctrl` / `control` | 左右任一 | 左侧 |
| `opt` / `option` / `alt` | 左右任一 | 左侧 |
| `shift` | 左右任一 | 左侧 |
| `cmd` / `command` | 左右任一 | 左侧 |
| `fn` | Fn | Fn |
| `left_cmd`、`right_opt` 等 | 只匹配指定一侧 | 指定一侧 |

需要发出一串按键时用数组，按顺序发出：

```toml
to = ["cmd+a", "cmd+c"]
```

### 修饰键的匹配规则

- **裸键匹配任意修饰键**，原有修饰键保留。`a → b` 时 ⇧A 变成 ⇧B。
- **带修饰键时精确匹配**。`from = "cmd+k"` 只匹配“只按着 ⌘”的 K；⌘⇧K 不会命中。
- 命中后，`from` 里写的修饰键被**替换**为 `to` 里的修饰键。`cmd+k → ctrl+l` 发出的是 ⌃L，不是 ⌘⌃L。

---

## 2. `[settings]`

```toml
[settings]
tap_timeout = 200     # 毫秒。单按/按住的判定时限
combo_timeout = 50    # 毫秒。同时按下的判定窗口
typing_streak = 150   # 毫秒。打字连击窗口，0 表示关闭
layer_hud = true      # 按住图层键约 0.4 秒后，在屏幕底部显示这一层的键位（只影响显示）
language = "zh-CN"    # "en" 或 "zh-CN"。不写时跟随系统语言（只影响显示）
```

`typing_streak`：距上一个非修饰键按下不到这么久时，认为正在连续打字。这时单按/按住键（包括图层的 `hold` 键）直接当单按、立即发出，组合键也不再等待判定。空格当图层键、字母当修饰键时，打字既不延迟也不会误触；代价是打完字要停顿一下才能按住它们。

三个默认值都取自同类工具的常用设置：`tap_timeout` 200 是 QMK、ZMK 的默认值；`typing_streak` 150 是 QMK 的 Flow Tap 和 ZMK 的 `require-prior-idle-ms` 的常用值，打字慢的话调大（经验值约 10500 ÷ 每分钟字数）；`combo_timeout` 50 是 Karabiner 的同时按下窗口。

`language` 只改变 kanami 显示的文字的语言，不影响按键匹配和其他行为。

---

## 3. `[remap]`：第一阶段改键

用来修正键盘布局本身，比如交换按键、把 Caps Lock 变成 Ctrl。它最先执行，后面所有规则看到的都是改过以后的键。

```toml
[remap]
caps_lock = "left_control"     # M1 起支持修饰键
right_command = "right_option"
grave = "escape"
# caps_lock = "hyper"          # 或者改成 Hyper（同一个键只能写一次）
f1 = "none"
```

规则：

- 左边只能是单个键；右边可以是单个键、修饰键组合（如 `hyper`）或 `none`。
- 不支持条件，永远生效。要按 App 生效请用 `[[rule]]`。
- 修饰键保留规则同上：`a = "b"` 时 ⇧A → ⇧B。

---

## 4. `[[rule]]`：规则

按文件里的书写顺序匹配，**第一条命中的规则生效**，后面的不再看。规则组里的规则也按它们在文件里的位置参与排序。

每条规则都可以写 `enabled = false` 临时关掉（写法仍会检查）。

### 4.1 普通映射

```toml
[[rule]]
from = "cmd+k"
to = "ctrl+l"
when.app = ["com.apple.Terminal", "com.googlecode.iterm2"]
```

### 4.2 单按 / 按住

```toml
[[rule]]
from = "caps_lock"
tap = "escape"
hold = "left_control"
```

`to`、`run`、`call`、`tap`/`hold` 只能写一种。`hold` 可以是修饰键、修饰键组合（如 `"hyper"`）或普通键。不写 `tap` 时，单按发出 `from` 本身（`caps_lock` 例外：单按什么都不发）。

tap/hold 规则的 `from` 只能是单个键，不能带修饰键。

#### 判定策略

单按和按住的区分是这类工具最影响手感的地方。`strategy` 决定按住期间又按了别的键时怎么判定：

```toml
[[rule]]
from = "caps_lock"
tap = "escape"
hold = "left_control"
strategy = "permissive"   # 默认值
```

| 策略 | 判定为“按住”的条件 | 适合 |
|---|---|---|
| `timeout` | 只看按住时长是否超过 `tap_timeout` | 很少和其他键连用的键 |
| `permissive`（默认） | 超时；**或者**在它松开之前，另一个键完整地按下并松开了 | 大多数场景，打字快时不容易误触 |
| `eager` | 超时；**或者**另一个键一按下就算 | 当修饰键用、追求最快响应，比如 Caps → Ctrl |

例子：快速打字时手指重叠，序列是 `空格↓ B↓ 空格↑ B↑`。用 `permissive` 时空格先松开，判为单按，输出“空格 B”；用 `eager` 会误判成图层键加 B。所以默认用 `permissive`。

单按判定会让该键的输出延后到松开时，这是所有单按/按住实现的固有代价，只影响配置了 `tap`/`hold` 的键。打字时想要它们立即发出，设置 [`typing_streak`](#2-settings)。

### 4.3 运行命令

```toml
[[rule]]
from = "hyper+t"
run = "open -a Terminal"
```

`run` 在后台线程异步执行，不阻塞按键。触发键本身被吞掉。

### 4.4 调用脚本

```toml
[[rule]]
from = "right_option"
call = "leader"
```

`call` 调用 `[script]` 里定义的全局函数，触发键本身被吞掉。`[[combo]]` 也可以写 `call`。脚本的写法见 [scripting.md](scripting.md)。

### 4.5 条件 `when`

| 字段 | 含义 |
|---|---|
| `app` | 前台 App 的 Bundle ID 在列表中 |
| `app_not` | 前台 App 不在列表中 |
| `input_source` | 当前输入法 ID 在列表中，如 `com.apple.keylayout.ABC`、`com.apple.inputmethod.SCIM.ITABC` |
| `input_source_not` | 当前输入法不在列表中 |
| `var` | 脚本变量等于给定值，如 `when.var = { mode = "vim" }`；值可以是字符串、布尔值或整数，按字符串比较 |
| `device` | 按键来自 `[device.*]` 里这些名字的键盘之一，见第 6 节“外接键盘” |
| `device_not` | 按键不来自这些键盘 |

同一个 `when` 里的多个字段是“且”的关系。前台 App 和输入法的 ID 都可以用 `kanami monitor` 查看。

在 Windows 上，`app` 写程序文件名（小写），如 `code.exe`；`input_source` 写键盘布局编号（8 位十六进制），如 `08040804`。

### 4.6 规则组 `[[group]]`

共用条件和开关的一组规则。组的 `when` 和组里每条规则自己的 `when` 同时满足才生效；`enabled = false` 关掉整组。

```toml
[[group]]
name = "终端里"                 # 界面里显示的组名
when.app = ["com.apple.Terminal", "com.googlecode.iterm2"]

[[group.rule]]
from = "cmd+k"
to = "ctrl+l"

[[group.rule]]
from = "cmd+d"
to = "ctrl+d"
```

---

## 5. `[layer.<名字>]`：图层

```toml
[layer.nav]
hold = "space"          # 按住空格进入图层；单按仍然是空格
map = { h = "left", j = "down", k = "up", l = "right", u = "cmd+left", o = "cmd+right" }

[layer.num]
toggle = "hyper+n"      # 按一次进入，再按一次退出
map = { j = "1", k = "2", l = "3" }
```

规则：

- 激活方式二选一：`hold`（按住期间生效，单按发出原键，可配 `strategy`）或 `toggle`（开关）。
- 按下 `hold` 触发键时如果按着任何修饰键，不进入图层，触发键直接当普通键。这样 ⌘空格（Spotlight）、⌃空格（切换输入法）不受影响。
- `hold` 图层的单按输出默认是触发键本身，可以用 `tap = "..."` 改成别的。
- 按住 `hold` 键超过 `tap_timeout`、但期间没按任何别的键就松开，仍然当单按发出，不会被吞掉。
- 多个图层同时激活时，**后激活的优先**。
- 图层里没映射的键会**穿透**到下一层，最后回到普通规则。
- `map` 的值写法和 `to` 一样，可以是组合键、数组或 `none`。

---

## 6. `[[combo]]`：同时按下

```toml
[[combo]]
keys = ["j", "k"]
to = "escape"

[[combo]]
keys = ["s", "d", "f"]
run = "open -a Safari"
timeout = 30            # 可选，覆盖 settings.combo_timeout
```

规则：

- 在 `combo_timeout` 窗口内按下 `keys` 里的全部键（顺序不限）即触发，发出一次单按；之后这些键的松开都被吞掉。
- 有多个组合共用前几个键时（如 `j k` 和 `j k l`），会等到窗口结束或按全较长的那个再决定。
- `keys` 里不能有修饰键，至少两个键。
- 代价：参与组合的键按下后要等窗口结束才能确定输出，会多出最多 `combo_timeout` 的延迟。不参与组合的键不受影响。设置了 `typing_streak` 时，连续打字中的键不等待，直接当普通键。

### 外接键盘

`[device.<名字>]` 描述一把键盘，写了的字段都要匹配。这把键盘专用的改键、规则和规则组直接写在它下面：

```toml
[device.niz]
product = "NIZ"          # 产品名包含这段文字（不区分大小写）
# vendor_id = 0x05ac
# product_id = 0x0220
# inherit = false        # 只用这里的改键和规则，不用对所有键盘生效的那些（默认 true）

# 只对这把键盘：交换 ⌥ 和 ⌘
[device.niz.remap]
left_option = "left_command"
left_command = "left_option"

[[device.niz.rule]]
from = "right_option"
to = "hyper"

[[device.niz.group]]
name = "终端里"
when.app = ["com.apple.Terminal"]

[[device.niz.group.rule]]
from = "cmd+k"
to = "ctrl+l"
```

- 键盘专用的改键先于 `[remap]` 查找，专用规则先于顶层的 `[[rule]]` 匹配，所以可以覆盖通用的设置。
- `inherit = false` 时，这把键盘不再使用 `[remap]` 和顶层的规则（图层和 `[[combo]]` 仍然共用）。
- `[device.*]` 下面的规则和组里不能再写 `when.device`。
- 也可以在任意规则的 `when.device` / `when.device_not` 里写键盘名，效果是只对这些键盘生效（排在规则列表里原来的位置）。

`kanami devices` 列出已连接键盘的 ID 和产品名。有些第三方键盘会冒用 Apple 的 ID，这时用 `product` 区分。

识别键盘需要“输入监控”权限（macOS），只有配置里用到 `[device.*]` 时才会开启。Windows 暂不支持按键盘区分。

---

## 7. 匹配顺序

```
物理按键
  │
  ▼
[device.*.remap]      第一阶段：当前键盘的专用改键
[remap]               对所有键盘的改键（inherit = false 的键盘跳过）
  │
  ▼
[[combo]]             同时按下判定
  │
  ▼
已激活的图层          从最后激活的开始；没映射就往下穿透
  │
  ▼
图层触发键            [layer.*] 的 hold / toggle
  │
  ▼
[[device.*.rule]]     当前键盘的专用规则（含规则组），按书写顺序
[[rule]]              对所有键盘的规则（含规则组），按书写顺序，第一条命中生效
  │
  ▼
原样放行
```

---

## 8. `[[test]]`：规则测试

kanami 的核心引擎是确定性的，可以离线回放按键序列，检查输出是否符合预期。`kanami test` 运行配置里的所有测试，不需要任何系统权限。

```toml
[[test]]
name = "caps 单按是 esc"
input = "caps_lock"
expect = "escape"

[[test]]
name = "caps 按住是 ctrl"
input = "+caps_lock c -caps_lock"
expect = "ctrl+c"

[[test]]
name = "空格层：快速打字不误触"
input = "+space +b -space -b"
expect = "space b"

[[test]]
name = "空格长按进入导航层"
input = "+space @300 j -space"
expect = "down"
```

序列写法：

| 写法 | 含义 |
|---|---|
| `a` | 按下并松开 A |
| `+a` / `-a` | 按下 A / 松开 A |
| `ctrl+c` | 按下 ⌃，单按 C，松开 ⌃ |
| `@300` | 等待 300 毫秒 |
| `"你好"` | 脚本用 `kanami.type` 输入的文字（只用于 `expect`） |

`expect` 用同样的写法描述输出（不能写等待）。`+x` / `-x` 里的 x 也可以写 `ctrl`、`cmd` 这类只代表一个键的修饰键写法，表示左侧的那个键。

测试结束时，还没判定的单按/按住、组合键和脚本定时器会按超时处理。可选字段：

| 字段 | 含义 |
|---|---|
| `name` | 测试名，失败时显示 |
| `app` | 模拟的前台 App Bundle ID，用来测试 `when.app` 条件 |
| `input_source` | 模拟的输入法 ID，用来测试 `when.input_source` 条件 |
| `device` | 模拟按键来自 `[device.*]` 里的哪把键盘 |

每个测试都从全新的状态开始，脚本也会重新加载。`run` 动作不出现在测试输出里。

---

## 9. 错误提示与热重载

- 所有错误带行号和列号；键名写错时给出最接近的建议，比如 `config.toml:3:7: 未知键名 "esacpe"，最接近的键名是 "escape"`。
- `kanami schema` 输出 JSON Schema。保存到配置文件旁边，并在配置第一行写 `#:schema ./kanami.schema.json`，支持 Taplo 的编辑器（如 VS Code 的 Even Better TOML）就能补全键名、实时校验：

  ```bash
  kanami schema > ~/.config/kanami/kanami.schema.json
  ```

- `kanami run` 每秒检查一次配置文件，保存后自动重载。新配置有错误时**继续使用旧配置**，并提示错误位置。
- 重载时如果有键处于按住状态，先补发松开事件再切换规则，避免出现“卡住的修饰键”。

## 10. 数量限制

| 项目 | 上限 |
|---|---|
| 图层 | 32 |
| `[[combo]]` | 64 |
| 一个数组动作里的按键表达式 | 32 |

---

## 11. 紧急停止

同时按住**左 Control + 空格 + Esc**，立即停用全部改键：所有按键和鼠标事件原样放行，Caps Lock 恢复系统原本的行为。再按一次恢复。这组按键是引擎内置的，不受配置影响，配置写坏了也能用；只认物理键，被改成 Control 的 Caps Lock 不算。

## 12. 脚本插件

声明式配置覆盖不了的需求用 Luau 脚本实现，见 [scripting.md](scripting.md)。tap dance、Leader 键、文本展开作为随附插件提供，在 `plugins/` 目录。

## 13. 尚未支持

- 鼠标按键和滚轮改键

实现细节见 [architecture.md](../architecture.md)。
