# 文件视图设计

状态：草案。日期：2026-11-18。

本文确定分组内的文件浏览和文本编辑。代码所有权见 [架构](architecture.md)，视觉见 [设计](design.md)。实现前本文保持草案；落地后把行为补进架构与路线图。

## 结论

NSH 0.5.0 没有文件浏览器、目录大纲或 `NSOutlineView` 封装。应用内树用 `nsh/base` 的 `Tree`。选一个尚未打开的根目录时，继续用已经接好的系统文件夹对话框（`NSOpenPanel`）。在访达中显示、用默认应用打开、移到废纸篓，用 `nsh-core` 里仅 Rust 可调用的路径操作。

`Tree` 把「有子节点」当成文件夹。空目录和尚未读取的目录在模型里一样。本方案在每个尚未展开的目录下放一个禁用的占位行，读完后换成真实子项；空目录换成一行禁用的「空文件夹」。

## 目标与范围

用户在当前分组里浏览该分组的目录，打开文本文件并用 NSH `Editor` 修改、保存。终端会话保持原来的生命周期。Git 的提交列表和 diff 继续用各自的 `List` 和 `DiffView`，不抽一层共用的虚拟列表。

第一版提供：

- 分组画布上的 Files 视图，与 Changes、History 并列。打开它时终端仍在 Rust 里运行，关闭视图后回到原来的分屏。
- 以分组目录 `PersistedGroup.cwd` 为根，只列出当前展开的那一层。目录树用 `Tree`。
- 目录优先、大小写不敏感的名称排序；隐藏点文件默认关闭，开关按组记住。
- 选中文本文件时在右侧用只读开关关掉的 `Editor` 编辑。语言由文件名决定，语法高亮用 `Editor` 自带的实现。单文件最多读 512 KiB。二进制和超限文件只显示说明。
- 显式保存写回该路径。内存里保留脏标记；未保存内容不写入工作区快照。关闭视图或换文件时，有未保存修改先确认。
- 刷新、在访达中显示、用默认应用打开、在此打开终端、复制路径、确认后移到废纸篓。
- 只监听已经展开的目录。产品预览不访问真实文件系统。

第一版不做这些事：再封装 `List` 或 `Tree`、重命名、新建、拖到终端、多选移动、全树搜索、语言服务器、远程或 SSH 文件系统、Quick Look、永久删除、跟随终端把根目录改掉、Windows / Linux 的目录监听。PDF、视频和压缩包不编辑。

## NSH 里能用的部分

| 能力 | 位置 | 本方案里的用途 |
|---|---|---|
| `Tree` | `nsh-base` `src/tree.rs`，TSX 为 `nsh/base` 的 `Tree` | 虚拟化大纲、键盘、展开和双击动作。Changes 已这样用 |
| `List` | `nsh` 的 `List`，底层 `virtual_list` | 提交历史和仓库选择器已经直接用它。文件树不用它，也不再包一层 |
| `Editor` | `nsh/base`，宿主默认包含 `editor` 与 `highlight` | 右侧文本编辑。`readOnly` 仅用于不能写的文件 |
| `Scroll`、`ResizablePanel`、`Button`、`Input`、`Menu` | 现有 UI | 预览滚动、分栏、工具栏、行菜单 |
| `OpenDialog::folders` + `pick_paths` | `nsh-core` `src/file_dialog.rs`，经 `Runtime.pick_folder` | 仅在用户要把分组目录换成另一个文件夹时使用，和新建分组相同 |
| `open_path`、`reveal_path`、`move_to_trash` | `nsh-core` `src/element/ambient.rs` | 默认应用、访达、废纸篓。这些函数没有暴露给 JS，要在 `termlevel/native` 包一层 |

`file_dialog.rs` 写明这些请求只在 Rust 侧发起，没有能力门闩。对话框只返回用户选中的路径。它们是系统面板和文件管理器动作，不能当作应用内浏览器。

`TreeItem` 的字段是 `id`、`label`、`children`、`disabled`。`renderItem` 收到的 `TreeEntry` 只有 `id`、`label`、`depth`、`folder`、`disabled`，没有自定义载荷。行的图标和占位文案要从 `id` 和一份侧表读出来。`onAction` 在叶子上按 Return，或在任意行上双击时触发；方向键只走 `onSelectedChange`。打开预览和「在此打开终端」放在 `onAction` 和显式按钮上。

## 放置

Files 是分组的第三种工具视图。标题栏 Files 按钮、File 菜单和分组选项打开当前分组；没有选中分组时说明需要先选一个。侧栏标签在打开后出现在 Changes、History 旁。选中后 `activeGitView` 那条「工具视图占满画布」的路径同样挡住分屏：`TilingCanvas` 在工具视图打开时挂载面板，终端 surface 保持挂载、隐藏并不接收命中。关闭 Files 回到该组原来的布局。

侧栏宽度默认 208 px，还要放垂直标签，放不下一棵可用的目录树。单独占一个 pane 会挤掉正在看的终端。分组工具视图已经证明：打开期间会话 ID 和 PTY 都不变。

类型上把 `GitView` 扩成 `"changes" | "history" | "files"` 会让 Git 模块携带文件浏览。快照另加可选字段 `files`，与 `gitViews` 并列。画布是否被挡住改为「Git 视图或 Files 视图有选中」。侧栏标签的开关沿用 `openGitView` / `closeGitView` 的形状，单独叫 `openFiles` / `closeFiles`。

## 数据通路

```text
TSX FilesPanel
  list(root, directory) ── Rust Files（只读）
                           blocking，引擎线程外
                           std::fs::read_dir + symlink_metadata
                           ▼
                    DirectoryListing { entries, truncated, error }

  展开的目录 ── watch / unwatch ── notify kqueue，仅这些目录本身
  变更合并后 ── 重新 list 这一层

  reveal / openWithDefaultApp / trash ── nsh_core::element::{reveal_path, open_path, move_to_trash}
  在此打开终端 ── 现有 Sessions.open(cwd, ...)
```

枚举留在 Rust。JS 只保存当前树需要的一层结果、展开集合和选中路径。不把目录项送进终端帧调度。

读取方式和 Git 只读查询相同：`blocking`、每次视图一个代数、卸载或替换目录时丢掉过期结果。全应用的四个 Git 读取名额不给文件列举使用，文件浏览另计，同时最多两个列举，避免点开大目录时堵住 `git status`。预览模式的 transport 返回夹具，不调用 `read_dir`。

### 列举契约

```rust
struct FileEntry {
    name: String,          // 单层文件名，不含分隔符
    kind: FileKind,        // File | Directory | Symlink | Other
    hidden: bool,          // 名称以 '.' 开头
    bytes: Option<u64>,    // 普通文件的长度；目录和读不到时为 None
}

struct DirectoryListing {
    entries: Vec<FileEntry>,
    truncated: bool,       // 达到 5_000 条
    unreadable: Option<String>,
}
```

规则：

- 参数是根和要列的目录。目录必须已经规范化，并且等于根或位于根之下。否则返回错误，不列举。
- 用 `read_dir` 和 `DirEntry::file_type`（Unix 上不跟随符号链接）。不经过 shell。
- 名称按目录在前、文件在后排序；同一类用 Unicode 小写比较，相同再按原始字节。这和访达的本地化排序不同，结果稳定，也不依赖当前 locale。
- 单层最多 5_000 条，与 Git 变更条目上限同量级。超出时 `truncated` 为真，面板说明只显示前 5_000 条。
- 点文件仍返回，由 UI 按 `showHidden` 过滤，这样切换开关不必再读盘。
- 权限失败时该层 `unreadable` 有说明，其他已展开的层保持上次成功结果。
- 根目录本身在打开视图时列一次。之后只在展开、刷新和监听到变更时列对应的那一层。

符号链接：`file_type` 标成 `Symlink`。展开时才解析目标。目标规范化后仍在根下，并且是目录，才列目标内容；行的 `id` 仍是链接自己的路径，避免同一目标以两条路径进入树。目标跑到根外、断开或不是目录时，不列举，这一层显示「链接不在此文件夹内」或「无法打开链接」。这样浏览不会顺着链接走出分组目录。

### Tree 模型

树的顶层是根目录的直接子项，根路径显示在工具栏，不占一行。

每个目录行的 `id` 是相对根的路径，用 `/` 连接，根自身的 `id` 是空字符串（只出现在状态里，不进 `Tree`）。尚未读取的目录有一个子项：

- `id`: 父路径 + `/\0pending`
- `label`: `Loading…`
- `disabled`: true

路径里不能出现 NUL，所以这个 id 不会和真实文件冲突。`renderItem` 见到该后缀就画占位行。用户展开时 `onExpand` 触发 `list`；结果回来后用真实子项替换占位。新的子目录再次带上自己的占位。读完且过滤后没有子项时，换成一行禁用的「空文件夹」（`/\0empty`），目录因此仍然是 `Tree` 眼中的文件夹，chevron 还在。

侧表 `Map<id, FileEntry>` 只存已经列出的项。图标、大小和是否为链接都从这张表来，不塞进 `TreeItem`。

展开集合最多记住 64 个路径。再展开时丢掉最久未使用的一层，并取消那一层的监听。选中项若落在被丢掉的层下，选择保留路径，滚动到仍可见的最近祖先。

Git 状态用和 Changes 相同的 `git status`。变更路径相对仓库根，再换算到分组目录之内；分组目录不在该仓库里的路径不显示。文件行用 Changes 的状态字母，已暂存时加一个点。目录行在里面还有变更时显示一个点。编辑区标题对当前文件重复这个字母，并写上暂存、未暂存、未跟踪或冲突。磁盘上已经删除、父目录还在的文件补一行。Git 读失败或这里不是仓库时不标状态，文件树照常浏览。预览不调用 Git。

### 监听

复用仓库里已固定的 `notify`（macOS 非递归 kqueue），和 `src/git_discovery/` 同一依赖，单独的 watcher，不占用 Git 发现的 1_024 目录额度。只监听展开集合里的目录本身，外加视图一直展开的根目录，不监听子目录。事件按 200 ms 合并后重新列举变化的那一层。折叠、关闭视图、删除分组时去掉监听。监听失败时工具栏保持可点的刷新，并在该层显示一行说明。第一版不在 Windows / Linux 上接监听；那里只保留手动刷新，和终端 renderer 一样把平台口子留出来。

### 预览

选中普通文件后，Rust 再读内容。上限 512 KiB，与 Git 单文件预览相同。先看是否含 NUL；有则视为二进制，只返回长度和「二进制文件」。否则以 UTF-8 解码，非法序列用替换字符，并标明文件不是合法 UTF-8。超限时不读后续字节，说明只显示开头。

NSH 没有按文件类型分发的预览器。文本高亮用 `nsh/base` 的 `Editor`：`readOnly`、按扩展名或文件名选语言，语法来自 syntect / two-face（bat 的语法集）。Markdown 和 HTML 用同一包的 `TextView`。图片用 NSH 的 `Image`。这三项都要在 `nsh-base` 上打开对应 feature；当前 `Cargo.toml` 是 `default-features = false` 且只开了 `term`，`editor` 和 `highlight` 都没开。PDF、视频、压缩包和 Quick Look 没有现成组件，第一版不做，只显示大小并提供「用默认应用打开」。

第一版把有界文本和文件名交给 `Editor`，`readOnly` 为 false。宿主已经带语法高亮，应用 crate 不必为界面再开 `highlight`。保存把 `Editor` 的当前文本交回 Rust，整文件写回，写前再次做根前缀检查。磁盘上的文件在打开后被别人改过时拒绝覆盖，并说明需要重新加载。选中变化或视图卸载时递增代数，迟到的读盘结果丢弃。未保存的缓冲区只留在该组的内存里，每个组同时只编辑一个文件。

### 动作

| 动作 | 行为 |
|---|---|
| 刷新 | 重新列举根和仍展开的层 |
| 显示隐藏文件 | 只过滤已有结果，并写入该组偏好 |
| 在访达中显示 | `reveal_path`。失败进入通知历史 |
| 用默认应用打开 | `open_path`。失败进入通知历史 |
| 在此打开终端 | 目录调用现有 `Sessions.open`，工作目录为该项；文件则用其所在目录。新标签属于当前分组，走 `addTab` 已有的会话创建，不在文件模块里 spawn |
| 复制路径 | 绝对路径进剪贴板 |
| 移到废纸篓 | 先用现有确认对话框。确认后 `move_to_trash`。成功则刷新父目录。没有永久删除 |

双击目录只展开或折叠。双击文件加载预览。双击不启动终端，避免和选择手势绑在一起。

「在此打开终端」若目录已不存在，`Sessions.open` 的现有失败路径保留标签和错误，文件面板同时刷新父目录。

## 持久化

`SNAPSHOT_VERSION` 保持 1。`files` 缺省表示该组没有打开 Files，与 `gitViews` 的加法字段相同。

```ts
files?: Record<string, {
  open: boolean;
  showHidden: boolean;
  expanded: string[];       // 最多 64 条，相对根
  selected: string | null;  // 相对根
}>
```

不保存列举缓存、文件内容、监听句柄和绝对路径展开状态。分组目录在下次启动时可能已经不同；恢复时丢掉不在新根下的 `expanded` 和 `selected`。运行中的滚动位置可以留在内存，关闭应用不写。

## 界面

工具栏一行：当前根路径（中间省略，tooltip 为全路径）、隐藏文件开关、刷新。根路径来自分组目录。

下面是可调整的两栏，左栏默认 280 pt，最小 200，最大 480。左栏是 `Tree`，行高与标签相同，缩进 `depth * 14`，样式沿用 Changes 树的 `tab-hover` / `tab-selected`。右栏是预览或空状态。

空状态和错误沿用 Git 面板的居中说明：没有分组目录、根不可读、空文件夹、二进制、预览读取失败，各有一句原因和可执行的下一步（刷新或在访达中打开）。截断时在树底部保留一行说明，不做成可选项。

行菜单提供上面的动作。废纸篓是危险色，其余是普通菜单项。

## 安全与隐私

这是用户本机上的桌面应用。风险是误删和符号链接把浏览带到分组目录之外。

- 列举和读预览都先做词法清理和 `canonicalize` 前缀检查。检查用组件边界，避免 `/proj` 匹配 `/proj-other`。
- 符号链接的目标必须落在同一根下才继续列。
- 废纸篓必须经过确认，且只对当前选中的一条路径。不提供清空目录。
- 不把 `open_path` 或任意路径读取暴露成通用脚本接口。TSX 只能调用文件模块上的这几个函数，参数是该视图已经列出的相对路径，Rust 再拼回根。
- 预览不跟随用户目录之外的内容，也不把文件内容写入工作区快照或通知正文。通知只写动作失败的短说明。

## 可观察性

应用没有指标系统。根列举失败和预览失败留在面板内，与 Git 读取失败相同。废纸篓、默认应用和访达失败写入现有通知历史（`src/notifications.rs`），因为这些动作在用户离开面板后仍需要留下结果。监听降级写成该层的一行说明，不每条 kqueue 事件发一条通知。

## 发布与回退

Files 没有独立开关。侧栏不出现标签时，行为与今天相同。快照里的 `files` 是可选的；旧版本读到未知字段时按现有快照解析忽略，新版本读到缺失字段时 Files 全部关闭。回退版本后，多出来的字段留在 JSON 里也无影响，只要解析继续忽略未知键。这一点实现时要对照 `workspace-snapshot.ts` 的现有解析，未知键今天如果会让整份文档失败，就把 `files` 的解析写进允许列表，而不是升级 `SNAPSHOT_VERSION`。

## 验证

- Rust：排序、点文件标记、5_000 截断、根外路径拒绝、逃出根的符号链接、断开的链接、不可读目录。临时目录由测试创建。
- Headless：夹具 transport 覆盖展开占位替换、空目录、选中文件后的预览代数、隐藏文件过滤。预览模式断言不调用真实 `Files.list`。
- `./scripts/nsh check` 与 `./scripts/nsh test`。
- 原生窗口的滚动手感、中文文件名和废纸篓确认另记，类型检查不能代替。

## 风险

| 严重度 | 风险 | 缓解 |
|---|---|---|
| 高 | 占位行让 `Tree` 在大目录展开时多一帧「Loading…」，替换时滚动跳动 | 替换后按 id 保持选中；`Tree` 行按 id 复用。加载行只有一行 |
| 中 | 单层上万条目时 5_000 上限藏住后面的文件 | 明确截断文案。全树搜索不在第一版，避免为了搜索把目录递归读完 |
| 中 | 展开很多层后 kqueue 数量上去 | 展开最多 64，折叠即取消 |
| 中 | `canonicalize` 会碰到不存在的竞态文件 | 列举用 `symlink_metadata`；前缀检查失败就显示不可用并允许刷新 |
| 低 | 分组目录被外部改名 | 根列举失败时面板给出原因，终端会话不受影响 |

## 备选方案

**系统打开面板当作浏览器。** `NSOpenPanel` 已经能选文件和文件夹。它离开应用窗口，不能和终端、预览、分组目录放在一起，也不能记住展开状态。只保留为「给分组另选一个文件夹」的入口，第一版连这个入口也不做，分组目录仍由现有的新建分组对话框决定。

**应用自己用 `List` 画扁平行。** 行模型可以自带 `folder` 位，空目录不必放占位。键盘、类型提前查找、展开动画和屏幕阅读器要自己做，`Tree` 已经实现了 WAI-ARIA tree。目录树的交互比「少一个占位行」更值钱，所以用 `Tree` 加占位。

**再封装一个虚拟列表给 Git 和 Files 共用。** `List` 已经是虚拟列表：`each`、`keyExtractor`、只创建视口附近的行。History 的提交和仓库选择器各自把行渲染传进去。Files 的层级、展开和键盘属于 `Tree`，Git diff 属于 `DiffView`。抽一个包装组件只会挡住这些 props，两边的行内容仍然不同。行的颜色、高度和图标沿用现有主题类，不另做列表引擎。

**放进侧栏或单独 pane。** 侧栏太窄，而且和垂直标签抢高度。pane 会改变分屏几何。分组工具视图在打开期间不销毁会话，和 Changes 一致。

**递归监听整棵项目树。** `git_discovery` 的额度是为仓库根和一层子目录准备的。文件浏览的展开集合更散，递归会把 `node_modules` 算进去。只监听展开的目录。

**把 Yazi 嵌进面板。** Yazi 是完整的终端文件管理器，界面由它自己的 TUI 画到终端上。`yazi-core` 在 crates.io 上写明不供外部使用，API 无稳定性保证，并带上 Tokio、Lua、自己的终端适配和另一套 `notify`。Termlevel 的界面属于 NSH，终端字节属于 Ghostty；再嵌一套 TUI 会有两套输入、预览和进程模型。目录列举本身用 `std::fs::read_dir` 就够，不值得为这一层依赖整个 Yazi。若用户机器上有 `yazi`，以后可以加一个动作：在当前目录用现有 `Sessions.open` 开一个终端再运行它。那是一条会话，不是 Files 面板。第一版不做这个动作。`Sessions.open` 目前只启动登录 shell，没有初始命令参数。

## 待决问题

1. 分组目录之外的「打开文件夹」要不要进第一版？本文把它留在现有新建分组流程里。若希望在 Files 里临时看另一个目录而不新建分组，需要另加根路径，以及快照里存不存这个根。
2. 废纸篓是否第一版就做？它是唯一会改磁盘的动作。可以只做浏览、预览、访达和新建终端，把废纸篓放到下一张 PR。

## Key Decisions

1. **用 `Tree` 加占位行，不用系统文件面板当浏览器。** NSH 没有目录组件；`Tree` 已虚拟化并带键盘语义。占位行补上「无子节点就不是文件夹」的模型缺口。
2. **Files 是分组工具视图，根是分组目录。** 与 Changes / History 共用画布切换，终端进程不随视图开关创建或销毁。
3. **Rust 按层列举，符号链接不得离开根。** JS 不持有完整文件系统，也不把列举放进 Git 的四个读取名额。
4. **文本编辑用 `Editor`，虚拟列表不重写。** 目录用 `Tree`，提交列表继续用 `List`，diff 继续用 `DiffView`。文件视图负责列举、打开、脏状态、保存，以及和 Changes 相同的 Git 状态字母。单文件 512 KiB，每个组同时编辑一个文件。
5. **展开层才监听，上限 64。** 避免对主目录或 `node_modules` 做递归监视。
6. **快照只记打开状态、隐藏文件、最多 64 个展开路径和选中路径。** schema 仍为版本 1 的可选字段。
7. **改磁盘的动作只有确认后的废纸篓，以及用户明确要求的新终端。** 新终端走现有 `Sessions.open`。

## PR Plan

### PR 1 · Rust 单层目录列举

- 标题：`Add bounded directory listings for the file browser`
- 影响：`src/files/`（`mod.rs`、列举与路径检查分开，单文件不超过 500 行）、`src/lib.rs` 注册导出、Rust 测试
- 依赖：无
- 内容：`list(root, directory)`、排序、隐藏标记、5_000 截断、根前缀、符号链接越界、不可读目录。预览模式不调用。不含 UI。

### PR 2 · 夹具驱动的只读树

- 标题：`Show a lazy file tree with fixture data`
- 影响：`ui/files/`（`FilesPanel`、类型、夹具 transport）、`tests/workspace/` 或 `tests/files/`
- 依赖：PR 1 的类型形状；测试可以先完全用夹具，原生导出只在接线 PR 使用
- 内容：占位行、展开后替换、空目录、隐藏文件过滤、截断说明。headless 不读磁盘。

### PR 3 · 接入分组视图与快照

- 标题：`Open the file browser as a group view`
- 影响：`ui/workspace/state/`、`workspace-snapshot.ts`、`Sidebar.tsx`、`TilingCanvas.tsx`、`WorkspaceApp.tsx` 标题、`ui/files/native-files.ts`
- 依赖：PR 1、PR 2
- 内容：侧栏 Files、画布切换、可选 `files` 字段、恢复时丢弃根外路径。打开和关闭不改变 session ID。真实列举只在非预览构建调用。

### PR 4 · 有界文本预览

- 标题：`Preview a selected file with a size cap`
- 影响：`src/files/` 读文件、`ui/files` 右栏
- 依赖：PR 3
- 内容：512 KiB、二进制判断、UTF-8 替换、代数取消。不加入编辑。

### PR 5 · 访达、默认应用、新终端与废纸篓

- 标题：`Add explicit file actions`
- 影响：`src/files/` 或 `src/runtime.rs` 对 `reveal_path` / `open_path` / `move_to_trash` 的薄封装、Files 菜单、确认对话框、通知
- 依赖：PR 3。废纸篓可按待决问题 2 拆成后续 PR，其余动作仍可独立合并
- 内容：动作失败进通知。新终端只调用现有会话 API。

### PR 6 · 展开目录监听

- 标题：`Refresh expanded folders from directory watches`
- 影响：`src/files/` watcher、面板刷新
- 依赖：PR 3
- 内容：非递归 kqueue、200 ms 合并、最多 64 个目录、失败时保留手动刷新。与 Git 发现的监听额度分开。

## References

- `nsh-base-0.5.0/src/tree.rs`：`TreeItem`、`TreeEntry`、`TreeEvent::Action`
- `nsh-core-0.5.0/src/file_dialog.rs`、`src/element/ambient.rs`：`pick_paths`、`open_path`、`reveal_path`、`move_to_trash`
- `src/runtime.rs`：`Runtime::pick_folder`
- `src/sessions/api.rs`：`Sessions::open`
- `ui/git/GitPanel.tsx`：`Tree` 的行样式与分栏
- `ui/workspace/state/index.ts`：`openGitView`、`canvasBlocked`、`addTab`
- `src/git_discovery/`：非递归目录监听的现有边界
