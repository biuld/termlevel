# 终端 Surface 控制器：事件归口、交互策略与帧调度

日期：2026-11-14

状态：**已实现并接入**（`src/session_view/surface.rs`）。上文保留设计时对【现状】的核对；落地差异见文末「落地记录」。原生 GUI 手感仍待用户人工验收。

模块路径更新（2026-11-15）：通用发布通道已抽到 `presentation.rs`；自定义 NSView 与绘制分别位于 `platform/macos/view.rs`、`platform/macos/renderer.rs`，物理输入位于 `platform/macos/keyboard.rs`。通用视图通过 `platform` 接入；新增后端契约见 [architecture.md](architecture.md) 的“终端平台接入边界”。下方带旧行号的缺口表保留为设计前快照。

## 范围

这一层是 NSH / AppKit 与 Rust `Session` 之间的一个 controller，每个 session 一个。它归一化事件、决定交互策略、登记失效，并调度唯一一次必要的快照与发布。

它**不**是通用事件总线，不重写终端协议，不替换 Ghostty 解析器，不改 NSH，不新增编码器。协议字节仍由 `crates/ghostty-vt` 的 `encode_*` 产生；PTY、进程、Ghostty 状态仍由 `SessionManager` / `Session` 拥有；绘制仍是 `platform/macos/view.rs` 的自有 NSView；TSX 仍只挂载组件和接收低频事件。

目标不是"把所有事件都异步化"，而是让每一类事件找到正确所有权：

- 输入（键、IME 提交、粘贴、按下 / 释放、焦点）保持同步、有序、可显式拒绝。
- 展示（窗口、选区、悬停、查找）只登记失效，每轮最多一次昂贵采样。
- 时间只有一个调度者；动画、闪烁、hold、查找各有明确到期时间。

## 现状：可核对的缺口

| # | 缺口 | 位置 | 具体表现 |
| --- | --- | --- | --- |
| 1 | 两个 16 ms 定时器 | `src/session_view.rs:142`、`src/session_view/view.rs:382` | 前者按 revision 触发输出采样和状态事件；后者处理 input_epoch、闪烁、图片动画和查找。没有统一帧预算与时序。 |
| 2 | 捕获入口分散 | `view.rs:26`（`Grid::capture`）被 `pin`/`reveal`/`wheel`/`output_changed` 和 `Display::new` 调用 | 16 ms revision 轮询不是全局 capture 上限；同一时段可能多次取 emulator 锁并复制窗口。 |
| 3 | 每个 motion 事件多次取锁 | `view.rs:573`（`motion`） | 报告模式：`reporting` 取锁、`report_n` 可能 `pin`（另一次 capture）、`mouse_repeat` 再取锁；宿主拖选：hyperlink 一次、`render_held` 一次、`extend_selection` 一次。 |
| 4 | 报告滚轮 32 截断 | `view.rs:881` | `floor(abs(delta)).min(32)`，余量只保留小数；单事件超过 32 的整数位移被丢弃。这是有损截断，不是限流。 |
| 5 | 历史滚轮逐事件捕获 | `view.rs:87`、`view.rs:868` | 整行事件立即 `capture`，亚行事件不捕获（正确）。整行滚动应与昂贵采样解耦。 |
| 6 | 输入代际会额外触发 pin/capture | `sessions.rs:371`、`view.rs:388` | 任何成功编码的鼠标报告都 `input_epoch += 1`；即使已经贴底，下一次时钟仍可能走一次 pin + capture。 |
| 7 | writer 队列不是送达协议 | `sessions.rs:86`、`sessions.rs:297` | `sync_channel(128)` + `try_send`；单次输入上限 1 MiB；队列满时 `fail()` 把会话标成 error。拒绝可见，但不保证送达。 |
| 8 | 场景覆盖正确，上游采样未收敛 | `native_surface.rs:67`、`view.rs:895` | 场景 latest-only，AppKit 合并失效；但这只避免绘制积压，不避免重复 capture / run 合并 / 可访问文本构造。 |

这些是代码可核对的缺口，**不**证明它们就是真实 Codex 顿挫的主因。实现前后都需要计数与延迟证据。

## 所有权与线程

```text
AppKit 主线程                        引擎线程                               reader 线程
  NSEvent local monitor                NSH 组件 / effects / set_interval       PTY 读取
  ├ 键 press/repeat/release            ├ 手势、滚轮、IME、粘贴、焦点           ├ 解析字节
  ├ 中键 / 侧键                        ├ SurfaceController                     ├ 更新 Ghostty 状态
  └ 手势（经 NSH 派发到引擎线程）      │   ├ Window / dirty / 悬挂中的指针     └ revision += 1
                                       │   ├ 一次 capture → Arc<WindowSnap>
                                       │   └ Signal 更新 → 场景发布
                                       └ Search（查找截止时间与分步预算）
```

- **引擎线程**：NSH 组件函数、`effect`、`set_interval` 回调、键位绑定和指针处理器都在这里，不在 AppKit 主线程（NSH 0.5.0 用 `run_on_main_thread` 才把任务交给 UI 线程）。`SurfaceController`、`Window`、`Signal`、`Search` 全部只属于这条线程。
- **AppKit 主线程**：`platform/macos/keyboard.rs` 的 local monitor 直接编码物理键与中键 / 侧键，再交给 writer；`platform/macos/view.rs` 的 `drawRect` 只从场景槽克隆 `Arc<Scene>`。两处都不等待引擎线程。
- **reader 线程**：持有 `Session.emulator` 解析字节，释放后 `revision += 1`。它不接触 controller 状态。
- **reaper / writer**：只碰 `control` 与 `info`。

锁顺序（既有代码已基本满足，作为约束写死）：

```text
emulator  →  control  →  info
```

- 任何线程不得持有 `emulator` 时同步等待主线程；主线程只在编码当前输入和克隆场景 Arc 时短持有。
- `resize_grid` 在 `emulator` 内完成 PTY resize 与网格 resize（`sessions.rs:393`），这个原子边界保留。
- 场景槽锁只用于替换 / 克隆 `Arc<Scene>`；绘制不持有槽锁，也不持有 `emulator`。

## 控制器状态

新增 `src/session_view/surface.rs`（模块名 `surface`，类型 `SurfaceController`，即交接中的 `TerminalSurface` controller）。

```text
SurfaceController
  session: Arc<Session>
  window: Window                  // 现有整数窗口策略，原样复用
  snap: Signal<Arc<WindowSnap>>   // 已发布窗口
  dirty: DirtySet                 // 只登记，不立即采样
  follow: bool                    // 下一次发布前需要贴底
  hover: Option<(u16, usize)>     // 挂起的指针格点（相对窗口）
  drag_end: Option<(u16, usize)>  // 挂起的宿主拖选终点
  last_error: Option<String>
  clock: Clock                    // reset 时刻、图片到期、查找到期
  stats: Stats                    // 仅 debug / 验收计数
```

`Window` 不变：它仍只拥有整数 origin、pinned、wheel_frac、行列数和主屏幕策略 stash；历史锚点仍由 Ghostty tracked reference 拥有，不能在 controller 里另造稳定行号。

`DirtySet` 只有三个布尔语义位：

- `capture`：窗口、输出、选区、尺寸、主题、字体、查找 reveal 中任一发生变化。
- `publish`：只影响绘制输入而不需要重新采样，例如查找命中集合变化、颜色配置变化。
- `search`：输出变化使旧查找结果失效。

重复登记同一失效不增加代价；一轮结束时一次性处理。

## 事件分类与合并矩阵

"线程"指事件进入 Rust 的线程。"策略"是这一层对它的处理承诺。

| 事件 | 线程 | 策略 | 顺序 / 正确性要求 | 现状 → 将改 |
| --- | --- | --- | --- | --- |
| 键 press / repeat / release、修饰键 | 主 | **同步编码**，不合并、不延后 | 同源严格保序；编码后只请求跟随，不捕获 | 现状同步；将改：不再 `pin()` 立即采样 |
| IME 提交 / 取消 | 引擎 | 同步提交最终差量 | 组合期间的按键仍归输入法；提交归属原 `Arc<Session>` | 不变 |
| 粘贴 | 引擎 | 同步编码 | ≤1 MiB；超限或队列满必须显式可见 | 不变 |
| 焦点进入 / 离开 | 引擎 | 同步编码焦点报告 | 不依赖偶尔丢失的 first responder | 不变 |
| 鼠标按下 / 释放（报告模式） | 引擎 | 同步编码 | 与滚轮、键同源保序；只请求跟随 | 现状先 pin 再编码；将改：编码优先，pin 延后 |
| 鼠标按下 / 释放（宿主选区） | 引擎 | 同步改选区状态 | release 前必须先应用挂起的拖选终点 | 现状逐事件改；将改：按下立即，拖动合并 |
| motion（报告模式） | 引擎 | **同步编码**，不合并 | 1002 / 1003 的位置序列不得被静默丢弃；编码器按当前模式决定是否输出 | 现状同步但会先 pin；将改：不捕获、只请求跟随 |
| motion（悬停链接） | 引擎 | **取最新格点**，每轮最多解析一次 | 结果只影响 tooltip；窗口滚动后按当轮 origin 重新映射 | 现状每事件一次锁 |
| motion（宿主拖选） | 引擎 | **取最新终点**，每轮最多应用一次 | 证明见下 | 现状每事件一次锁 |
| 滚轮（报告模式） | 引擎 | **同步、无损编码**；小数余量保留；按消息上限分块，不丢弃整数 | 不得出现"旧事件套新模式"：不在事件与编码之间插入任何 terminal 状态观察 | 现状丢 >32 的整数 |
| 滚轮（历史 / Shift） | 引擎 | 立即更新整数 `Window`，只登记 `capture` | 亚行只累积手势；方向反转按到达顺序逐段处理，不做净额抵消 | 现状逐事件采样 |
| 中键 / 侧键 | 主 | 同步编码 | 与普通鼠标同一条 `mouse_repeat`；命中检查 bounds 与 visibleRect | 不变 |
| 输出 revision | reader → 引擎 | **合并**，每轮最多观察一次 | 观察后只登记 `capture` | 现状定时器 + capture 内再次观察 |
| resize | 引擎 | 同步，PTY 与网格同锁 | 字体变化即使行列不变也要更新 cell size | 不变；登记 `capture` |
| 查找 | 引擎 | 截止时间驱动，分步有界 | 每步 feed + ≤32 tick；输出变化作废旧扫描 | 不变；由统一时钟驱动 |
| 图片动画 | 引擎 | 到期时间驱动 | 换帧时登记 `capture` 并使选区失效 | 不变 |
| 主题 / 字体 / cell 度量 | 引擎 | 登记 `capture` + `publish` | 字体键不能只看 cell 宽度 | 现状经 revision 间接触发 |
| 可见性 | 引擎 | 隐藏停止采样 / 动画 / 查找；恢复立即观察一次 | 非活动但可见继续绘制 | 不变 |
| 卸载 | 引擎 | 取消定时与挂起工作 | 不得向已释放或新绑定会话投递旧事件 | 现状依赖 owner 生命周期 |

### 为什么拖选可以取最新终点

`extend_selection` 读取当前 tracked selection，只替换 end（`crates/ghostty-vt/src/selection.rs`）。起点在按下时确定，中间的 end 不参与后续调用。因此对同一次拖动，按到达顺序应用全部终点与只应用最后一个终点，最终 selection 相同。双击词、三击行在按下时一次性确定，Motion 不改变它们；矩形选择只由当轮修饰键决定。**报告模式 motion 不适用此证明**，所以它保持同步。

### 为什么滚轮不做延后队列

滚轮报告由 Ghostty 按编码时的终端模式决定是否输出。若把事件排队到下一轮再编码，期间程序输出可能切换鼠标模式 / 屏幕，旧事件就会套上新模式。所以**输入路径不排队**：滚轮在事件到达时同步编码。工作量上限有两种无损做法：

1. 单个事件内把重复报告分块成多条有界 writer 消息（例如每条 ≤256 个报告）。
2. 亚行 / 小数余量继续留在累加器里，等下一事件或下一行。

两者都不丢弃位移。真实的触控板 / 滚轮整数位移是个位数到几十行；分块上限只用于隔离异常值，不改变语义。

## 顺序与屏障

1. **同源保序**：同一线程内，PTY 方向的字节按事件到达顺序产生；展示采样不插队、不重排输入。
2. **分歧边界**：主线程（AppKit monitor）与引擎线程（NSH 处理器）是两个独立事件源，OS 不提供二者的总序。本层只保证各自源内有序，不假装有全局序；也不把其中一个源的事件搬到另一个源去"统一顺序"。
3. **release 屏障**：释放前应用挂起的拖选终点；随后按双击 / 单击语义决定是否清空。
4. **模式变化只影响未来事件**：控制器从不缓存待编码键 / 鼠标 / 滚轮。输出中观察到的模式、屏幕或尺寸变化，只影响之后到达的事件。
5. **查找 reveal 是展示动作**：只修改 `Window` 并登记 `capture`，不编码输入。
6. **hold**：贴底窗口允许发布当前事务开始边界的完整 owned Frame，同一 Arc 不重复发布；历史窗口仍冻结。保留脏位直到结束、静默 150 ms（单次 hold 最长 1 s）、EOF 或原生 resize，以发布最终事务结果。不得依赖 tick 恰好落在两次事务之间。

## 唯一帧调度

`SessionTerminal` 组件只保留**一个** `set_interval(16 ms)`，回调调用 `SurfaceController::tick(now)`。删除 `Display::new` 里的第二个时钟。NSH 的 interval 不堆积迟到的运行，引擎繁忙时自然跳帧。

一次 tick 的固定顺序：

```text
1. 观察 session.revision         → 变了就 dirty.capture，并使旧查找失效
2. 观察 session.input_epoch      → 变了就 follow = true
3. 闪烁相位                      → 只更新独立光标 overlay 的 Signal
4. 图片动画到期？                → 至多一次 tick_images；换帧则 dirty.capture
5. 查找到期？                    → 至多一次有界 step；完成 / reveal 则 dirty
6. 应用挂起的 hover / drag_end   → 每轮至多一次锁
7. 若 dirty.capture 且可见 → 一次 capture；hold 中只允许 owned 边界帧，变化则 set snap
8. 若 dirty.publish 或 snap 变化 → 发布场景（既有反应式 prop 路径）
```

时间来源只有这些：

| 用途 | 来源 |
| --- | --- |
| 输出 / 输入 / 帧节拍 | 唯一 16 ms interval |
| 光标闪烁 | 每控制器 reset 时刻的 500 ms 相位 |
| 图片动画 | `tick_images` 返回的下一到期时间 |
| 查找安静期 / 分步 | 150 ms 截止时间 + 每次 ≤64 行或 ≤32 tick |
| 同步输出 hold | reader 侧 `flush_sync`：非空输出静默 150 ms，单次 hold 最长 1 s；UI 只观察 revision |
| 原生重画 | AppKit `setNeedsDisplay` 合并 |

不新增第二套兜底定时器。动画与光标不触发终端重新采样；只有 `capture` 位会。

## 失效、快照与发布

- **捕获判据**仍是 frame 身份、窗口 origin、count、screen identity。`Frame.dirty` 不能用：capture 会 clean 它。
- `capture()` 只在 tick 内调用（初次挂载与可见性恢复各允许一次立即调用）。事件处理器只登记 `dirty` 与 `follow`。
- `follow` 在 capture 前应用：若 `!window.pinned`，`window.pin()`；已贴底则什么都不做。这样每批输入最多一次窗口更新，而不是每次输入一次。
- 捕获返回三态：`Held`（保留脏位）、`Cached(Arc)`（复用上次窗口）、`Fresh(Arc)`。只有 `Fresh` 且与已发布窗口不同才更新 Signal。
- 发布保持现状：`Scene { snap, config, hits, current }` 由反应式 prop 发布，跨 NSH 属性仍只有槽 ID 与 revision。
- 计数分开记录（仅 debug / 验收）：
  - `capture_attempted` / `capture_held` / `capture_cached` / `capture_fresh`
  - `snapshot_published`
  - `scene_published`（`Publisher` 内自增）
  - `native_draw`（已有 `frames`）
  - `lock_wait`（capture 前后、按需）

  这些数字用于证明"重复采样是否收敛"，不能把 NSH FPS 当作终端发布帧率，也不能把空 canvas 的 0 当作零成本。

## 拥塞与拒绝

这是有限内存下的诚实契约，不承诺无限送达：

- **键 / 焦点 / 滚轮 / 按下释放**：同步尝试编码并写入 writer；`try_send` 失败必须显式可见，不静默丢弃。同源顺序保持。
- **粘贴**：单次 ≤1 MiB，超出立即拒绝并可见；不截断。
- **writer 队列**：固定 128 条消息。它是拒绝边界，不是背压协议；不改成无界队列，也不把队列满说成可恢复。
- **场景**：latest-only，后发布覆盖前一个；绘制不会积压。
- **采样**：每轮至多一次 capture；输出再密集也只合并。
- **不做的事**：不靠拉长 debounce 换"看起来稳定"；不对可靠事件套 debounce。

现状 `fail()` 会把会话标成 error。设计上保留"拒绝可见"这一要求，但建议把**瞬时输入拒绝**与**会话失败**区分成两个低频状态（例如状态行显示"输入繁忙"，进程仍 running）。这一条在实现前单独确认，不阻塞调度层。

## 生命周期

- **隐藏**：停止 capture、查找推进和动画；revision / bell / status 仍观察，侧栏标记不受影响。
- **恢复可见**：立即观察 revision 并做一次 capture，不等待下一次 tick 的旧状态。
- **非活动但可见**：继续按同一节拍刷新；`active` 只控制输入授权与焦点。
- **卸载**：interval 随 owner 释放，键监听器走既有 cleanup。控制器不持有 PTY 生命周期；不得把挂起的指针 / 查找结果投递给已释放或新绑定的 session。
- **重绑**：新组件用当前 emulator 状态重新初始化 `Window` 和快照，不依赖旧 JS 副本，也不补发旧事件。

## 将移除的旧路径

实现完成后不应长期双轨：

- `Display::new` 的第二个 `set_interval`。
- `Display::pin` 在报告 / 文本 / 组合 / 粘贴路径里的立即 capture。
- `Grid::wheel` / `reveal` 的逐事件 capture；改为登记。
- `view.rs` 中悬停 / 拖选的逐事件 emulator 锁。
- 报告滚轮的 `min(32)` 丢弃。
- `session_view.rs` 与 `view.rs` 各自持有的 revision / epoch 观察副本。

`Window`、`paint.rs`、`platform/macos/view.rs`、`platform/macos/keyboard.rs` 的职责不变；`Search` 从 `view.rs` 原样移到独立模块即可，不改语义。

## 测试计划

先纯状态机 + 可控时钟，再接真实 Ghostty 编码器：

1. **调度**：一轮内 100 次 revision → 恰好 1 次 capture；无失效 → 0 次；`Held` 保留脏位并在下一次提交。
2. **跟随**：贴底时按键 / 鼠标报告不触发 capture；未贴底时一次输入只触发一次 pin + capture。
3. **滚轮无损**：单事件 65 行 → 65 个报告，无丢弃；亚行累计 4 次才移动一行；方向反转按到达顺序产生对应段，不做净额。
4. **屏障**：press → motion × N → release 只应用最后终点；release 前终点已应用；键与滚轮交错时各自同源保序。
5. **可见性**：隐藏时输出不 capture；恢复时立即 capture 最新窗口；卸载后挂起工作不执行。
6. **hold**：连续事务在每次 tick 都 held 时仍推进已完成边界帧；同一边界复用，未完成文字和切屏不泄漏，历史保持冻结，脏位保留至事务释放；resize 后提交新尺寸。
7. **查找**：安静期与分步预算；输出变化作废旧 query；完成不强制跳转。
8. **真实编码器**：报告模式下滚轮分块字节与实际模式一致；中键 / 侧键与普通鼠标同一序列。

测试沿用现有模式：纯 `Window` 数学留在 `window.rs`；真实库测试放在 `terminal/tests.rs` 或新的 `surface` 测试模块；不以墙钟阈值断言性能。

## 验收

自动化之外仍需用户人工运行：

```sh
./scripts/nsh run --window-size 1200x780
```

重点看真实 Codex 的快慢滚动与反向手势、Shift 历史滚动、拖选 / 双击 / 三击、中键 / 侧键、中文 / 日文候选窗、多 pane 持续输出和焦点切换。性能对照沿用 `scripts/test-native-scroll`，保持相同窗口、字号、命令与负载，并记录构建模式；新增计数只用于说明采样是否收敛，不作为手感结论。

## 实现顺序（承接交接文档）

1. 本设计。
2. 纯状态机与可控时钟测试。
3. 引入 `SurfaceController`，收拢两个时钟与全部 capture 入口；`platform/macos/keyboard.rs` 的中键 / 侧键继续走同一条输入路径。
4. 加 debug 计数并做同负载对照。
5. 删除旧入口，更新 `architecture.md`、`terminal-grid.md`、`roadmap.md` 与 `validation.md`。

## 落地记录（2026-11-14）

`SurfaceController` 已落地在 `src/session_view/surface.rs`，`src/session_view/view.rs` 改为 NSH 绑定与事件策略接线。

已删除的旧路径：

- `view.rs` 里的 `Grid` 与第二个 16 ms `set_interval`；现在只有 `session_view.rs` 的一个帧调度。
- 报告 / 文本 / 组合 / 粘贴路径里的立即 `pin()` + capture；输入只通过 `input_epoch` 请求跟随，已贴底时不采样。
- `Grid::wheel` / `reveal` 的逐事件 capture；历史滚轮只改整数窗口并登记失效。
- 报告滚轮的 `min(32)` 整数丢弃；每轴整数位移全部编码，小数余量保留。
- 悬停与宿主拖选的逐事件 emulator 锁；两者在轮内各取最新值，一次解析 / 应用。
- `Session::mouse_input` / `mouse_repeat` / `selection_changed`。所有鼠标入口（含 AppKit 中键 / 侧键）统一走 `Session::mouse_reports`：同一把锁判定当前模式并编码，返回是否被程序接收。

与设计稿的差异：

- `Search` 仍留在 `view.rs`，只改为持有 `SurfaceController` 与会话；没有为它新增模块。
- 滚轮不做延后队列，因此不需要整数余量的跨轮保留：编码在事件到达时完成。单个事件 ≤1 MiB 只发一条 writer 消息；异常大的序列才按 1 MiB 分块。
- `release` 是显式屏障：取指针最终格同步应用终点，并丢弃挂起的中间终点，避免点击清空后被旧终点重新选区。
- 恢复可见时在 effect 内立即 `request_capture()` + `tick()`，其余采样都在帧内。
- 计数仅在 debug 构建、设置 `TERMLEVEL_SURFACE_STATS` 时每 120 个 tick 输出一次（attempted / held / cached / fresh / published）。

仍需人工确认：真实 Codex 手感；瞬时输入拒绝是否要与会话 `error` 状态分离。writer 队列满目前仍保留原有可见错误语义，未改成独立状态。

## 连续同步事务修复（2026-11-15）

真实 Codex / Claude 复现日志中出现 111 次报告滚轮、115 次 revision 变化，但约 2 秒仅发布一次；controller 连续等待发布达 2100.8 ms。原有 hold 全跳过策略会错过两次事务之间的完成边界。

现在 `begin_window` 对贴底窗口直接复用 hold 回调已保存的完整 Frame、屏幕身份、颜色和图片，锁外完成 run 合并，不额外抓取 live cells。历史窗口保留冻结策略；主屏幕策略 stash 通过候选 Window 验证后再应用，避免提前消费。无须缩短超时或显示尚未完成的事务内容。

`held` 计数现在表示 hold 内复用旧窗口的轮次，`hold ms max` 表示直到可发布帧推进的等待时间；`attempted/cached` 包括 hold 中的廉价边界检查。当前 debug 构建每两秒记录诊断，`TERMLEVEL_SURFACE_LOG` 可指定文件（默认 `/tmp/termlevel-surface.log`）；不需要 `TERMLEVEL_SURFACE_STATS` 开关。

确定性测试覆盖连续 160 次事务、同边界复用和未完成切屏隔离。原生滚轮 fixture 新增 `--overlap-sync` 模式，要求输入持续期间每 500 ms 都能看到帧号推进。具体执行结果见 [validation.md](validation.md)。实际 Codex / Claude 修复后的手感仍待用户复验。

用户随后复验并确认“可以了”；本轮滚动修复已获人工确认，其他兼容性和持续多 pane 验收边界不变。
