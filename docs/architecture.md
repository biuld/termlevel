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

src/lib.rs                       注册 termlevel/native 导出与原生 renderer factory
src/runtime.rs                   版本、启动目录、显式预览开关与原生 cell 测量
src/dialog_keys.rs               对话框 Escape 与 pane 缩放模式的原生键盘优先级
src/terminal.rs                  Ghostty 状态 / RenderState 到 NSH 的适配；预览 provider
src/sessions.rs                  SessionManager、PTY、进程、reader/writer/reaper
  sessions/api.rs                TSX 可调用的显式会话操作
src/session_view.rs              按 ID 绑定会话、低频状态通知、唯一 16 ms 帧调度
  session_view/surface.rs         每会话显示控制器：窗口、失效、轮内合并与一次采样
  session_view/view.rs            NSH 绑定：固定格网、原生 Input、事件策略接线
  session_view/window.rs         展示窗口策略；原生内容锚点留在 Ghostty
  session_view/paint.rs          owned 绘制场景、颜色和行语义
  session_view/presentation.rs   平台无关的最新场景槽、弱注册表与原生属性绑定
  session_view/platform/mod.rs   编译期平台选择与终端挂载 / 输入 / 注册入口
    platform/macos/view.rs       NSView 容器、失效和主线程场景读取
    platform/macos/renderer.rs   CoreText / Core Graphics 绘制与原生缓存
    platform/macos/keyboard.rs   AppKit local monitor 与 IME 按键仲裁
    platform/macos/probe.rs      debug-only 原生输入与绘制验收驱动
    platform/unsupported.rs      后续平台入口；明确显示 renderer 尚未实现
  session_view/input.rs          组合输入和提交状态
  session_view/keys.rs           NSH 按键到 Ghostty 输入事件的映射
```

TSX 在 QuickJS 中执行，经 NSH 引擎由原生平台绘制。`termlevel/native` 是同进程 Rust 绑定，不是网络 RPC。类型声明由 NSH 生成到 `.nsh/types`，不手工维护。目录按职责划分，不为每个组件另建 crate。

## 终端数据通路

默认使用独立 Rust 封装的官方 `libghostty-vt =1.0.1` C ABI 和经过 SHA256 校验的 macOS 预编译静态库。应用不再依赖 `alacritty_terminal`。`portable-pty =0.9.0` 创建 PTY 和子进程。

```text
TSX 工作区 ── open / close / list / shutdown ── Rust SessionManager
                                                   │ 强持有
                                     PTY + child + Ghostty terminal state
                                        │                 │
                               reader 原始字节 ────────────┘
                                        │
                               Rust revision → 原生视图（合并刷新）
                                        │
                               Ghostty RenderState → 固定格网 / 光标 / Ghostty 选区
                                        │
                               NSH 原生输入 → writer → PTY

Rust → TSX：标题、运行/退出/错误状态、响铃；不发送终端 chunk 或 cell grid
```

每个会话有独立 Ghostty terminal、持久 parser、原生 RenderState 与 Rust 帧缓存。默认展示最新 10,000 行历史，设置可在 1,000 到 100,000 之间按 1,000 行调整；备用屏幕没有历史。Ghostty 底层按页回收，物理存储可能额外保留一页。安全层用两个屏幕各自的 tracked grid reference 记录逻辑起点，因此展示的上限精确，改小后再改大不会重新出现已丢弃的旧行。设置不写入磁盘。

reader 直接将原始字节送入同一个 Rust/C parser，保留跨 chunk 的 UTF-8 和转义状态。活动屏幕按需生成拥有独立内存的 RenderState 帧；固定格网只复制当前窗口的行，历史锚点由 Ghostty tracked reference 随重排和淘汰维护，不在每个输出 chunk 复制整份历史。宽字符与组合字符保留列坐标，ASCII 同样式 run 合并。所有终端字节、网格、历史与匹配都留在 Rust。

⌘F 的 ASCII 查询使用 Ghostty 分步 feed/tick，保留原生软换行匹配；Unicode 使用分批复制行、锁外小写匹配。每次最多处理一批，输出变化后等待 150 ms 安静期重扫，不在绘制快照锁里全量查找。低频 SearchChange 只发送 query、计数、索引与 searching；查找栏显示 Searching…，异步完成不覆盖别的 query。新输出不强制跳到命中。预览仍使用 terminal.text() 计次。⌘K 仍按标题、目录和分组查找会话。

程序指定的 ANSI / 256 色 / truecolor 与 OSC 调色板覆盖保留语义；未指定的颜色跟随主题。NSH 的基础 256 色写入 Ghostty 的默认 palette，主题写入默认前景 / 背景，程序覆盖不会被主题变更清掉。颜色、光标和设备协议回复直接经 Rust 回到 writer。字符尺寸查询有实际 rows / cols；NSH 此处只有 point 测量，没有设备像素尺寸，像素回复仍为 0。

同步输出由 mode 2026 的原生回调在确切边界捕获可见屏幕和屏幕身份。已挂载的贴底 surface 可推进到 hold 开始边界保存的完整帧，同一边界复用 Arc；历史窗口保持冻结。初次绑定同样展示 held 帧，不采样事务中的实时屏幕。正常结束、150 ms 超时和 EOF 都会提交输出。真实库测试确认 1.0.1 原生 resize 也会结束 hold，随后发布新尺寸。安全层会让独立 renderer 在回调消费 dirty state 后重新建立快照，避免把一次渲染的状态误当成另一个 renderer 已处理。PTY 与网格 resize 仍在同一个 emulator 锁内完成，防止 SIGWINCH 后的新尺寸输出落入旧尺寸网格。

## 生命周期与并发

`SessionManager` 在 Rust 中独立于视图持有会话。只有 `Sessions.open` 创建进程；`SessionTerminal` 只能按 ID 获取已有会话，已关闭或丢失的 ID 返回错误，不会隐式重新创建进程。ID 由 Rust 生成，与标题、排序、分组、选择、PID 和视图位置无关。

- 默认工作区恢复 `Sessions.list()` 的仍存活或已退出会话；没有会话时启动一个新 shell。这个恢复只发生在同一个原生宿主进程内，不是应用重启恢复。
- TSX 工作区销毁/重建以及原生终端视图卸载/重挂均不关闭会话。重新绑定读取当前 Rust 网格和状态，不依赖 JS 中的输出副本或事件补发。
- 标签切换、分组折叠、重命名、排序与主题变更保留同一 session ID。工作区保留按 session ID 挂载的终端显示；布局树决定哪些会话同时展示，暂未展示的会话隐藏、禁用命中并移出无障碍遍历。焦点 pane 决定唯一输入目标，其他可见 pane 仍持续刷新。
- 显式关闭标签调用 `Sessions.close`；关闭组逐个关闭其会话。Shell 退出码为 0（含空行 Ctrl+D）时同样关闭标签和 pane，不自动重启。非 0 退出和启动失败保留输出与状态。
- 关闭主窗口或退出应用调用 `Sessions.shutdown`。Unix 上结束前台进程组，向 shell 发送 HUP，让它清理普通后台作业；50 ms 后仍未退出则强制结束 shell。主动脱离终端会话的 daemon 不属于可恢复会话模型。
- child 的检查、终止和回收共用控制锁，避免对已回收并可能复用的 PID 发信号。关闭后 reader/writer 取消，reaper 回收 child；应用退出在 NSH 的 200 ms 清理预算内轮询 `Sessions.settled()`。

reader/writer/reaper 在 Rust 工作线程中运行。Unix PTY 使用非阻塞 I/O；reader 以 8 KiB 块解析，writer 的输入队列最多 128 个请求，单次输入上限 1 MiB，拒绝会显示错误，不静默截断。每个会话只有一个 16 ms 帧调度：同一轮最多观察一次输出 revision 与输入代际，并把窗口、悬停、宿主拖选、选区、尺寸、查找 reveal 合并为至多一次采样与发布；事件处理器只登记失效，不直接捕获。亚行滚轮的整数位移与小数余量都保留，报告滚轮不再按单次 32 行截断，而是分块写入且不丢位移。工作区传入低频 visible 属性；隐藏 pane 暂停抓帧、动画推进和查找，恢复可见时立即刷新，非活动但可见的 pane 仍继续绘制。视图刷新计时器随视图销毁，会话工作线程不会随之销毁。

`SessionTerminal` 通过 NSH 0.5.0 的 Native Components 公开接口嵌入应用自己的 NSView。src/lib.rs 的 #[native_components(macos)] 同时供开发与打包宿主注册；无需修改框架。测量视图是唯一 PTY resize 输入，格子宽高来自当前 Frame 行列数；剩余像素留在格子外。贴底 origin=line_count-Frame.rows，ED2 保留的历史不能切入屏幕。绘制规则与验收边界见 [终端格网设计](terminal-grid.md)。

快照分成锁内复制 Frame/历史 cells/图片、锁外合并 run 与可访问文本两阶段。Rust 引擎以 Arc<Scene> 发布完整场景，主线程只短暂锁住场景槽以克隆最新 Arc，随后在无 emulator 锁时绘制；跨 NSH 属性只有槽 ID 与 revision。槽注册表保存 Weak，发布者和原生组件随挂载释放，不拥有会话。字体、CTLine 与 CGImage 留在主线程 renderer。

原生 NSView 按图片层 0、单元格背景、图片层 1、搜索和字形、图片层 2 绘制。字体与字形排版有界缓存，图片按 owned 像素身份复用。run 按 Ghostty 列坐标锚定并独立裁剪，主字体基线一致，关闭可选连字。复杂 fallback、emoji 与细粒度 AX 仍需验收。一个无绘图操作的 Canvas Drawing 仅提供非空可见行 CanvasNode 语义给 NSH 测试模型；原生容器同时有整个窗口文本标签，不挂逐行视图。光标独立，闪烁不重绘字形。预览 TerminalEngine 仍是视图独享模拟器。

唯一原生 Input 与格子同一原点，位置来自展示快照的 input_cursor，y 夹在格子内；组合时显示 marked text 和系统输入法交互，平时文字透明。开始输入/组合请求贴底，hold 期间延后到下一次发布。DECTCEM 隐藏输出光标不移除输入锚点。

`InputState` 缓存组合过程，只有结束后提交最终差量；取消删除 marked text 不发送残留拼音。原生值先确认，再通过 NSH `after_answer` 清空，避免 Rust host 属性比较跳过相同的空值；若已进入下一次组合，暂不清空。普通输入处理清空尚未到达平台时的累积值，避免快速输入重复。组合期间暂停终端按键绑定，Enter/方向键/Escape 交给输入法；NSH 脚本回退绑定只在该会话 active 且 Input focused 时生效；生产 local monitor 以 active 授权，避免布局导致 field editor 失焦时丢键。Command 编辑快捷键不占用工作区新建/关闭标签快捷键。

离开 macOS field editor 时，AppKit 可以确认当前 marked text；回调仍绑定旧 `Arc<Session>`，不向新活动会话转发。挂载、隐藏与焦点不拥有 PTY 生命周期。

## 终端平台接入边界

`SessionTerminal` / `view.rs` 只通过 `platform` 挂载 surface、安装物理输入适配与查询 NSH 脚本输入回退。`SurfaceController`、`Window`、`WindowSnap`、`paint::Scene` 和搜索仍属于通用 Rust 路径。发布通道独立在 `presentation.rs`，仅持有 owned `Arc<Scene>`；宿主通过 token 解析槽，每次绘制调用 `Slot::latest()` 克隆最新场景后立即释放锁。原生对象和 PTY 都不属于这个通道。

当前后端在编译期选择，不使用动态插件或通用 renderer trait。新增平台需要：

1. 在 `platform/mod.rs` 选择新的后端，实现 `surface(scene, label)`、`install_input(session, active, cell_width, line_height)` 和 `synthetic_keys()`；debug 驱动可实现对应的 `track`。
2. 经目标平台的 NSH Native Components 入口注册同名组件 `termlevel.terminal-surface`。`src/lib.rs` 是宿主注册的唯一外部接线点；macOS 使用 `#[native_components(macos)]` → `platform::register_components`。
3. 使用 `presentation::element` 绑定场景生产闭包与无障碍文本闭包；组件收到 `frame` 属性后通过 `presentation::resolve` 取得槽并请求重绘。复用现有 latest-only 传输及释放语义，不向 JS 传 cell grid。
4. 平台宿主与 renderer 负责自己的 UI 线程亲和性、字体 / 图片缓存和资源销毁；surface 不截获指针或拥有 PTY。公共 NSH 输入仍负责 IME 提交、粘贴、布局与普通指针，平台输入钩子只补足原生事件能力。

`platform/macos` 是目前唯一实际实现，内部再区分 NSView 宿主、CoreText 绘制、键盘监听和 debug 探针。其他平台有明确的未实现占位入口，尚不能运行真实 renderer。本次只整理终端 surface 边界；Ghostty 预编译库仍仅支持 macOS，系统剪贴板 helper、工作区原生快捷键和其他平台构建 / 打包也需后续接入。不能从这些钩子推导 Windows / Linux 已可编译或通过验收。

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

真实 PTY、预览 provider 和 Rust 原生网格视图均已切到 Ghostty。渲染由 Termlevel 自有 AppKit NSView 承担，IME 继续使用 NSH field editor；不等同于嵌入 Ghostty 应用的 GPU renderer。

- 已接通应用光标键、F1–F24 与方向 / 编辑键的 Shift / Ctrl / Alt 组合、括号粘贴、焦点报告、标题、响铃、备用屏幕、滚动历史、重排、颜色与尺寸查询。
- macOS 以 AppKit local monitor 提供物理键、布局文字、按下 / repeat / release 与修饰键事件，再交给 Ghostty 协议编码器。监听器按当前 active session 授权，遇到查找、重命名和设置的外部文本编辑器则透传；需要输入法时先取得本会话 field editor；工作区 Command 快捷键、Copy / Paste / Select All、IME marked text 保持原有归属。非 ASCII 输入源上的字符键仍交给 IME，直到提交；无组合文本时 Enter、方向键和功能键继续编码。NSH 快捷键绑定在生产路径透传，避免两个监听器争抢协议按键。`scripts/nsh test` 显式开启仅 debug 构建支持的脚本输入回退；原生协议测试再显式关闭该回退，并向前台 AppKit 窗口同步投递事件。
- 鼠标点击 / 释放 / 拖动 / 移动 / 双轴滚轮按当前 DEC 模式编码。Shift 临时绕过程序鼠标报告，允许终端选区 / 历史滚动；普通拖选、双击词、三击行、Alt 矩形选区由 Ghostty tracked selection 持有，Cmd+C / Cmd+A 面向终端选区。OSC8 目标随 cell 保留，链接有下划线与提示，Cmd+点击打开 HTTP(S) / mailto。
- 原生视图接入 Kitty 内联 RGB / RGBA / PNG 图片，按原生 placement 的裁剪、缩放、偏移、三层 z 顺序绘制；像素留在 Rust owned RGBA，并在原生 renderer 缓存 CGImage，动画使用原生 generation 更新。历史图片快照恢复 viewport 时保留“跟随底部”语义，并使其他 renderer 的脏行缓存失效。文件 / 临时文件 / 共享内存 medium 仍关闭。
- 光标支持 block / bar / underline、程序指定的颜色与闪烁模式；组合输入期间隐藏输出光标。
- OSC52 / 文本剪贴板回调同步接入 macOS 系统剪贴板，单次上限 1 MiB。默认允许复制，读取需设置中的“Read and write”明确开启，重启复位。回调使用有超时的 pbcopy / pbpaste helper，不在持有 parser mutex 时等待 AppKit 主线程，避免主线程与 reader 互等；不支持非文本 MIME 和 primary / selection 剪贴板。
- 实际系统输入法候选列表、跨行选择复制、复杂 emoji、多会话持续输出的性能仍需独立验收；不声称 Neovim/tmux 完整兼容。
- macOS arm64 已完成自动化验证；x86_64、Windows/Linux 尚未进行应用平台验收。
- 布局与设置尚未持久化；应用重启会结束并丢失会话，PID 不能作为恢复凭证。

### libghostty-vt 独立 Rust 封装

工作区新增两个 crate，均不依赖 NSH、PTY 或应用的会话管理：

- `crates/ghostty-vt-sys`（包名 `termlevel-ghostty-vt-sys`）：官方 libghostty-vt **1.0.1** C ABI、原版头文件、预生成绑定，以及通过固定 SHA256 验证的官方 macOS XCFramework。构建提取目标架构静态库，不需要 Zig。支持在 macOS host 上构建 arm64 / x86_64，其他目标明确报错。
- `crates/ghostty-vt`（包名 `termlevel-ghostty-vt`）：句柄 RAII、字节解析、resize、文本导出、拥有独立内存的 viewport 快照、协议回复和元数据事件，以及随当前终端模式编码的键盘 / 鼠标 / 焦点 / 文本粘贴。原生句柄经线程契约审查后为 `Send`、保持 `!Sync`，每次访问均由 `Session.emulator` 的 Mutex 串行化；普通同步 C 回调复制拥有的数据；剪贴板回调调用受限的 Send + Sync host adapter，禁止重入 terminal 或等待 UI 线程。PNG decoder 在创建任何 safe terminal 前通过 Once 安装，输出使用 Ghostty 提供的 allocator。复制后的帧可发送到绘制线程。

这两层只拥有终端模拟状态，Rust `SessionManager` 继续拥有 PTY 与进程。生产路径已接入这两层，旧 Alacritty 依赖已移除。安全层包含历史窗口、搜索、颜色默认值、render hold、tracked selection、超链接目标、拥有像素的图片 placement 与同步剪贴板 adapter，完整 C 声明仍保留在 raw API。默认整 viewport 复制是首版接口，不作为最终 renderer 性能结论。

下载、缓存、离线构建和版本来源见 [sys crate](../crates/ghostty-vt-sys/README.md)，安全层边界见 [wrapper](../crates/ghostty-vt/README.md)。两包尚未发布到 crates.io。

## 主题与预览

`theme.json` 是配色、字体、尺寸与圆角的唯一设计入口。组件使用语义类和 `token()`。侧栏和浮层均为实色，通过低对比分界、细边框和轻阴影表达层级；不设置玻璃或背景模糊。主题不拥有会话生命周期。详情见 [design.md](design.md)。

`TERMLEVEL_PREVIEW=1` 显式启用确定性 UI 预览，仍不执行系统命令。预览测试通过替换 `Runtime.previewMode` 进入该模式；真实会话测试使用单独的原生绑定与工作区 fixture，以及默认入口的原生 fish 验收。
