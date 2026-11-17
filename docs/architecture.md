# Architecture

目录归属、依赖规则与验证入口见 [开发指南](development.md)。

## NSH mixed 项目

`nsh.json` 声明 Rust crate 和主题；`App.tsx` 是默认 TSX 入口。CLI、`nsh-core`、`nsh-base` 均固定 0.5.0。UI 使用 macOS 原生 AppKit 后端，不引入 WebView。

```text
App.tsx                                      mount 工作区，仅作入口
  ui/app/WorkspaceRoot.tsx                   先读持久化文档，再挂载工作区
  ui/app/WorkspaceApp.tsx                    窗口、菜单、快捷键、对话框与会话操作接线
  ui/workspace/workspace-snapshot.ts         工作区文档 schema、校验与会话 ID 还原
  ui/workspace/persistence.ts                NSH Storage 的读取与保存
  ui/workspace/Sidebar.tsx                   vertical tabs、分组与响铃标记
  ui/workspace/state/index.ts            会话、Git 视图选择与平铺操作、唯一逻辑焦点
  ui/workspace/state/groups.ts           分组、移动与组内固定操作
  ui/workspace/state/initial.ts          工作区与布局恢复
  ui/workspace/tiling.ts                     布局树变换、约束、几何与方向导航
  ui/workspace/TilingCanvas.tsx              按 session ID 稳定挂载、pane 工具栏与分隔拖动
  ui/workspace/GroupEditor.tsx               侧栏锚定的小型分组编辑浮层：组名、颜色和组操作
  ui/git/GitWorkspace.tsx              仓库选择与任务组合
  ui/git/scheduler.ts                 Git 视图周期任务调度
  ui/git/operation-gate.ts            跨视图的仓库操作互斥
  ui/git/GitPanel.tsx                  变更文件树、单文件 diff、刷新与状态
  ui/git/HistoryPanel/index.ts         History 公共导出
  ui/git/HistoryPanel/panel.tsx        History 界面与组合入口
  ui/git/HistoryPanel/state.ts         视图状态及派生数据
  ui/git/HistoryPanel/data.ts          Git 数据读取、刷新与订阅
  ui/git/HistoryPanel/actions.ts       Git 操作及对话框动作
  ui/git/HistoryPanel/views.tsx       分支、提交和详情视图
  ui/git/HistoryPanel/common.tsx      绘图和格式化工具
  ui/git/commit-graph.ts               车道数据契约与测试替身的确定性布局
  ui/git/git-types.ts                  TSX 侧的 Git 快照契约与 transport 接口
  ui/git/native-git.ts                 经 termlevel/native 调用系统 git CLI
  ui/shared/theme.ts                         分组标识到主题 token 的映射
  ui/terminal/TerminalSurface.tsx            真实终端的显示、焦点属性与低频状态
    native SessionTerminal       绑定已有 Rust 会话

src/lib.rs                                   注册 termlevel/native 导出与原生 renderer factory
src/runtime.rs                               版本、启动目录、显式预览开关与原生 cell 测量
src/dialog_keys.rs                           对话框 Escape 与 pane 缩放模式的原生键盘优先级
src/git_discovery/                         共享仓库目录、上下文发现与目录监听
src/git/                                     只读 Git CLI 和视图读取状态
  requests.rs                               读取租约、可取消的异步准入和 NSH blocking
  cli.rs                                    有界输出、deadline、进程组取消和 stdin
  refs.rs / decorations.rs                  完整 refs 快照、版本、分批传输和标记预览
  history.rs / graph.rs                      固定 revision tips 的 log 和后台跨页车道
  status.rs / diff.rs / types.rs             状态、差异和绑定数据契约
src/git_ops/                                 显式 Git 操作：fetch、fast-forward pull、push、checkout、建分支、worktree add/remove/lock/prune
src/terminal.rs                              Ghostty 状态 / RenderState 到 NSH 的适配；预览 provider
src/sessions.rs                              SessionManager、PTY、进程、reader/writer/reaper
  sessions/api.rs                TSX 可调用的显式会话操作
src/session_view.rs                          按 ID 绑定会话、低频状态通知、唯一 16 ms 帧调度
  session_view/surface.rs         每会话显示控制器：窗口、失效、轮内合并与一次采样
  session_view/view.rs            NSH 绑定：固定格网、原生 Input、事件策略接线
  session_view/window.rs         展示窗口策略；原生内容锚点留在 Ghostty
  session_view/paint.rs          owned 绘制场景、颜色和行语义
  session_view/presentation.rs   平台无关的最新场景槽、弱注册表与原生属性绑定
  session_view/platform/mod.rs   编译期平台选择与终端挂载 / 输入 / 注册入口
    platform/macos/view.rs       NSView 容器、失效和主线程场景读取
    platform/macos/renderer/     CoreText / Core Graphics 绘制、字体布局与原生缓存
    platform/macos/keyboard.rs   AppKit local monitor 与 IME 按键仲裁
    platform/macos/probe.rs      debug-only 原生输入与绘制验收驱动
    platform/unsupported.rs      后续平台入口；明确显示 renderer 尚未实现
  session_view/input.rs          组合输入和提交状态
  session_view/keys.rs           NSH 按键到 Ghostty 输入事件的映射
```

TSX 在 QuickJS 中执行，经 NSH 引擎由原生平台绘制。`termlevel/native` 是同进程 Rust 绑定，不是网络 RPC。类型声明由 NSH 生成到 `.nsh/types`，不手工维护。目录按职责划分，不为每个组件另建 crate。

字体资源由 `src/lib.rs` 的 `Registry::assets(assets!())` 统一注册：NSH 在首次布局前以进程作用域加载，并在打包时包含原始字体、OFL 许可证、作者声明和来源校验清单。默认终端字体链为 JetBrains Mono 2.304 → Sarasa Term SC 1.0.43 → 系统等宽；字体与版本清单见 `assets/fonts/`。新工作区使用该默认值，已有字体选择不覆盖。renderer、格宽/输入测量共用 CSS family list 和 CoreText cascade；标准 JetBrains Mono 的编程连字保留多格 advance。

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

Rust → TSX：标题、运行/退出/错误状态、响铃、OSC 7501 程序状态摘要、OSC 9;4 进度、OSC 7 实时目录、OSC 22 鼠标形状与 OSC 133 prompt 状态；不发送终端 chunk 或 cell grid
```

OSC 逐项盘点、状态业务规则与 UI / UX 统一见 [OSC 集成与终端状态设计](osc-support.md)。安全封装注册 `GHOSTTY_TERMINAL_OPT_PROGRESS_REPORT` 与 `GHOSTTY_TERMINAL_OPT_PROGRAM_STATUS`，在回调内复制有界 owned 文本；OSC 7501 的完整 id 记录表由 `Session` 持有，`SessionInfo` 只带聚合摘要，避免 16 ms 轮询克隆整张表。7501 记录保留状态代次和已读，版本确认不删除程序事实；blocked 仍可见，已读 done/error 不再参与聚合。Rust 的 `TerminalStatusInfo` 统一生命周期、活动与真实进度，侧栏、tooltip 和无障碍不再分别推导；`shellPhase` 区分未集成与已知阶段。注册 program status 回调后，libghostty-vt 会通过已注册的 write-PTY 回复 `OSC 7501;?` 查询，程序因此认为终端支持该协议。进度百分比在 Rust 侧合并发布，restart 不恢复这些运行期元数据。OSC 7 的实时目录写入 `SessionInfo.currentCwd`；UI 用运行期 `Tab.liveCwd` 显示和搜索，保存文档时写入 `cwd: liveCwd ?? cwd`，因此下次启动使用最后目录，schema 不变。OSC 22 形状和 OSC 133 prompt 状态同样是运行期元数据。

每个会话有独立 Ghostty terminal、持久 parser、原生 RenderState 与 Rust 帧缓存。默认展示最新 10,000 行历史，设置可在 1,000 到 100,000 之间按 1,000 行调整；备用屏幕没有历史。Ghostty 底层按页回收，物理存储可能额外保留一页。安全层用两个屏幕各自的 tracked grid reference 记录逻辑起点，因此展示的上限精确，改小后再改大不会重新出现已丢弃的旧行。滚动历史上限随工作区文档持久化。

reader 直接将原始字节送入同一个 Rust/C parser，保留跨 chunk 的 UTF-8 和转义状态。活动屏幕按需生成拥有独立内存的 RenderState 帧；固定格网只复制当前窗口的行，历史锚点由 Ghostty tracked reference 随重排和淘汰维护，不在每个输出 chunk 复制整份历史。宽字符与组合字符保留列坐标，ASCII 同样式 run 合并。所有终端字节、网格、历史与匹配都留在 Rust。

⌘F 的 ASCII 查询使用 Ghostty 分步 feed/tick，保留原生软换行匹配；Unicode 使用分批复制行、锁外小写匹配。每次最多处理一批，输出变化后等待 150 ms 安静期重扫，不在绘制快照锁里全量查找。低频 SearchChange 只发送 query、计数、索引与 searching；查找栏显示 Searching…，异步完成不覆盖别的 query。新输出不强制跳到命中。产品预览不创建终端会话。⌘K 仍按标题、目录和分组查找会话。

程序指定的 ANSI / 256 色 / truecolor 与 OSC 调色板覆盖保留语义；未指定的颜色跟随主题。NSH 的基础 256 色写入 Ghostty 的默认 palette，主题写入默认前景 / 背景，程序覆盖不会被主题变更清掉。颜色、光标和设备协议回复直接经 Rust 回到 writer。字符尺寸查询有实际 rows / cols；NSH 此处只有 point 测量，没有设备像素尺寸，像素回复仍为 0。

同步输出由 mode 2026 的原生回调在确切边界捕获可见屏幕和屏幕身份。已挂载的贴底 surface 可推进到 hold 开始边界保存的完整帧，同一边界复用 Arc；历史窗口保持冻结。初次绑定同样展示 held 帧，不采样事务中的实时屏幕。正常结束和 EOF 都会提交输出；看门狗在最后一段非空 PTY 输出后静默 150 ms 时提交，单次 hold 最长 1 s。持续到达的图片分块会延后静默截止时间，避免提交旧图已删除、新图未完成以及光标暂时隐藏的中间状态。真实库测试确认 1.0.1 原生 resize 也会结束 hold，随后发布新尺寸。安全层会让独立 renderer 在回调消费 dirty state 后重新建立快照，避免把一次渲染的状态误当成另一个 renderer 已处理。PTY 与网格 resize 仍在同一个 emulator 锁内完成，防止 SIGWINCH 后的新尺寸输出落入旧尺寸网格。

### 终端身份

Settings → Terminal 的 Terminal identity 默认是 Termlevel，也可显式选择 Ghostty compatibility。应用接线持有该偏好，随工作区文档保存，并在创建工作区前读取，以便启动恢复的新 shell 使用所选身份。旧文档缺少该字段或字段未知时回退 Termlevel，保留 schema v1 和既有布局。

每次 `Sessions.open` 单独传入身份，由 Rust 在 spawn 前白名单映射：两种模式均设置 `TERM=xterm-256color` 和 `COLORTERM=truecolor`；默认设置 `TERM_PROGRAM=termlevel` 与应用版本，兼容模式设置 `TERM_PROGRAM=ghostty` 并移除 `TERM_PROGRAM_VERSION`，不伪造 Ghostty 版本。继承的 `LC_ALL` / `LC_CTYPE` / `LANG` 都不是 UTF-8 时，补上 `en_US.UTF-8`，避免会话里的 `pbcopy` 把 OSC 52 同期写入的中文按 MacRoman 存盘。shell 启动文件仍可能覆盖环境。设置变化不修改已有会话、不创建或重启进程，也不改变 Ghostty 协议解析与原生 renderer。身份覆盖仅供客户端识别兼容，不代表被模拟终端的全部能力或真实 Codex Pets 已通过验收。

## Git 视图数据通路

GitWorkspace coordinates repository selection and background tasks. Discovery checks only the group directory and its direct child directories using git rev-parse --show-toplevel, accepts only working-tree roots, publishes incremental results, and is cancelled on view disposal or replacement. A single view-scoped scheduler refreshes data every 5 seconds, scans repositories every 30 seconds, and fetches the selected repository every 5 minutes while the app is active. Manual refresh also scans. Preview runs no background tasks. A shared per-repository operation gate prevents overlapping Git mutations across view remounts. Automatic fetch failures are displayed without repeated notifications.

Git 视图由系统 `git` CLI 提供数据，应用不内嵌 Git 实现。最低版本为 Git 2.20；版本过低、找不到 git 或仓库不可读时返回带说明的错误，由 UI 显示提示。读取集中在 `src/git/`，显式写操作集中在 `src/git_ops/`。

```text
TSX GitPanel      ── status / diff ───────────────────────┐
TSX HistoryPanel  ── branches / worktrees / log / commit ──┤ Rust Git（只读）
                                                           │ blocking（engine 线程外）
                                                           │ git -C <repo> ...
                                                           ▼
                                    RepoStatus / RepoBranches / RepoWorktrees
                                    RepoLog / RepoCommitDetail / unified patch

TSX HistoryPanel  ── fetch / pull / push / checkout / createBranch / worktree* ── Rust GitOps
                                                           │ 显式操作，60 秒超时
                                                           ▼
                                    git checkout / branch / worktree ...
```

- 状态使用 `git status --porcelain=v2 --branch -z --untracked-files=all`，只解析文档化的机器格式；rename 用 `2` 记录的两个 NUL 路径，冲突用 `u`，untracked 用 `?`。
- 差异使用 `git diff --no-ext-diff --no-textconv --no-color --unified=3 [--cached] -- <path>`；untracked 文件由 Rust 读取有界文本生成新文件 patch。
- 分支使用 `git for-each-ref` 读取 `refs/heads`、`refs/remotes`、`refs/tags`，跳过 `origin/HEAD`；annotated tag 的提交目标使用 peeled object。Rust 保留完整 refs 快照、HEAD 和版本，UI 每次最多接收 256 条。相同版本只返回 unchanged 标记，不重新传数组或构建 Tree；超过 32 MiB 明确报错并保留上次成功快照。
- Worktree 使用 `git worktree list --porcelain`，解析 worktree / HEAD / branch / bare / detached / locked / prunable；`current` 由 group 目录的 canonical path 与各 worktree 路径按组件前缀匹配，取最长匹配。
- 历史使用 `git log --topo-order --max-count=<limit+1> --skip=<n>`；读取首屏时从 refs 快照固定可达提交 tips，后续页沿用同一集合，经 `--stdin` 传入，避免 refs 移动导致重复/漏项以及参数长度限制。HEAD detached 时也包含当前 HEAD。提交元数据不通过 `%D` 全量展开 refs；Rust 按提交附带最多 8 个标记和完整数量，完整名称可在分支 / 标签列表查看。提交详情另读 `git show -s` 元数据、`--name-status` 变更路径和 `--first-parent --unified=3` patch。
- 所有命令以参数数组启动，不经过 shell；读取设置 `GIT_OPTIONAL_LOCKS=0`、`GIT_TERMINAL_PROMPT=0`、`GIT_NO_LAZY_FETCH=1`、`GIT_PAGER=cat` 和 `LC_ALL=C`。写操作不设置 `GIT_OPTIONAL_LOCKS=0`，允许 index 锁与仓库自身 hooks。
- 读取命令 3 秒超时、历史 5 秒；每条命令终止自己的进程组。状态、diff、log 和 commit patch 各限制 2 MiB，单文件预览 512 KiB，条目限制 5,000；提交页默认 200 条，通过底部 Load more 加载下一页。超时覆盖管道排空，即使 Git 本身退出而 helper 仍持有管道也会清理进程组。
- 分支名先经 `git check-ref-format --branch`，起点先经 `git rev-parse --verify <start>^{commit}`；worktree 删除、锁定只接受 `git worktree list` 中的路径。写操作不使用 `--force`。
- 先在 group 路径找仓库根，再以仓库根运行命令，因此路径是仓库根相对路径；group 路径是仓库子目录时，worktree `current` 仍匹配所属工作树。
- 操作进行中通过 worktree 的 git dir 标记检测 merge / rebase / cherry-pick / revert / bisect，只读查询不执行 hooks。
- Git 查询保持只读；History 的显式动作包括 fetch、fast-forward-only pull、push、checkout、新建/删除分支和 worktree 管理。Fetch 使用 Git 配置的默认远端；Pull 更新当前 worktree 的当前分支，分叉时 Git 拒绝，操作失败后仍刷新远端引用。Push 仅推送当前 worktree 当前分支到已配置的上游远端分支，不推其他分支或标签，不强推；缺少上游或 detached HEAD 时提示用户。网络操作使用系统 Git 和已配置的凭据助手；终端交互式提示被禁用。删除分支使用 `git branch -d`，不 force；Git 拒绝当前分支、未合并分支或已在 worktree 中检出的分支。不 stage、unstage、commit、创建 merge commit、rebase、cherry-pick、reset、stash，也不解决冲突。
- 产品预览不提供 Git 数据，也不启动 git；测试夹具可注入确定性 Git 替身。

### 线程与读取生命周期

NSH 0.5.0 的 AppKit 主线程负责窗口、系统输入和平台绘制提交；独立 engine 线程运行 QuickJS、signals、组件与布局。`async` 的同步部分仍在 engine 执行。NSH 已提供 Tokio runtime，应用通过 `blocking` 把 Git CLI、解析、排序、版本计算和车道布局放到其阻塞线程池，不创建另一个 runtime、不直接依赖 Tokio。

每个 group / 工作树会话拥有独立读取租约，Changes / History 挂载时复用该会话。`requests` 的异步准入 future 在 engine 上挂起，全应用最多执行 4 个读取任务；排队时不占阻塞线程。每个租约、每种查询只接受最新请求。切换仓库、替换查询和卸载会取消旧请求，运行中的只读 Git 进程组被终止并回收；generation/disposed 检查同时阻止迟到结果更新 UI。大缓存最终释放也在后台执行，避免 close 或 engine future 收尾时同步释放整张 refs 表。取消读取不取消显式 Git 写操作，也不影响 PTY。写操作继续使用跨视图的仓库互斥 gate。Discovery 使用独立的单线程 worker 与可取消 Git 探测，不占用前台读取的四个名额。

### 多仓库目录与选择

参考 [Zed Repository Activation](https://github.com/zed-industries/zed/blob/main/docs/src/git.md)、[GitStore](https://github.com/zed-industries/zed/blob/main/crates/project/src/git_store.rs) 与 [RepositorySelector](https://github.com/zed-industries/zed/blob/main/crates/git_ui/src/repository_selector.rs) 的发现 / 激活 / 选择分离、工作树身份及名称消歧；Termlevel 用终端 liveCwd 代替编辑器的文件激活上下文。

发现、选择和加载 Git 数据分别管理。Rust `git_discovery/` 按 group 根目录共享目录源；识别最近的上级仓库、根目录的直接子目录，以及终端 liveCwd / 显式添加路径所属的仓库，不递归遍历整棵项目目录。新建、删除、移动目录或 `.git` 变更触发合并后的后台重扫；macOS 使用非递归 kqueue 目录监听，不每 30 秒扫描全部仓库。每个源最多监听 1024 个目录，监听失败或超限明确显示错误并提供 Rescan。无订阅时停止监听，最多保留 8 个闲置源；一个视图取消不影响同源其他订阅。

每个工作树以 canonical root / git dir 分配进程内稳定 ID；linked worktree 独立展示，submodule 通过 Git 元数据识别。common dir 单独用于 Rust 与 TSX 的写操作互斥，避免两个 linked worktree 同时修改共享 refs。目录快照有 revision，每页最多 256 条，版本变化重启组装，未变化不重传目录。选择器使用可搜索的原生虚拟 List；重名用最少父路径消歧，长名称中间省略、完整路径置于副标题与 tooltip。未激活仓库只展示目录元数据及已有的状态摘要，不为列表逐个执行 status / fetch。

Changes / History 共用 group 的选择。默认跟随活动终端目录；手动选择后固定，显式 Follow terminal directory 才恢复自动跟随。所选仓库暂时不可用时保留选择并提示，不跳到其他仓库。所选 canonical root、手动模式及最多 32 个显式路径写入工作区快照；运行时 ID 和 Git 数据不持久化。

运行期按 group / root 保留搜索、scope、选中提交 / 文件、展开目录、diff 布局与滚动位置；重数据最多缓存三个会话，每个最多 5000 个提交 / 32000 条 refs，超限仅保留轻量偏好。会话恢复须验证 Rust history scope / count 游标一致；取消分页导致不一致时重载首页，避免缓存与原生车道错位。逐出或删除组释放读取租约及缓存，关闭 Git 面板不改变 PTY 生命周期。

### History 视图

Changes 与 History 是按需打开的分组视图，关闭不影响终端或分屏；工作区快照保存打开的视图和所选视图。History 左栏用虚拟化 Tree 展示 Local / Remote / Tags，中栏用原生回收式 List 展示提交，每页 200 条；只创建视口附近的提交行。行保留 List 提供的数据访问器，按 SHA 标识，选择状态在回收行内随当前 SHA 更新。调整分栏、更新 refs 和加载更多均保留列表容器及滚动位置。

Rust `graph` 按父关系跨页分配固定槽位，使用目标到槽位的索引，复用空槽而不收缩其他槽位。记录真实 incoming 标记，新分支 tip 没有上方来线；合入已有父提交时连接父槽位，穿行线保持原位。每行最多传输前 24 条穿行线，超出车道共用末列并显示简化提示，提交记录仍保留。Canvas 根据实际列宽压缩车道间距，图列最多占提交面板四分之一宽度。搜索只过滤已加载提交，保留原车道数据；完整 ref 名称的匹配在 Rust 的 refs 快照中执行，UI 合并匹配 SHA，未出现在 chip 预览中的分支仍可搜到；隐藏中间提交时仅绘制点，避免连线暗示不存在的相邻关系。`commit-graph.ts` 的布局函数只供注入的测试 / 预览 transport 使用，生产从 Rust 获取几何数据。

提交行最多展示 2 个有宽度上限的 ref chip 和额外数量，所有 chip 的总宽不超过标题区域的 45%。名称使用中间省略，保留前缀和末尾区别，悬停显示完整名称；左栏和顶部当前分支 / scope 使用同样的省略策略。完整 refs 仍在 Tree 中，详情最多展示 8 个标记，避免单个提交的数万 decorations 创建数万视图。标题栏摘要可以收缩，全局 Git 操作和搜索始终保有空间。

右栏展示提交信息、变更路径和 DiffView；画布不足 1000 pt 时将详情移到提交列表下方。选中文件只显示对应 patch；patch 不 trimEnd，保留末尾 context 行。Changes 的目录树、worktree / changed files 的列表和选中 overlay 保持各自的数据通路。

每 5 秒刷新 status / refs / worktrees。Rust refs 版本、HEAD attached/detached 状态变化时才重载 log，未变化的 refs / worktrees 不更新对应 signals；查询失败保留上次成功快照并显示错误。SHA 只标识行身份，行与详情上的 decorations 来自最新日志快照。显式操作继续由 Git 拒绝 dirty 冲突、分叉或非法 worktree 删除，应用不自动 stash 或 force。

## 历史滚动与跨屏拖选

真实终端在 Rust `session_view` 内显示历史滚动条；视口位置仍由唯一 SurfaceController / Window 管理，不增加 ScrollView 或 JS 网格副本。`scroll.rs` 只负责滚动条几何和按时间计算的边缘滚动速度。滑块根据已发布 WindowSnap 的 origin / count / rows 更新；拖动滑块反算 origin，并走 controller 的显式 reveal。滚动条覆盖在终端外层右边缘，宽度与边距来自 theme.json（默认 10px 命中宽度、2px 右边距），独立于文本内边距，不从终端可用列宽中扣除。滚动、视口变化或指针进入命中区域时显示，悬停 / 拖动期间保持显示，离开后空闲 800ms 再用 180ms 淡出；显示变化复用原有 Rust 帧调度，不改变列数。备用屏幕与无历史时不显示滑块。

按下立即在 Ghostty 建立 tracked 选区，移动后由 NSH responder 捕获原网格手势，指针离开网格后仍收到移动 / 释放。controller 在已有 16 ms 帧调度中推进边缘自动滚动，并按新视口 origin 更新选区终点；速度随离开上下边缘的距离增加，单轮时间上限避免暂停后的追赶跳跃。鼠标停止在边缘仍可持续滚动，回到网格中部速度归零，释放 / 取消 / 隐藏 / modal 阻断 / resize 终止拖选跟踪。选区本身仍由 Ghostty 保存，拖动滑块只改视口，不清除选区。

坐标分工明确：按下和释放使用绝对历史行；hover 与排队的 drag 使用视口内局部行，在 controller 中只加一次 origin。程序鼠标报告不参与宿主拖选合并；Shift 绕过报告后开始的宿主手势保持归属，避免中途松开 Shift 把同一拖选拆成程序事件。同步输出 hold 期间不修改选区或历史视口。双击词 / 三击行沿用原有选择语义，尚未新增按词或按整行扩展的拖选模式。

## 通知面板

标题栏铃铛提供应用内通知历史，侧栏收起后仍可使用。`src/notifications.rs` 是宿主进程内历史的唯一所有者，独立于会话视图挂载和会话关闭；不写入 workspace 文档，应用重启后清空。最多保留最新 100 条，每条记录来源、级别、session ID、时间和未读状态；同一会话相同内容在两秒内合并计数并重新置为未读。标题最多 160 字符、正文最多 4096 字符，控制字符过滤后保留换行和制表。

Ghostty safe wrapper 注册官方 desktop notification 回调，复制 owned 标题和正文，再经 terminal BackendEvent 到 Session 写入历史；支持 libghostty-vt 1.0.1 回调覆盖的 OSC 9 和 OSC 777，未声明 OSC 99 或系统桌面通知支持。OSC 7501 每条记录进入 blocked / error 的跃迁会额外写入一条带记录身份和代次的通知，working、done 和 OSC 9;4 进度不进入历史，详见 [OSC 集成与终端状态设计](osc-support.md)。BEL 仍保留原有侧栏标记，同时记录终端响铃。会话启动 / I/O / 编码等错误、输入拒绝警告、非零进程退出与应用文件夹对话框错误以 app 来源进入同一历史。终端底部不再显示状态条；详细错误和输入拒绝仅进入通知面板。清空历史不会清除会话错误或结束进程。

Fetch / Pull 结束后由应用级通知中心写入成功或失败记录，并在窗口右上角显示 5 秒 toast；操作期间切换视图不会丢失结果，关闭 toast 不移除历史。`ui/notifications/notifications.ts` 在应用根生命周期按 250 ms 检查历史 revision，仅变化时读取 bounded 低频数据；不依赖任何 pane 的刷新或事件补发，隐藏会话也可通知。preview 不读取或修改真实通知历史。`NotificationPanel.tsx` 负责锚定、前三条实测高度和滚动；`NotificationRow.tsx` 负责消息层级、语义图标、展开与行内操作，通过应用传入的 session ID 回调选中已有标签，不创建会话。7501 历史的 `program` 元数据携带 kind/app/title/message，来自 Rust 已清洗的记录；UI 直接消费实际消息，不从生成的通知标题推断任务类型。打开面板标记已读，支持逐项移除、清空、回到来源；来源已关闭时保留内容并禁用跳转。历史按内容高度展示最多三条，受窗口可用高度限制，更多内容可滚动且有提示与分隔线。面板打开期间阻止终端输入，Escape / 外部点击关闭后恢复。

## 生命周期与并发

`SessionManager` 在 Rust 中独立于视图持有会话。只有 `Sessions.open` 创建进程；`SessionTerminal` 只能按 ID 获取已有会话，已关闭或丢失的 ID 返回错误，不会隐式重新创建进程。ID 由 Rust 生成，与标题、排序、分组、选择、PID 和视图位置无关。

- 同一原生宿主进程内重建 TSX 工作区时，从 `Sessions.list()` 重新绑定仍存活或已退出的会话。应用启动则读取持久化文档：标签按保存的顺序、分组和分屏恢复，并为每个标签在保存的目录重新启动 shell；首次启动、没有文档时保持空白，不自动创建 shell。
- 持久化文档带 schema 版本，保存标签组、标签顺序与固定状态、分屏树、选中标签、侧栏宽度与展开状态、字号、滚动历史、字体和外观。session ID 不跨进程恢复，布局中旧的 session ID 在还原时映射到新会话；进程 PID 从不写入文档。NSH Storage 在 macOS 上位于 `~/Library/Application Support/app.termlevel.desktop/storage.json`，损坏时应用以空工作区启动。剪贴板读取权限不持久化，重启复位。OSC 7501 状态、OSC 9;4 进度、OSC 22 形状和 OSC 133 prompt 状态是运行期元数据，不写入工作区文档；应用重启后为空，UI 热重载由 `Sessions.list()` 从当前会话恢复。实时目录不同：它通过 `liveCwd ?? cwd` 写入标签的 `cwd`，所以重启会从最后目录打开新 shell。
- TSX 工作区销毁/重建以及原生终端视图卸载/重挂均不关闭会话。重新绑定读取当前 Rust 网格和状态，不依赖 JS 中的输出副本或事件补发。
- 标签切换、分组折叠、重命名、排序与主题变更保留同一 session ID。工作区保留按 session ID 挂载的终端显示；布局树决定哪些会话同时展示，暂未展示的会话隐藏、禁用命中并移出无障碍遍历。焦点 pane 决定唯一输入目标，其他可见 pane 仍持续刷新。
- 显式关闭标签调用 `Sessions.close`；关闭组逐个关闭其会话。Shell 退出码为 0（含空行 Ctrl+D）时同样关闭标签和 pane，不自动重启。非 0 退出和启动失败保留输出与状态。
- 关闭主窗口或退出应用调用 `Sessions.shutdown`。Unix 上结束前台进程组，向 shell 发送 HUP，让它清理普通后台作业；50 ms 后仍未退出则强制结束 shell。主动脱离终端会话的 daemon 不属于可恢复会话模型。
- child 的检查、终止和回收共用控制锁，避免对已回收并可能复用的 PID 发信号。关闭后 reader/writer 取消，reaper 回收 child；应用退出在 NSH 的 200 ms 清理预算内轮询 `Sessions.settled()`。

reader/writer/reaper 在 Rust 工作线程中运行。Unix PTY 使用非阻塞 I/O；reader 以 8 KiB 块解析，暂无可读数据时等待可读事件（4 ms 上限，用于检查关闭与同步提交截止），不对每个块间空隙固定睡眠，避免分块图片传输累积输入回显延迟。writer 使用每会话独有的 FIFO 字节队列：总待发送上限 4 MiB（含 writer 持有的未完成块），单次输入上限 1 MiB，相邻小输入合入最多 8 KiB 的写块。生产端只做有界拷贝并唤醒 writer，不等待子进程读取；键盘、粘贴、鼠标与协议回复共享顺序，PTY 短写 / WouldBlock 保留剩余数据。超限整条拒绝，通过独立 inputWarning 提示未发送和重试，不把运行中的会话标为失败。只有在拒绝之后接受的输入成功写入后才清除此提示，拒绝前的在途写入不能误清；真实读写故障仍由 error/state 表达。关闭和进程退出清空队列并唤醒等待线程。每个会话只有一个 16 ms 帧调度：同一轮最多观察一次输出 revision 与输入代际，并把窗口、悬停、宿主拖选、选区、尺寸、查找 reveal 合并为至多一次采样与发布；事件处理器只登记失效，不直接捕获。亚行滚轮的整数位移与小数余量都保留，报告滚轮不再按单次 32 行截断，而是分块写入且不丢位移。工作区传入低频 visible 属性；隐藏 pane 暂停抓帧、动画推进和查找，恢复可见时立即刷新，非活动但可见的 pane 仍继续绘制。视图刷新计时器随视图销毁，会话工作线程不会随之销毁。

`SessionTerminal` 通过 NSH 0.5.0 的 Native Components 公开接口嵌入应用自己的 NSView。src/lib.rs 的 #[native_components(macos)] 同时供开发与打包宿主注册；无需修改框架。测量视图是唯一 PTY resize 输入，格子宽高来自当前 Frame 行列数；剩余像素留在格子外。贴底 origin=line_count-Frame.rows，ED2 保留的历史不能切入屏幕。绘制规则与验收边界见 [终端格网设计](terminal-grid.md)。

快照分成锁内复制 Frame/历史 cells/图片、锁外合并 run 与可访问文本两阶段。Rust 引擎以 Arc<Scene> 发布完整场景，主线程只短暂锁住场景槽以克隆最新 Arc，随后在无 emulator 锁时绘制；跨 NSH 属性只有槽 ID 与 revision。槽注册表保存 Weak，发布者和原生组件随挂载释放，不拥有会话。字体、CTLine 与 CGImage 留在主线程 renderer。

原生 NSView 按图片层 0、单元格背景、图片层 1、搜索和字形、图片层 2 绘制。字体与字形排版有界缓存，图片按 owned 像素身份复用。run 按 Ghostty 列坐标锚定并独立裁剪，主字体基线一致，启用字体连字；同样式 ASCII 片段中合成连字但不合并终端格子。跨样式/选区边界的连续连字 shaping、复杂 fallback、emoji 与细粒度 AX 仍需扩展或验收。一个无绘图操作的 Canvas Drawing 仅提供非空可见行 CanvasNode 语义给 NSH 测试模型；原生容器同时有整个窗口文本标签，不挂逐行视图。光标独立，闪烁不重绘字形。测试夹具可注入独立的终端模拟器。

唯一原生 Input 与格子同一原点，位置来自展示快照的 input_cursor，y 夹在格子内；组合时显示 marked text 和系统输入法交互，平时文字透明。开始输入/组合请求贴底，hold 期间延后到下一次发布。DECTCEM 隐藏输出光标不移除输入锚点。

`InputState` 缓存组合过程，只有结束后提交最终差量；取消删除 marked text 不发送残留拼音。原生值先确认，再通过 NSH `after_answer` 清空，避免 Rust host 属性比较跳过相同的空值；若已进入下一次组合，暂不清空。普通输入处理清空尚未到达平台时的累积值，避免快速输入重复。组合期间暂停终端按键绑定，Enter/方向键/Escape 交给输入法；NSH 脚本回退绑定只在该会话 active 且 Input focused 时生效；生产 local monitor 以 active 授权，避免布局导致 field editor 失焦时丢键。Command 编辑快捷键不占用工作区新建/关闭标签快捷键。

离开 macOS field editor 时，AppKit 可以确认当前 marked text；回调仍绑定旧 `Arc<Session>`，不向新活动会话转发。挂载、隐藏与焦点不拥有 PTY 生命周期。

## 终端平台接入边界

空白格的背景来源属于 Ghostty 安全封装：活动帧与历史行均将 packed cell 的背景 content tag 合入 owned `Cell.style.background`。字形的格子适配属于平台 renderer：macOS 对画线 / 块字符复用系统等宽字体的矢量轮廓及共享参考平面，普通文字继续使用 CoreText 文本路径；不在 Session 或 TSX 中维护字形表。

`SessionTerminal` / `view.rs` 只通过 `platform` 挂载 surface、安装物理输入适配与查询 NSH 脚本输入回退。`SurfaceController`、`Window`、`WindowSnap`、`paint::Scene` 和搜索仍属于通用 Rust 路径。发布通道独立在 `presentation.rs`，仅持有 owned `Arc<Scene>`；宿主通过 token 解析槽，每次绘制调用 `Slot::latest()` 克隆最新场景后立即释放锁。原生对象和 PTY 都不属于这个通道。

当前后端在编译期选择，不使用动态插件或通用 renderer trait。新增平台需要：

1. 在 `platform/mod.rs` 选择新的后端，实现 `surface(scene, label)`、`install_input(session, active, cell_width, line_height)` 和 `synthetic_keys()`；debug 驱动可实现对应的 `track`。
2. 经目标平台的 NSH Native Components 入口注册同名组件 `termlevel.terminal-surface`。`src/lib.rs` 是宿主注册的唯一外部接线点；macOS 使用 `#[native_components(macos)]` → `platform::register_components`。
3. 使用 `presentation::element` 绑定场景生产闭包与无障碍文本闭包；组件收到 `frame` 属性后通过 `presentation::resolve` 取得槽并请求重绘。复用现有 latest-only 传输及释放语义，不向 JS 传 cell grid。
4. 平台宿主与 renderer 负责自己的 UI 线程亲和性、字体 / 图片缓存和资源销毁；surface 不截获指针或拥有 PTY。公共 NSH 输入仍负责 IME 提交、粘贴、布局与普通指针，平台输入钩子只补足原生事件能力。

`platform/macos` 是目前唯一实际实现，内部再区分 NSView 宿主、CoreText 绘制、键盘监听和 debug 探针。其他平台有明确的未实现占位入口，尚不能运行真实 renderer。本次只整理终端 surface 边界；Ghostty 预编译库仍仅支持 macOS，系统剪贴板 helper、工作区原生快捷键和其他平台构建 / 打包也需后续接入。不能从这些钩子推导 Windows / Linux 已可编译或通过验收。

## 原生交互与工作区 UI

本地文件拖放使用 NSH 原生 `on_file_drop`，由目标 SessionTerminal 接收 file URL，在 Rust `session_view/file_drop.rs` 解码并生成 shell 字面参数（单引号与内部引号转义），经过已有 Session.paste / Ghostty bracketed-paste 路径送入该会话，不发送 Enter。支持图片和其他本地文件的多路径输入，不读取图片或上传附件。隐藏、modal 输入阻断、IME 组合中及已退出会话忽略 drop；可见非活动 pane 先请求工作区焦点再输入。远程 URL 和含控制字符的路径整批拒绝。`inputBlocked` 独立于 `active`，避免把非活动 pane 与对话框阻断混为同一状态。

多 pane 已接入工作区，完整交互和验收边界见 [多终端平铺设计](terminal-tiling.md)。

`DialogKeys` 在原生层为对话框取得 Escape 优先级；界面与对话框状态仍在 TSX。`GroupEditor` 使用 NSH `Overlay` 和组头 `ViewRef` 定位，修改即时写入工作区，关闭不回滚。

统一窗口栏固定在顶部；下方侧栏与终端作为独立圆角 pane，外边距和间距由 `gap-pane` 控制。Sidebar 唯一实例在窗口根层绝对定位，避免被 44 pt 原生分栏裁剪。分栏边界两侧各分配半个间距，根层侧栏扣除这半个间距，与终端 pane 顶底对齐。分栏内测量 View 的 `onLayout` 同步拖动中的实际宽度；`onResize` 只保存最终宽度。收起时鼠标悬停展开覆盖终端，不改变终端分配宽度和挂载位置；编辑组时保持展开。

会话标题来自低频元数据，用户重命名后不再被程序标题覆盖。非活动会话响铃时显示标记，选择后清除。垂直拖动标签在侧栏内排序或放入分组。落到已有分组的标签上会加入该组，落到没有分组的标签上会组成新组，路径取被拖到的标签；放入折叠组时展开该组。分组保存自己的路径，组内新建的终端从这里启动，组名默认是路径名。取消拖动不改分组，session ID 不变。工具栏目录使用 OSC 7 / ConEmu / iTerm2 报告的实时 cwd，没有实时值时回退到启动目录；tooltip 标明当前目录。退出码非 0 或启动失败时，pane 标题栏显示简短状态与新建终端入口，已有输出保留，详细信息进入通知历史。退出码 0 会关掉这个终端。

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
- 布局与设置已持久化；应用重启仍会结束旧会话，保存的标签以新 shell 重开，PID 不能作为恢复凭证。首次启动显示空白引导页。

### libghostty-vt 独立 Rust 封装

工作区新增两个 crate，均不依赖 NSH、PTY 或应用的会话管理：

- `crates/ghostty-vt-sys`（包名 `termlevel-ghostty-vt-sys`）：官方 libghostty-vt **1.0.1** C ABI、原版头文件、预生成绑定，以及通过固定 SHA256 验证的官方 macOS XCFramework。构建提取目标架构静态库，不需要 Zig。支持在 macOS host 上构建 arm64 / x86_64，其他目标明确报错。
- `crates/ghostty-vt`（包名 `termlevel-ghostty-vt`）：句柄 RAII、字节解析、resize、文本导出、拥有独立内存的 viewport 快照、协议回复和元数据事件，以及随当前终端模式编码的键盘 / 鼠标 / 焦点 / 文本粘贴。原生句柄经线程契约审查后为 `Send`、保持 `!Sync`，每次访问均由 `Session.emulator` 的 Mutex 串行化；普通同步 C 回调复制拥有的数据；剪贴板回调调用受限的 Send + Sync host adapter，禁止重入 terminal 或等待 UI 线程。PNG decoder 在创建任何 safe terminal 前通过 Once 安装，输出使用 Ghostty 提供的 allocator。复制后的帧可发送到绘制线程。

这两层只拥有终端模拟状态，Rust `SessionManager` 继续拥有 PTY 与进程。生产路径已接入这两层，旧 Alacritty 依赖已移除。安全层包含历史窗口、搜索、颜色默认值、render hold、tracked selection、超链接目标、拥有像素的图片 placement 与同步剪贴板 adapter，完整 C 声明仍保留在 raw API。默认整 viewport 复制是首版接口，不作为最终 renderer 性能结论。

下载、缓存、离线构建和版本来源见 [sys crate](../crates/ghostty-vt-sys/README.md)，安全层边界见 [wrapper](../crates/ghostty-vt/README.md)。两包尚未发布到 crates.io。

## 主题与预览

`theme.json` 是配色、字体、尺寸与圆角的唯一设计入口。组件使用语义类和 `token()`。侧栏和浮层均为实色，通过低对比分界、细边框和轻阴影表达层级；不设置玻璃或背景模糊。主题不拥有会话生命周期。详情见 [design.md](design.md)。

`TERMLEVEL_PREVIEW=1` 显式启用确定性 UI 预览，不执行系统命令、不访问真实工作区存储；Git 和终端会话显示不可用提示。预览测试通过替换 `Runtime.previewMode` 进入该模式；真实会话测试使用单独的原生绑定与工作区 fixture，以及默认入口的原生 fish 验收。
