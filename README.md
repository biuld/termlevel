# Termlevel

用垂直标签页和分组组织终端会话。UI 参考新版 Firefox，使用 NSH 的原生桌面后端，以 TSX 编写。

当前为 **M1 交互原型**：使用 NSH 默认 `Terminal`，支持标签、分组和演示输入。尚未连接真实 shell；`echo`、`help`、`pwd`、`colors`、`clear` 是预览命令，不会启动系统进程。重启后预览数据重置。

## 运行

需要 Rust 1.97+ 和平台构建工具；macOS 首先支持 macOS 14+，安装 Xcode Command Line Tools。项目固定使用 NSH **0.5.0**。

```sh
./scripts/bootstrap                         # 安装到本项目 .tools/，并验证 SHA-256
./scripts/nsh run --window-size 1200x780     # 默认 macOS AppKit，TSX 热更新
```

如果 `.tools/nsh` 已存在且完整，可以直接运行第二条命令。也可自行安装全局 nsh 0.5.0，脚本会在没有项目内版本时使用它。第一次运行 mixed 项目需要下载并编译 Rust 依赖；开发时不需要手动安装 Node 或 npm UI 框架。

## 原型功能

- 侧栏贯顶布局：可调整宽度、可收起的单行垂直标签，右侧终端直接铺满。
- 创建、关闭、重命名、固定终端；右键菜单排序及跨组移动。
- 分组创建、折叠、重命名、颜色、取消分组与关闭组确认。
- Firefox 风格分组编辑浮层：右键组头或点更多，直接改名称、选择九种颜色；新建组直接进入编辑。
- 查找终端、字体大小调整、系统菜单与快捷键。
- 中性灰主题；设置中可切换浅色、深色和跟随系统。
- 每个终端保留独立输出与输入历史，切换或折叠组不重建实例。

| 操作 | macOS 快捷键 |
|---|---|
| 新建终端 | ⌘T |
| 关闭当前终端 | ⌘W |
| 新建分组 | ⌘⇧G |
| 查找终端 | ⌘K |
| 收起 / 展开侧栏 | ⌘B |
| 切换下一个 / 上一个终端 | Ctrl+Tab / Ctrl+Shift+Tab |
| 切换前九个终端 | ⌘1 … ⌘9 |
| 设置 | ⌘, |

Windows / Linux 的应用快捷键使用 Ctrl，平台验收属于后续工作。拖动标签尚未实现，当前通过菜单移动。

## 检查

```sh
./scripts/nsh check     # Rust + TypeScript 类型检查
./scripts/nsh test      # 原生 crate 检查 + headless 交互测试
./scripts/nsh build     # 生成 QuickJS bundle
```

`nsh check` 默认下载 TypeScript 原生编译器；已有 TypeScript 时可用 `--tsc /path/to/tsc`。网络受限且依赖已缓存时可设置 `CARGO_NET_OFFLINE=true`。验证记录见 [docs/validation.md](docs/validation.md)。headless 测试不能替代实机输入法、选区、拖动和视觉验收。

## 结构与后续

```text
App.tsx                       窗口、菜单、快捷键、主界面
ui/                           分组侧栏、控件、工作区状态
terminal/                     默认 Terminal 与独立演示 transport
src/lib.rs                    Rust 原生模块入口
scripts/                      固定版本工具安装及启动
```

UI 始终以 TSX 编写。真实 PTY 和 shell 生命周期将在 Rust 中实现，再接入 `libghostty-vt`；逐字节输出和高频绘制留在 Rust。详见 [架构](docs/architecture.md) 与 [Roadmap](docs/roadmap.md)。

修改视觉从 [theme.json](theme.json) 开始，token 分工及使用规则见 [UI 与主题](docs/design.md)。组件不直接保存配色，分组颜色也由当前主题解析。
