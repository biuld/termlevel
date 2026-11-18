# Termlevel

用垂直标签页和分组组织终端会话。UI 参考新版 Firefox，使用 NSH 的原生桌面后端，以 TSX 编写。

当前默认连接 **真实 PTY 和用户 shell**。Rust 使用官方 `libghostty-vt` 1.0.1 的预编译静态库解析输出、维护网格和滚动历史，Rust 原生网格视图负责显示，NSH 原生 Input 负责文字与输入法组合。会话独立于 UI 生命周期；切换标签、分组、主题及重新挂载不会终止进程。

关闭终端会结束它的前台进程与 shell；关闭主窗口或退出应用会结束全部会话。工作区结构与设置通过 NSH Storage 持久化；应用重启会为保存的标签重新启动 shell，不恢复旧进程或输出。首次启动显示空白引导页，可直接打开 home 终端或新建分组。尚不声称 Neovim/tmux 完整兼容。
## 运行

需要 Rust 1.97+ 和平台构建工具；macOS 首先支持 macOS 14+，安装 Xcode Command Line Tools。项目固定使用 NSH **0.6.0**。

```sh
./scripts/bootstrap                         # 安装到本项目 .tools/，并验证 SHA-256
./scripts/nsh run --window-size 1200x780     # 默认 macOS AppKit，TSX 热更新
```

如果 `.tools/nsh` 已存在且完整，可以直接运行第二条命令。也可自行安装全局 nsh 0.6.0，脚本会在没有项目内版本时使用它。第一次运行 mixed 项目需要下载并编译 Rust 依赖；开发时不需要手动安装 Node 或 npm UI 框架。

## 工作区功能

- 独立 pane 布局：统一窗口栏，可调整宽度、可收起的单行垂直标签，终端使用带间距与圆角的面板。
- 创建、关闭、重命名终端；固定终端在所属分组或 Ungrouped 内置顶，右键菜单支持排序及跨组移动。
- 分组创建、折叠、重命名、颜色、取消分组与关闭组确认。
- 侧栏的 Ungrouped 入口切换到未分组工作区；标题栏 `+` 在当前工作区创建 terminal，提示文字显示创建目标。标题栏 Git 菜单按当前分组打开 Changes 或 History tab；每组每种视图最多一个，关闭不会结束终端。打开状态与选中视图随工作区恢复。组头右键提供组设置和操作。Git 数据由系统 `git` CLI 提供，需要 Git 2.20+。
- Changes 展示变更文件树和单文件 diff。History 按分支 scope 显示提交车道、提交详情；变更文件列表可点击，右侧只显示选中文件的 patch，默认第一个文件。工具栏可 Fetch 或 fast-forward Pull 当前分支；左栏管理 Local / Remote / Tags 与 worktree，可 checkout、新建 / 删除分支、新建 worktree、把 worktree 打开为新分组、删除或锁定 worktree。合并、rebase 与冲突解决尚未接入。
- Firefox 风格分组编辑浮层：右键组头或点更多，直接改名称、选择九种颜色；新建组直接进入编辑。
- 查找终端、字体大小调整、系统菜单与快捷键。
- 内置 JetBrains Mono（支持编程连字）和更纱终端黑体 SC，中文与常用符号使用明确的回退链；无需额外安装字体，已有字体选择会保留。
- 多终端定向分屏、方向焦点与位置交换；拖动侧栏标签或 pane 标题安排已有会话。
- pane 缩放、键盘调整、均分与临时放大；收起 pane 保留进程，关闭 terminal 才结束会话。
- 窗口过窄时临时展示焦点 terminal，空间恢复后还原布局。
- 中性灰主题；设置中可切换浅色、深色和跟随系统。
- 每个终端保留独立 Rust 会话与输出，切换或折叠组保留 session ID；原生视图重新挂载可重新绑定。
- 标签组、标签顺序、固定状态、分屏布局、选中标签、侧栏宽度、字号、滚动历史、字体和外观持久化；首次启动不自动创建 shell。
- 本地图片和文件可从 Finder 拖入终端 pane：插入转义后的路径，支持多个文件，不自动回车；远程图片或无文件 URL 的图片数据暂不支持。
- 真实 shell、中文文本输入与粘贴、resize、Ctrl+C、标题与响铃。Ctrl+D 或退出码 0 会关掉终端；非 0 退出和启动失败留在 pane 里。

| 操作 | macOS 快捷键 |
|---|---|
| 新建终端 | ⌘T |
| 关闭当前终端 | ⌘W |
| 向右 / 向下分屏 | ⌘D / ⌘⇧D |
| 聚焦相邻 pane | ⌘⌥方向键 |
| 与相邻 pane 交换会话 | ⌘⌥⇧方向键 |
| 放大 / 还原 pane | ⌘⇧Enter |
| 新建分组 | ⌘⇧G |
| 查找终端 | ⌘K |
| 在终端内容中查找 | ⌘F |
| 收起 / 展开侧栏 | ⌘B |
| 切换下一个 / 上一个终端 | Ctrl+Tab / Ctrl+Shift+Tab |
| 切换前九个终端 | ⌘1 … ⌘9 |
| 设置 | ⌘, |

Windows / Linux 的应用快捷键使用 Ctrl，平台验收属于后续工作。AppKit 组合输入期间按键优先交给输入法。平铺的实现和验收边界见 [多终端平铺](docs/terminal-tiling.md)。

## 打包

macOS 应用包使用 `nsh.json` 中配置的 [图标 PNG](assets/brand/termlevel-icon-v1.png)，由 NSH 自动生成原生图标资源：

```sh
./scripts/nsh package --target macos
```

默认输出到 `.nsh/out`；也可用 `--out` 指定输出目录。macOS 包默认使用 ad hoc 签名，分发签名与公证另行配置。

## 检查

```sh
./scripts/nsh check     # Rust + TypeScript 类型检查
./scripts/nsh test      # 原生 crate 检查 + headless 交互测试
./scripts/nsh build     # 生成 QuickJS bundle
```

`nsh check` 默认下载 TypeScript 原生编译器；已有 TypeScript 时可用 `--tsc /path/to/tsc`。网络受限且依赖已缓存时可设置 `CARGO_NET_OFFLINE=true`。验证记录见 [docs/validation.md](docs/validation.md)。headless 测试不能替代实机输入法、选区、拖动和视觉验收。

开发约定与目录归属见 [开发指南](docs/development.md)。

## 结构与后续

默认后端使用独立的 [libghostty-vt Rust 封装](crates/ghostty-vt/README.md)：sys 层固定官方 1.0.1 C ABI 和经过 SHA256 校验的 macOS 预编译静态库，安全层负责终端状态、渲染快照和输入编码；日常构建不需要 Zig。第一次构建会下载预编译库，离线时可通过 `GHOSTTY_VT_ARCHIVE` 指定官方 zip。独立验证命令：`cargo test -p termlevel-ghostty-vt-sys -p termlevel-ghostty-vt`。

```text
App.tsx                                      薄的 TSX 挂载入口
ui/app/WorkspaceApp.tsx                      窗口、菜单、快捷键、对话框及应用接线
ui/                                          按 app、workspace、settings、terminal、shared 分工
ui/terminal/TerminalSurface.tsx              真实原生终端的展示与低频状态
ui/git/                                      Git 变更、历史和操作界面
tests/terminal/*.test.tsx                    内核、真实 PTY 和工作区重新绑定测试
src/lib.rs                                   Rust 原生导出注册
src/sessions.rs                              独立会话管理、PTY 与进程生命周期
src/session_view.rs                          Rust 网格视图、光标锚定原生 Input 与组合输入
src/terminal.rs                              Ghostty RenderState 到 NSH 的网格适配
scripts/                                     固定版本工具安装及启动
```

UI 始终以 TSX 编写；真实终端的逐字节输出、解析、输入和高频刷新在 Rust 内完成。TSX 只处理显式会话操作与低频元数据。详见 [架构](docs/architecture.md) 与 [Roadmap](docs/roadmap.md)。

NSH 当前视图的鼠标报告、完整功能键/修饰键、现代键盘协议、光标形状等能力仍需补齐；组合输入已有 AppKit 自动化验证；实际系统候选列表、复杂 Unicode、选区和 GUI 性能仍需人工验收。物理像素尺寸未知，像素尺寸查询返回 0。

## 确定性 UI 预览

```sh
TERMLEVEL_PREVIEW=1 ./scripts/nsh run --window-size 1200x780
```

此模式显示确定性的工作区界面，但 Git 和终端会话不可用；**不启动 shell、不执行系统命令、不访问真实工作区存储**。测试夹具可注入模拟数据，预览测试不能作为真实 PTY 证据。

修改视觉从 [theme.json](theme.json) 开始，token 分工及使用规则见 [UI 与主题](docs/design.md)。组件不直接保存配色，分组颜色也由当前主题解析。

## 许可证

Termlevel 应用使用 [Apache License 2.0](LICENSE)。独立的 `termlevel-ghostty-vt` 和 `termlevel-ghostty-vt-sys` 封装保留各自声明的 MIT 许可证。内置的第三方代码、字体和品牌资源的来源与许可见各自目录中的说明。
