# Architecture

## NSH mixed 项目

`nsh.json` 声明 Rust crate 和主题；`App.tsx` 是默认 TSX 入口。CLI、`nsh-core`、`nsh-base` 均固定 0.5.0。UI 使用 macOS 原生 AppKit 后端，不引入 WebView。

```text
App.tsx                          mount 工作区，仅作入口
  ui/WorkspaceApp.tsx             窗口、菜单、快捷键、对话框与会话操作接线
  ui/Sidebar.tsx                  vertical tabs、分组与响铃标记
  ui/GroupEditor.tsx              锚定组头的名称、颜色和组操作
  ui/workspace.ts                 会话、分组与平铺操作、唯一逻辑焦点
  ui/tiling.ts                    布局树变换、约束、几何与方向导航
  ui/TilingCanvas.tsx             按 session ID 稳定挂载、pane 工具栏与分隔拖动
  ui/theme.ts                    分组标识到主题 token 的映射
  terminal/TerminalSurface.tsx    真实终端的显示、焦点属性与低频状态
    native SessionTerminal       绑定已有 Rust 会话
  terminal/PreviewTerminal.tsx    显式预览模式的 NSH Terminal 与焦点
    preview-session.ts           确定性演示 transport，不执行命令

src/lib.rs                       只注册 termlevel/native 的原生导出
src/runtime.rs                   版本、启动目录、显式预览开关与原生 cell 测量
src/dialog_keys.rs               对话框 Escape 与 pane 缩放模式的原生键盘优先级
src/terminal.rs                  Alacritty parser/grid 到 NSH 的适配；预览 provider
src/sessions.rs                  SessionManager、PTY、进程、reader/writer/reaper
  sessions/api.rs                TSX 可调用的显式会话操作
src/session_view.rs              按 ID 绑定会话、低频状态通知
  session_view/view.rs           Rust 行视图、滚动/光标坐标、原生 Input
  session_view/input.rs          组合输入和提交状态
  session_view/keys.rs           基础终端按键编码
```

TSX 在 QuickJS 中执行，经 NSH 引擎由原生平台绘制。`termlevel/native` 是同进程 Rust 绑定，不是网络 RPC。类型声明由 NSH 生成到 `.nsh/types`，不手工维护。目录按职责划分，不为每个组件另建 crate。

## 终端数据通路

固定使用 crates.io 上游 `alacritty_terminal =0.26.0`，不引入 Zed 的 GPL 终端封装或其 Alacritty fork。`portable-pty =0.9.0` 创建 PTY 和子进程。

```text
TSX 工作区 ── open / close / list / shutdown ── Rust SessionManager
                                                   │ 强持有
                                     PTY + child + Alacritty parser/grid
                                        │                 │
                               reader 原始字节 ────────────┘
                                        │
                               Rust revision → 原生视图（合并刷新）
                                        │
                               Rust 单元格行视图 / 光标 / NSH 选区
                                        │
                               NSH 原生输入 → writer → PTY

Rust → TSX：标题、运行/退出/错误状态、响铃；不发送终端 chunk 或 cell grid
```

每个会话有独立 `Term`、持久的 `Processor`、事件队列与有限滚动历史。reader 直接处理原始字节，保留跨 chunk 的 UTF-8 和转义序列状态。同步输出协议在正常结束、超时和 EOF 时提交，不能因缺少结束序列而永久藏住输出。网格映射保留组合字符，跳过宽字符占位单元，并合并相邻同样式 run；Alacritty 负数历史行映射为 NSH 从最老历史起的行号。

程序指定的 ANSI、256 色、truecolor 和调色板覆盖保留其语义；未指定的默认颜色跟随主题。默认色查询使用工作区传入的主题颜色，之后随主题更新；协议回复直接从 Rust 进入 writer。PTY 与网格 resize 在 parser 锁内一致更新，避免 SIGWINCH 后的新尺寸输出落入旧尺寸网格。

## 生命周期与并发

`SessionManager` 在 Rust 中独立于视图持有会话。只有 `Sessions.open` 创建进程；`SessionTerminal` 只能按 ID 获取已有会话，已关闭或丢失的 ID 返回错误，不会隐式重新创建进程。ID 由 Rust 生成，与标题、排序、分组、选择、PID 和视图位置无关。

- 默认工作区恢复 `Sessions.list()` 的仍存活或已退出会话；没有会话时启动一个新 shell。这个恢复只发生在同一个原生宿主进程内，不是应用重启恢复。
- TSX 工作区销毁/重建以及原生终端视图卸载/重挂均不关闭会话。重新绑定读取当前 Rust 网格和状态，不依赖 JS 中的输出副本或事件补发。
- 标签切换、分组折叠、重命名、排序与主题变更保留同一 session ID。工作区保留按 session ID 挂载的终端显示；布局树决定哪些会话同时展示，暂未展示的会话隐藏、禁用命中并移出无障碍遍历。焦点 pane 决定唯一输入目标，其他可见 pane 仍持续刷新。
- 显式关闭标签调用 `Sessions.close`；关闭组逐个关闭其会话。Shell 退出码为 0（含空行 Ctrl+D）时同样关闭标签和 pane，不自动重启。非 0 退出和启动失败保留输出与状态。
- 关闭主窗口或退出应用调用 `Sessions.shutdown`。Unix 上结束前台进程组，向 shell 发送 HUP，让它清理普通后台作业；50 ms 后仍未退出则强制结束 shell。主动脱离终端会话的 daemon 不属于可恢复会话模型。
- child 的检查、终止和回收共用控制锁，避免对已回收并可能复用的 PID 发信号。关闭后 reader/writer 取消，reaper 回收 child；应用退出在 NSH 的 200 ms 清理预算内轮询 `Sessions.settled()`。

reader/writer/reaper 在 Rust 工作线程中运行。Unix PTY 使用非阻塞 I/O；reader 以 8 KiB 块解析，writer 的输入队列最多 128 个请求，单次输入上限 1 MiB，拒绝会显示错误，不静默截断。原生视图最多约每 16 ms 检查一次 revision，只在有更新时更新 Rust 显示 signal，不经过 TSX。视图刷新计时器随视图销毁，会话工作线程不会随之销毁。

`SessionTerminal` 直接读取同一个 Rust 会话网格，使用 NSH 原生虚拟列表绘制。ASCII 同样式单元格合并；宽字符与组合字符按 Alacritty 的列号单独锚定，字体 fallback 的 glyph advance 不影响下一列的位置。字符背景占完整单元格，单行文本裁剪，不以省略号代替终端内容。预览的 `TerminalEngine` 仍使用 `TerminalEmulatorFactory` 创建视图独享的 Alacritty 模拟器，预览生命周期不能照搬到真实 PTY。

一个稳定的原生 Input 放在滚动视图之外，与网格列表共用宿主 padding 以内的内容容器；两者必须具有同一个坐标原点。终端顶部不留内边距，左右及底部留 16 pt；滚动内容与 Input 均位于 pane 工具栏下方的同一内边距容器。按当前网格光标、滚动 offset 和底部对齐余量定位，宽度为当前行剩余空间。未组合时文字透明，由 PTY echo 决定显示；组合时由原生输入控件显示 marked text、插入光标和系统输入法交互。开始输入/组合时跟随滚动末尾。即使 TUI 隐藏输出光标，输入法锚点仍读取实际网格位置。

`InputState` 缓存组合过程，只有结束后提交最终差量；取消删除 marked text 不发送残留拼音。原生值先确认，再通过 NSH `after_answer` 清空，避免 Rust host 属性比较跳过相同的空值；若已进入下一次组合，暂不清空。普通输入处理清空尚未到达平台时的累积值，避免快速输入重复。组合期间暂停终端按键绑定，Enter/方向键/Escape 交给输入法；基础键盘绑定只在该会话 active 且 Input focused 时生效。Command 编辑快捷键不占用工作区新建/关闭标签快捷键。

离开 macOS field editor 时，AppKit 可以确认当前 marked text；回调仍绑定旧 `Arc<Session>`，不向新活动会话转发。挂载、隐藏与焦点不拥有 PTY 生命周期。

## 原生交互与工作区 UI

多 pane 已接入工作区，完整交互和验收边界见 [多终端平铺设计](terminal-tiling.md)。

`DialogKeys` 在原生层为对话框取得 Escape 优先级；界面与对话框状态仍在 TSX。`GroupEditor` 使用 NSH `Overlay` 和组头 `ViewRef` 定位，修改即时写入工作区，关闭不回滚。

统一窗口栏固定在顶部；下方侧栏与终端作为独立圆角 pane，外边距和间距由 `gap-pane` 控制。Sidebar 唯一实例在窗口根层绝对定位，避免被 44 pt 原生分栏裁剪。分栏边界两侧各分配半个间距，根层侧栏扣除这半个间距，与终端 pane 顶底对齐。分栏内测量 View 的 `onLayout` 同步拖动中的实际宽度；`onResize` 只保存最终宽度。收起时鼠标悬停展开覆盖终端，不改变终端分配宽度和挂载位置；编辑组时保持展开。

会话标题来自低频元数据，用户重命名后不再被程序标题覆盖。非活动会话响铃时显示标记，选择后清除。垂直拖动标签在侧栏内排序或放入分组。落到已有分组的标签上会加入该组，落到没有分组的标签上会组成新组，路径取被拖到的标签；放入折叠组时展开该组。分组保存自己的路径，组内新建的终端从这里启动，组名默认是路径名。取消拖动不改分组，session ID 不变。工具栏目录表示初始启动目录，tooltip 明确说明；尚未实现 shell integration 的实时 cwd 跟踪。退出码非 0 或启动失败时，进程状态在终端底部显示，已有输出保留。退出码 0 会关掉这个终端。

### 平铺状态与稳定展示

平铺采用 TSX 拥有的二叉树、pane ID、比例与 `focusedPaneId`。每个标签组各有一棵树，未分组会话共用一棵；画布只投影当前组。一棵树有两个及以上 pane 时，侧栏把这些会话收成一个分屏单元，成员与 pane 一一对应。单元可以留在组外，也可以整块移入一个标签组。侧栏选择 `activeId` 从当前画面的焦点叶子派生。选中已在该组分屏里的会话时，整个右侧换成这组分屏并聚焦它。选中分屏以外的会话时，整个右侧换成这一个终端，分屏树保持不动。选中其他组则换成那一组的画面。关闭组一次性移除相关会话并丢掉该组的树。折叠和组内排序不改变树。把其他组的终端拖进画面会改掉它的组，并放进当前树。

`TilingCanvas` 的 surface 层只按 session ID keyed，不把 terminal 放进递归 Split 组件。树生成绝对位置与尺寸；交换、移动和放大只修改宿主几何，避免 Input 和滚动视图因树重排重建。pane 间分隔通过原生 View 的 pan/responder 事件更新 TSX 比例；这是工作区几何，不传递终端字节或网格。侧栏仍使用 NSH `ResizablePanelGroup`。未展示 surface 保留最后有效矩形，Rust 的非正 viewport 检查阻止向 PTY 发送 0×0。

`TerminalMetrics` 在原生视图作用域测量 cell 宽度，通过低频事件供 TSX 计算 40 列 × 8 行的最小正文尺寸；它与 Rust renderer 共用字体测量函数。窗口无法容纳整棵树时临时只展示焦点 pane，保存树及用户比例，空间恢复后还原。用户主动放大与这个临时投影分别管理。

`SessionTerminal.active` 控制输入授权和焦点，刷新定时器不依赖它。点击或实际 Input 聚焦时，通过 `FocusRequest` 同步工作区选择。对话框和编辑浮层临时阻止 terminal 自动取得焦点；pane 键盘缩放使用在 terminal 之后挂载的原生 `ResizeKeys`，结束后卸载，让方向键和 Enter 回到终端。AppKit 在 marked text 期间将按键交给输入法，不能假定工作区快捷键此时仍会执行。

## 当前能力边界

Alacritty 内核接入与真实 PTY 通路已实现，真实会话使用自己的 Rust 网格视图与 NSH 0.5.0 原生 Input/选区，不等同于嵌入 Alacritty 应用的 GPU renderer。

- 可用通路包括应用光标键、括号粘贴、标题、响铃、光标位置与协议回复、备用屏幕、滚动历史、网格重排、调色板与默认色查询。
- 鼠标报告、焦点报告、功能键/修饰键的完整编码、现代键盘协议、超链接元数据、光标形状与精细下划线尚未接通；下划线变体暂投影为普通下划线。Kitty 键盘协议与 OSC52 显式禁用。
- 字符尺寸查询有实际 rows/cols；物理像素尺寸未知，回复为 0，不能声称像素协议已支持。
- 实机 IME 候选窗、选区复制、大量输出的 GUI 性能和复杂 Unicode 绘制需独立验收；不声称 Neovim/tmux 完整兼容。
- macOS 首先验证；Windows/Linux 尚未进行平台验收。
- 工作区布局与设置尚不持久化；重启会结束并丢失会话，不能把 PID 当作恢复凭证。

后续继续补齐 Rust 原生终端视图的协议与交互能力。Ghostty 保留为后续对比方案，不是必经迁移阶段。

## 主题与预览

`theme.json` 是配色、字体、尺寸与圆角的唯一设计入口。组件使用语义类和 `token()`。侧栏和浮层均为实色，通过低对比分界、细边框和轻阴影表达层级；不设置玻璃或背景模糊。主题不拥有会话生命周期。详情见 [design.md](design.md)。

`TERMLEVEL_PREVIEW=1` 显式启用确定性 UI 预览，仍不执行系统命令。预览测试通过替换 `Runtime.previewMode` 进入该模式；真实会话测试使用单独的原生绑定与工作区 fixture，以及默认入口的原生 fish 验收。
