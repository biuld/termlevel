# Validation

记录：2026-11-11 初始化，2026-11-12 更新侧栏贯顶布局。环境：macOS arm64，Rust / Cargo 1.99.0，NSH 0.5.0 (`f4d68f7`)。

最新记录见本文末尾。默认真实会话和预览已接入 Ghostty 1.0.1；独立 Rust 封装、真实 PTY 和生产 UI 的原生验收分别记录，前文保留逐阶段历史。

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


## 2026-11-13 · 独立 pane 与统一窗口栏

布局改为 40 pt 统一窗口栏，下方侧栏和 terminal 为独立圆角 pane。pane 顶部直接接窗口栏下沿，左右、底部及两栏之间保留 8 pt；移除首版多出的顶部 8 pt 留白。侧栏缩放边界位于间隙中，悬停展开仍覆盖终端而不改变终端分配宽度，收起及临时展开禁止缩放。真实终端与预览使用相同的正文内边距：顶部 8 pt，左右及底部 16 pt。

验证：`./scripts/nsh build`、`./scripts/nsh check` 通过。`./scripts/nsh test --junit artifacts/floating-panes-headless.xml` 覆盖 Rust 内核/PTY、工作区交互和挂载恢复，共 50 项。

原生 AppKit 使用临时 `nsh/test` 脚本，在 `TERMLEVEL_PREVIEW=1` 下运行 `./scripts/nsh test .pane-visual.test.tsx --target macos --junit artifacts/floating-panes-native.xml`，深色与浅色两项通过。已查看 1100×720 pt、scale 2 的展开、收起、悬停覆盖和浅色截图：`artifacts/panes-expanded.png`、`artifacts/panes-collapsed.png`、`artifacts/panes-hover.png`、`artifacts/panes-light.png`。截图确认 pane 顶部位于 40 pt，悬停浮层未被窄分栏裁剪。临时脚本已移除，截图和 JUnit 为忽略的本地产物。

这次原生验收使用显式预览会话，只证明布局与覆盖显示；真实 PTY 的工作区操作和恢复由上述 headless 集成测试覆盖。系统候选窗、复制选区、标题栏拖动和长期日用仍保留独立人工验收边界。


## 2026-11-13 · Fleet Dark / Light 配色映射

`theme.json` 的完整深浅色语义 token 改为 Fleet Islands UI 配色映射，固定来源与用途表见 [design.md](design.md#fleet-dark--light-配色)。工作区窗口、pane、弹层、控件、分组和终端默认前景/背景/光标均已覆盖；不修改 NSH 固定 ANSI 16 色或程序 truecolor。源码中的透明中性色预合成为不透明表面。

`./scripts/nsh build`、`./scripts/nsh check` 通过。headless 回归 `./scripts/nsh test --junit artifacts/fleet-palette-headless.xml` 的 50 项测试全部通过。主题切换测试仍检查颜色随外观更新、显式外观覆盖系统选择、终端实例和输出保留；移除此前要求终端文字与标签文字颜色必须不同的断言，因为 Fleet 两者主文字均使用相同颜色。

原生 AppKit 以显式预览模式执行临时 `.palette-visual.test.tsx`，深色与浅色两项通过（`artifacts/fleet-palette-native.xml`），分别截取并查看工作区、设置面板及侧栏悬停状态：`artifacts/fleet-dark.png`、`artifacts/fleet-light.png`、对应的 `-settings.png` / `-hover.png`。临时脚本已删除，所有截图与报告均为本地忽略产物。此处为原生视觉与交互自动化记录，不代替实际系统输入法及长期日用人工验收。

## 2026-11-13 · 分组平铺

去掉展开分组的整块彩色背景和圆角外框、组头按钮的悬停填色，保留行位置、彩色组标、成员引导线以及活动终端的一层中性底色。组头悬停仍显示更多按钮，键盘焦点环和拖放目标反馈保留。

`./scripts/nsh build`、`./scripts/nsh check` 通过。原生预览脚本在 Dark / Light 下悬停组头并检查侧栏覆盖态，两项通过，截图为 `artifacts/flat-groups-dark.png`、`artifacts/flat-groups-light.png` 和对应的 `-hover.png`，报告为 `artifacts/flat-groups-native.xml`。临时脚本已移除。50 项 headless 测试全部通过，交互回归报告：`artifacts/flat-groups-headless.xml`。原生截图不代替真实系统输入法、选区等人工验收。

## 2026-11-13 · 引导线与 tab 分离

展开态成员容器通过 `gap-group-guide: 6px` 留出引导线到 tab 左缘的间隔，删除活动 tab 内重复的彩色短竖条；选中底色和彩色终端图标保留。收起态不增加这段缩进。`./scripts/nsh build`、`./scripts/nsh check` 通过。原生预览 Dark / Light 两项通过，分别截图展开、收起和悬停覆盖态（`artifacts/group-guide-{dark,light}.png` 与对应的 `-collapsed.png`、`-hover.png`；报告 `artifacts/group-guide-native.xml`），已查看展开、收起与悬停的间隔和选中标识。临时原生脚本已移除。headless 回归为 49 项通过、1 项 Rust PTY 测试 `real_shell_utf8_resize_title_and_exit_status` 等待状态超时（报告 `artifacts/group-guide-headless.xml`）；使用 `./scripts/nsh test --filter real_shell_utf8_resize_title_and_exit_status --junit artifacts/group-guide-pty-retry.xml` 单独重跑通过。本次未修改 Rust 或超时阈值，不把单项重跑等同于整轮全绿。

## 2026-11-13 · 去掉终端正文顶部内边距

真实终端和预览终端均删除 `pt-(--padding-terminal-top)`，同时移除主题中不再使用的顶部内边距 token；左右、底部仍为 16 pt。滚动网格和 Input 继续共用原来的内容容器。`./scripts/nsh build`、`./scripts/nsh check` 通过。原生默认入口使用真实 fish 会话截图（`artifacts/terminal-no-top-inset.png`），临时原生测试通过（`artifacts/terminal-no-top-inset-native.xml`）且脚本已移除。50 项 headless 测试全部通过，报告：`artifacts/terminal-no-top-inset-headless.xml`。

## 2026-11-13 · 多 terminal 定向平铺

实现见 [多终端平铺](terminal-tiling.md)。布局、方向导航、约束和拖放使用 `ui/tiling.ts`；`ui/workspace.ts` 组合会话与布局操作；`ui/TilingCanvas.tsx` 按稳定 session ID 保留 surface，只更新原生宿主几何。Split 不拥有 PTY。pane 缩放通过原生 View pan/responder 更新 TSX 比例，不声称是完全绕过 JS 的原生分栏链路。

### 自动化证据

- `./scripts/nsh check`、`./scripts/nsh build`、`cargo fmt --check` 与 `git diff --check` 通过。
- 全量命令：`./scripts/nsh test --junit artifacts/tiling-all-tests.xml`，58 / 58 通过。覆盖原有 50 项及新增的纯布局、平铺交互和真实双 PTY 测试。
- `ui/Tiling.test.tsx` 的 7 项覆盖几何与邻居选择、分屏和替换、放大还原、窄窗口恢复、方向焦点与交换、缩放及按键归还、关闭组、侧栏拖入、标题边缘移动、中央交换、尺寸预检。拖放测试通过独立 fixture 读取树快照，确认边缘插入改变方向、中央交换改变会话分配、Escape 后整棵树完全不变。不能只看预览消失或会话数量不变。
- `terminal/TilingSession.test.tsx` 使用两个真实 `/bin/sh` PTY，验证左右输出同时更新、点击输入不串到另一会话、放大时 `stty size` 改变、收起和重新展示后 PID 不变、显式关闭移除会话。测试读取的是真实 Rust 会话元数据，不用预览输出替代。
- 既有两项 UI 测试改为先选择 Color test 再输入，避免测试驱动直接向隐藏 terminal 注入文字绕过实际焦点规则。预览的输入发送也受唯一焦点和模态状态控制。

### 原生 AppKit

`./scripts/test-native-tiling` 通过，报告 `artifacts/tiling-native.xml`。该脚本将 `terminal/fixtures/Tiling.acceptance.tsx` 临时注册为原生测试并在退出时清理，使用真实 shell，非预览 transport。

验收覆盖：

- 双 pane 都有真实输出，原生方向焦点快捷键更新当前 pane。
- `NativeInputProbe.geometry` 对比两个会话实际挂载的 Input 与网格光标页面坐标，交换、拖动缩放、放大和还原后各自误差均小于 0.5 pt。
- marked text 存在时，通过 fixture 调用同一个工作区交换操作：文字继续标记、候选锚点水平移动超过 100 pt，确认输入仍提交到原 session，另一个 session 没有收到中文。
- AppKit 在组合输入时优先把全部按键交给输入法；不声称交换快捷键在 marked text 期间必定执行。测试将普通快捷键与组合过程中的布局变换分开证明。
- 三 pane、640×480 窄窗口临时聚焦、恢复原布局和侧栏悬停覆盖均有截图。查看了 `artifacts/tiling-native-two-panes.png`、`tiling-native-ime-swap.png`、`tiling-native-three-panes.png`、`tiling-native-narrow.png` 和 `tiling-native-sidebar-overlay.png`。
- 原生驱动不支持 `dragAndHold`，AppKit 验收使用完整 `drag`；中途预览和取消由 headless 的树快照断言证明。

既有 `./scripts/test-native-input` 同样通过（`artifacts/native-ime-tests.xml`），覆盖组合确认一次、取消、快速连续输入、滚动、字号和原生切换。共享字体测量改动后，Input / 光标坐标仍一致。

另以默认 `App.tsx` 和真实 fish 执行临时生产入口截图测试，深浅色两项通过（`artifacts/tiling-production-native.xml`），查看 `artifacts/tiling-production-dark.png` 与 `artifacts/tiling-production-light.png`。截图前等待窗口布局稳定，避免将窗口过渡帧当成界面结果。临时生产截图测试已删除，截图和报告为本地忽略产物。

### 验收边界

已实现与验证的是布局、会话保留、输入隔离和原生输入协议。实际系统输入法候选列表、复杂组合序列、跨行选择与复制、长时间持续输出和大量会话的 CPU / 内存 / 帧耗时，以及手动拖动的跟手感仍需独立人工验收。布局暂不持久化，UI 工作区重建从现存 Rust 会话退回单 pane；应用重启不恢复活进程。

## 2026-11-13 · 标题栏全局按钮

将新建分组、搜索终端、设置从侧栏底部移到窗口标题栏右上角，按钮区域禁用窗口拖动。侧栏展开时底部只保留会话数，收起时不再显示底部工具条。

`./scripts/nsh check`、`./scripts/nsh build` 与 `git diff --check` 通过。`./scripts/nsh test App.test.tsx --junit artifacts/global-actions-tests.xml` 为 18 / 18 通过。临时原生测试在 Dark / Light 下截图展开和收起状态，并验证收起后标题栏按钮分别打开设置、搜索和新分组编辑器，2 / 2 通过（`artifacts/global-actions-native.xml`）。已查看 `artifacts/global-actions-dark-collapsed.png` 与 `artifacts/global-actions-light.png`，临时测试已移除。

## 2026-11-13 · 标签组工作区与干净退出

每个标签组单独保存一棵平铺树，画布只显示当前组。切到别的组会换上那一组的布局，再回来时分屏还在。把其他组的标签拖进画面会加入当前组并按落点放置；菜单里的「显示已有终端」只列出当前组。空行 Ctrl+D 和退出码 0 关闭标签与 pane。退出码非 0 留下状态条。两个及以上 pane 在侧栏收成一个分屏卡片，成员与 pane 一一对应；卡片可以留在组外，也可以整块移入已有标签组，分屏本身不新建组。侧栏选中分屏以外的标签时，整个右侧换成这个终端，分屏树保留；再选中分屏里的标签，整组分屏回到画面。

`./scripts/nsh check` 与 `./scripts/nsh build` 通过。`./scripts/nsh test --junit artifacts/group-layout-tests.xml` 为 58 通过、1 失败：失败的是新加的真实双 PTY 步骤对着已经收起的 terminal 输入。改正步骤后，`./scripts/nsh test --filter "two real PTYs"` 通过。该次确认 `/bin/sh` 的 `exit 3` 保留标签和状态条，空行 Ctrl+D 关掉另一个仍在运行的会话。预览里的 Ctrl+D 与 `exit 1` 由 `ui/Tiling.test.tsx` 覆盖。本次没有重跑原生 AppKit 平铺验收。

## 2026-11-14 · 终端内容查找与滚动上限

⌘F 在当前终端内容里查找，大小写不敏感。真实会话的匹配、滚动和高亮留在 Rust，不把网格送进 JS。预览只显示命中计数。滚动历史上限为 10,000 行，设置里的 Terminal 页写明这个数字。⌘K 仍是按标题、目录和分组找终端。

`./scripts/nsh check`、`./scripts/nsh build` 与 `git diff --check` 通过。`./scripts/nsh test --filter search` 为 3 通过（含两条 Rust 匹配测试，以及既有的「search opens a terminal…」）。`./scripts/nsh test --filter scrollback_limit` 为 1 通过。`./scripts/nsh test terminal/Find.test.tsx` 为 2 通过：预览里查找 `preview` 得到「N of N」，关闭查找条后消失；设置 Terminal 页包含 `10,000`。本次没有重跑完整测试套件，也没有做原生 AppKit 验收。高亮和滚到命中只在真实会话路径上，预览测试看不到。

## 2026-11-14 · 设置里的滚动上限

Terminal 设置可以按 1,000 行调整滚动历史，范围 1,000 到 100,000，默认 10,000。改动立刻作用于已打开的真实会话，减小会丢掉最老的行；之后新建的会话使用这个值。预览终端按新上限重新建立网格。快捷键页补上关闭终端、上一个终端、新建分组、前九个终端和设置。外观、字号和字体本来就会立刻生效。这些值仍不写入磁盘。

`./scripts/nsh check` 通过。`./scripts/nsh test --filter set_scrollback` 通过。`./scripts/nsh test terminal/Find.test.tsx` 为 2 通过：设置里从 10,000 调到 9,000 再调回，快捷键页能看到 Close terminal、Previous terminal 和 Find in terminal。没有重跑完整套件，也没有做原生 AppKit 验收。

## 2026-11-14 · 窗格工具栏改为搜索入口

去掉单窗格工具栏上的新建按钮。非分屏终端仍由 ⌘T、窗口菜单和侧栏「New terminal」创建；没有终端时的空状态按钮保留。每个 pane 工具栏左侧第一个动作改为搜索，选中该 pane 并打开已有的内容查找条。

`./scripts/nsh check` 通过。`./scripts/nsh test terminal/Find.test.tsx` 为 2 通过：预览查找关闭后，工具栏没有 `titlebar-new-terminal`，按下 `pane-find` 会再次出现查找条。没有重跑完整套件，也没有做原生 AppKit 验收。

## 2026-11-14 · libghostty-vt 预编译 Rust crate

新增 `termlevel-ghostty-vt-sys` / `termlevel-ghostty-vt` 两个独立 workspace 包，固定官方 libghostty-vt **1.0.1**。官方 XCFramework 的 SHA256 为 `c0f9631277f726154c8f521954d17cdc41aac4626fc0df8cf925a19ee58e829b`；sys 层下载并验证后提取 macOS 目标架构静态库。原版 C 头文件和 bindgen 0.72.1 生成声明均提交为源码；构建不运行 Zig 或 bindgen。详细接口与边界见 [crate README](../crates/ghostty-vt/README.md)。

验证环境为 macOS arm64、Rust 1.99.0。两个 native crate 的测试均调用真实预编译库：9 项覆盖分段 UTF-8 / 转义序列、独立终端、备用屏幕、resize / reflow / history、reply / title / cwd / bell / reset、快照释放后可用、宽字符 / 组合字符 / truecolor / 下划线 / 光标、legacy / Kitty 按键、鼠标 / 焦点 / 粘贴编码和无效参数。另有 1 项使用 Clang 将 7 个关键 C 结构的尺寸与对齐和 Rust 对照，3 项 doctest 包含 `!Send` / `!Sync` 编译失败约束。

构建路径另外验证：

- 清除 archive/offline 环境覆盖后执行 `cargo test -p termlevel-ghostty-vt-sys -p termlevel-ghostty-vt`，自动下载官方库、校验并静态链接通过。
- 缺包时 `GHOSTTY_VT_OFFLINE=1 CARGO_NET_OFFLINE=true` 正确拒绝下载；指定损坏 zip 时正确以 SHA256 不匹配失败。
- 通过 `GHOSTTY_VT_ARCHIVE` 指定已验证 zip 可以在网络受限环境完成构建。缓存位于每次 Cargo 构建的 `OUT_DIR`，不同构建配置可能需要再次提供 archive 或下载。
- 重新生成 bindings 与 helpers 得到完全相同内容；36 个 vendored 头文件逐字节匹配官方 XCFramework。
- `cargo package --allow-dirty -p termlevel-ghostty-vt-sys` 的独立打包及校验编译通过，源码包压缩后约 153 KiB，不包含预编译二进制。包尚未发布。

项目回归：`./scripts/nsh check`、`./scripts/nsh build` 通过；`./scripts/nsh test --junit artifacts/ghostty-crates-headless.xml` **80 / 80** 通过，包含新 crate 的 native / doctest 和原有 Rust / TSX 测试。受限网络下使用已下载 zip 的 `GHOSTTY_VT_ARCHIVE` 与 `CARGO_NET_OFFLINE=true`。两个新包的 `cargo clippy --all-targets -- -D warnings`、定向 `cargo fmt --check` 与 `git diff --check` 通过。

首次全量运行发现既有 `large_output_stays_bounded_and_finishes_before_view_attaches` 仍按 3,000 行旧默认值断言。测试现显式配置 3,000 行，按该上限与实际屏幕行数检查边界，并保留最老行淘汰和最新行保留断言；生产默认的 10,000 行不变。修正后完整重跑取得上述 80 / 80。

本轮没有把 Ghostty 接入生产 `SessionManager` / NSH surface，也没有进行原生 GUI 验收。x86_64 目标已配置，但未安装对应 Rust target，尚未执行该架构的 Rust 构建或运行；Windows / Linux 尚未接入。搜索、选区、超链接、图片、剪贴板和同步输出超时等安全层能力仍待补齐，不能据此声称生产终端兼容性已完成。

## 2026-11-14 · Ghostty 接入默认应用

默认 `SessionManager` 和显式预览 provider 均改用官方 libghostty-vt 1.0.1，移除应用的 Alacritty 依赖和旧适配实现。Rust 继续拥有会话 / PTY / reader / writer / reaper，UI 仍按稳定 session ID 绑定。活动屏幕使用 Rust RenderState 帧缓存，历史行按需读取；没有把完整网格或输出流发送到 JS。

安全层补齐原生搜索、颜色默认值与调色板、尺寸查询回复、历史行与精确逻辑窗口、同步输出边界快照及主动提交。原生 page-granular 历史之外，通过各屏幕的 tracked reference 保留精确逻辑起点；改小再调大不会复活旧行。同步输出正常结束、150 ms 超时、EOF 和 resize 都有处理；另有回归覆盖 hold 快照消费 dirty state 后，另一个 renderer 不得漏掉之前的修改，以及同一 chunk 在开始 hold 前产生历史时逻辑窗口仍严格受限。

线程契约经审查后调整为 Send / !Sync，应用以同一 `Session.emulator` Mutex 串行化解析、渲染、查询和输入编码，说明与上游契约来源见 [THREADING.md](../crates/ghostty-vt/THREADING.md)。原生终端和 renderer 的跨线程移动有独立测试。按键、IME 最终提交文字、粘贴和焦点事件经 Ghostty 编码；原生 Input 的组合确认 / 取消与稳定挂载机制保留。视图接入 block / bar / underline 光标几何。

### 自动化回归

- `GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip CARGO_NET_OFFLINE=true ./scripts/nsh check` 与 `./scripts/nsh build` 通过。路径为本地已校验的官方 zip；可用同版本其他路径替代。
- `./scripts/nsh test --junit artifacts/ghostty-integration-headless.xml`：**85 / 85** 通过，使用相同 archive/offline 环境。包括独立 C ABI / safe crate、全部旧终端回归迁移到 Ghostty、真实 PTY、UI 重绑、多 pane 输入隔离和全部 TSX 交互。
- 新增真实 raw PTY 字节对照：应用编码的 SS3 方向键、Ctrl+Shift+F5、中文、保留换行的 bracketed paste 和焦点报告共 42 字节，与子进程通过 `dd | od` 实际读到的结果逐字节一致。初次测试把 od 的双空格写成单空格而超时；现在等待 reader 排空后解析 hex token 并比较完整字节数组，未为测试修改编码行为。
- `cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check` 与新脚本 `sh -n` 通过。

### macOS AppKit

以下原生测试均连接真实 PTY，没有使用 preview transport。首次受限沙箱阻止本地 dev server 监听；在允许本地开发服务器和 AppKit 宿主后执行通过。

- `./scripts/test-native-input`：1 / 1 通过。中文 marked text、确认一次、取消、快速输入、切换会话后的提交归属、滚动与字号变更后的 Input / 光标坐标对齐。报告 `artifacts/native-ime-tests.xml`。
- `./scripts/test-native-tiling`：1 / 1 通过。真实双 PTY、pane 交换时继续组合、确认文字仍到原会话、方向焦点、分隔缩放、放大 / 还原和窄窗口恢复。报告 `artifacts/tiling-native.xml`；已查看两 pane 截图。
- 新增可重复执行的 `./scripts/test-native-ghostty`：2 / 2 通过，fixture 为 `terminal/fixtures/Ghostty.acceptance.tsx`。直接打开默认 `App.tsx`，使用本机默认 fish，在 Dark / Light 下确认中文命令执行、真实搜索计数 / 高亮 / 跳转，以及 `seq 1 200 | /usr/bin/less -R` 跳到末尾、显示 200 / (END)、退出后恢复 shell 内容。报告 `artifacts/ghostty-production-native.xml`。截图包括 `artifacts/ghostty-production-{dark,light}{,-find,-less}.png`，已查看深色搜索与浅色 less。脚本的临时注册文件在退出时清理，截图 / 报告均为忽略的本地产物。

### 尚未验证与未接通

以上证明 macOS arm64 的内核迁移、真实输入与指定原生流程，不代表完整日用兼容性。鼠标编码器尚未接到 UI，图片 / 超链接 / 终端选区手势和系统剪贴板协议尚未接入；精细下划线仍退化为普通下划线。NSH 绑定没有完整 key release / repeat action / 物理布局信息，不能声称完整 Kitty 键盘协议。真实系统输入法候选列表、复杂 emoji、跨行选择复制和持续输出 / 大量会话性能仍需独立验收。当前 PATH 未找到 Neovim / tmux，未做这些 TUI 的实际兼容性测试；less 的上述限定流程已通过。x86_64 和其他平台尚未验收。

## 2026-11-14 · 固定终端格网替换

工作目录 `/Users/biu/Projects/termlevel`，`main` 基于 `4e8c7ff` 的未提交工作树。保留交接时已有改动，没有提交或推送。NSH 仍固定 0.5.0，libghostty-vt 为 1.0.1；使用本机 `/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip` 与 `CARGO_NET_OFFLINE=true`。

生产 surface 已删除 virtual_list、逐行 canvas、PaintedScreen、image_strip、滚轮遮罩和列表滚动 API，改为两张画布、三层图片、独立光标和原 field editor。测量盒与整格绘制矩形分开，历史以整数窗口和 Ghostty tracked anchor 定位。锁内只取拥有内存的 Frame/历史 cells/图片与颜色上下文，锁外合并 run、整理语义文本和录制 Painter。查找改为原生 ASCII 分步 feed/tick、Unicode 64 行批次，SearchChange 提供异步计数；隐藏 pane 不随新输出抓帧。设计和限制见 [terminal-grid.md](terminal-grid.md)。

### 自动化

- `./scripts/nsh check`、`./scripts/nsh build` 通过。
- `./scripts/nsh test --junit artifacts/terminal-grid-tests.xml`：**102 / 102** 通过。包含真实 PTY、工作区重绑、多 pane 输入隔离、Unicode 异步查找和旧查询替换，以及独立 C ABI / safe crate 测试。
- 新增真实库证据：历史锚点跟随缩窄/放宽的 reflow、备用屏幕和淘汰；held 窗口冻结屏幕身份与历史；选区使快照失效；活动图片共享 Frame 像素；纯窗口跟随与亚行触控板累加；每个 run 的独立 begin_path/clip；锁外第二阶段在 terminal 被修改并销毁后仍输出旧快照。
- 本机固定 1.0.1 **resize 会结束 hold**。新增测试先证明切换备用屏幕本身仍保留 hold，再证明 resize 后提交新屏幕和尺寸；修正了旧 handoff 中「resize 后还保持 hold」的假设。
- `cargo clippy --workspace --all-targets --offline -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check` 通过。

### macOS AppKit 分项验收

- `./scripts/test-native-input`：1 / 1 通过，实际 field editor 的 marked text、确认/取消、会话切换、滚动/resize/字号变化后的输入与光标坐标。报告 `artifacts/native-ime-tests.xml`。这不是用户实际系统输入法候选窗验收。
- `./scripts/test-native-protocols`：最终完整复验 **3 / 3** 通过。实际 Kitty press/repeat/release、PNG、OSC8 文本、拖选复制、Select All、OSC52 和 SGR 鼠标报告/读取设置。报告 `artifacts/protocols-native.xml`。已查看 `native-terminal-graphics.png` 和 `native-terminal-selection.png`；三层重叠图片与动画压力仍待单独验收。
- `./scripts/test-native-ghostty`：2 / 2 通过。默认生产 App、默认用户 shell、1200×780、Dark/Light，覆盖分步查找、less 到末尾与退出、ED2 clear。断言 clear 后可见区域包含 GRID_CLEAR_OK 且不含旧 GHOSTTY_PROD_ 行。报告 `artifacts/ghostty-production-native.xml`。已查看 `ghostty-production-dark-clear.png` 与 `ghostty-production-light-less.png`，前者没有顶部历史细条，后者显示到 200 / (END)。

首次原生脚本因沙箱不能监听本地开发服务器而未进入交互；获准启动本地 NSH 服务器与 AppKit 窗口后完成验收。早期文本断言依赖逐行 Text 视图，现改为读取画布语义内容；CanvasNode 提供可见行命中元数据，但不挂载 NSH 行视图。自定义未标记 NSEvent 会被 NSH 测试宿主过滤，该临时调试方法已删除；最终拖选验收使用 NSH 自身驱动，并实际读取复制结果。测试失败没有通过直接设置模拟器选区来绕过。

### 尚未证明

没有测量 Codex 连续滚动的 FPS、输入延迟或原生 CoreText replay；不能从 102 项测试或 Painter 录制耗时推导性能达标。调试采样曾记录个别展示更新超过 8 ms，后来将宿主 run 合并移到锁外；没有据此声称整体帧预算已经满足。原生 capture 仍会复制 viewport。

实际系统 IME 候选列表、复杂 emoji/fallback 基线/连字、三层图片重叠与动画、多 pane 长时间持续输出、真实触控板手感和辅助功能日用仍需人工验证。用户可运行 `./scripts/nsh run --window-size 1200x780` 做日用验收。Kitty file/temp/shared-memory 继续关闭，pane 工具栏仍是启动路径，不做 OSC7。截图和报告均在被忽略的 artifacts 下。

## 2026-11-14 · 应用自有 AppKit renderer 嵌入 NSH

在同一 `main` / `4e8c7ff` 未提交工作树上继续；保留已有 dirty changes，不提交、不推送。NSH 的源码没有修改，CLI 和所有 `nsh-*` 仍固定 **0.5.0**，libghostty-vt 仍为 **1.0.1**。

### 实现边界

使用 NSH 的公开 `#[native_components(macos)]` + `NativeComponent` 注册 `TermlevelTerminalSurface`，由 Termlevel Rust 自己用 CoreText / Core Graphics 绘制网格和三层图片。NSH 保留工作区布局、容器、独立光标和原有唯一 Input / field editor；不使用 Ghostty 应用的内部嵌入 ABI，不是整帧转 RGBA 后交给 NSH。两张绘图 canvas 与 NSH 图片子树已经删除；剩下的 canvas **只有 CanvasNode 语义 metadata，没有任何绘图操作**，供 NSH 测试驱动按可见行定位。

引擎发布最新 `Arc<Scene>`，原生视图仅在复制该 Arc 时短暂取得场景槽锁；绘制不持有 emulator 锁。跨 NSH 的 frame 属性仅包含 Rust 场景槽 ID / revision。原生组件不成为第一响应者，hitTest 返回 nil，格子事件和原有 Ghostty 输入编码保持同一条路径。字体 / CTLine 有界缓存，图片按 owned 像素身份缓存，挂载生命周期不拥有 PTY。

### 已执行的验证

相同的 archive / offline 环境：

```sh
GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip
CARGO_NET_OFFLINE=true
```

- `./scripts/nsh check`、`./scripts/nsh build` 通过，日志 `artifacts/native-surface-{check,build}.log`。
- `./scripts/nsh test --junit artifacts/native-surface-tests.xml`：最终 **106 / 106** 通过。新增缓存达到 2048 项后的淘汰与最近使用保留测试；原生位图测试覆盖图片三层顺序、源区域裁剪、显式背景；字体与 CTLine 缓存跨重绘/主题复用、字体改变失效；最新帧覆盖与槽释放；语义 adapter 不录制绘图操作。
- `cargo clippy --workspace --all-targets -- -D warnings` 通过，日志 `artifacts/native-surface-clippy.log`。
- `./scripts/test-native-input -v`：**1 / 1** 通过，日志 `artifacts/native-surface-ime.log`。实际 marked text 确认/取消、会话切换和输入锚点；记录 input/grid 坐标均为 `(319.5, 234)`。不是实际系统输入法候选列表的人工验收。
- `./scripts/test-native-protocols -v`：**3 / 3** 通过，日志 `artifacts/native-surface-protocols.log`。实际 Kitty 按键、PNG、OSC8 文本、拖选 / Select All 复制、OSC52、SGR 鼠标报告。已查看新视图绘制的 `artifacts/native-terminal-graphics.png`。
- `./scripts/test-native-ghostty -v`：**2 / 2** 通过，日志 `artifacts/native-surface-ghostty.log`。生产工作区默认 shell、Dark / Light、查找、less、ED2 clear；已查看新 renderer 的 `artifacts/ghostty-production-dark-clear.png`，未见历史细条。

### 原生滚动性能对照

`scripts/test-native-scroll` 使用真实 PTY 中的 `terminal/fixtures/scroll_probe.py`：1200×780，Menlo 13 pt、21 pt 行高，备用屏幕、1000/1006 鼠标报告、2026 同步输出、中英文和彩色 run。滚轮经 debug AppKit 驱动进入实际 grid 的 NSH wheel handler，再由原 Ghostty encoder 编成鼠标字节；没有用后台直接 feed 绕过滚轮和 PTY。fixture 的 JS interval 只是自动化输入发生器，生产数据与绘制不走 JS。

- 旧 Canvas 路径：`artifacts/scroll-before.log` 和 `artifacts/scroll-before.sample.txt`。NSH 帧率计约 **38–44 / 60 FPS**，Canvas 原生回放均值约 **24–26 ms**、p95 约 **44 ms**；CPU 采样热点包括 `Replay::text -> make_font` 与 CoreText 字体匹配。
- 自有 NSView 第一轮：`artifacts/scroll-native-surface.log`，原生测试 **1 / 1** 通过。稳态绘制均值约 **2.6–2.9 ms**，NSH 帧率计多数 **54–59 / 60 FPS**，也出现约 50 FPS 区间。原生组件记录从 8 次到 496 次实际 drawRect；不是仅更新测试文本而未绘制。已查看 `artifacts/native-scroll-end.png`，中英文、颜色和整行位置正常。
- 25 秒滚动完整复验：`artifacts/scroll-native-surface-sustained-final.log`，**1 / 1** 通过。缓存达到 **2048 项** 后绘制均值约 **2.7–3.1 ms**，后段 NSH 帧率计约 **54–60 / 60 FPS**，原生组件累计 **905** 次实际绘制。fixture 明确断言实际原生视图绘制计数大于 10，不能以空组件通过。
- 首次 25 秒压力轮次已达到 2048 项缓存上限，原生绘制仍约 2.7–2.8 ms，但测试连同启动超过默认 30 秒而被测试器终止；随后给该长负载明确设置 60 秒测试预算，保留失败日志 `artifacts/scroll-native-surface-sustained.log`。
- 两组都是本机 macOS arm64 debug build。NSH 的 `canvas=0` 仅说明不再走 Canvas 回放；新绘制耗时使用原生 renderer 自己的统计，不能据此宣称绘制零成本。NSH FPS 与终端快照发布次数不是同一指标。第一轮性能值为受控 fixture 的观测，不是严格隔离的机器基准，也不是实际 Codex 的录制。

### 尚未证明

真实 Codex 的连续触控板滚动、输入延迟、实际系统 IME 候选窗、复杂 emoji / fallback / 粗斜体、动画图片压力、多 pane 持续输出及细粒度 AX 日用仍需人工验收。当前原生 renderer 只实现 macOS；其他平台需要自己的后端。没有把 CoreText 路线称作 Ghostty 应用的 Metal / atlas renderer。

运行 `./scripts/nsh run --window-size 1200x780` 验证日用。Kitty file/temp/shared-memory 继续关闭；pane 标题仍为启动路径，未做 OSC7；没有 footer、wheel cancel 或框架补丁。

## 2026-11-14 · 每会话 SurfaceController

在 `main` / `57a846e` 之后的未提交工作树上实现 [终端 Surface 控制器](terminal-surface.md)：每会话一个 `SurfaceController`，唯一 16 ms 帧调度、单一采样入口，删除 `view.rs` 的第二个时钟与 `Grid` 的逐事件捕获。输入路径不排队：键、IME、粘贴、按下 / 释放、报告滚轮都在到达时同步编码；悬停与宿主拖选取最新值、每轮最多应用一次；历史滚轮只更新整数窗口。报告滚轮取消 32 行截断，整数位移全部编码、小数余量保留。所有鼠标入口（含 AppKit 中键 / 侧键）统一走 `Session::mouse_reports`，在同一把锁内判定模式并编码。release 作为屏障同步应用最终格，避免点击清空后被挂起终点重新选区。

### 自动化

- `./scripts/nsh test --junit artifacts/surface-controller-tests.xml`：**112 / 112** 通过。新增 surface 状态机测试覆盖：已贴底时跟随不触发采样、未贴底跟随只采样一次、亚行滚轮累计、一轮最多一次捕获、用户 origin 标记、悬停 / 拖选保留最新值。
- `view.rs` 纯函数测试：65 / 33 行滚轮位移不被截断；release 使用最终格而不是中间终点。
- `cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check` 通过。
- `./scripts/nsh check`、`./scripts/nsh build` 通过。

### macOS AppKit 分项验收

- `./scripts/test-native-input -v`：**1 / 1** 通过。marked text 确认 / 取消、网格光标坐标与输入锚点仍一致。
- `./scripts/test-native-protocols -v`：**3 / 3** 通过。Kitty 键、PNG、OSC8、拖选复制 / Select All、OSC52、SGR 鼠标报告与读取设置。
- `./scripts/test-native-ghostty -v`：**2 / 2** 通过。生产工作区默认 shell、Dark / Light、查找、less、ED2 clear。
- `./scripts/test-native-tiling -v`：**1 / 1** 通过。多 pane 几何、IME 归属与状态在拆分 / 移动后保持。
- `./scripts/test-native-scroll -v`：**1 / 1** 通过。受控鼠标报告 TUI 滚动 34 s；本次运行原生绘制约 3.4–3.7 ms、NSH 帧率计 55–57 / 60，缓存 2048 行。该数值来自单次运行，不是隔离基准，也不与 25 秒轮次的变化作因果结论。

### 尚未证明

真实 Codex 手感、实际系统输入法候选窗、跨行选区、三层图片动画与多 pane 持续输出仍需用户人工验收。`TERMLEVEL_SURFACE_STATS` 的计数只用于确认采样收敛，不代替输入延迟或端到端帧预算测量。writer 队列满仍沿用原有可见错误语义，未改成独立的瞬时状态。

## 2026-11-15：连续同步输出的展示饥饿

用户实际运行 Codex 和 Claude 后的日志保存于 `artifacts/codex-claude-stutter-before.log`（原 `/tmp/termlevel-codex-stutter.log`）。第 44 行：约 2 秒内 111 次报告滚轮、115 次 revision 变化、126 个 tick，但 125 次 hold 跳过且仅发布一次；连续等待发布达到 2100.8 ms。日志没有程序标签，不能据此分别归因到 Codex / Claude；该时长也不代表单次 2026 事务的持续时间。

修复使贴底窗口能够推进到当前 hold 开始前的完整 owned 边界帧，同一 Arc 复用；历史窗口冻结，不读取未完成的 live cells / 图片 / 屏幕身份。继续保留脏位直到事务结束或超时，以提交最后一帧。详见 [terminal-surface.md](terminal-surface.md)。

本轮验证（macOS arm64 debug）：

- `./scripts/nsh check`、`./scripts/nsh build` 通过。
- `./scripts/nsh test --junit artifacts/sync-boundary-tests.xml`：**114 / 114**。新增真实 Ghostty 测试连续 160 次“结束上一事务 + 开始下一事务”，每次采样仍处于 hold，要求逐次发布完成帧；另验证同边界复用、未完成文字和切屏隔离。既有历史窗口冻结与 resize 提交用例保留通过。
- `cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check` 通过。
- `./scripts/test-native-scroll -v`：**2 / 2**。保留 25 秒常规滚动，并新增真实 PTY / AppKit 滚轮的连续事务场景，要求持续输入时连续六次、每隔 500 ms 的可见帧号都推进。日志：`artifacts/sync-boundary-native-scroll.log`，截图：`artifacts/native-scroll-consecutive-sync.png`。
- 连续事务场景的三个约 2 秒窗口分别发布 **121 / 116 / 115** 次，最长等待推进约 **22 ms**；原生绘制均值约 **2.6–2.8 ms**。这是受控复现验证，不是实际 Codex / Claude 修复前后的等负载基准，也不是端到端输入延迟测量。
- 原生回归串行通过：`./scripts/test-native-input -v` **1 / 1**（组合确认 / 取消、切换会话和输入锚点）；`./scripts/test-native-protocols -v` **3 / 3**（键盘、图片、鼠标报告和剪贴板）；`./scripts/test-native-ghostty -v` **2 / 2**（生产 Dark / Light、shell、查找、less 和 clear）。日志均为 `artifacts/sync-boundary-native-{input,protocols,ghostty}.log`。

首次原生测试因沙箱禁止 dev server 监听失败；授予原生测试所需权限后重跑通过。测试宿主链接仍有既有 `__eh_frame` 过大警告。用户实际 Codex / Claude 修复后的手感、持续多 pane 负载尚未复验。

用户随后重新试用并确认“可以了”，本轮 Codex / Claude 滚动卡顿修复获得人工确认。该确认不扩展到持续多 pane、复杂输入法或全部终端兼容性验收。

## 2026-11-15：中文高亮背景的 run 接缝

用户截图中的蓝色高亮在中文及样式分段处出现细竖缝。旧 renderer 跟随文字 run 分别填充背景，小数格宽与几何抗锯齿会使相接矩形的覆盖率不足。现在每行连续同色背景独立合并（含 inverse 与宿主选区），共享边界在 CGContext device space 统一取整，仅背景阶段关闭几何抗锯齿，随后恢复绘制状态。文字布局与图片层次保持原有职责。

本轮验证（macOS arm64 debug）：

- `./scripts/nsh check`、`./scripts/nsh build` 通过。
- `./scripts/nsh test --junit artifacts/background-seams-tests.xml`：**116 / 116**，日志 `artifacts/background-seams-tests.log`。新增原生 RGBA 位图用例在 1× / 2× backing scale、小数格宽高和原点偏移下逐像素检查中文 / ASCII 分段、相邻行、显式背景 / inverse / 选区和相邻不同色矩形的实心覆盖；既有三层图片与源裁剪用例通过。
- `./scripts/test-native-protocols -v`：**4 / 4**。新增真实 PTY 中的中文 / ASCII / 粗体混合高亮列表，截图 `artifacts/native-continuous-backgrounds.png` 已查看：蓝色色带连续，未见原截图的分段竖缝。既有 Kitty 键盘、图片、选区复制、鼠标和 OSC52 回归通过；日志 `artifacts/background-seams-native.log`。
- `cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check` 通过。

原生测试使用所需的沙箱外本地测试服务权限；宿主链接仍有既有 `__eh_frame` 警告。以上为受控位图和真实原生窗口验证，用户截图中实际 Codex resume 界面尚未由用户复验。

## 2026-11-15：终端 renderer 的平台边界整理

旧 `native_surface.rs` 分为平台无关的 `presentation.rs`（最新场景槽 / 弱注册表 / 属性绑定）、`platform/macos/view.rs`（NSView 宿主）和 `platform/macos/renderer.rs`（CoreText / Core Graphics 绘制及缓存）。原 `keyboard.rs`、`native_probe.rs` 同步移入 macOS 后端。通用 `view.rs` 经 `platform` 接入 surface 与输入，不再包含 macOS 条件编译或引用具体后端；只有 crate 根的宿主注册入口需要平台类型。新增后端的具体契约见 [architecture.md](architecture.md)。

源码对照确认 renderer 主体、键盘与原生探针保持原有逻辑，仅调整模块可见性和引用路径。共享发布测试迁出 macOS renderer，并增加 token 解析、宿主持槽和最后释放后 token 失效的验证。会话生命周期、同步输出修复、背景像素边界、NSH 0.5.0 与目标平台依赖版本均未改变。

本轮验证（macOS arm64）：

- `./scripts/nsh check`、`./scripts/nsh build` 通过。
- `./scripts/nsh test --junit artifacts/platform-boundary-tests.xml`：**116 / 116**，日志 `artifacts/platform-boundary-tests.log`。
- 原生脚本串行通过：Input **1 / 1**、Protocols **4 / 4**、生产 Ghostty Dark / Light **2 / 2**、Scroll **2 / 2**（包括连续同步事务）。日志为 `artifacts/platform-boundary-native-{input,protocols,ghostty,scroll}.log`。已查看更新后的 `artifacts/native-continuous-backgrounds.png`，高亮色带连续。
- `cargo clippy --workspace --all-targets -- -D warnings`、fmt 和 diff check 通过。
- `cargo check --release -p termlevel-native` 通过；当前诊断代码仍产生 unused imports / dead code 警告，release 未声称 warning-free。

本次只有 macOS 后端通过构建与原生验收。非 macOS 的 `unsupported.rs` 为明确占位，尚未进行目标平台编译；Ghostty 预编译提取仍只支持 macOS。系统剪贴板、工作区原生快捷键和其他平台的构建 / 打包仍是后续任务。

## 2026-11-15：图形字形断裂与空白格背景丢失

用户提供 Termlevel 和正常终端的 Codex / Claude 对照截图：块字符图标上下断开、画线出现接缝，提示区域背景只跟随文字长度而形成阶梯。诊断用真实 PTY 输出相同 Unicode 字符及带背景色的 `EL` / `ED` 指令，截图 `artifacts/glyph-audit-before.png` 已确认复现。

背景根因在安全封装：只读取原始 style，漏读空白格 packed cell 的 RGB / palette 背景 content tag。新增真实库测试在修复前失败于活动帧第 0 行第 1 列（实际 Default，预期 RGB），日志 `artifacts/erased-background-red.log`。现在活动帧、hold 回调帧与历史行共用 `Style::with_cell_background`，并保留 palette 索引以跟随 OSC4 更新；renderer 无需猜测空白区域的颜色。

图形字符不再按普通文字基线和行距绘制。macOS renderer 复用系统等宽字体选出的矢量轮廓（含 CoreText fallback），按块 / 画线族的共享参考平面映射到完整像素格，避免 raster hinting 缩边；半块与象限保持相对比例。普通文字继续使用用户选择的字体和原文字路径。未引入手写字符形状表，也未接入 Ghostty 应用内部 sprite ABI；现有 libghostty-vt 1.0.1 没有导出该 renderer。参考平面与轮廓在原有有界 renderer 缓存中复用。

本轮验证（macOS arm64 debug）：

- `./scripts/nsh check`、`./scripts/nsh build` 通过。
- `./scripts/nsh test --junit artifacts/graphics-background-tests.xml`：**120 / 120**，日志 `artifacts/graphics-background-tests.log`。新增覆盖 RGB / palette 空白背景、EL / ED、活动 / held / 历史快照和 OSC4；原生位图逐像素验证完整背景、实心块覆盖、半块比例，以及细线 / 粗线 / 双线边框跨格、跨行连接。普通字体选择切换为 Menlo / Monaco / monospace 时图形比例仍保持。
- 原生回归串行通过：Protocols **5 / 5**、生产 Ghostty Dark / Light **2 / 2**、Input **1 / 1**、Scroll **2 / 2**（保留连续同步事务回归）。日志为 `artifacts/graphics-background-native-{protocols,ghostty,input,scroll}.log`。
- 新增 `terminal/fixtures/glyph_probe.py`，原生截图 `artifacts/native-terminal-glyphs.png` 已查看：图标连接、块色带与横线连续，提示背景包括空白行和擦除到行末部分，呈完整矩形。该 fixture 不启动模型或发送 API 请求。
- `cargo clippy --workspace --all-targets -- -D warnings`、fmt 与 diff check 通过。

实际 Codex / Claude 修复后的用户复验尚未进行。字体回退中的其他 Unicode 图形族、emoji、任意字体及平台的完整终端兼容性仍不在本轮验收范围内；没有据受控截图宣称完整 Ghostty 应用 renderer 等价性。

## 2026-11-15：打包应用图标

`nsh.json` 的 `icon` 指向 `assets/brand/termlevel-icon-v1.png`，生成提示词与来源保留在同目录 Markdown 中。图标为带 alpha 的正方形 PNG，由 NSH 0.5.0 自动转换为 macOS 图标资源；常规命令是 `./scripts/nsh package --target macos`，无需额外图标参数。README 已记录打包方式。

- `./scripts/nsh check`、`./scripts/nsh build` 通过。
- `./scripts/nsh package --target macos --out .nsh/package-icon-check` 通过，生成 `.nsh/package-icon-check/Termlevel.app`，日志 `artifacts/package-icon.log`。离线构建使用固定 Ghostty archive 与 `CARGO_NET_OFFLINE=true`。
- 检查 `Info.plist`：bundle ID 为 `app.termlevel.desktop`，`CFBundleIconFile` 指向 `Contents/Resources/AppIcon.icns`。解码图标为 1024×1024 RGBA，alpha 范围 0–255；与源 PNG 同尺寸缩放后的像素均值差约 0.427 / 255，确认实际采用所选图标。
- `codesign --verify --deep --strict .nsh/package-icon-check/Termlevel.app` 通过；包使用默认 ad hoc 签名，不等同于分发签名或公证。
- 沙箱内首次调用 `iconutil` 报 Invalid Iconset；使用打包所需的系统工具权限重跑后成功。首次 release 宿主编译约三分钟，仍有此前诊断代码的 9 条警告。

本次只改变图标资源与打包配置，没有重跑完整交互测试；验证针对真实应用包和图标资源，未额外启动打包应用确认 Finder / Dock 的图标缓存表现。打包与编译产物均位于忽略目录，不提交。

## 2026-11-15：工作区与设置持久化

首次启动不再自动创建 shell：窗口打开后画布显示空白引导，可直接在 home 打开终端或新建分组。保存的标签组、标签顺序、固定状态、分屏树、选中标签、侧栏宽度与展开状态、字号、滚动历史、界面 / 终端字体和外观写入 NSH Storage，路径为 `~/Library/Application Support/app.termlevel.desktop/storage.json`。应用重启时标签按保存的目录重新启动 shell，旧 session ID 映射到新会话；进程 PID 和终端输出不写入文档。剪贴板读取权限仍按设计在重启后复位。

文档带 schema 版本。损坏、未知版本或非法布局节点会被丢弃，应用以安全默认值继续启动；标签引用不存在的分组时回到未分组区域，布局中无法映射的 pane 会被剪掉。`ui/workspace-snapshot.ts` 负责解析与校验，`ui/persistence.ts` 负责 NSH Storage 读写，`ui/WorkspaceRoot.tsx` 在挂载工作区前应用字体与外观，避免首帧闪烁。

本轮验证（macOS arm64 debug）：

- `./scripts/nsh check`、`./scripts/nsh build`、`cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings` 通过。
- `./scripts/nsh test --junit artifacts/persistence-tests.xml`：**126 / 128**。新增 `ui/Persistence.test.tsx` 5 项与 `ui/Snapshot.test.tsx` 3 项全部通过：首次启动空白引导、home 终端启动与保存、空白引导新建 home 分组、分组 / 固定 / 分屏恢复、折叠状态保留、目录消失后回退 home、损坏与只有分组的文档回退、schema 解析与引用规范化、重复 pane / session ID 的布局丢弃。
- 两项失败与本次改动路径无关，未通过修改测试绕过：
  - `sessions::tests::real_shell_utf8_resize_title_and_exit_status` 等待 PTY 状态超时。该项在 `docs/validation.md` 中已有超时后单独重跑通过的记录；本轮单独重跑仍超时，未修改 Rust PTY 或超时阈值。
  - `terminal/Session.test.tsx` 的中文粘贴断言在 headless 下得到 `--------` 而不是 `PASTE_中文`。该测试与 `SessionFixture` 本轮未修改，历史记录通过的是 `--target macos` 原生运行。
- 预览模式显式不读写真实存储，`App.test.tsx` 仍使用确定性演示工作区。
- 持久化测试使用 headless 测试自己的 NSH Storage，`app.storage()` 可读回保存文档，不触碰用户真实 `storage.json`。

尚未进行真实窗口重启人工验收：需要在原生宿主里确认首次空白页、重建分组后退出再打开时 shell 数量与目录、侧栏宽度 / 外观 / 字体恢复，以及关闭窗口和退出应用时最后一次保存能落盘。headless 证据不等于原生 GUI 验收。

## 2026-11-15：目录结构与开发规范

在 `717918f` 基础上的未提交整理：TSX 生产代码归入 ui/app、workspace、settings、terminal、shared；交互测试与 fixtures 集中到 tests/workspace、tests/terminal，原生 acceptance 放 tests/native，Python 探针放 tests/probes。50 个迁移源文件逐一与 HEAD 比较，差异仅为 import、open 和探针路径字符串。Rust 所有权与实现未变。旧章节的文件路径与命令保留为历史证据，当前路径见 development 和 architecture。

更新 AGENTS 的真实 PTY 与重启恢复说明；新增 development 开发指南；五组原生脚本共享 scripts/test-native，保持原入口、输出报告名及 scroll 的 fps 选项。fixture 邻近 .nsh 缓存纳入忽略范围。

本轮 macOS 验证：

- `./scripts/nsh check`、`./scripts/nsh build`、`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`git diff --check` 通过。
- `./scripts/nsh test --junit artifacts/directory-headless.xml`：**128 / 128**；日志 `artifacts/directory-headless.log`。测试发现数量保持 128，包含工作区、持久化、真实会话与重新绑定。此前持久化记录的两个失败本轮未复现，不据此认定间歇性问题已根治。
- `./scripts/test-native-protocols`：**5 / 5**；日志 `artifacts/directory-native-protocols.log`，报告仍为 `artifacts/protocols-native.xml`。验证新 acceptance / fixture / Python 路径与共享脚本，覆盖 AppKit 键盘、背景、字形、图片、选择、鼠标与 OSC52。初次受限运行无法监听本地 dev server，放开该限制后通过。
- 所有原生入口通过 shell 语法检查；本轮未运行其他四组原生场景，未进行人工 GUI 验收。

## 2026-11-15：OSC52 中文复制乱码

用户反馈 Claude 中复制中文后粘贴乱码。定位到 SystemClipboard 启动 pbcopy / pbpaste 时继承宿主 locale，而两者按 locale 决定字节编码。Rust / Ghostty 的接口传 UTF-8；C 或无法识别的 locale 会让 pbcopy 将这些字节按旧编码解释。修复为只在 helper 子进程固定 LANG、LC_CTYPE、LC_ALL 为 macOS 支持的 en_US.UTF-8，不更改应用、shell 或用户环境。

新增 tests/probes/clipboard_utf8_probe.py 和原生协议用例：真实 PTY 发送 OSC52，写入中文标点、重音文字、组合字符、emoji 与换行，再查询 OSC52；通过 AppKit NSPasteboard.stringForType 独立核对系统剪贴板的完整 Unicode 字符串，测试前后保存 / 恢复剪贴板。只检查 helper 往返不足以证明正确，因为两端同样用错编码时仍可能还原原字节。

修复前：`LC_ALL=C LANG=C LC_CTYPE=C ./scripts/test-native-protocols` 为 **5/6**；新用例失败，AppKit 实际读到 `Claude ‰∏≠Êñá…`，而 OSC52 往返仍显示 OK。日志 artifacts/clipboard-utf8-before.log。

修复后相同命令 **6/6**，Unicode 字符串逐字匹配且读取权限的既有拒绝 / 允许用例通过；日志 artifacts/clipboard-utf8-after.log。`./scripts/nsh check`、`./scripts/nsh build`、`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`git diff --check` 通过。

本轮未重新运行完整 headless 套件；上一轮目录整理记录为 128/128。已用相同 OSC52 系统路径复现并验证修复，未直接操作用户当前 Claude 会话，实际 Claude 内复制仍待用户复验。

## 2026-11-15：本地图片与文件拖入终端

SessionTerminal 使用 NSH 0.5.0 的原生 on_file_drop，Rust 解码本地 file URL 并转义为 shell 字面路径，按拖放顺序通过既有 Session.paste 输入，不自动回车、不读取或上传图片。可见非活动 pane 请求焦点后接收输入；隐藏、modal 阻断、组合输入及已退出会话忽略 drop。新增 inputBlocked 属性区分阻断状态与 pane 焦点。远程 URL 和带控制字符的路径整批拒绝。UI 预览未接入文件拖放。

- `./scripts/nsh test tests/terminal/FileDrop.test.tsx --junit artifacts/file-drop-focused.xml`：**2/2**，headless + 真实 /bin/sh，验证中文空格、多文件顺序、单引号与命令替换字符保持字面、无自动提交、非活动 pane 焦点、modal 阻断和远程 URL 忽略。日志 artifacts/file-drop-focused.log。
- `cargo test -p termlevel-native session_view::file_drop::tests -- --nocapture`：**2/2**，验证 URL 单次解码、顺序、转义与非法混合批次拒绝；日志 artifacts/file-drop-rust.log。
- `./scripts/nsh check`、`./scripts/nsh build`、`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`git diff --check` 通过。
- 本轮未重跑完整套件。NSH 0.5.0 的 macOS 测试驱动明确不支持 dropFiles（只支持 headless / Linux），因此未声称 Finder 原生拖入或 Claude 附件识别已通过验收。人工应检查 Finder 单张 / 多张图片拖入、中文与空格文件名、分屏目标和对话框阻断；无本地 file URL 的浏览器图片、剪贴板位图不在本次实现范围。

## 2026-11-15：macOS 开发宿主磁盘不足恢复

用户 `./scripts/nsh run` 在原生宿主链接阶段失败，致命错误是 `ld: write() failed, errno=28 (No space left on device)`；macOS 最低版本与重复库提示属于警告。现场剩余 341 MiB；target 约 40 GB，其中 target/debug/incremental 约 3.7 GB，另有 .nsh/rust/target 约 4.9 GB。

仅删除这两处可再生成的缓存，保留源码、artifacts、工具与 Ghostty 下载缓存。随后 `CARGO_TARGET_DIR=/Users/biu/Projects/termlevel/target cargo build --manifest-path .nsh/rust/macos-host/Cargo.toml` 成功（含原生宿主链接，约 5 秒）；日志 artifacts/macos-host-disk-recovery.log，完成后约 7.1 GiB 可用。未启动用户工作区或声称 GUI 验收通过，原有链接警告仍在。

## 2026-11-15：终端身份设置

基于 `55a0afdc335d13a5befd053c5fa7f11346986500` 的未提交改动。Settings → Terminal 新增 Termlevel 默认 / Ghostty compatibility 两项选择；随 workspace v1 文档保存，旧字段缺失或未知时回退默认。身份在每次显式 open 时传入 Rust，仅影响新进程；Ghostty 模式移除版本变量，保留 xterm-256color / truecolor。未改动用户 Codex 配置或私有 pet 素材。

命令使用 `GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip CARGO_NET_OFFLINE=true`：

- `./scripts/nsh test --junit artifacts/terminal-identity-headless.xml`：136/136，通过 Rust、文档与 headless TSX 测试，包含真实 PTY 的身份隔离、旧会话保留、未知值回退以及设置存储。日志 artifacts/terminal-identity-headless.log。首次开发检查曾漏传已有 Rust 测试调用的新参数，补齐后重跑通过；新增 UI 测试也曾把多会话输入定位到整个 panel，改为按 session ID 定位后通过。
- 全量运行后补充了恢复 shell 的测试，并调整选中状态样式。`./scripts/nsh test tests/terminal/Session.test.tsx --filter 'terminal identity' --junit artifacts/terminal-identity-focused.xml`：2/2，真实 /bin/sh 验证切换身份后旧进程环境不变、新会话使用所选身份，以及保存的 Ghostty 身份在恢复 shell 创建前生效；日志 artifacts/terminal-identity-focused.log。
- 最终样式下 `./scripts/nsh test tests/workspace/Persistence.test.tsx --filter 'terminal identity' --junit artifacts/terminal-identity-persistence.xml`：1/1，日志 artifacts/terminal-identity-persistence.log。
- 原生 AppKit 自动化：将 Session.test.tsx 临时复制到根目录，修正 fixture 相对路径并在选择身份后截图，运行 `./scripts/nsh test .native-identity.test.tsx --filter 'terminal identity' --target macos --junit artifacts/terminal-identity-native.xml`：2/2。临时驱动已删除；可用 `./scripts/nsh test tests/terminal/Session.test.tsx --filter 'terminal identity' --target macos` 重跑已提交用例。日志 artifacts/terminal-identity-native.log；已查看 artifacts/terminal-identity-ghostty-native.png，选中态明确、控件与说明无裁切。沙箱初次禁止 dev server 监听，经权限流程后原生验收通过。宿主构建仍有一条链接警告。
- 最终 `./scripts/nsh check`、`./scripts/nsh build`、`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings` 与 `git diff --check` 通过；对应 check/build/clippy 日志在 artifacts/terminal-identity-*.log。

预览不执行真实命令或访问真实 workspace storage。上述真实 PTY 与 AppKit 结果不代表 Codex Pets 验收：未启动真实 Codex 检查 picker、持续动画、resize、切 pane 与清理；用户 shell 启动文件和 multiplexer 环境仍可能影响客户端最终检测。人工日用验收另记。

## 2026-11-15：输入队列背压与提示恢复

基于 `55a0afdc335d13a5befd053c5fa7f11346986500` 的未提交状态，叠加同日终端身份改动。用户截图显示 `Terminal input unavailable: sending on a full channel`。代码确认旧队列按 128 条消息限制且 try_send 拒绝会永久置 state=error；截图本身没有绑定触发事件，不能确定那次是键盘、鼠标或协议回复洪峰。

Rust Session 独有的 InputQueue 改为 FIFO 总字节预算：4 MiB 待发送（含在途块）、1 MiB 单次输入、8 KiB 合并写块。生产端不等待 PTY，不会为消除排队报错而阻塞 UI / reader。超限在复制前整条拒绝，独立 inputWarning 明确提示未发送，不覆盖真实 error；只有拒绝之后接受的输入成功写入 PTY 后才恢复提示，旧在途写入不能误清。真实读写故障仍报错。关闭 / 退出清空队列并唤醒 writer，保留 Session 生命周期与默认身份设置。

验证命令均使用 `GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip CARGO_NET_OFFLINE=true`：

- `cargo test -p termlevel-native sessions:: -- --nocapture`：18/18，通过真实 PTY 会话与队列测试。日志 artifacts/input-queue-sessions.log。确定性测试覆盖 20,000 次单字节输入保序、4 MiB 饱和（含在途字节）、拒绝不部分写入、旧写入不清新警告、后续输入恢复以及关闭唤醒。
- 新增 tests/probes/input_backpressure_probe.py：raw PTY 连续输出 20,000 次光标查询，发完后才读取输入，逐字节核对全部 120,000 字节回复与顺序。Rust 直启探针和 UI 启动均通过。该受控客户端不是用户截图中的真实 Codex。
- `./scripts/nsh test --junit artifacts/input-queue-headless.xml`：144/144，通过 Rust / doc tests / headless TSX，含两项 InputBackpressure 测试：真实 /bin/sh 的超限粘贴警告与后续输入恢复，以及上述查询洪峰。日志 artifacts/input-queue-headless.log。初次专项测试为 1/2：工作区 shell 默认在 home，找不到相对探针路径；改用从项目目录启动的 ProtocolsFixture 后全量通过，保留初次日志 artifacts/input-queue-ui.log。
- `./scripts/nsh test tests/terminal/InputBackpressure.test.tsx --target macos --junit artifacts/input-queue-native.xml`：2/2，同样的两项行为在原生 AppKit 窗口与真实 PTY 通过。日志 artifacts/input-queue-native.log。宿主构建仍有一条链接警告。
- `./scripts/nsh check`、`./scripts/nsh build`、`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`git diff --check` 均通过。check/build/clippy 日志分别为 artifacts/input-queue-{check,build,clippy}.log。

未关闭或重启用户正在使用的终端。Rust 改动需重启应用后生效，当前运行实例不会热替换。真实 Codex 持续日用和截图中那次具体事件源尚未复现；不把上述自动化当作人工验收。有限内存无法保证永久不读输入的客户端无限接收，真正超过预算仍明确拒绝且要求重试，不静默丢弃、不无限扩容。

## 2026-11-15：应用内通知面板

基于 `bc3c8f7` 的未提交改动。此前终端身份和输入队列修复已提交为 `bc3c8f7`；原有 docs/codex-pets-handoff.md 保持未跟踪、未修改。标题栏新增铃铛、未读提示和历史面板，收录终端 OSC 9 / OSC 777、BEL，以及应用错误 / 警告。Rust 是运行期历史的唯一所有者，最多 100 条，两秒内连续相同通知合并计数；不随终端视图卸载或来源关闭而消失，不跨应用重启保存。支持单项移除、清空和按已有 session ID 返回来源；清空历史不清除原会话错误。preview 不调用真实通知服务。

验证使用 `GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip CARGO_NET_OFFLINE=true`：

- `cargo test --workspace notification -- --nocapture`：safe crate 的 1 项、应用的 3 项通过；日志 artifacts/notifications-rust.log。覆盖 OSC 分块与 owned 字符串在 terminal 销毁后保留、bounded 历史淘汰、未读 / 重复计数、ID 不复用、无挂载视图的真实 PTY 通知和关闭来源后仍保留历史。
- `./scripts/nsh test --junit artifacts/notifications-headless.xml`：152/152，包括新增三项真实 PTY 通知交互和 preview 空面板 / 侧栏收起入口用例。日志 artifacts/notifications-headless.log。通知专项最终 headless 3/3；日志 artifacts/notifications-focused.log。
- 原生 AppKit 自动化：临时将 Notifications.test.tsx 复制为根目录 .native-notifications.test.tsx，修正相对 fixture 路径并插入截图，执行 `./scripts/nsh test .native-notifications.test.tsx --target macos --junit artifacts/notifications-native.xml`：3/3。日志 artifacts/notifications-native.log。临时驱动已删除；仓库内用例可用 `./scripts/nsh test tests/terminal/Notifications.test.tsx --target macos` 重跑。验证后台 shell 通知、跳回来源、关闭来源后的历史、应用输入警告恢复后的历史保留、逐项移除、五条历史滚动与清空。宿主仍有一条链接警告。
- 已查看 artifacts/notifications-source-native.png 和 artifacts/notifications-history-native.png：单条按内容收缩，侧栏收起仍可打开；640×420 窄窗口最多三条可见，分隔线、滚动提示和清空按钮可见。面板高度根据根窗口可用空间及前三条实际测量高度计算；长正文可滚动查看。
- `./scripts/nsh check`、`./scripts/nsh build`、`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`git diff --check` 通过。日志 artifacts/notifications-{check,build,clippy}.log。

开发中专项曾因测试共享 Rust 历史而依赖了错误的通知编号；夹具现显式清空自己的历史，用例按具体通知的可访问性标签定位。原生还收到了额外的真实 BEL 通知，最初通用关闭标签不唯一，现标签包含通知标题并按单项移除验证，不假定移除一项后历史为空。上述最终回归已覆盖修正。

本次是应用内通知历史，不发系统桌面推送；未实现 OSC 99、跨重启历史或 shell integration 的自动命令完成推断。真实 Codex 通知策略与用户长期日用尚未人工验收，受控 OSC / BEL 用例不能替代。Rust 改动需重新运行应用才能生效；未关闭用户当前会话。

### 同日补充：移除终端底部状态条（amend 通知面板提交）

按用户要求，从真实 TerminalSurface 和确定性 PreviewTerminal 移除底部状态条。输入拒绝和详细错误统一由通知面板展示；pane 标题栏保留 Exited / Error 简短状态与“Open new terminal”入口，已有输出和原 session ID 保留，点击入口才显式创建新会话。清空通知历史不清除退出状态，也不自动重启旧进程。当前 architecture 已同步，以上旧验证章节的状态条描述保留为历史记录。

- `./scripts/nsh test tests/terminal/Notifications.test.tsx --target macos --junit artifacts/notification-status-removal-native.xml`：4/4。新增真实 /bin/sh 验证非零退出保留输出、底部不重复显示错误、通知历史包含退出原因、清空历史后标题栏仍显示退出状态，以及显式新建终端后可继续输入。日志 artifacts/notification-status-removal-native.log。宿主仍有一条链接警告；不作为长期人工日用验收。
- 更新 InputBackpressure 和真实 / 预览平铺回归，分别验证通知入口与标题栏状态，不再依赖底部状态条。
- `./scripts/nsh check`、`./scripts/nsh build`、`git diff --check` 通过。日志 artifacts/notification-status-removal-{check,build}.log。本次没有 Rust 代码变化。
- `./scripts/nsh test --junit artifacts/notification-status-removal-headless.xml`：153/153，日志 artifacts/notification-status-removal-headless.log。

## 2026-11-15：历史滚动条与跨屏拖选

基于 `0f8fe2b` 的未提交改动。真实终端此前没有历史滚动条，拖选终点被夹在可见网格内，也没有边缘持续滚动。代码同时确认 motion 的历史行已经包含 origin，却在 controller 内再次加 origin，影响历史拖选和链接命中。

现将局部 cell / 绝对历史行分开；NSH responder 在移动后捕获已开始的宿主手势，保留起点按下事件并在网格外接收拖动 / 释放。SurfaceController 在已有帧调度中更新唯一 Window、边缘滚动和 Ghostty tracked 选区终点，单轮只采样一次。新 scrollbar 直接使用 WindowSnap 的 origin / count / rows；预留 gutter 不随历史出现或备用屏幕切换改变列数。导航不清除选区，鼠标释放 / 取消 / modal / 隐藏 / resize 停止跟踪。滚动条点击与捕获释放分别收尾，不能把框架取消 click 当成物理鼠标释放。同步输出 hold 判定与选区更新共用 emulator 锁，避免 PTY 在两者之间开启新事务。

- 纯 Rust 覆盖滚动条上下边界 / 中点反算与裁剪、无历史退化、边缘持续速度、暂停后不追赶跳跃、连续 motion 保留不足一行的累计量、反向清除余量；Window 用例区分历史起点非零时的局部行与绝对行。
- tests/terminal/Selection.test.tsx 使用真实 /bin/sh 输出 800 行含中文的文本，验证停在上边缘时继续滚动、释放后停止、滚动条到顶 / 到底仍保留选区、下边缘自动滚动与备用屏幕隐藏历史滚动条。读取选区只用 debug NativeInputProbe.selection，不修改系统剪贴板；生产路径不把选区或网格数据送到 JS。
- `./scripts/test-native selection` 的两项原生 AppKit 验收分别验证向上跨屏拖选（可见样本约 140 行）、释放停止、滑块导航保留选区，以及从历史顶部向下跨屏拖选。已查看 artifacts/native-selection-autoscroll.png、native-selection-history-top.png，确认高亮、历史内容和滑块位置对应；其他截图为 native-selection-history-bottom.png、native-selection-autoscroll-down.png。
- `./scripts/test-native protocols`：6/6，日志 artifacts/selection-protocols-native.log；保留 Kitty 键盘、程序 SGR 鼠标报告、OSC8 / 选区复制 / 图片和 OSC52 行为。原生构建仍有一条链接警告。

开发验证中的失败保留：最初 headless 拖动定位了外层 SessionTerminal，改为真实 grid 后用例才覆盖正确入口；原生直接投递自建鼠标事件被 NSH 测试隔离过滤，已删除该探针，改用 NSH 自带 AppKit drag。一次全量发现与 scripts/test-native 并行，误发现根目录临时原生测试并改变 synthetic input 开关，已停止该无效运行（exit 143），日志 artifacts/selection-headless-contaminated.log；development 现明确禁止这类并行。随后全量曾 157/159：scrollbar 在 click 被框架取消时过早停止跟踪，修正为真实 mouse-up / responder-release 后专项通过，保留 artifacts/selection-headless-before-release-fix.{log,xml}。

自动化不替代实际鼠标 / 触控板的长期手感验收。当前仍沿用双击选词、三击选行语义，未扩展为按词 / 整行拖选；并发历史淘汰、窗口外失焦和多 pane 长时间负载仍需日用复验。不声称 Neovim / tmux 全兼容。没有提交或重启用户终端。

最终验证使用 `GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip CARGO_NET_OFFLINE=true`：

- `./scripts/nsh test --junit artifacts/selection-headless.xml`：159/159，日志 artifacts/selection-headless.log。全量后补齐滑块在视口内释放时应用最终坐标，以下最终专项覆盖该版本。
- `./scripts/nsh test tests/terminal/Selection.test.tsx --junit artifacts/selection-focused.xml`：3/3，日志 artifacts/selection-focused.log。
- `./scripts/test-native selection`：2/2，日志 artifacts/selection-native.log，包含两方向跨屏选择与视口导航 / 保留选区。headless 另验证鼠标停在边缘不动时继续滚动；原生用 NSH 自带 drag 路径验证持续拖动及释放。
- `./scripts/nsh check`、`./scripts/nsh build`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check` 通过。日志 artifacts/selection-check.log、selection-scroll-build.log、selection-clippy.log。原生构建的一条链接警告仍保留。

### 同日修正：滚动条改为覆盖式

按用户确认，将历史滚动条从网格内并列 gutter 改为终端外层的覆盖元素。默认命中宽度 10px、pane 右边距 2px，由 theme.json 提供；文本内边距保持独立，列数计算不再扣除滚动条宽度。滚动 / 视口变化 / 进入右侧命中区时显示，悬停、滑块拖动或文本拖选期间保持显示；空闲 800ms 后用 180ms 淡出。淡出只更新 Rust 滑块的 opacity，不改变网格、PTY 尺寸或 tracked 选区。保留前述已有未提交拖选改动，未自动提交或重启用户终端。

最终验证均使用前述固定 Ghostty archive 和离线环境：

- `./scripts/nsh test tests/terminal/Selection.test.tsx --junit artifacts/scrollbar-overlay-focused.xml`：4/4，日志 artifacts/scrollbar-overlay-focused.log。新增真实 PTY 列数检查：无历史、历史出现、悬停、空闲淡出及滑块导航前后列数相同；原有跨屏拖选 / 选区保留 / 备用屏幕用例继续通过。夹具现使用与生产相同的水平及底部文本内边距。
- `./scripts/test-native selection`：3/3，日志 artifacts/scrollbar-overlay-native.log。已查看 artifacts/native-scrollbar-overlay-hover.png 与 native-scrollbar-overlay-idle.png，滑块位于 pane 右边缘附近，悬停显示、移开后消失，正文位置与行列保持一致；原生样本始终为 110 列。该运行中的 debug getter 未使用 import 警告已移除，最终严格 Clippy 通过；原生宿主原有链接警告保留。
- `./scripts/nsh test --junit artifacts/scrollbar-overlay-headless.xml`：160/160，日志 artifacts/scrollbar-overlay-headless.log。与原生脚本串行执行，无根目录临时测试串扰。
- `./scripts/nsh check`、`./scripts/nsh build`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check` 通过；日志 artifacts/scrollbar-overlay-{check,build,clippy}.log。

覆盖式方案取代上文历史记录中的预留 gutter 方案；当前行为以 architecture 和代码为准。人工长期手感验收仍与上述自动化分开。

## 2026-11-15：侧栏终端状态与进度（OSC 7501 / OSC 9;4）

基于 `2f1ae0c` 的未提交改动。侧栏 tab 现在同时显示三层信息：标题继续来自 OSC 0 / OSC 2；OSC 7501 的聚合成状态替换默认 terminal / pin 图标（working 使用三点呼吸图标，不用旋转圆环）；OSC 9;4 的进度作为标题列下方的 2 pt 细线，收起侧栏时缩到图标下方。working 同时有 9;4 和 7501 百分比时以 9;4 为准，7501 百分比只在 9;4 缺失时兜底；working 没有显式百分比时不合成 indeterminate 线，blocked / error 保留满格状态线。视觉、优先级和生命周期见 [OSC 集成与终端状态设计](osc-support.md)。7501 的完整 id 记录表只存在于 Rust `Session`，`SessionInfo` 发送聚合摘要；blocked / error 的进入跃迁各写入一条通知，working、done 和 9;4 不写。选中 tab 调用 `Sessions.acknowledge` 清除 done / blocked / error，应用恢复时的初始选中不算用户确认。运行期字段不进入 workspace 文档。

验证使用 `GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip CARGO_NET_OFFLINE=true`：

- `cargo test -p termlevel-native`：86/86，日志 artifacts/terminal-status-rust.log。新增聚合优先级与 app 继承、子树清除、真实 `/bin/sh` 下 9;4 / 7501 进入 `SessionInfo`、acknowledge 以及退出清除 working 的用例。
- `cargo test -p termlevel-ghostty-vt --test native`：23/23，日志 artifacts/terminal-status-wrapper.log。新增 9;4 五种状态与缺失百分比、分块输入、连续进度合并、7501 blocked 解码、子记录 clear、`OSC 7501;?` 支持回复、RIS 先 clear 后 reset，以及 9;4 不产生通知的用例。
- `./scripts/nsh test --junit artifacts/terminal-status-headless.xml`：179/179，日志 artifacts/terminal-status-headless.log。新增 tests/terminal/Status.test.tsx 四项真实 PTY 用例：42% / indeterminate / remove 的进度线；working → done → blocked 的图标与可访问标签、9;4 优先于 7501 百分比、blocked 未读通知和选中确认；working 无百分比时不合成进度线；error 通知与确认。Snapshot 用例同时确认持久化 JSON 中的 `progress` / `activity` 被丢弃。
- `./scripts/nsh test tests/terminal/Status.test.tsx --target macos --junit artifacts/terminal-status-native.xml`：4/4，日志 artifacts/terminal-status-native.log。原生宿主仍有一条既有链接警告。最终调整进度线轨道高度与 9;4 / 7501 百分比优先级后，`./scripts/nsh test tests/terminal/Status.test.tsx --junit artifacts/terminal-status-focused.xml` 复验 4/4，日志 artifacts/terminal-status-focused.log。
- `./scripts/test-native status`：1/1，日志 artifacts/terminal-status-native-acceptance.log。已查看 artifacts/native-status-progress.png、native-status-working.png、native-status-blocked.png、native-status-collapsed.png：42% 进度线位于标题文字下方；working 使用三点呼吸图标，不再出现旋转圆环；blocked 使用圆圈感叹号与整条强调线，标题栏出现未读通知；收起侧栏后图标下方保留短进度线。原生宿主仍有一条既有链接警告。
- `./scripts/nsh check`、`./scripts/nsh build`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check` 通过；日志 artifacts/terminal-status-{check,build,clippy,fmt,diff}.log。

未完成项与边界：静态截图不能证明三点 pulse 的实际呼吸效果、系统减少动态效果下的退化、不同字号和窄侧栏下的 2 pt 线可读性，也未做键盘 / 读屏的完整人工走查；没有用真实 cargo / terraform / coding agent 的长任务验证 7501 策略，也没有验证高频 9;4 更新下的长期 CPU / 重绘表现。preview 仍不接这两个协议，不作为证据。未接 OSC 133 提示符生命周期，因此 working / blocked 目前只在进程退出和 RIS 时自动清除；记录列表、按 app 图标和窗口级聚合留待后续。真实程序支持与长期日用仍需人工验收。

## 2026-11-15：OSC 支持盘点与 PWD / OSC 22 / OSC 133 映射

基于 `2f1ae0c` 的未提交改动（含上一节侧栏状态与进度）。新增 [OSC 集成与终端状态设计](osc-support.md)，逐项列出 libghostty-vt 1.0.1 识别、内部处理、需要应用回调和不支持/不接入的 OSC。应用新增三处映射：

- OSC 7 / ConEmu 9;9 / iTerm2 1337 CurrentDir：解码 `file://` URI 与裸绝对路径，拒绝远程 host、query/fragment、控制字符和超长值；写入 `SessionInfo.currentCwd`，UI 用运行期 `Tab.liveCwd` 更新工具栏、Cmd-K 搜索、分组路径建议、从该 tab 新建终端和下次保存。`Tab.cwd` 仍是启动目录，分组 cwd 不变。
- OSC 22：安全封装读取 `DATA_MOUSE_SHAPE`，`GhosttyEmulator` 仅在形状变化时发布；`SessionInfo.mouseShape` 传到 `SessionTerminal` 的 host cursor，grid 与 scrollbar 各自保持光标。
- OSC 133：注册 `SEMANTIC_PROMPT`，回调内复制 kind、prompt kind、exit code、command、error 和 `CURSOR_AT_PROMPT`；`PROMPT_START` 清除 7501 的 working/blocked，`done`/`error` 保留到确认；`SessionInfo.atPrompt` 只作为运行期元数据，未新增可见 UI。

验证：

- `cargo test -p termlevel-native`：89/89，日志 artifacts/osc-rust.log。新增 PWD URI/裸路径校验、真实 `/bin/sh` 的 PWD / OSC 22 / OSC 133 生命周期，以及 backend 映射用例。
- `cargo test -p termlevel-ghostty-vt --test native`：25/25，日志 artifacts/osc-wrapper.log。新增 OSC 22 全部关键形状、空值/reset，OSC 133 四种 kind 与 `cursor_at_prompt`。
- `./scripts/nsh test --junit artifacts/osc-headless.xml`：186/186，日志 artifacts/osc-headless.log。新增 tests/terminal/Osc.test.tsx 两项真实 PTY 用例：OSC 7 更新工具栏/保存/新终端目录；OSC 133 清除 working 并暴露 prompt 状态，OSC 22 暴露 shape。
- `./scripts/nsh test tests/terminal/Osc.test.tsx --target macos --junit artifacts/osc-native.xml`：2/2，日志 artifacts/osc-native.log；原生宿主仍有一条既有链接警告。
- `./scripts/test-native osc`：1/1，日志 artifacts/osc-native-acceptance.log，JUnit artifacts/osc-native-acceptance.xml。已查看 artifacts/native-osc-live-cwd.png（工具栏从项目目录更新为 `/private/tmp`）和 native-osc-prompt.png（OSC 133 A 后 working 图标消失）。
- `./scripts/nsh check`、`./scripts/nsh build`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check` 通过；日志 artifacts/osc-{check,build,clippy,fmt,diff}.log。

未完成项与边界：OSC 22 的实际 NSCursor 形状未在原生截图中验证（截图不包含指针），且 NSH 可能只在指针下次移动时更新 cursor，需人工在终端内运行 `printf '\033]22;pointer\007'` 后移动指针确认；滚动条/工具栏光标隔离同样需人工验收。OSC 1、99、ConEmu 非进度键、OSC 66/72/3008、OSC 1337 其他键和不支持的 OSC 5522 授权/MIME 细节按盘点文档保持未接入。OSC 133 需要 shell integration；本机 `/bin/sh` 用例用 printf 模拟协议，不代表用户 shell 已配置。旧验证章节中“工具栏仍是启动路径 / 不做 OSC 7”的历史记录保留为当时证据，本节及当前 architecture/terminal-tiling 为准。未提交或重启用户会话。


## 2026-11-15 · OSC 状态 UI / UX 重设计落地

基于 `c2f9488` 的未提交工作树，保留此前 OSC、目录整理等未提交改动。设计统一在 [OSC 集成与终端状态设计](osc-support.md)，此前状态 / OSC 章节仍记录旧版实现的历史证据。

本次把程序报告和用户已读分开：7501 记录带状态代次，`Sessions.acknowledge(id, observedRevision)` 只确认已呈现版本；blocked 查看后继续存在，已读 done/error 不再遮挡 working，重复同状态报告不会重新置未读。通知改为记录级跃迁并附带记录身份 / 代次，同文案的不同任务不会因聚合仍为 blocked 而漏掉。`TerminalStatusInfo` 在 Rust 统一生命周期、活动和真实进度，Sidebar、tooltip 与无障碍共用摘要；取消 blocked/error 和未知 pause 的伪满格线。新 prompt 清理上一轮 9;4，shellPhase 区分 unknown 与已知阶段。原生 FocusRequest 区分用户操作与自动输入焦点，重挂不确认结果。

自动化证据（真实 PTY 均为受控 `/bin/sh`，不是 preview，也不是实际 agent 日用验收）：

- `./scripts/nsh test --junit artifacts/status-redesign-headless.xml`：192/192，日志 `artifacts/status-redesign-headless.log`。包含 Rust 应用测试 93 项、安全封装 native 25 项、ABI、doc tests 与 TSX；这里的 Rust native 测试名称不代表前台 AppKit GUI 验收。
- 状态 TSX 六项：双协议进度一致、9;4 remove / indeterminate、blocked 查看仍保留、error 确认后不压住新 working、未知 pause/error 不虚构百分比、生产 TerminalSurface 重挂保留未读 done。日志 `artifacts/status-redesign-focused.log`，报告 `artifacts/status-redesign-focused.xml`。
- Rust 新增记录级通知、旧版本确认不能吞掉新代次、确认保留原始记录、reset 后不复用代次、prompt / exit 清理、100% 不等于 done、记录整体替换与 256 条淘汰；全量套件通过。此前单独 `cargo test -p termlevel-native --lib` 为 92/92（新增容量测试前），日志 `artifacts/status-redesign-rust-unsandboxed.log`；以最终全量中的 93 项为最终代码证据。
- `./scripts/nsh check`、`./scripts/nsh build`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check` 通过。日志 `artifacts/status-redesign-{check,build,clippy}.log`；fmt / diff 直接执行，无单独日志。文档本地链接检查通过。

前台 AppKit 原生自动化，均串行执行：

- `./scripts/test-native status`：1/1，日志 `artifacts/status-redesign-native.log`，报告 `artifacts/status-native.xml`。已查看 `native-status-progress.png`、`native-status-working.png`、`native-status-blocked.png`、`native-status-collapsed.png`、`native-status-blocked-no-progress.png`、`native-status-tooltip.png`、`native-status-blocked-read.png`、`native-status-error.png`、`native-status-done.png`、`native-status-paused.png`（均在 artifacts）。确认暗色原生窗口下的展开 / 收起、真实进度短线、blocked 无伪满格且查看后仍有图标、失败 / 完成图标、静态未知暂停轨道，以及可读两行 tooltip。
- `./scripts/test-native osc`：1/1，日志 `artifacts/status-redesign-native-osc.log`，报告 `artifacts/osc-native-acceptance.xml`。实时 cwd 与新 prompt 清理活动标签复验通过。OSC 22 实际 NSCursor 形状仍需人工检查，截图不能证明。
- 原生宿主构建仍有一条既有链接警告；未把无详细 warning 文本的摘要推断为新问题。

验证期间的失败及处理：

- 首次 Cargo 构建无法解析 `release.files.ghostty.org`。后续命令使用 `GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip CARGO_NET_OFFLINE=true`，build.rs 校验固定 SHA256，不跳过库校验。
- 沙箱内已有 `clipboard::tests::osc52_bytes_reach_the_pasteboard_as_unicode` 因 `pbcopy rejected the sample` 失败：单独 Rust 91/92，第一次全量 189/190。沙箱外访问系统 pasteboard 后该用例通过；保留 `artifacts/status-redesign-rust.log` 记录，不修改其断言。
- 一轮验证时在运行中改变 FocusRequest 的事件签名，已启动的旧 runner 与新 JS 不匹配，9 项出现 `Cannot convert undefined or null to object`；停止修改生产源码并重建后消失。日志 `artifacts/status-redesign-headless-before-focus-rebuild.log`。
- 新重挂 fixture 结束时仅 dispose，遗留 PTY 使后续平铺测试出现重复 Terminal 1；已增加显式 close，最终状态与平铺连续运行通过。失败日志 `artifacts/status-redesign-headless-before-fixture-cleanup.log`。
- 原生状态首轮错误查询了仅 headless 可观察的空 Text 诊断节点，现改查实际产品标签并截图；已读语义由 Rust / headless 直接检查，原生截图确认图标保留。原生 OSC 首轮仍查询旧版 working 文案，已同步新可读标签。失败日志分别为 `artifacts/status-redesign-native-fixture-diagnostic.log` 和 `artifacts/status-redesign-native-osc-old-label.log`，没有靠提高超时修复。

未完成的人工验收：真实 cargo / terraform / agent 的持续报告；系统减少动态效果开关、VoiceOver 实听；明暗主题 / 多字号 / 长消息和分组固定标签的完整组合；窗口激活和分屏切换的长期手感；高频进度与多会话长时间负载。本次已做原生截图审查，但未宣称上述日用项目通过。


## 2026-11-15 · 通知行图标与内容层级

基于 `efafe22` 的未提交改动。原通知行标题、正文和操作权重接近，本次拆出 `ui/notifications/NotificationRow.tsx`：实际 message / 正文使用 15 pt semibold 主文；任务 / 事件标题为上下文，来源、程序名、类别、时间缩为次级信息。左侧加入 32 pt 淡色底座及语义图标（批准、提问、登录、失败、普通消息），底部单独放来源跳转、展开 / 收起、移除。程序没有图片附件，不虚构应用品牌图标。7501 历史追加经过 Rust 清洗的 kind/app/title/message 元数据，界面不靠解析生成标题来判断任务类别。

长内容提供有界摘要与 Show more / Show less，展开后长主文使用 12 pt 常规字重，完整历史文本保留。历史仍按前三条实测高度确定 viewport，保留分隔线与滚动提示；面板宽度上限为窗口宽减 16 pt，默认宽 420 pt、历史最高 384 pt。打开 / 清空 / 回到来源不改变已有会话所有权或程序状态确认规则。

验证（真实 PTY 为受控 `/bin/sh`；截图里的 128 tests 是测试消息样例，不是本轮测试计数）：

- `./scripts/nsh test --junit artifacts/notification-row-headless.xml`：194/194，日志 `artifacts/notification-row-headless.log`。包含 Rust 对结构化 kind/app/message 的检查、原有状态与工作区回归及通知六项 TSX。随后仅调整长主文展开后的字重 / 字级，相关六项与原生验收单独复验。
- `./scripts/nsh test tests/terminal/Notifications.test.tsx --junit artifacts/notification-row-focused.xml`：6/6，日志 `artifacts/notification-row-focused.log`。覆盖背景来源跳转、已关闭来源、输入警告、三条以上滚动、退出信息、程序请求主文 / 语义图标、长文本展开 / 滚动 / 收起。初次扩展用例的正文能完整放入 viewport，不应滚动；改用确实超过 viewport 的长正文验证滚动，没有提高超时或放宽产品断言。
- `./scripts/test-native notifications`：2/2，日志 `artifacts/notification-row-native.log`、报告 `artifacts/notifications-native.xml`。覆盖深色和浅色下的普通消息、任务失败、需要批准、提问、登录；1000×720、640×420、360×420 布局、空态、长正文展开及末尾收起。已查看 artifacts 下的 `native-notifications-{dark,light}.png`、`native-notifications-{dark,light}-compact.png`、`native-notifications-light-narrow.png`、`native-notifications-{dark,light}-empty.png`、`native-notifications-expanded.png`、`native-notifications-expanded-end.png`；实际消息突出，语义图标可辨，窄窗口内面板和操作可用。
- `./scripts/nsh build`、`./scripts/nsh check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check` 通过。日志 `artifacts/notification-row-{build,check,clippy}.log`；fmt / diff 直接执行。

构建与测试使用已校验的离线库：`GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip CARGO_NET_OFFLINE=true`。完整套件在沙箱外运行已有系统 pasteboard 用例；原生宿主仍报告一条既有链接警告，本次未新增 Rust / TSX 检查失败。原生截图审查不等同于 VoiceOver 实听、大号系统字体或真实 agent 日用验收。


## 2026-11-15 · 通知字级统一到 Interface body

上一版通知主文使用对话框 title 字级 15 / 20，与普通界面 body 12 / 16 不一致。本次在 NotificationRow 统一主文与上下文为 body，只用字重、颜色和图标表达重点；长内容展开仅切换字重，不改变字号或行高。字体族继续由 Type 读取 interfaceFont，来源 / 时间继续使用现有 caption 字级；不修改全局对话框 title token。

`./scripts/nsh check`、`./scripts/nsh build`、`git diff --check` 通过（日志 `artifacts/notification-type-{check,build}.log`）；通知专项 6/6（`artifacts/notification-type-focused.{log,xml}`）；`./scripts/test-native notifications` 2/2（`artifacts/notification-type-native.log`、`artifacts/notifications-native.xml`）。已重新查看原生深色通知和浅色 360 pt 窄窗口截图，主文与侧栏 / 普通界面文字字级一致。原生宿主仍有既有链接警告；本次未重跑与字级无关的完整 Rust / 工作区套件，上一节 194/194 为此前记录。

## 2026-11-15 · Codex pet 消失与同步刷新看门狗

基于 `4090aa6` 的未提交改动。真实 Codex 0.169.0 的右下角 Kitty pet 先删除旧图，再分块传输新图；旧看门狗从 mode 2026 开始固定计时 150 ms，传输仍在继续也会提交事务中的状态。原生逐帧日志实际捕获 `SYNC_TIMEOUT force=false` 后 `DRAW images=0 frameimages=0`，随后恢复为 1。提前提交也可能暴露事务中临时隐藏的光标。

看门狗改为最后一段非空 PTY 输出后静默 150 ms 才提交，单次 hold 最长 1 s。正常结束标记、EOF、resize 仍按原有通路释放；边界快照和会话生命周期不变。

- 确定性回归：250 ms 分块 Kitty 替换期间保留旧红像素和可见光标，提交后显示绿像素；另验静默释放和持续输出 1 s 上限。将截止时间临时恢复为旧固定 150 ms 后，`active_chunked_image_transfer_survives_the_initial_sync_deadline` 在传输期间失败；恢复修复后通过。旧策略失败日志：`artifacts/pets-watchdog-regression-before.log`。
- 真实 PTY / 原生自动化：临时原生用例打开生产 `WorkspaceFixture.tsx`，执行 `TERM_PROGRAM=ghostty codex`，空闲观察 20 s；无提交 prompt。临时逐绘制图片数量和超时诊断已移除。修复前 89 个绘制帧中，pet 首次出现后 2 帧图片为空，23 次超时；修复后 110 个绘制帧中没有图片空帧或超时。日志 `artifacts/pets-watchdog-{before,after}-draw.log`、`artifacts/pets-watchdog-native.log`；截图 `artifacts/pets-trace.png`。计数是有界实测，不等于长时间日用验收。
- `cargo test -p termlevel-native terminal::tests -- --nocapture`：31/31。包括连续换图、分块替换、光标可见性、空闲超时、持续输出上限、相邻同步事务和历史窗口冻结。

安装中的 `/Applications/Termlevel.app` 未替换，用户人工验收仍待进行；正常的光标定时闪烁不作为 pet 消失证据。首轮排查把未设置 cell 像素尺寸的测试误认成 hold 快照丢图，补齐尺寸后排除；最终修复依据为真实原生空帧与超时关联，以及旧看门狗下失败的分块回归。

最终检查：`./scripts/nsh check`、`./scripts/nsh build`、`cargo fmt --all -- --check`、`git diff --check` 通过；`./scripts/nsh test --junit artifacts/pets-watchdog-tests.xml` 在沙箱外运行，197/197（`artifacts/pets-watchdog-tests.log`）。`cargo clippy --workspace --all-targets -- -D warnings` 初次因沙箱网络限制无法下载官方库；设置 `GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip CARGO_NET_OFFLINE=true` 后通过（`artifacts/pets-watchdog-clippy.log`）。原生开发宿主保留一条既有链接警告。

## 2026-11-15 · 分块图片传输期间的输入回显停顿

基于 `ea48542` 的未提交改动。用户在实际 Codex composer 中报告：上一版同步刷新看门狗修复后，输入英文或中文仍要等一段时间才显示。`ea48542` 只调整同步提交截止时间；本轮定位到 PTY reader 在 `WouldBlock` 后的固定 `thread::sleep(4 ms)`。Codex pet / 图片按块输出时，块与块之间都可能进入该分支，等待随块数累积。

reader 改为 `poll(POLLIN, 4 ms)`：下一块可读时立即唤醒，无数据时仍以 4 ms 为上限检查会话关闭与同步提交截止时间。非 Unix 分支保留原 sleep。

新增 `tests/terminal/fixtures/GraphicsInputFixture.tsx` 与 `tests/probes/graphics_input_probe.py`：真实 PTY 持续输出分块 Kitty RGB 图片，同时经实际 AppKit field editor 做 marked text 和中英文提交，分别记录组合输入调用与字符回显耗时。原生用例要求 24 次 marked 调用 < 150 ms、8 次回显 < 300 ms。

- 对照（仅把 reader 恢复为固定 4 ms sleep）：8 次回显为 612 / 746 / 897 / 692 / 715 / 733 / 662 / 736 ms，超过 300 ms 断言失败；日志 `artifacts/ime-reader-baseline-final.log`。
- 恢复 `poll` 后：回显 14–24 ms，24 次 marked 调用最大 6 ms；`./scripts/test-native input -v` **2 / 2** 通过，日志 `artifacts/ime-reader-native-final.log`、报告 `artifacts/native-ime-tests.xml`。

同轮排查发现 `native AppKit composition commits once…` 在干净 `ea48542` 上已失败：中文 marked text 的 `firstRectForCharacterRange` 返回 `(0,0,0,0)`，ASCII 正常；暂存 reader 改动后同样失败，确认与 reader 无关。上一版“按组合内容扩展 field editor”只用受控 `value` 测宽，而 marked text 存在原生 field editor 内，中文组合时字段仍为 1 格。现在分开测量 `value` 与 `composition_text` 并取较大值；修复后中文候选锚点为 `(845.5,937)`，与组合前同一基线，用例通过。首次失败记录保留在 `artifacts/ime-reader-composition.log`。

常规检查：`./scripts/nsh check`、`./scripts/nsh build`、`./scripts/nsh test --junit artifacts/ime-reader-tests.xml` **197 / 197**、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check` 通过；日志 `artifacts/ime-reader-{check,tests,build,clippy}.log`。原生宿主仍有一条既有 `__eh_frame` 链接警告。

受控探针不是实际 Codex 负载，也不等同于系统输入法候选窗的人工验收；用户实机手感、真实 IME 候选窗位置及其他平台仍未复验。

## 2026-11-15 · 可配置的 working 指示器

working 状态原先固定为三段横线呼吸。用户查看候选后认为三段横线的视觉辨识度太差，已从选项中移除；Spectrum 与 Equalizer 过于接近，也已替换为 Vinyl（黑胶转盘）。Reduce Motion 显示所选形状的静态版本。现在 Settings → Interface 提供 `Working indicator`：Equalizer（三柱，默认）、Vinyl（黑胶转盘）、Wave（三点）、Beat（节拍环）。选择写入工作区存储的 `workingIndicator`；缺失、未知、旧 `dashes` / `spectrum` 或旧文档一律回退为 `bars`，不提升快照版本。指示器颜色跟随标签所属分组的 folder 颜色，未分组回退 `primary`。只在 `working` 且没有进度时播放；有进度时仍显示静态图标和进度线，blocked / done / error 不变。

`ui/shared/WorkingIndicator.tsx` 使用 NSH `createMotion` 的 `scaleY` / `translateY` / `rotate` keyframes，30 fps，无图片资源；每个子动效在卸载时 `stop()`。设置页的四个选项本身是实时预览，对话框关闭时暂停动画。

- `./scripts/nsh test --junit artifacts/working-indicator-tests.xml`：**199 / 199**，日志 `artifacts/working-indicator-tests.log`。新增快照测试覆盖缺失 / 未知 / 合法值及旧 `dashes` / `spectrum` 回退，持久化测试覆盖未知值回退后重写、界面选择后写入 `vinyl` / `beat`。
- 原生截图：`artifacts/working-indicator-{bars,vinyl,wave,beat}.png` 与 `artifacts/working-indicator-settings.png`。使用带 purple 分组的临时 fixture，确认侧栏指示器为分组紫色而不是 primary；设置页四个选项和实时预览可见。临时 fixture 与测试已删除，日志 `artifacts/working-indicator-native.log`。`artifacts/working-indicator-{dashes,spectrum}.png` 是移除前候选的历史截图，不再作为当前选项。
- `./scripts/nsh check`、`./scripts/nsh build`、`git diff --check` 通过；原生宿主仍有既有 `__eh_frame` 链接警告。
- 设置页预览固定使用 primary，不随活动标签的分组颜色变化；实际侧栏按每个标签自己的分组颜色渲染。动画手感、真实 Codex working 往返和长时间 CPU 占用尚未由用户人工验收。

## 2026-11-16 · 组概览 History、分支与 worktree

基于 `bf27737` 的未提交改动。新增 `HistoryPanel`、`commit-graph.ts`、`src/git_ops.rs`，并在组概览头部加入 Changes / History 切换。验证环境为 macOS arm64、Rust / Cargo 1.99.0、NSH 0.5.0；窗口 1200 × 780。

实现边界：

- 只读查询仍在 `src/git.rs`：porcelain v2 status、diff、`for-each-ref` 分支 / 标签、`worktree list --porcelain`、`git log`、单提交元数据 / 文件 / patch。读取不设置 optional locks、不执行 hooks。
- 显式写入集中在 `src/git_ops.rs`：checkout、创建分支、删除分支（`git branch -d`，不 force）、worktree add / remove / lock / unlock / prune。写操作允许 index 锁和仓库 hooks，但不用 `--force`；分支名经 `check-ref-format --branch`，起点经 `rev-parse --verify`，worktree 路径必须匹配 `worktree list`。当前分支、未合并分支和已在 worktree 中检出的分支由 Git / 预检查拒绝。
- 提交车道由 `ui/dashboard/commit-graph.ts` 从父关系计算，固定首列用 `Canvas` 绘制；提交行是 `Scroll` + `Each` 中的整行 `Button`，每页 200 条、底部 Load more；表头和行按实际分栏宽度计算，Commit 列填充剩余空间，拖动分栏或缩放窗口会增减 Author / Date / Hash 列，列集合变化时重挂载列表以免沿用旧行宽度。选中行由内容层按选中 index 绝对定位的 primary overlay 绘制，不依赖行组件重渲染；hover 使用 native `hover:bg-tab-selected`。画布 ≥1000 pt 时三栏；更窄时隐藏右栏，提交列表下方显示可调的详情 / patch 分栏。Changed files 行可点击、带选中 overlay 和 hover；默认选中第一个文件，右侧 `DiffView` 只显示该文件的 patch，按提交 patch 的文件段切分。`DiffView` 不再接收 `trimEnd()` 后的 patch：末尾空 context 行被删掉会让 hunk 计数不足，`45aba59` 曾因此完全不显示。面板每 5 秒轻量刷新 status / branches / worktrees，HEAD 目标变化或 scope 失效时才重载提交。
- 合并、rebase、cherry-pick、reset、stash、冲突解决与 MR / PR 均未实现；没有 stage / unstage / commit。

验证：

- `./scripts/nsh check`、`./scripts/nsh build`、`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`git diff --check` 通过。
- `cargo test --lib git`：**13 / 13**，覆盖版本解析、porcelain v2、refs、worktree、log、name-status、临时仓库 log / branch / commit、merge commit 对第一父提交的文件与 patch，以及分支创建、切换、删除、worktree add / remove。
- `./scripts/nsh test --junit artifacts/headless-tests.xml`：**218 / 218**，日志 `artifacts/history-panel-tests.log`。新增 `History.test.tsx` 覆盖 Changes / History 切换、分支与 worktree 列表、提交与 patch、点击提交后详情更新、Changed files 默认选第一个文件、点击其他文件后 patch 切换、视口缩到 880 pt 时切为窄布局并隐藏 Hash、放大到 1600 pt 后回到宽布局、新建（不 checkout）分支、删除分支对话框、新建并删除 worktree 的对话框流程；`commit-graph.test.ts` 覆盖线性历史与合并车道。首次全量运行出现一次 `sessions::tests::closing_allows_shell_to_hang_up_background_jobs` 失败（“background job survived closing its terminal”），该测试与本次改动无关；单独重跑和随后两次全量重跑均通过，保留为并行负载下的既有不稳定记录。
- 原生自动化：`./scripts/test-native history` **1 / 1**，截图 `artifacts/history-merge-graph.png`、`artifacts/history-selection.png`、`artifacts/history-hover.png`、`artifacts/history-file-hover.png`、`artifacts/history-file-diff.png`、`artifacts/history-worktrees.png`；`./scripts/test-native gitlive` **1 / 1**，在真实仓库上查看 History 与 Worktrees，并把窗口缩到 920 × 680 检查窄布局、放大到 1600 × 900 检查列填充，截图 `artifacts/dashboard-history-live.png`、`artifacts/dashboard-history-worktrees.png`、`artifacts/dashboard-history-920.png`、`artifacts/dashboard-history-wide.png`。原生宿主保留既有 `__eh_frame` 链接警告。
- 预览 fixture 不执行 git；真实仓库截图显示 master 当前分支、27 个变更文件、提交车道、提交详情和选中文件 patch。原生 hover 截图通过像素采样确认被 hover 的行为 `tab-selected`、选中行为 primary tint；文件列表 hover 也可见。原生宿主后台运行，不作为人工 GUI 验收。
- 最后一批 UI 调整之后，按用户要求只跑了定向检查：`./scripts/nsh check`、`./scripts/nsh build`、定向 `History.test.tsx`、`./scripts/test-native history`、`./scripts/test-native gitlive`、`cargo fmt --check`、`git diff --check` 均通过；**没有重跑全量 218 项**。上面的 218 / 218 和 13 / 13 是这批 UI 调整前的最近全量结果。
- 用户验收发现并修正：`45aba59` 的 `docs/architecture.md` patch 不显示，根因是 `patch().trimEnd()` 删掉了 hunk 末尾的空 context 行；History 与 Changes 都改为原样传给 `DiffView`。小窗口放大后 Author 列出现但行内作者为空，根因是列表行没有随分栏宽度重渲染；提交列表改为 `Scroll` + `Each`，列布局变化时重挂载，并在 headless 用例中验证 700 → 1600 后 Author 出现。点击提交后详情更新但行背景不变，改为选中 index 的绝对定位 overlay；Changed files 和 worktree 同样处理。定向复验：`./scripts/nsh check`、`History.test.tsx`、`./scripts/test-native gitlive` 通过；真实仓库截图 `artifacts/dashboard-history-45aba59.png` 确认选中行背景、文件选中和 patch 显示。仍未跑全量。

尚未验证：从 UI 在真实仓库切换 / 删除分支后运行中 shell 的表现、dirty 工作树下的 Git 拒绝提示、从 UI 新建 / 删除 worktree 后的分组与 shell 生命周期、长历史滚动、窄窗口三栏布局、实际输入法与选区回归。删除分支的 Rust 用例与 headless 对话框流程通过，但没有在真实已合并 / 未合并 / 已检出分支上逐项人工验收。预览截图和 headless 交互不能替代这些实机验收。

## 2026-11-16：新建终端入口与分组上下文

基于 `bcf08b7` 的未提交改动。`./scripts/nsh check`、`./scripts/nsh build`、`git diff --check` 通过。`./scripts/nsh test --junit artifacts/headless-tests.xml` 为 217 / 218：`clipboard::tests::osc52_bytes_reach_the_pasteboard_as_unicode` 失败，提示 `pbcopy rejected the sample`；本次分组、未分组及概览入口的 headless 交互通过。JUnit 记录位于 `artifacts/headless-tests.xml`。

原生窗口自动化及人工 GUI 验收未执行；headless 预览不能替代真实窗口交互验收。
