# TerminalSurface 中间层交接

日期：2026-11-14

> 历史交接快照：下文“尚未实现”和工作区状态描述对应当时的交接。当前 controller 已接入，2026-11-15 又修复连续同步事务的发布饥饿。接手请以 [terminal-surface.md](terminal-surface.md)、[validation.md](validation.md) 和实际 git status 为准。

> 后续模块整理：旧 `native_surface.rs` 已拆为通用 `presentation.rs`、`platform/macos/view.rs` 与 `platform/macos/renderer.rs`；`keyboard.rs`、`native_probe.rs` 分别迁至 `platform/macos/keyboard.rs`、`platform/macos/probe.rs`。下文路径保留作历史记录，当前接入契约见 [architecture.md](architecture.md) 的“终端平台接入边界”。

## 当前任务与工作区

仓库：`/Users/biu/Projects/termlevel`，分支 `main`。

最新提交：**`57a846e` — Render terminal grids in an app-owned AppKit surface**。上一提交为 `4e8c7ff`（鼠标报告、内联图片、选区、OSC52、Kitty 键盘）。当前渲染代码、设计与验证记录都已在 `57a846e` 提交，**没有推送**。生成这份交接前工作区干净；交接文档本身留作未提交文件。

用户接下来要推进一个可靠的终端 surface 中间层，统一事件适配、交互策略、失效与帧调度。**这层尚未实现**；下面是交接方向和待验证的约束，不是已经落地的 API。当前这一轮只生成交接，不继续改实现。下一位接手后先核对 checkout/status 和 AGENTS.md，再读 [architecture.md](architecture.md)、[roadmap.md](roadmap.md)、[terminal-grid.md](terminal-grid.md)。

不要直接把这个任务理解为引入一套通用事件总线、重写终端协议、重换 renderer 或修改 NSH。用户偏向找到正确所有权，避免在每个回调里分别加补丁。

## 用户已确认的方向

用户先怀疑 NSH 与 Ghostty 集成不足，确认卡顿发生在 **Codex 界面内部的鼠标报告滚动**，不是宿主历史列表。固定格网替换虚拟列表后仍觉得卡；原生 CPU 采样发现 NSH Canvas 文字回放中的字体创建 / CoreText 匹配昂贵。

曾讨论修改 NSH 或整帧转图片，但随后查到 **固定的 NSH 0.5.0 已有公开 Native Components hook**。用户确认采用“自己实现 Ghostty 的渲染层，原生嵌入 NSH”。该实现已提交。不要再绕回改框架或全帧图片方案。

用户进一步问事件是否全是 Ghostty 原生、是否做好防抖限流。已说明：事件接收与策略属于宿主，协议编码属于 Ghostty；现在只有部分合并和队列保护，不能声称完整限流。用户认可讨论一个可靠中间层，并要求先提交工作区，然后留下本交接。

## 产品与不可破坏的边界

- Termlevel 是 NSH **0.5.0** mixed 应用。TSX 拥有工作区，Rust SessionManager 独立持有真实 PTY、进程与 Ghostty；挂载/卸载 surface 不创建或关闭会话。
- 所有 `nsh-*` 和 CLI 同一固定版本。libghostty-vt **1.0.1**，安全封装在 `crates/ghostty-vt`，sys 在 `crates/ghostty-vt-sys`。
- libghostty-vt 负责解析、网格/历史、模式、选区及输入协议编码，**不包含应用 renderer、原生事件循环、历史滚动策略或 IME**。
- Terminal / RenderState 是 Send + !Sync，继续通过 `Session.emulator` 互斥访问。不能为了调度方便共享无锁裸句柄。
- 预览 transport 不执行命令。稳定 session ID 不随选择、折叠、排序和换组改变。
- 不改 NSH 源码，不升级版本，不引入 WebView，不把终端流、完整 cell grid 或图片像素绕到 TSX。
- pane 标题保持启动路径 `tab.cwd`；不做 OSC7。Kitty file/temp/shared-memory medium 继续关闭，直接内联 PNG 继续支持。
- 不恢复 virtual_list、PaintedScreen、逐行视图、列表遮罩、footer 或列表滚动 API；不添加 wheel cancel。
- 唯一原生 Input / field editor 保留；非 ASCII 字符键继续交给 IME，查找/重命名/设置的编辑器不能被终端键盘监听器截走。
- 未再次明确要求提交时，不自动提交；不推送。

## 已落地的路径与代码入口

```text
TSX workspace
  SessionTerminal(session ID, active, visible, 配置)
    AppKit / NSH 事件适配
    Display / Window / Search（目前职责散在这里）
      Session -> Ghostty encode_* -> 有界 writer 队列 -> PTY
      Session.emulator -> owned WindowSnap -> Arc<Scene>
        自有 NSView -> CoreText / Core Graphics
```

| 文件 | 当前职责 / 接手重点 |
| --- | --- |
| `src/lib.rs` | crate 根的 `#[native_components(macos)]` 注册 app-owned renderer；同时注册原有 native module |
| `src/session_view.rs` | 组件绑定、低频状态与搜索事件；一个 16 ms revision 检查定时器 |
| `src/session_view/view.rs` | `Grid::capture/pin/reveal/wheel`、Display、Search、指针策略、另一个 16 ms 时钟、布局/Input/光标接线；中间层主要提取来源 |
| `src/session_view/keyboard.rs` | AppKit local monitor，键盘 / IME 仲裁，以及中键、侧键入口；不能只迁移 view.rs 而漏掉这条路径 |
| `src/session_view/window.rs` | 纯整数窗口策略、滚轮小数、主屏幕策略 stash 与命中 |
| `src/session_view/paint.rs` | owned Scene / PaintConfig、图片矩形、行语义 metadata |
| `src/session_view/native_surface.rs` | Publisher / 弱槽注册表、实际 NSView、字体/CTLine/CGImage 缓存、原生绘制 |
| `src/terminal.rs` | Ghostty 状态适配、begin_window / WindowCapture::finish、输入编码与选区接口 |
| `src/sessions.rs` | PTY reader/writer/reaper、revision/input_epoch、输入队列、resize 原子边界 |
| `src/session_view/native_probe.rs` | debug-only AppKit 自动化、实际 wheel 投递、原生绘制次数；不是生产接口 |

新 renderer 是自有 **CoreText / Core Graphics NSView**，不是 Ghostty 应用的 Metal renderer，也不依赖 Ghostty 内部嵌入 ABI。NSH 容器负责布局与原来事件命中。绘制 NSView 的 hitTest 返回 nil，不成为 first responder。

场景由 Rust 引擎以 Arc 发布；跨 NSH 的 frame 属性只有槽 ID / revision。主线程取场景锁仅用于克隆 Arc，随后无 emulator 锁绘制。槽只保存最新场景，注册表保存 Weak；发布者和组件释放后注销。原生字体、CTLine、CGImage 不放进 Session。

绘制顺序：图片层 0 → 单元格背景/选区 → 图片层 1 → 搜索/文字/装饰 → 图片层 2，上面仍是独立 NSH 光标和唯一 Input。字体/CTLine 缓存最多 2048 项与 256 KiB 源文字；字体变化失效，颜色变化不重新排版。图片缓存只保留当前快照引用的 Pixels。

还有一个 **无任何绘图操作** 的 canvas，仅提供 CanvasNode 可见行 metadata 给 NSH 测试驱动，不是第二套 renderer。容器带整个可见窗口文本标签；细粒度 AppKit AX 行子元素仍未实现。

## 中间层需要解决的具体缺口

1. `session_view.rs` 和 `Display::new` 各有 16 ms 定时器。前者按 revision 触发输出采样；后者处理 input_epoch、闪烁、动画和查找。它们没有统一的帧预算或时序。
2. `Grid::pin/reveal/wheel` 可立即 capture；鼠标事件、输入、定时器可能在同一时段多次取 emulator 锁/快照。**16 ms revision 轮询不是全局 capture 上限**。
3. `Display::motion` 逐事件读取链接、修改选区或编码协议，没有统一的语义合并。要分别看 hover、宿主拖选和程序鼠标报告，不能一律取最新点。
4. `Display::wheel` 的报告模式每事件每轴 `floor(abs(delta)).min(32)`，但余量只保存小数。**超出 32 的整数位移被丢掉**；这是有损的单次工作量截断，不是正确的时间限流。
5. 非报告历史滚轮按整行累积，但仍可能逐事件 capture。应该让逻辑窗口更新与昂贵采样解耦。
6. `Session::mouse_repeat` 单锁编码一批字节，减少了每格一次锁；但不是跨事件调度器。成功编码的所有鼠标入口会增加 input_epoch，即使当前已贴底，下一次时钟也可能走 pin/capture。需测量并区分“用户要求跟随”和“确实有展示变化”。
7. writer 用 `sync_channel(128)` + `try_send`，单次输入上限 1 MiB。满了会明确报错，**不是保证送达的背压协议**。不可把当前机制描述成无损可靠队列，也不能用无界队列掩盖拥塞。
8. 新 NSView 使用 setNeedsDisplay / AppKit 合并失效，但没有统一的显示同步调度器；场景覆盖可避免积压绘制，**不能避免上游重复采样**。

这些是代码可核对的缺口，不代表已证明它们是剩余 Codex 顿挫的主因。实现前后需要事件/快照计数和端到端延迟证据。

## 建议的抽象与可靠性契约

以每个 session 对应的 `TerminalSurface` controller 为边界，协调可见性、窗口、交互和刷新。它可以持有 Arc<Session>；SessionManager 继续是进程生命周期的拥有者。不要一开始建立大规模 trait 层次或通用 event bus；先用小而可测试的状态机明确行为，再决定具体模块/API。

```text
AppKit / NSH adapter
    -> surface controller（归一化事件、交互策略、调度）
        -> 原 Session / Ghostty 接口
        -> owned RenderSnapshot / Scene
            -> native renderer
```

必须在设计与测试中明确：

- **可靠事件**：按键 press/repeat/release、IME 提交、粘贴、鼠标按下/释放、焦点变化不能套普通 debounce；保持顺序，拒绝必须显式可见。区分“正常负载不丢”和“过载如何拒绝”，不要承诺有限内存下无限保证送达。
- **可合并状态**：hover 可取最新位置；宿主拖选在边界事件之间可尝试取最新终点，但双击词、三击行、矩形选区等语义须有证明。程序报告模式要核对 Ghostty 编码器的模式与状态，不能假定所有 motion 都可丢。
- **滚轮不丢位移**：限制的是一轮工作量，未处理整数余量必须保留。方向反转不能随意净和抵消：TUI 和历史边界夹取都可能让 down/up 与净位移产生不同结果。需要有序分段，或以测试证明可合并条件。
- **事件屏障**：键、按下/释放、修饰键变化、模式/屏幕/尺寸变化与先前积压滚轮的顺序要明确。延后编码不能把旧事件套上新的 terminal mode；中键/侧键与普通鼠标不能变成两条互不保序的通路。
- **一次帧调度**：输出、窗口、选区、主题、搜索、图片变化只登记失效；同一调度轮次最多做一次必要 capture / publish。缓存命中、原生重画与终端重新采样是不同操作，要分别计数。
- **时间由唯一调度者管理**：动画按到期时间，光标只改独立 overlay；hold 超时、查找分步预算、可见性恢复不能因删定时器而失效。可以先统一到一个明确时钟，再评估公开显示同步接口；不要新增第二套兜底定时器。
- **生命周期**：隐藏时停止无用展示工作，恢复显示立即取得当前状态；非活动但可见 pane 继续绘制。卸载取消回调、清理 pending 工作，不能向已释放或新绑定会话投递旧事件。
- **并发**：定义锁顺序和线程亲和性；不要从主线程同步等待引擎、同时引擎又等待主线程。原生 UI 对象仍留在主线程，绘制不锁 Ghostty。

新抽象复用现有 encode_key / encode_mouse / encode_focus / encode_paste，不另写编码器。实现后移除旧入口中的重复策略与旧时钟，不能长期双路径或依赖功能开关。

## 必须保留的展示与输入不变量

- 根 padding、唯一 resize 测量盒、顶左整格绘制矩形是三块不同矩形；格子外剩余像素不滚动、不起选区、不接中键。process_mouse 检查 bounds 和 visibleRect。
- 贴底 origin = line_count - Frame.rows；不能拿像素底部对齐重新露出 ED2 保留的历史。历史与活动屏幕共用同一格网。
- 历史内容锚点由 Ghostty tracked reference 拥有；不退回按 native_top 数字或 scroll_base 猜测。备用屏幕 stash 主屏幕策略。
- 正常快照一次 emulator 锁内取 Frame、窗口所需 cells/图片与颜色上下文，锁外合并 runs。活动图片直接用 frame.images；不把按 live State.size 算的 images_at 用于 held 帧。
- 同步输出期间冻结完整已发布 WindowSnap，初次绑定用 held 帧；不要混入 live 屏幕身份或尺寸。**本机固定 1.0.1 resize 会结束 hold**；切备用屏幕本身不会结束 hold。已有测试证明，不能恢复旧交接的错误假设。
- 不以 Frame.dirty 判断是否跳过：capture 会 clean，它是旧的捕获值。依赖 owned frame 身份、窗口、配置和搜索失效。
- 输入法继续原 field editor；active 授权不依赖偶尔丢失的 first responder。需要 IME 时先 claim_field，非 ASCII 字符键留给输入法。组合提交、取消、after_answer 清空及切换会话后提交归属保留。

## 推荐推进顺序

1. 先补充/修正设计，给出 controller 的状态、线程、事件合并矩阵、屏障规则、拥塞策略和时间来源；标清哪些是现状、哪些将改变。只在必要处新增模块。
2. 用纯 Rust 状态机与可控制时钟验证调度和顺序，再接真实 Ghostty 编码器。至少覆盖大滚轮 >32、亚行累计、方向反转、模式/屏幕切换、鼠标 press→motion→release、键/滚轮交错、队列满、隐藏恢复和卸载取消。
3. 收拢现有 Display/Grid 的失效和 capture 入口，统一两个时钟；保留 renderer。把中键/侧键和键盘监听器接到明确的边界，验证不会重复消费或同时跨两条通路重排。
4. 给快照尝试/实际复制/场景发布/原生 draw 分别加可用于 debug 验收的计数；测量最高频与过载情形，不靠增加 debounce 时长换“看似稳定”。
5. 移除旧策略与调度入口，更新 architecture、terminal-grid、roadmap、validation；完整回归并做同负载性能对照。用户仍需人工验收真实 Codex。

如果中途发现“全部事件先排队”会破坏当前输入延迟或按错误模式编码，应修正抽象边界，不为结构整齐强行改成异步。也不要把可靠层等同于所有事件都延迟一帧。

## 已有验证与性能证据

`57a846e` 提交前通过：

- `./scripts/nsh check`、`./scripts/nsh build`。
- `./scripts/nsh test --junit artifacts/native-surface-tests.xml`：**106 / 106**。
- `cargo clippy --workspace --all-targets -- -D warnings`、fmt、diff check。
- 原生 Input **1 / 1**、Protocols **3 / 3**、生产 Ghostty Dark/Light **2 / 2**、Scroll **1 / 1**。详细边界见 [validation.md](validation.md) 最后一节。

本机被忽略的 `artifacts/` 中已确认存在：

| 文件 | 证据 |
| --- | --- |
| `scroll-before.log` / `scroll-before.sample.txt` | 旧 NSH Canvas 回放约 24–26 ms，NSH 帧率计约 38–44/60；采样热点含逐 fill_text 字体创建 |
| `scroll-native-surface.log` | 第一轮自有 NSView 绘制约 2.6–2.9 ms，NSH 帧率计多数 54–59/60 |
| `scroll-native-surface-sustained-final.log` | 25 秒滚动完整通过；缓存达到 2048 后约 2.7–3.1 ms，后段 NSH 计约 54–60/60；实际原生绘制计数 905 |
| `native-surface-tests.xml` / `native-surface-tests.log` | 106 项通过 |
| `native-surface-{check,build,clippy,ime,protocols,ghostty}.log` | 分项命令与原生流程 |

性能 fixture：1200×780、Menlo 13 pt、21 pt 行高，真实 PTY 中的鼠标报告 TUI，2026 同步输出、中文与彩色 run。它是受控类似负载，**不是实际 Codex 录制**。NSH FPS 不等于终端快照发布 FPS，也不等于输入延迟；不能把 scene latest-only 或空 canvas 的 0 ms 当作零成本。具体原生 draw 成本由自有 renderer 单独统计。

第一次延长到 25 秒时碰到测试默认 30 秒总预算，失败日志 `scroll-native-surface-sustained.log` 保留；已将该长测试显式设置 60000 ms，最终通过。不是通过删断言绕过失败。

## 下一位可执行的验证命令

官方 archive 在本机 `/tmp`，不需要网络或 Zig。脚本会校验固定 NSH 版本。

```sh
cd /Users/biu/Projects/termlevel
export GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip
export CARGO_NET_OFFLINE=true

./scripts/nsh check
./scripts/nsh test --junit artifacts/surface-controller-tests.xml
./scripts/nsh build
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
git diff --check

./scripts/test-native-input -v
./scripts/test-native-protocols -v
./scripts/test-native-ghostty -v
./scripts/test-native-tiling -v
./scripts/test-native-scroll -v
```

原生 GUI 脚本串行运行，避免抢焦点；性能轮次期间不要并行编译或跑其他负载。日志另存新文件，保留现有前后对照。脚本会在仓库根创建临时 `.native-*.test.tsx`，退出时清理。截图/XML/log 都在忽略的 artifacts，不提交。受限沙箱可能禁止本地 dev server 监听；需要按工具权限流程运行，不通过换成 headless 冒充原生通过。

`test-native-scroll` 的 fixture JS interval 仅用于产生自动化输入，不是生产渲染的数据通路；它通过 NativeInputProbe.wheel 投递实际 AppKit wheel。NSH 自带 test scroll 要求 ScrollView，不能直接用它滚当前非滚动的 grid。协议/选区测试保存并恢复系统剪贴板，不破坏此行为。

用户人工运行：

```sh
./scripts/nsh run --window-size 1200x780
```

重点看实际 Codex 的快慢滚动和反向手势、Shift 历史滚动、clear 顶边、中文/日文候选窗、拖选/复制、OSC8、图片层与动画、多 pane 持续输出以及焦点切换。不要从测试数量宣布手感已解决。
