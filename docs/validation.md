# Validation

记录：2026-11-11 初始化，2026-11-12 更新侧栏贯顶布局。环境：macOS arm64，Rust / Cargo 1.99.0，NSH 0.5.0 (`f4d68f7`)。

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
