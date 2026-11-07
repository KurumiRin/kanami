//! Simplified Chinese catalog: `(English source string, translation)`.

pub const ENTRIES: &[(&str, &str)] = &[
    (
        "Toggling Caps Lock failed: {error}",
        "切换大小写锁定失败：{error}",
    ),
    (
        "The from of a tap/hold rule must be a single key without modifiers",
        "tap/hold 规则的 from 只能是单个键，不能带修饰键",
    ),
    (
        "{key} is already the trigger key of another layer",
        "{key} 已经是另一个图层的触发键",
    ),
    ("The key list cannot be empty", "按键列表不能为空"),
    ("A combo needs at least two keys", "combo 至少需要两个键"),
    (
        "The modifier {key} cannot be used in a combo",
        "combo 里不能用修饰键 {key}",
    ),
    (
        "{key} appears more than once in the combo",
        "combo 里重复出现 {key}",
    ),
    (
        "The maximum number of {what} is {max}",
        "{what}最多 {max} 个",
    ),
    ("devices", "设备"),
    ("layers", "图层"),
    ("combos", "combo "),
    ("keys in a sequence", "按键序列的长度"),
    (
        "There is no device named {name}; define it in [device.{name}] first",
        "没有名为 {name} 的设备，请先在 [device.{name}] 里定义",
    ),
    (
        "Failed to read config {path}: {error}",
        "读取配置 {path} 失败：{error}",
    ),
    (
        "Device {name} needs at least one of vendor_id, product_id and product; run `kanami devices` to see them",
        "设备 {name} 至少要写 vendor_id、product_id、product 中的一个，用 `kanami devices` 查看",
    ),
    ("Test {number}", "第 {number} 个测试"),
    ("There is no device named {name}", "没有名为 {name} 的设备"),
    (
        "Failed to read script {path}: {error}",
        "读取脚本 {path} 失败：{error}",
    ),
    ("inline script", "内联脚本"),
    (
        "The value of variable {name} must be a string, boolean or integer",
        "变量 {name} 的值只能是字符串、布尔值或整数",
    ),
    (
        "This is already inside [device.{name}], so when.device cannot be used here",
        "已经在 [device.{name}] 里了，这里不能再写 when.device",
    ),
    (
        "strategy can only be used with tap/hold",
        "strategy 只能和 tap/hold 一起用",
    ),
    (
        "The rule is missing to, run, call or tap/hold",
        "规则缺少 to、run、call 或 tap/hold",
    ),
    (
        "A rule can have only one of to, run, call and tap/hold",
        "每条规则只能写 to、run、call、tap/hold 其中一种",
    ),
    ("tap requires hold as well", "写了 tap 就必须同时写 hold"),
    (
        "The toggle layer {name} does not support tap or strategy",
        "toggle 图层 {name} 不支持 tap 和 strategy",
    ),
    (
        "Layer {name} needs either hold or toggle",
        "图层 {name} 需要写 hold 或 toggle 其中一个",
    ),
    (
        "A combo needs one of to, run and call",
        "combo 需要写 to、run、call 其中一个",
    ),
    (
        "Unknown strategy \"{other}\"; the options are timeout, permissive and eager",
        "未知策略 \"{other}\"，可选 timeout、permissive、eager",
    ),
    (
        "Unknown language \"{tag}\"; the options are \"en\" and \"zh-CN\"",
        "未知语言 \"{tag}\"，可选 \"en\"、\"zh-CN\"",
    ),
    (
        "Malformed key expression \"{expr}\"; write it like \"cmd+shift+k\"",
        "按键表达式 \"{expr}\" 格式不对，应写成 \"cmd+shift+k\" 这样",
    ),
    (
        "\"{expr}\" has only modifiers; a key is needed here",
        "\"{expr}\" 只有修饰键，这里需要一个键",
    ),
    ("none cannot be used here", "这里不能写 none"),
    (
        "[remap] can only change to a single key or a set of modifiers; use [[rule]] for \"{expr}\"",
        "[remap] 只能改成单个键或一组修饰键，\"{expr}\" 请改用 [[rule]]",
    ),
    ("\"{name}\" is not a single key", "\"{name}\" 不是单个键"),
    (
        "Unknown key \"{name}\"; the closest name is \"{guess}\"",
        "未知键名 \"{name}\"，最接近的键名是 \"{guess}\"",
    ),
    (
        "Unknown key \"{name}\"; run `kanami keys` to see all key names",
        "未知键名 \"{name}\"，用 `kanami keys` 查看所有键名",
    ),
    (
        "Unknown modifier \"{name}\"; the closest name is \"{guess}\"",
        "未知修饰键 \"{name}\"，最接近的写法是 \"{guess}\"",
    ),
    (
        "Unknown modifier \"{name}\"; run `kanami keys` to see all key names",
        "未知修饰键 \"{name}\"，用 `kanami keys` 查看所有键名",
    ),
    ("input cannot contain text", "input 里不能写文字"),
    (
        "The text is missing its closing double quote",
        "文字缺少结尾的双引号",
    ),
    ("expect cannot contain a wait", "expect 里不能写等待"),
    (
        "\"{token}\" should be written as @milliseconds, such as @300",
        "\"{token}\" 应写成 @毫秒数，如 @300",
    ),
    ("(no output)", "（无输出）"),
    (
        "Could not find the APPDATA directory",
        "找不到 APPDATA 目录",
    ),
    ("Could not find the HOME directory", "找不到 HOME 目录"),
    (
        "Connecting to {path} failed ({error}); make sure kanami is running",
        "连接 {path} 失败（{error}），请确认 kanami 正在运行",
    ),
    (
        "IPC is not implemented on Windows yet",
        "Windows 上的 IPC 还没有实现",
    ),
    ("kanami did not reply", "kanami 没有回复"),
    (
        "Calling hidutil failed: {error}",
        "调用 hidutil 失败：{error}",
    ),
    (
        "Another program has already set a key mapping through hidutil; to avoid overwriting it, Caps Lock rules will not take effect",
        "已有其他程序通过 hidutil 设置了键位映射，为避免覆盖，Caps Lock 相关规则不会生效",
    ),
    ("other key", "其他键"),
    ("hidutil exited with {status}", "hidutil 退出码 {status}"),
    (
        "Note: modifier keys were changed in System Settings ({remaps}), so kanami's caps_lock rules will not apply to any key. To make them apply to Caps Lock itself, restore the default in “System Settings → Keyboard → Keyboard Shortcuts… → Modifier Keys”",
        "注意：系统设置里改过修饰键（{remaps}），kanami 的 caps_lock 规则不会作用在任何键上。想让它作用在 Caps Lock 本身，请在“系统设置 → 键盘 → 键盘快捷键… → 修饰键”里恢复默认",
    ),
    (
        "Note: modifier keys were changed in System Settings ({remaps}), so kanami's caps_lock rules will actually apply to {keys}. To make them apply to Caps Lock itself, restore the default in “System Settings → Keyboard → Keyboard Shortcuts… → Modifier Keys”",
        "注意：系统设置里改过修饰键（{remaps}），kanami 的 caps_lock 规则实际作用在 {keys} 上。想让它作用在 Caps Lock 本身，请在“系统设置 → 键盘 → 键盘快捷键… → 修饰键”里恢复默认",
    ),
    ("Accessibility", "辅助功能"),
    ("Input Monitoring", "输入监控"),
    (
        "Could not create the event tap. In “System Settings → Privacy & Security → {permission}”, allow the terminal app that runs kanami, then restart that terminal",
        "无法创建事件监听。请在“系统设置 → 隐私与安全性 → {permission}”中允许运行 kanami 的终端 App，然后重新启动该终端",
    ),
    (
        "Cannot tell which keyboard a key came from: in “System Settings → Privacy & Security → Input Monitoring”, allow the app that runs kanami, then restart kanami",
        "无法识别按键来自哪把键盘：请在“系统设置 → 隐私与安全性 → 输入监控”里允许运行 kanami 的 App，然后重新启动 kanami",
    ),
    (
        "Restoring Caps Lock failed ({error}); run `kanami reset`",
        "恢复 Caps Lock 失败（{error}），请运行 `kanami reset`",
    ),
    (
        "macOS has no {key} key; ignored",
        "macOS 上没有 {key} 键，已忽略",
    ),
    ("timeout", "超时"),
    ("user input", "用户操作"),
    (
        "Functions that rules `call` are not defined in any script: {names}",
        "规则里 call 的函数在脚本里没有定义：{names}",
    ),
    ("Script execution timed out", "脚本执行超时"),
    (
        "Initializing the script environment failed: {error}",
        "初始化脚本环境失败：{error}",
    ),
    (
        "Enabling the script sandbox failed: {error}",
        "启用脚本沙箱失败：{error}",
    ),
    (
        "Loading script {name} failed: {error}",
        "加载脚本 {name} 失败：{error}",
    ),
    ("kanami: script error: {error}", "kanami：脚本出错：{error}"),
    ("Function {name} does not exist", "函数 {name} 不存在"),
    (
        "This kanami function can only be used inside a callback (not while a script is loading)",
        "kanami 的这个函数只能在回调里使用（加载脚本时不能调用）",
    ),
    ("Layer {name} does not exist", "图层 {name} 不存在"),
    ("[script] {message}", "[脚本] {message}"),
    (
        "Creating the single-instance lock failed",
        "创建单实例锁失败",
    ),
    (
        "Another kanami is already running",
        "已经有一个 kanami 在运行",
    ),
    ("Installing the keyboard hook failed", "安装键盘钩子失败"),
    (
        "kanami: IPC on Windows (settings window communication) is not implemented yet",
        "kanami：Windows 上的 IPC（设置窗口通信）还没有实现",
    ),
    ("Invalid JSON: {error}", "不是合法的 JSON：{error}"),
    ("Unknown command {command}", "未知命令 {command}"),
    ("Missing cmd field", "缺少 cmd 字段"),
    ("The service is quitting", "服务正在退出"),
    (
        "kanami: IPC listening failed ({error}); the settings window will not be able to connect",
        "kanami：IPC 监听失败（{error}），设置窗口将无法连接",
    ),
    ("Remapping paused", "已暂停改键"),
    (
        "Config has errors, using the previous config",
        "配置有误，正在使用上一份配置",
    ),
    ("Remapping", "正在改键"),
    ("Remapping: {summary}", "正在改键：{summary}"),
    ("Resume Remapping", "恢复改键"),
    ("Pause Remapping", "暂停改键"),
    ("Open Settings…", "打开设置…"),
    ("Event Viewer…", "事件查看器…"),
    ("Open Config in Editor", "在编辑器中打开配置"),
    ("Reload Config", "重新加载配置"),
    ("Stop Remapping and Quit", "停止改键并退出"),
    (
        "This kanami was built without the script feature, so [script] and call are unavailable",
        "这个 kanami 编译时没有启用脚本功能（script），无法使用 [script] 和 call",
    ),
    ("Cannot find the HOME directory", "找不到 HOME 目录"),
    (
        "Failed to run launchctl: {error}",
        "调用 launchctl 失败：{error}",
    ),
    (
        "Cannot find the config file {path}: {error}",
        "找不到配置文件 {path}：{error}",
    ),
    (
        "Failed to create {path}: {error}",
        "创建 {path} 失败：{error}",
    ),
    (
        "Cannot find the current executable: {error}",
        "找不到当前程序：{error}",
    ),
    (
        "Failed to copy the executable to {path}: {error}",
        "复制程序到 {path} 失败：{error}",
    ),
    (
        "Failed to write {path}: {error}",
        "写入 {path} 失败：{error}",
    ),
    (
        "launchctl bootstrap failed: {error}",
        "launchctl bootstrap 失败：{error}",
    ),
    (
        "kanami service installed and started",
        "已安装并启动 kanami 服务",
    ),
    ("Program: {path}", "程序：{path}"),
    (
        "Config: {path} (reloaded automatically when saved)",
        "配置：{path}（保存后自动重载）",
    ),
    ("Log: {path}", "日志：{path}"),
    (
        "On first launch macOS asks for Accessibility permission. In “System Settings → Privacy & Security → Accessibility”,\nallow {path}. The service starts working automatically once it is allowed.",
        "第一次启动时 macOS 会请求辅助功能权限，请在“系统设置 → 隐私与安全性 → 辅助功能”里\n允许 {path}。授权后服务会自动开始工作。",
    ),
    (
        "Note: running `kanami service install` again replaces the executable, which may need to be authorized again.",
        "注意：重新执行 `kanami service install` 会替换程序，可能需要重新授权。",
    ),
    (
        "Failed to delete {path}: {error}",
        "删除 {path} 失败：{error}",
    ),
    (
        "kanami service stopped and uninstalled (config and log are kept)",
        "已停止并卸载 kanami 服务（配置和日志保留）",
    ),
    ("The kanami service is not installed", "kanami 服务没有安装"),
    (
        "The kanami service is not installed. Run `kanami service install` first.",
        "kanami 服务没有安装，先运行 `kanami service install`",
    ),
    ("kanami service restarted", "已重启 kanami 服务"),
    ("unknown", "未知"),
    ("running", "运行中"),
    ("not running", "未运行"),
    ("kanami service: {state}", "kanami 服务：{state}"),
    ("Process: {pid}", "进程：{pid}"),
    ("Last exit code: {code}", "上次退出码：{code}"),
    ("Recent log:", "最近的日志："),
    (
        "kanami: {error}\n  This takes effect once the config is fixed; until then all keys pass through unchanged",
        "kanami：{error}\n  修好配置后自动生效，在此之前所有按键原样放行",
    ),
    ("kanami: Loaded {path}", "kanami：已加载 {path}"),
    (
        "kanami: Loaded {path} ({summary})",
        "kanami：已加载 {path}（{summary}）",
    ),
    (
        "kanami: Loaded {path}; press Ctrl+C to quit",
        "kanami：已加载 {path}，Ctrl+C 退出",
    ),
    (
        "kanami: Loaded {path} ({summary}); press Ctrl+C to quit",
        "kanami：已加载 {path}（{summary}），Ctrl+C 退出",
    ),
    (
        "kanami: Listening for keys; press Ctrl+C to quit",
        "kanami：正在监听按键，Ctrl+C 退出",
    ),
    ("(repeat, ignored)", "（重复，忽略）"),
    ("(waiting to decide)", "（等待判定）"),
    ("(swallowed)", "（吞掉）"),
    ("(timer)", "（定时）"),
    (
        "kanami: Config has errors, keeping the previous config\n  {error}",
        "kanami：配置有误，继续使用旧配置\n  {error}",
    ),
    ("kanami: Config reloaded", "kanami：已重载配置"),
    (
        "kanami: Failed to open the config file: {error}",
        "kanami：打开配置文件失败：{error}",
    ),
    (
        "kanami: Cannot find the settings window program (kanami-settings); the settings window is still in development",
        "kanami：没有找到设置窗口程序（kanami-settings），设置窗口还在开发中",
    ),
    (
        "kanami: Failed to open the settings window ({app}): {error}",
        "kanami：打开设置窗口失败（{app}）：{error}",
    ),
    (
        "kanami: All remaps are paused; press Left Control + Space + Esc together to resume",
        "kanami：已停用全部改键；再同时按左 Control + 空格 + Esc 恢复",
    ),
    ("kanami: Remapping resumed", "kanami：已恢复改键"),
    ("(unknown)", "（未知）"),
    ("Frontmost app: {app}", "前台 App：{app}"),
    ("Input source: {source}", "输入法：{source}"),
    (
        "kanami: Failed to create the menu bar icon: {error}",
        "kanami：创建菜单栏图标失败：{error}",
    ),
    (
        "kanami: The event tap was paused by the system because of {reason} and has been restored; keys pressed meanwhile did not go through kanami",
        "kanami：事件监听因{reason}被系统暂停，已恢复；暂停期间的按键没有经过 kanami",
    ),
    (
        "passed through (sent by another program, process {pid})",
        "原样放行（其他程序发出，进程 {pid}）",
    ),
    (
        "kanami: The release of {key} did not go through kanami (secure input, a paused event tap or sleep); it has been supplied",
        "kanami：{key} 的松开没有经过 kanami（安全输入、监听暂停或睡眠），已补上",
    ),
    (
        "kanami: Failed to run `{command}`: {error}",
        "kanami：执行 `{command}` 失败：{error}",
    ),
    ("[other program, process {pid}]", "〔其他程序，进程 {pid}〕"),
    ("input source {source}", "输入法 {source}"),
    (
        "(event tap paused by the system because of {reason}, restored)",
        "（事件监听因{reason}被系统暂停，已恢复）",
    ),
    (
        "kanami — a small, fast, programmable keyboard remapper\n\nUsage:\n  kanami run     [-c <config>] [--trace] [--tray]\n                                   Remap keys using the config (runs in the foreground, Ctrl+C quits, saving the config reloads it)\n                                   --trace prints the input and output of every key, for troubleshooting\n                                   --tray  shows the menu bar / tray icon (shown by default for the background service)\n  kanami ctl     status|pause|resume|reload|devices|events|quit\n                                   Control a running kanami; events keeps printing the input and output of every key\n  kanami monitor                   Listen only; print key events and the frontmost app\n  kanami check   [-c <config>]     Validate the config\n  kanami test    [-c <config>]     Run the [[test]] cases in the config\n  kanami keys                      List key names and how to write modifiers\n  kanami devices                   List connected keyboards, for use in [device.*]\n  kanami schema                    Print the JSON Schema of the config file\n  kanami reset                     Remove the Caps Lock mapping kanami left behind (use after an abnormal exit)\n  kanami service install [-c <config>]\n                                   Install as a background service: starts at login, restarts after a crash\n  kanami service uninstall         Stop and uninstall the background service\n  kanami service status            Show the background service status and recent log\n  kanami service restart           Restart the background service\n\nThe config file defaults to $KANAMI_CONFIG, then ~/.config/kanami/config.toml (Windows: %APPDATA%\\kanami\\config.toml)",
        "kanami — 小而快、可编程的键盘改键工具\n\n用法：\n  kanami run     [-c <配置文件>] [--trace] [--tray]\n                                   按配置改键（前台运行，Ctrl+C 退出，保存配置自动重载）\n                                   --trace 打印每个按键的输入和输出，用于排查问题\n                                   --tray  显示菜单栏 / 托盘图标（后台服务默认显示）\n  kanami ctl     status|pause|resume|reload|devices|events|quit\n                                   控制正在运行的 kanami；events 持续打印按键的输入和输出\n  kanami monitor                   只监听，打印按键事件和前台 App\n  kanami check   [-c <配置文件>]   校验配置\n  kanami test    [-c <配置文件>]   运行配置里的 [[test]]\n  kanami keys                      列出键名和修饰键写法\n  kanami devices                   列出已连接的键盘，用于 [device.*]\n  kanami schema                    输出配置文件的 JSON Schema\n  kanami reset                     撤销 kanami 留下的 Caps Lock 映射（异常退出后使用）\n  kanami service install [-c <配置文件>]\n                                   安装为后台服务：登录后自动启动，崩溃后自动拉起\n  kanami service uninstall         停止并卸载后台服务\n  kanami service status            查看后台服务状态和最近的日志\n  kanami service restart           重启后台服务\n\n配置文件默认是 $KANAMI_CONFIG，其次是 ~/.config/kanami/config.toml（Windows：%APPDATA%\\kanami\\config.toml）",
    ),
    (
        "Unknown command \"{command}\". Run `kanami help` for usage.",
        "未知命令 \"{command}\"，用 `kanami help` 查看用法",
    ),
    ("Unexpected argument \"{arg}\"", "多余的参数 \"{arg}\""),
    (
        "{flag} needs a config file path after it",
        "{flag} 后面需要配置文件路径",
    ),
    (
        "Unrecognized argument \"{arg}\"",
        "无法识别的参数 \"{arg}\"",
    ),
    (
        "Cannot find the HOME directory. Specify a config file with -c.",
        "找不到 HOME 目录，请用 -c 指定配置文件",
    ),
    (
        "remaps: {remaps}, rules: {rules}, layers: {layers}, combos: {combos}",
        "{remaps} 条改键、{rules} 条规则、{layers} 个图层、{combos} 个组合键",
    ),
    (", scripts: {scripts}", "、{scripts} 个脚本"),
    (
        "{path}: Config is valid, {summary}",
        "{path}：配置有效，{summary}",
    ),
    (
        "{path} contains no [[test]] cases",
        "{path} 里没有 [[test]]",
    ),
    ("✗ {name} (line {line})", "✗ {name}（第 {line} 行）"),
    ("Expected: {expected}", "期望：{expected}"),
    ("Actual: {actual}", "实际：{actual}"),
    (
        "{passed} passed, {failed} failed",
        "{passed} 个通过，{failed} 个失败",
    ),
    ("Key names:", "键名："),
    ("Modifier names for key chords:", "组合键里的修饰键写法："),
    ("Config is valid: {summary}", "配置有效：{summary}"),
    (
        "Unknown service subcommand \"{sub}\". Available: install, uninstall, status, restart.",
        "未知的 service 子命令 \"{sub}\"，可选 install、uninstall、status、restart",
    ),
    (
        "Usage: kanami service install|uninstall|status|restart",
        "用法：kanami service install|uninstall|status|restart",
    ),
    ("Caps Lock mapping removed", "已撤销 Caps Lock 映射"),
    ("No mapping to remove", "没有需要撤销的映射"),
    (
        "Failed to run hidutil: {error}",
        "调用 hidutil 失败：{error}",
    ),
    (
        "{what} is only supported on macOS for now",
        "{what}目前只支持 macOS",
    ),
    (
        "Usage: kanami ctl status|pause|resume|reload|devices|events|quit",
        "用法：kanami ctl status|pause|resume|reload|devices|events|quit",
    ),
    ("Monitoring", "监听"),
    ("No keyboards found", "没有找到键盘"),
    ("Product name", "产品名"),
    (
        "Write it like this in the config; remaps and rules for this keyboard go under it ([device.my_keyboard.remap], [[device.my_keyboard.rule]]):",
        "在配置里这样写，这把键盘专用的改键和规则写在它下面（[device.my_keyboard.remap]、[[device.my_keyboard.rule]]）：",
    ),
    (
        "# Every field you write must match; keyboards with the same IDs can be told apart with product = \"part of the product name\"",
        "# 写了的字段都要匹配；ID 相同的键盘可以改用 product = \"产品名的一部分\" 区分",
    ),
    ("Per-keyboard settings", "按键盘区分"),
    ("The background service", "后台服务"),
    ("Disable", "禁用"),
    ("Run", "命令"),
    ("Script", "脚本"),
    (
        "kanami: Waiting for Accessibility permission. Allow {path} in “System Settings → Privacy & Security → Accessibility”",
        "kanami：等待辅助功能授权，请在“系统设置 → 隐私与安全性 → 辅助功能”里允许 {path}",
    ),
    (
        "kanami: Accessibility permission granted",
        "kanami：已获得辅助功能授权",
    ),
    (
        "Failed to open {path}: {error}",
        "打开 {path} 失败：{error}",
    ),
    (
        "Another kanami is already running (process {pid}). If it is the background service, check it with `kanami service status`",
        "已经有一个 kanami 在运行（进程 {pid}）。如果是后台服务，用 `kanami service status` 查看",
    ),
    (
        "The config file is not valid TOML: {error}",
        "配置文件不是有效的 TOML：{error}",
    ),
    ("{path} is not an array", "{path} 不是数组"),
    ("{path} is not a table", "{path} 不是表"),
    ("{path} has no item {number}", "{path} 没有第 {number} 项"),
    ("{path} is not a key", "{path} 不是键"),
    (
        "There is already an entry named {name}",
        "已经有叫 {name} 的了",
    ),
    ("The path cannot be empty", "路径不能为空"),
    (
        "The config has no {path}; it may have just been changed elsewhere",
        "配置里没有 {path}，可能刚被别处修改过",
    ),
    ("Cannot move something into itself", "不能挪到自己里面"),
    ("The value cannot be empty", "值不能为空"),
    ("A table is needed here", "这里需要一个表"),
    ("kanami Settings", "kanami 设置"),
    ("Unsupported command {command}", "不支持的命令 {command}"),
    (
        "Failed to read {path}: {error}",
        "读取 {path} 失败：{error}",
    ),
    (
        "Failed to save {path}: {error}",
        "保存 {path} 失败：{error}",
    ),
    ("{message} (line {line})", "{message}（第 {line} 行）"),
    (
        "Failed to open the editor: {error}",
        "打开编辑器失败：{error}",
    ),
    ("Unknown permission {permission}", "未知的权限 {permission}"),
    (
        "Cannot find the kanami command-line program",
        "没有找到 kanami 命令行程序",
    ),
    ("Key remapping", "改键"),
];
