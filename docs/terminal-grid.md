# 终端视图：固定单元格网格

日期：2026-11-14

状态：原生 NSView renderer 已接入；受控滚动性能有对照，真实 Codex 与日用验收待完成

事件归口、交互策略与唯一帧调度的设计与落地记录见 [终端 Surface 控制器](terminal-surface.md)（已实现）。

## 问题与边界

Termlevel 使用官方 libghostty-vt 1.0.1 解析终端输出、维护网格和历史、编码输入。它不包含 Ghostty 应用的 renderer、滚动手势或输入法。接通协议不代表已经得到完整终端 surface。

原来的生产路径把终端逻辑行放进 NSH `virtual_list`，可见行各自挂绘制、图片、链接和光标节点。`ListAlignment::Bottom` 在非整行视口里能切入上一行历史；程序鼠标模式的滚轮则触发 TUI 重画活动屏幕，列表节点和贴底操作仍参与刷新。继续调整列表估计高度、overdraw 和遮罩不是本设计的路径。

列表是已确认的结构性问题，固定格网替换后用户仍感到 Codex 内滚动卡顿。随后在同尺寸、同字体的鼠标报告 TUI 中测得 NSH Canvas 原生回放约 24–26 ms；CPU 采样集中在逐 fill_text 的字体构建和 CoreText 匹配。节点数减少没有解决文字回放成本，因此绘制进一步移到应用自己的 NSView。该受控负载不是实际 Codex 的录制。

所有权保持：TSX 管工作区；Rust `Session` 管 PTY、进程与 Ghostty；Rust 原生 surface 管展示窗口和交互；NSH 管工作区布局和挂载；Termlevel 的自定义 AppKit NSView 管绘制；原来的 NSH field editor 管原生文本输入。Terminal 与 RenderState 为 Send + !Sync，访问继续由 `Session.emulator` 串行化。稳定 session ID 不随折叠、重排、换组和选择改变。工作区以低频 visible 属性通知实际展示状态；隐藏 surface 不因输出抓帧、推进查找或发布绘制场景，恢复展示时立即抓取最新窗口。非活动但仍可见的分屏继续刷新。

不改 NSH 或其固定 0.5.0 版本，不把终端流和 cell grid 送进 TSX。预览 transport 仍不执行命令。pane 标题保持启动路径 `tab.cwd`，不做 OSC 7。Kitty file/temp/shared-memory medium 继续关闭；图片 RGBA 不写成文件。OSC 52 策略不变。不加 wheel cancel，不往滚动内容里放 footer。

## 业界实现提供的约束

参考的是上游当前主分支的架构，不声称其内部 API 与本项目所用 1.0.1 一致。落地 API 必须核对仓库内的固定版本头文件。

- [Alacritty text renderer](https://github.com/alacritty/alacritty/blob/master/alacritty/src/renderer/text/mod.rs) 直接消费可绘制单元格，结合 glyph cache、atlas 和批量绘制。固定网格符合这一方向，但两张 NSH canvas 并不等同于这种文字 renderer。
- [WezTerm Screen](https://github.com/wezterm/wezterm/blob/main/term/src/screen.rs) 区分物理行号和 StableRowIndex，并维护历史淘汰偏移。宿主必须定义稳定锚点，不能把当前 screen 行号误当成永久内容 ID。
- [Ghostty render state](https://github.com/ghostty-org/ghostty/blob/main/src/terminal/render.zig) 支持锁内 beginUpdate、锁外 endUpdate。[Alacritty display](https://github.com/alacritty/alacritty/blob/master/alacritty/src/display/mod.rs) 同样尽早释放 terminal 锁。目标是短临界区，不是追求「任何事情都放进一把锁」。
- [Ghostty renderer](https://github.com/ghostty-org/ghostty/blob/main/src/renderer/generic.zig) 在同步输出期间暂停画面更新，并缓存文字 shaping。冻结展示状态与缓存排版都是独立于 UI 节点数的契约。
- [Ghostty AppKit surface](https://github.com/ghostty-org/ghostty/blob/main/macos/Sources/Ghostty/Surface%20View/SurfaceView_AppKit.swift) 自己实现 NSTextInputClient，协调 interpretKeyEvents、marked text、提交文本和候选窗几何。Termlevel 当前的 local monitor + NSH field editor 是不同适配路线，需要单独证明输入正确。

采用固定网格与应用拥有的原生 renderer。NSH 0.5.0 的公开 Native Components 接口允许应用 crate 用 #[native_components(macos)] 注册 NativeComponent，返回自己的 NSView，再以 native(name) 参与布局。无需修改 NSH、升级框架或整帧转图片。接口说明见 [NSH Native Components](https://nsh.dev/docs/native-components/)，落地签名以本机固定的 0.5.0 crate 为准。

本次实现是 Termlevel 自己的 CoreText / Core Graphics renderer，不是 Ghostty 应用 Metal renderer 的嵌入，也不依赖 Ghostty 内部嵌入 ABI。libghostty-vt 继续只提供状态和编码。未来是否需要 GPU atlas 由新的实际瓶颈决定。当前只实现 macOS renderer，其他平台需独立实现和验收。

## 三块矩形

1. SessionTerminal 根继续保留宿主 padding 和终端背景；点击根可以请求焦点。
2. 没有 test id 的 flex_1 测量视图是唯一 resize 输入。按尺寸与 cell metrics 的 floor 算 rows/cols，至少 1。非正 viewport 不发 0×0。PTY 和模拟器 resize、cell size 更新在同一把 emulator 锁内进行。字体变化即使没有改变行列数，也必须更新 cell size。
3. terminal-grid-{id} 是测量视图中顶左对齐的显式矩形。宽为绘制 Frame.cols × cell_width，高为 Frame.rows × line_height。不能反过来用这个矩形的布局做 resize 输入。

格子外的剩余像素只显示终端背景，不滚动、不开始选区、不接中键。格子内的拖动越界才夹到首末格。AppKit process_mouse 同时检查格子 bounds 和 visibleRect，避免 hold 或缩小窗口时命中被父视图裁掉的区域。

## 整数窗口与稳定历史锚点

`Window` 只拥有整数 origin、pinned、wheel_frac、绘制行列数、逻辑总行数和主屏幕策略 stash。它不存一个伪稳定的 native_top 数字。

- 贴底：origin = line_count.saturating_sub(Frame.rows)。绘制从 y=0 开始。额外像素留在格子之外，ED2 clear 保留的历史不能切入活动屏幕。
- 用户改变窗口：夹到 0..=bottom；实际整行位移落在 bottom 时恢复 pinned 并清掉小数。小于一行的滚轮事件只累积手势，不发布新窗口；即使仍贴底也要保留这些尚未凑成一行的小数，否则慢速触控板永远无法离开底部。新输出跟随、显式 pin 和到达边界会清掉累积。手势累积不作为亚行绘制偏移。
- 未贴底跟随输出：从 Ghostty tracked reference 解析锚点，再减当前 retained range.start；被淘汰或不可用时夹到最老的逻辑行。
- 缩窄、放宽和重排后，锚点跟随原内容；不能只靠当前行号减去 scroll_base。

安全封装的 Terminal 拥有一个宿主滚动锚点，提供 set_scroll_anchor / scroll_anchor_row。native tracked handle 不出 Terminal，设置、读取、释放都随 terminal 串行化；不把独立裸句柄交给 UI。一个生产 session 只有一个稳定 surface，预览有独立模拟器。

进入备用屏幕保存主屏幕的 pinned 和 wheel_frac，origin=0；主屏幕的 tracked anchor 留在 Terminal。备用屏幕的输出、clear 和非报告滚轮不修改主屏幕 stash。离开时恢复策略，再解析主屏幕 anchor。屏幕身份来自 ACTIVE_SCREEN，不按行数猜测。

## 快照与同步输出

WindowSnap 是拥有内存的展示窗口，包含 Arc<Frame>、origin、count、screen identity、可见 GridRuns 和图片。一次正常展示更新分成锁内取数、锁外整理两阶段。锁内只拿一把 emulator 锁：

1. 取 State::frame。
2. 用这份 Frame.rows/cols、当前 screen identity 和 retained range 更新 Window；历史锚点在同一守卫内解析。
3. 活动屏幕保留拥有内存的 Arc<Frame>，历史窗口只复制需要的 screen_row cells；同时复制颜色解释所需的 palette/defaults。不在锁内合并活动屏幕 run。
4. 活动窗口直接克隆 frame.images，行号加活动屏幕起点；只有历史窗口才用独立 image_renderer。活动路径不调用按 State.size.rows 判断的旧 images_at 快路径。
5. 保留光标和输入锚点所需信息，然后放锁。WindowCapture::finish 在锁外用同一个纯 grid_runs_from 合并活动/历史行并整理可访问文本；原生 renderer 再消费 WindowSnap。safe wrapper 的 native capture 当前仍在锁内复制 viewport，不声称已经实现库级的 begin/end 分离。

同步输出期间不得采样尚未完成的实时屏幕。贴底窗口可发布当前 hold 开始边界已有的 owned Frame（它包含此前已完成的输出），同一边界按 Arc 身份复用。历史窗口仍冻结整个 WindowSnap，包括历史、origin、屏幕身份、光标和行列数。用户历史滚轮、搜索 reveal、选择、选区复制和链接打开暂不作用于后台实时屏幕。输入仍编码发送；PTY resize 仍立刻执行。真实库测试确认本项目的 1.0.1 在 resize 时会结束 hold。hold 内切屏本身不结束 hold，也不能提前消费主屏幕 stash。

如果 surface 初次绑定时已经处于 hold，则用回调在 hold 边界保存的 frame、total_rows 和 screen identity 初始化，贴底展示；不从后台实时屏幕拼接历史。已有贴底 surface 同样可以推进到更新的 hold 边界帧；历史 surface 保留最后发布的画面。不能要求轮询恰好落在两次同步事务之间，否则持续滚轮可能导致数秒不发布。

结束、150 ms 超时、EOF 或原生 resize 释放 hold 后，下一次采样一次性发布新窗口。绘制代码仍以 Frame 尺寸为准，不以 State.size 推测帧高；测试分别验证切屏期间冻结与 resize 提交新尺寸。

## 失效、绘制与文字能力

跳过快照更新看 Arc<Frame> 身份、窗口 origin、count 和 screen identity。不能用 Frame.dirty 跳过：capture 会 clean native 状态，缓存里的 dirty 是捕获时的旧值。snapshot signal 只在展示数据改变时更新。

原生场景读取 WindowSnap、cell_width、line_height、font family、font size、默认颜色、主题和 search.rev。字体键不能只看测得的 cell_width；相同宽度的字体切换也要失效。光标保留独立 NSH 节点，闪烁不触发文字重绘。

### 原生嵌入与线程边界

src/session_view/native_surface.rs 的 TermlevelTerminalSurface 是实际 NSView 子类。src/lib.rs 在 app crate 根通过 #[native_components(macos)] 注册；NSH 的 development host 与打包宿主负责调用。NSH 只创建容器、设置几何并发送失效属性，绘制不调用 NSH fill_text。

引擎线程持有 Publisher，以 Arc<Scene> 发布完整的 owned snapshot、PaintConfig、搜索命中与当前索引。主线程通过受互斥保护的槽取得最新场景；读取锁只用于克隆 Arc，绘制期间不持有槽锁或 Session.emulator 锁。积压场景覆盖而不排队。跨 NSH 的 frame 属性只有槽 ID 和 revision 两个标量，没有 cells、文字数组或图片像素；这条 Rust 属性不经过 TSX。

全局注册表只保存 Weak<Slot>。Publisher 和原生组件各持有自己的 Arc，最后一个使用者释放槽并注销。挂载和卸载不创建、关闭或持有 PTY。CoreText 字体、CTLine 和 CGImage 只属于主线程 renderer，不能把它们放回 Session 或 Ghostty。

NSView 是透明、flipped 的绘制子视图，hitTest 返回 nil，NSH 容器与原来的格子父视图继续负责鼠标事件。它不成为 first responder，不创建 NSTextInputClient，不截走原来的 field editor。窗口缩放和 Retina backing 由 AppKit 原生点坐标处理。

### 绘制顺序与缓存

一个原生 NSView 依序绘制：

1. 图片层 0，BELOW_BACKGROUND
2. 单元格背景与选中色
3. 图片层 1，BELOW_TEXT
4. 搜索矩形、文字、下划线和删除线
5. 图片层 2，ABOVE_TEXT

其上仍是独立光标、单个悬停 tooltip、唯一 terminal-input-{id}。默认终端背景属于根视图，不在图片层 0 之上额外覆盖不透明底色。每次原生绘制先清空自身透明 backing，裁到当前 Frame 的整格范围。

结构节点不随历史、行列或 run 数量增长。一个不含任何绘图操作的 Canvas Drawing 仅承载 CanvasNode 行语义，让 NSH 测试模型继续按 cell 坐标定位文字；它不画背景、字形或图片。原生组件容器带整个可见窗口的 accessibility label；AppKit 的细粒度 AX 行子元素仍未实现。隐藏样式不暴露文字。不为了测试重新挂载逐行视图。

每个 run 的 x=col×cell_width，y=row×line_height。Core Graphics save→clip_to_rect→draw→restore 为每个 run 建立独立裁剪，不共享可变路径。颜色与 dim alpha 在当前场景设置；hidden 只保留背景。CoreText 文字翻转回 y 向上，以主字体统一基线绘制 fallback 字形，关闭可选连字。ASCII 同样式 run 合并，非 ASCII grapheme 保留 Ghostty 给出的格子起点和跨度。

renderer 缓存所选字体的普通/粗体/斜体/粗斜体，缓存 CTLine 的键为文字与字面样式。family 或 size 改变时清掉字体和文字缓存；颜色、选区、搜索不参与 shaping 键。文字缓存最多 2048 项和 256 KiB 源文字，按最近使用淘汰。缓存上限不是 CoreText 对象的绝对内存上限，CoreText 自身内存需要持续输出压力测试。

图片继续消费已复制 RGBA；原生 CGImage 按 Arc<Pixels> 身份缓存，仅保留当前快照引用的像素。源裁剪、偏移、缩放和三层顺序保留，动画 generation 换像素时重建。没有每帧整屏 raster、图片编码或 ImageSrc 上传。Kitty file/temp/shared-memory medium 继续关闭。

复杂 emoji、字体回退列表、粗斜体合成、曲线下划线与下划线颜色仍需分别扩展或验收；这次替换不声称已经具备 Ghostty 应用的完整文字 renderer。

## 输入与命中

不增加编码器或第二条事件总线。键、鼠标、焦点和粘贴仍走现有 encode_*，写入 Rust PTY。Ghostty 事件仍由 Session::handle_events 和 SystemClipboard 处理。

- 格子局部坐标按 cell metrics 除法命中；选区和 OSC8 接收 origin + row_in_window。
- 鼠标报告模式且无 Shift：未贴底的左键、右键和滚轮先请求贴底；已贴底时只编码，不改 origin。中键和侧键从 AppKit 进入同一 Session 输入代际机制；所有成功编码的鼠标入口在下一份窗口发布前应用贴底请求。
- Shift 或非报告模式：主屏幕按整行调整 origin；备用屏幕不滚历史。Points 除以 line_height，Lines 直接使用。小数向零截取，剩余留在 wheel_frac；正 dy 朝更新的行移动。水平历史滚动忽略。
- 报告滚轮的双轴累加器与历史累加器分开，单轴一次最多 32 下，沿用 mouse_repeat 编码。
- 滚轮监听者就是非滚动的格子视图，使用 NSH 现有命中规则，不加 cancel。
- 选区复制、双击词、三击行、矩形选区和受限协议的 Cmd+链接继续保留。

唯一 field editor 随 input_cursor 定位；输出光标隐藏时仍保留输入锚点。y 夹到格子内部，宽度为当前行剩余空间。组合期间隐藏终端光标；组合提交差量、after_answer 清空和原 Arc<Session> 的提交归属不变。非 ASCII 字符键仍交给 IME。原生绘制替换不代表焦点与候选窗已经验收。

## 查找独立于帧刷新

绘制快照不再顺便全量重扫查找。查找调用立即返回当前可用结果；新增低频 SearchChange，携带 query、count、index、searching，供查找栏显示 Searching… 和最终计数。异步结果必须按 query 检查归属。TSX 不接收单元格或命中列表。

ASCII 使用本仓库 1.0.1 search.h 已有的 ghostty_search_feed / tick 分步接口，保留原生跨软换行匹配和屏幕跟踪；不在每次 revision 调用阻塞的 ghostty_search_run。每次推进只做一次 feed 和至多 32 tick。

Unicode 保持 Rust 小写匹配，每次最多复制 64 行的 GridRun，放锁后匹配。输出变化使旧结果失效，安静 150 ms 后分批重扫；跨批次 revision 改变就丢弃这次扫描。新的输出不强制跳到搜索结果，只有新的查询首次完成与用户 next/previous 会 reveal。

分步搜索的结果导出成本仍与命中数有关。不能把 bounded feed 写成整次查找的绝对耗时保证。Unicode 查询仍沿用逐物理行匹配范围，不扩展跨行语义。

## 验证与性能门槛

自动化分别验证窗口数学、真实 Ghostty tracked anchor、hold 内 resize/切屏、选区失效、图片快路径、原生增量搜索、原生缓存失效、图片层顺序与源裁剪、无绘制操作的行语义以及宽字符坐标。不能只用模拟的递增 scroll_base 证明历史稳定。

使用：

```sh
export GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip
export CARGO_NET_OFFLINE=true
./scripts/nsh check
./scripts/nsh test
./scripts/nsh build
```

原生人工验收单独记录：

```sh
./scripts/nsh run --window-size 1200x780
```

至少看 Codex 报告模式滚动、主屏幕历史与 Shift 滚轮、clear 后是否露上一行、hold 内 resize 和主/备用切换、中文/日文/死键组合及候选窗、跨行选区、OSC8、三层图片和动画，以及多 pane 持续输出。

性能对照必须保持窗口尺寸、字号、命令与负载一致，并记录构建模式。分别记录 emulator 锁等待与快照、run 合并与发布、原生 drawRect、NSH 呈现间隔和输入响应。原生 drawRect 的统计包含字体/CTLine cache miss、图片转换和 CoreText 绘制；不把 NSH 的空 canvas 指标当作终端绘制耗时。debug 每秒汇总原生绘制次数、均值、最大值及缓存数量，不以墙钟阈值编写库测试。

受控性能 fixture 为 scripts/test-native-scroll：1200×780、Menlo 13 pt、21 pt 行高、鼠标报告 TUI、2026 同步输出、中英文和彩色文本。旧路径 artifacts/scroll-before.log 与 CPU sample、新路径 artifacts/scroll-native-surface.log 分别记录。新路径约 2.6–2.9 ms 原生绘制，NSH 帧率计稳定阶段多数 54–59 FPS；旧路径约 24–26 ms Canvas 回放、38–44 FPS。25 秒延长负载达到 2048 项缓存上限后，绘制约 2.7–3.1 ms，后段 NSH 帧率计约 54–60 FPS；见 artifacts/scroll-native-surface-sustained-final.log。新路径也出现约 50 FPS 区间和冷启动间隔，因此不声称锁定 60 FPS。NSH 帧率计与实际终端快照发布次数不是同一指标；此 fixture 不能替代真实 Codex 手势体验。

## 落地与回退

固定窗口、tracked anchor、owned snapshot、分步搜索保留。生产网格的背景/文字两张 canvas 和 NSH 图片节点已经替换为一个原生组件；只保留无绘制操作的语义 metadata adapter。没有旧 renderer 功能开关或长期双路径，也不回到 virtual_list、PaintedScreen 或列表滚动 API。

本次代码同时更新 architecture、roadmap 和实际 validation 记录。NSH 的依赖源代码不改；nsh-platform-macos 与其他 nsh-* 固定同一 0.5.0。用户未要求提交时不提交，不推送。
