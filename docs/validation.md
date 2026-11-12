# Validation

记录：2026-11-11 初始化，2026-11-12 更新侧栏贯顶布局。环境：macOS arm64，Rust / Cargo 1.99.0，NSH 0.5.0 (`f4d68f7`)。

最新：默认已接入 Alacritty + 真实 PTY，40 项检查与 3 项原生场景通过。详见本文末尾“Alacritty + 真实 PTY 接入完成”；前文为逐阶段的历史记录。

## 已通过

| 检查 | 结果 |
|---|---|
| 官方 CLI 发行包 SHA-256 | 与发行包校验文件一致 |
| `cargo check --all-targets --offline` | 通过 |
| `cargo fmt --check` | 通过 |
| `sh -n scripts/nsh scripts/bootstrap` | 通过 |
| `CARGO_NET_OFFLINE=true ./scripts/nsh check --tsc /path/to/tsc` | Rust + TypeScript 通过；本次使用已有 TypeScript 编译器 |
| `CARGO_NET_OFFLINE=true ./scripts/nsh build` | 通过；生成 `.nsh/out/bundle.nshc` |
| `CARGO_NET_OFFLINE=true ./scripts/nsh test --junit artifacts/headless-tests.xml` | 14 passed，0 failed |
| macOS 原生开发宿主 | 编译通过 |

Headless 测试使用真实 NSH 引擎、默认 Terminal 与 QuickJS，桌面视口为 1200 × 780。覆盖：

1. 切换标签与折叠组后保留终端输出。
2. 创建组、重命名标签后保留输出。
3. 跨组移动、取消分组后保留标签。
4. 关闭活动组后选择剩余标签；关闭所有标签后可重新创建。
5. 查找折叠组内标签；紧凑侧栏仍可切换和创建标签。
6. 固定、菜单排序后保留当前会话输出。
7. 浅色 / 深色 / 跟随系统切换：验证工具栏、终端默认文字、分组颜色实际更新，显式主题覆盖系统变化，且保留终端输出和实例。
8. 窄窗口隐藏路径栏仍可操作会话菜单；Escape 关闭重命名对话框，收起侧栏后恢复路径栏并可继续输入。
9. 分组编辑浮层展示九种颜色，直接修改名称和颜色、组内新建终端后保留已有终端输出。
10. 收起侧栏时右键打开编辑浮层，选色后 Enter 关闭；点击外部关闭，更多按钮可再次点击关闭。
11. 收起侧栏中的折叠组保留活动标签，隐藏计数随选择从 +1 更新为 +2，点击计数展开并保留终端输出。
12. 鼠标移入收起侧栏时展开、移出收回，分组编辑浮层打开期间保持展开；窄窗口的终端工具栏仍维持原宽度布局，已有终端输出与实例保留。原生浮层绘制与鼠标命中仍需本机验收。

13. 分界拖动保持按下时，窄窗口工具栏已按新的内容宽度隐藏路径；松开、收起、悬停后仍可进入分组编辑并保留终端输出。
14. 收起侧栏及悬停浮层状态下拖动分界均不改变内容布局和已保存宽度；手动展开后恢复可调整宽度。

2026-11-12 浮层裁剪修正：用户实机截图确认将 Sidebar 放在 44 pt 分栏内会裁剪展开内容。当前 Sidebar 改回窗口根层，固定展开时通过分栏内测量 View 的 `onLayout` 同步实际宽度；`onResize` 仅保存拖动结束尺寸。13 项交互回归通过；浮层的完整原生绘制、命中和逐帧拖动跟随仍需实机验收，headless 交互通过不能证明没有原生裁剪。

2026-11-12 平台材质：固定侧栏改为 `sidebar` 视觉效果，悬停侧栏、会话工具栏、分组编辑浮层和 tooltip 改为 `regular` 玻璃加模糊回退。`./scripts/nsh check` 通过，headless 14 项通过。Liquid Glass 只在 macOS 26 绘制，材质采样、对比度和窗口激活态仍需实机窗口验收。

本地产物在被忽略的 `.nsh/out/` 和 `artifacts/`。Debug runner 链接时有 1 条 linker warning：`__eh_frame section too large (max 16MB) to encode dwarf unwind offsets in compact unwind table`，提示异常处理性能可能受影响；构建和测试均成功。尚未验证 release 构建是否存在同样警告。

## 2026-11-12 侧栏表面与层级调整

固定展开和收起侧栏改用接近终端的不透明底色，收起态取消分界填充；悬停浮层保留玻璃、右侧圆角和阴影。组名使用中性色，组标保留颜色，引导线降低对比，移除底部分隔线。侧栏开关两种状态采用相同的圆形无底色常态。

- `./scripts/nsh build`、`./scripts/nsh check`、`git diff --check` 通过。
- `./scripts/nsh test --junit artifacts/headless-tests.xml`：14 passed，0 failed。分组编辑测试调整为检查可访问的组标颜色名称，组名颜色保持中性；继续验证名称更新、会话输出与实例保留。
- 两次 `./scripts/nsh test App.tsx --target macos` 原生步骤通过，已查看深色展开、深色收起、深色悬停、浅色展开和浅色收起的截图。首次受限环境仍报本地服务器 `Operation not permitted`，允许本地开发服务器后成功。截图位于忽略的 `artifacts/sidebar-quiet-*.png`。
- 截图确认固定侧栏无玻璃顶部亮边，悬停展开内容没有被 44 pt 分栏裁剪。原生宿主在后台运行，截图窗口按钮为失焦状态；浅色及悬停截图采用默认窄视口，不作为所有窗口宽度的验收。
- 原生宿主报告 `SF Pro Text` 不可用并回退到系统字体；链接仍有既有 warning。截图中的预览输出出现窄列换行，本轮未更改终端布局或重排，未定位其原因。

本轮不代表标题栏拖动、IME、选区复制、高 DPI、激活态切换、不同桌面背景或系统“减少透明度”已通过。以下为此前验收限制的历史记录；本轮已取得上述原生截图。

## 此前未通过的实机验收记录

`./scripts/nsh test App.tsx --target macos` 已编译原生宿主，但启动测试时环境禁止监听本地开发服务器，报 `cannot start a dev server: Operation not permitted (os error 1)`。因此没有得到原生测试截图。

直接用编译好的宿主加载 bundle 的进程可以启动，但出现 macOS 系统服务连接日志，未取得可检查的窗口截图。不能据此声称视觉或原生交互验收通过。

需在普通终端执行：

```sh
./scripts/nsh run --window-size 1200x780
```

人工检查标题栏拖动、侧栏缩放与收起、分组菜单、鼠标选择与复制、输入法、字体与高 DPI。当前没有真实 PTY，因此 Neovim / tmux / shell 行为均不在本轮验证范围。

主题重设计后再次尝试原生截图测试，仍因同样的本地开发服务器权限限制失败。语义 token、外观切换和状态保留有 headless 证据；原生最终视觉效果尚未验收。

2026-11-12 已切换到选定的侧栏贯顶布局：全高侧栏、28 pt 单行标签、36 pt 会话工具栏和铺满的终端；并实现 Firefox 风格的分组编辑浮层。类型检查、构建和 10 项交互测试通过。原生截图仍被开发服务器权限限制阻断；系统窗口按钮、收起态与分组浮层的实际位置需要本机窗口验收。

## 环境记录

初始化时遇到过磁盘不足；恢复空间后已重新解压完整工具并成功完成构建。Cargo 下载在受限命令中遇到 DNS 失败，通过独立 `cargo fetch` 缓存依赖后，以 `CARGO_NET_OFFLINE=true` 完成检查。

项目文件已创建；本环境禁止创建此目录的 `.git` 元数据，`git init` 返回 `Operation not permitted`，因此尚未初始化 Git 仓库、也没有提交。可在普通终端运行 `git init`。

## 2026-11-12 全部移除玻璃效果

按用户要求，悬停侧栏统一使用实色 `sidebar`，覆盖层级由细边框、右侧圆角和轻阴影表达；工具栏移除胶囊底座，目录与按钮直接排列；分组编辑和 tooltip 改为实色 `popover`、细边框和轻阴影。删除玻璃辅助模块、IconButton 的玻璃分支以及两套外观中的玻璃 token。应用 TSX 与主题中不再有 `glassEffect`、玻璃色调、背景模糊或平台 backdrop 配置；模态对话框的均匀遮罩保留。

`./scripts/nsh build`、`./scripts/nsh check`、`git diff --check` 通过；`./scripts/nsh test --junit artifacts/headless-tests.xml`：14 passed，0 failed。

`./scripts/nsh test App.tsx --target macos` 原生截图步骤通过，取得深色展开、收起、悬停、分组编辑和浅色展开截图，位于忽略的 `artifacts/sidebar-solid-*.png`；已查看悬停、编辑浮层和浅色截图，确认底色保持实色、工具栏无胶囊底座、编辑内容完整显示。原生宿主仍报告系统字体回退，链接有既有 warning；预览窄列换行仍存在。此次截图不代替 IME、选区复制、激活态切换和长期日用验收。

## 2026-11-12 Alacritty 内核首次接入

前置 UI 调整已提交：`ea2d600`（Refine sidebar hierarchy and use solid UI surfaces）。本轮在该提交之上接入 crates.io `alacritty_terminal =0.26.0`，新增 `nsh-base =0.5.0`（仅 `term`），所有 NSH 依赖继续固定 0.5.0。应用根层的原生 `TerminalEngine` 安装模拟器 factory，现有预览终端实际采用 Rust Alacritty parser/grid；不是只加入依赖。当前依然是确定性演示 transport，不创建 PTY 或 shell。SessionManager 和与视图分离的生产生命周期尚未实现。

| 检查 | 结果 |
|---|---|
| `cargo fmt --check` | 通过 |
| `./scripts/nsh check` | 通过 |
| `./scripts/nsh build` | 通过；生成原生声明和应用 bundle |
| `./scripts/nsh test --junit artifacts/alacritty-tests.xml` | 24 passed，0 failed：8 Rust + 14 工作区 + 2 内核绑定交互 |
| `git diff --check` | 通过 |
| `./scripts/nsh test App.tsx --target macos` | 1 原生步骤场景通过；仅预览 transport |

Rust 测试验证跨字节 UTF-8/转义序列与组合字符、宽字符占位转换、ANSI/truecolor 与样式、背景空格、有限历史顺序、备用屏幕恢复、宽度缩放重排、标题/响铃/光标查询回复、输入模式、调色板修改与重置、多模拟器隔离和 reset。NSH 绑定测试发送默认 VT 不支持的 REP (`CSI b`) 并验证重复字符，避免 factory 未生效时静默回退；另验证 truecolor、中文组合字符、备用屏幕、resize、应用光标键、括号粘贴、标题与协议回复通路。

原生步骤输入 `echo alacritty-native-output`（预览命令，不是 shell），收起侧栏、悬停切换标签并切回，再切换浅色。截图位于忽略的 `artifacts/alacritty-expanded.png`、`artifacts/alacritty-collapsed.png`、`artifacts/alacritty-light.png`，均已查看。与同窗口尺寸此前的实色截图相比，预览内容不再保持窄列换行；这是此场景中的重排证据，不能推广为所有 Unicode/字体场景均正确。

本轮明确缺口：NSH 的 `TerminalEmulator` 只表达两种输入模式、简单光标和行样式，无法完整承载 Alacritty 的鼠标报告、焦点报告、现代键盘协议、光标形状、超链接及多种下划线；后者暂降为普通下划线。颜色设置覆盖可显示，颜色查询与像素尺寸查询回复尚无完整通路。OSC52 和 Kitty 键盘协议显式禁用。不得把这些未接通的能力当作已支持。

原生宿主仍报告 `SF Pro Text` 回退系统字体，以及既有链接 warning。未验证真实 zsh/fish、Neovim/tmux、IME、选区复制、大量输出性能、关闭进程与泄漏、热更新后的 Rust 会话重新绑定。

## 2026-11-12 Alacritty + 真实 PTY 接入完成

基于 UI 提交 `ea2d600`，当前工作区默认打开用户的真实登录 shell。预览仅由 `TERMLEVEL_PREVIEW=1` 显式启用，预览解释器仍不会执行任何系统命令。Rust `SessionManager` 强持有 PTY、Alacritty parser/grid 和子进程；原生视图按稳定 session ID 绑定，卸载不会结束进程，已关闭 ID 不会隐式重新启动。全部流量留在 Rust：reader → parser/grid → 原生行视图，原生输入 → writer → PTY；TSX 只收低频元数据和响铃。

代码按 NSH mixed 方式整理：`App.tsx` 只挂载 `ui/WorkspaceApp.tsx`；`src/lib.rs` 只注册原生导出，Runtime、DialogKeys、会话管理、API、内核适配和原生视图分别成模块。结构参考 [NSH Project structure](https://nsh.dev/docs/project-structure/) 与 [Building with Rust](https://nsh.dev/docs/rust/)，不引入多 crate 或 web 前端。

### 当前工作区检查

| 命令 | 当前结果 |
|---|---|
| `cargo fmt --check` | 通过 |
| `./scripts/nsh check` | Rust + TSX 通过 |
| `./scripts/nsh build` | 通过 |
| `./scripts/nsh test --junit artifacts/alacritty-live-tests.xml` | **40 passed，0 failed**：21 Rust + 19 JS 交互 |
| `./scripts/nsh test terminal/Session.test.tsx --target macos --junit artifacts/alacritty-native-tests.xml` | **3 passed，0 failed** |
| `git diff --check` | 通过 |
| `cargo tree --prefix none --offline` | nsh-base/core/macros/updater 均为 0.5.0 |

解析两份 JUnit 报告确认分别有 40/3 个 testcase，无 failure/error。报告与截图放在被忽略的 `artifacts/`，不提交生成目录。

### 行为证据

- Rust 内核测试：跨 chunk UTF-8/转义序列、组合与宽字符、颜色/样式/背景空格、有限历史顺序、备用屏幕、重排、光标/输入模式、标题/响铃/回复、调色板修改/重置、多模拟器隔离，以及同步输出的原子提交和超时。
- 真实 PTY 测试：sh 与 zsh 的中文文本、实际 `stty size`、标题与响铃、Ctrl+C 结束前台 `sleep` 而保留 shell、退出码及输出保留、会话隔离和重新绑定、无效 cwd/程序的启动错误、4000 行输出后的有限历史与完成标记、关闭前台任务与普通后台作业、manager shutdown 后的工作线程结束。zsh 测试使用 `-f` 和 `/dev/null` 历史，避免依赖个人配置。
- 协议回复由真实子进程接收并用 `od` 回读验证：DSR 光标位置及 OSC 默认背景色查询。默认色在 reader 启动前设置为 UI 主题，避免首次绘制前的查询得到错误颜色。物理像素大小仍未知，像素尺寸查询回复为 0。
- 同步输出由真实子进程发送不完整的 mode 2026 序列；进程仍在阻塞等输入时画面可因超时提交，EOF 也能强制提交残留输出。resize 时 parser 锁覆盖 PTY ioctl 和网格调整，避免新尺寸输出先落入旧网格。
- 三项原生 AppKit 场景：① 中文粘贴、缩小视口、卸载/重新挂载同一会话、继续输入及退出码；② 生产工作区新建/关闭标签、收起侧栏和切换外观后保留输出；③ 整个 TSX 工作区销毁/重建后从 Rust 快照绑定同一会话并继续执行命令。fixture 显式选择真实 `/bin/sh`，所以普通 `./scripts/nsh test` 不需要更改用户的 SHELL，也不会写入个人 fish 历史。原生宿主在关闭最后一个窗口时可先断开连接再回 close 请求，测试仅在显式 close 操作中接受该预期断连。
- 默认 `App.tsx` 另以真实用户 fish 运行：`fish_history='' ./scripts/nsh test App.tsx --target macos`，执行 `printf` 和真实 `pwd`，验证中文输出、重命名、侧栏收放、新建后关闭另一终端、切换浅色，最后关闭会话并显示空工作区。已查看 `artifacts/alacritty-fish-live.png`、`alacritty-fish-collapsed.png`、`alacritty-fish-light.png`；真实 shell 与演示输出有明确区分。
- 原生关窗清理另作独立探针：真实 `/bin/sh` 将自身 PID 写入 `artifacts/native-shell.pid`，关闭原生主窗口；驱动场景通过。随后 `os.kill(pid, 0)` 返回 ProcessLookupError，确认测试 shell PID 38371 已退出。Rust reaper 与 close 串行，Unix 先 HUP shell 清理普通作业，50 ms 后仍存活再强制终止；应用退出在 NSH 200 ms 预算内等待 `Sessions.settled()`。

### 兼容性边界

本轮完成内核和真实会话数据/生命周期通路，不等于 Alacritty 应用 renderer 或完整终端兼容性。NSH 0.5.0 的鼠标报告、焦点报告、完整功能键/修饰键、现代键盘协议、光标形状及超链接元数据仍需 M3 补齐。Kitty 键盘协议与 OSC52 显式禁用，下划线变体投影为普通下划线。工具栏目录是初始启动目录，不冒充实时 shell cwd。

中文输入/粘贴自动化不证明真实 IME 候选窗、复杂 emoji 或选区复制验收；4000 行 Rust 输出测试不证明 GUI 吞吐和帧耗时。机器未发现 nvim/tmux 可执行文件，本轮没有安装或声称它们全兼容。Windows/Linux 未验收。原生宿主仍有既有 `SF Pro Text` 回退系统字体提示与链接 warning，未新增编译告警。

应用重启会结束会话，布局/设置持久化仍属 M1.1。当前确认的是同一原生宿主内的视图与工作区重新绑定，不是后台常驻或跨应用重启恢复。集成改动仍在工作区，未与前置 UI 提交混在一起。

## 2026-11-12 Rust 网格视图与原生组合输入

真实会话已从默认 `nsh-base::Terminal` 切到 `SessionTerminal` 自有的 Rust 网格行视图与原生 Input。预览继续使用原来的 NSH Terminal。NSH CLI/所有 `nsh-*` 仍固定 0.5.0；没有等待或引入上游升级。会话 manager、PTY 和 parser 的生命周期保持独立。

输入控件保持稳定挂载，位置使用 Alacritty 当前光标、滚动 offset、底部对齐余量；文字、样式和字符宽度按网格列号显示。组合期间由原生控件显示 marked text，并暂停终端按键绑定。普通字符先确认平台值，再在 NSH `after_answer` 清空，避免相同空属性被差异比较跳过。组合结束只发送最终差量；取消删除 marked text 不写入 PTY。切换标签时 AppKit 的失焦确认仍进入原会话。

| 命令 | 结果 |
|---|---|
| `./scripts/nsh check` | Rust + TSX 通过 |
| `./scripts/nsh test --junit artifacts/native-input-tests.xml` | 46 passed：27 Rust + 19 JS |
| `./scripts/nsh test terminal/Session.test.tsx --target macos --junit artifacts/native-session-tests.xml` | 3 passed |
| `./scripts/test-native-input` | 1 个完整 AppKit 场景通过，报告 `artifacts/native-ime-tests.xml` |
| `./scripts/nsh build` | 通过 |
| `cargo fmt --check` / `git diff --check` | 通过 |

原生输入脚本使用 NSH 的 macOS development host、真实 `/bin/sh` PTY，以及实际 AppKit field editor 的 `NSTextInputClient` 接口：

- `setMarkedText` 注入拼音和转换后的中文；在 Rust parser 快照中验证二者尚未进入 PTY，而 native editor 保持 marked text。
- `insertText` 确认后运行 `printf`，验证实际 shell 输出完整中文且只提交一次；输入控件最终清空。取消以空字符串替换 marked range，验证拼音没有漏到 PTY，随后普通命令仍可执行。
- `firstRectForCharacterRange` 验证候选锚点随命令前缀横移、随两行 shell 输出纵移；包含滚动历史和 viewport resize。该坐标来自原生输入接口，并非 TSX 模拟坐标。
- 组合中切换两个仍挂载的真实会话，新会话不含旧会话的中文；新会话能执行命令，切回后原会话也可继续输入。
- 修改字体后执行 `stty size`，验证真实 PTY 的 rows/cols 相应减小；尺寸由 viewport、字号/字宽和行高共同驱动，不依赖窗口恰好发生另一次 layout。
- Rust 测试另验证快速连续输入、延迟清空、重复汉字、带旧前缀的取消；CJK 与组合字符的显式列位置，以及隐藏 TUI 光标时仍可读取输入锚点。

NSH 0.5.0 的测试驱动没有 `compose()`，因此 `scripts/test-native-input` 临时注册原生测试文件并在结束时删除；用例源保留在 `terminal/fixtures/Input.acceptance.tsx`，不混入默认 headless 测试。`src/session_view/native_probe.rs` 只在 macOS debug build 注册，不向生产终端 API 暴露输入注入接口。

已查看 `artifacts/native-ime-preedit.png` 与 `native-ime-after.png`：前者显示命令行光标处的中文 marked text，后者显示缩小窗口后的历史、光标和字号变化。报告和截图继续位于忽略的 artifacts 目录。

边界：这轮验证的是原生输入协议、显示与 PTY 提交；没有操作真实系统输入法的候选列表。拼音/日文/韩文实际候选窗、Escape/方向键与输入法的完整交互、行末长预编辑、复杂 emoji、跨行选区/复制仍需人工验收。鼠标报告、完整功能键/修饰键和现代键盘协议仍未补齐，不声称 Neovim/tmux 全兼容。原生宿主仍有既有的 SF Pro Text 回退和链接 warning。

### 实机反馈修正：生产内边距导致输入法上移、左移

用户的生产窗口截图暴露了此前原生 fixture 没有包含的 `padding-terminal: 16px`。原先网格列表参与 padding 布局，绝对定位的 Input 却从外层原点定位；此前测试只比较候选锚点的相对移动，没有比较它与实际网格光标的页面坐标，因而漏过这个错误。

已先给 fixture 添加与 `TerminalSurface` 相同的 padding，并通过 debug probe 的 `ElementRef.measure()` 读取实际挂载的 Input 和网格光标：修复前 `(446.5,19)` 对 `(462.5,35)`，水平/垂直均差 16 pt，断言失败。修正为在宿主 padding 内建立共同内容容器，让列表与 Input 共享原点，不对主题值添加硬编码补偿。

修复后在普通输入、65 行滚动历史、缩小窗口、增大字号四个阶段，页面坐标分别是 `(462.5,35)`、`(252,585)`、`(252,245)`、`(319.5,241)`，每次 Input 与网格光标完全一致。扩展后的 `./scripts/test-native-input` 通过，已重新查看包含生产 padding 的 `artifacts/native-ime-preedit.png`，marked text 与命令行同一基线。仍区分这项原生布局/输入协议证据和真实系统候选列表的人工验收。

### 去掉终端上边距

生产窗口里，满屏输出的裁切线停在工具栏下方约 16 pt，中间是空的终端底色。这是 `padding-terminal` 的上边。终端改为只保留左右和底部内边距，滚动内容裁切在工具栏下沿。Input 与网格仍共用这块内容容器的原点。

原生窗口 1100×740、scale 2 的截图像素：工具栏文字在约 13–22 pt，终端第一行像素从 36 pt 开始，行距 21 pt。改动前同一位置的第一行从 52 pt 开始。

### 侧栏拖动分组

垂直拖动标签可在侧栏内重排，或拖到组头放入该组。拖到已有分组的标签上会加入该组；拖到没有分组的标签上会组成新组，组名和路径取被拖到的标签，不打开分组编辑。组内新建的终端从分组路径启动。拖动中组头高亮表示放入该组，标签高亮表示与该标签成组，标签之间的线表示插入位置。松手在侧栏外，或横向滑开，分组不变。拖进折叠的组会展开该组。标签之间的空隙不会把未固定标签插进固定区。拖动不改变 session ID，终端输出保留。

`./scripts/nsh test App.test.tsx` 16 项通过，其中两项覆盖拖入展开组、拖入折叠组，以及横向拖动取消。`./scripts/nsh check` 通过。这次是 headless 指针驱动，还没有在原生窗口里用手拖验收命中和跟手。
