# 开发指南

## 文档入口

README 负责安装、运行和产品能力；本指南负责目录与开发约定；architecture 负责当前所有权与数据通路；roadmap 负责尚待完成的工作；design 负责视觉规范。terminal-grid、terminal-surface、terminal-tiling 记录终端展示设计；osc-support 统一记录 OSC 能力、终端状态的业务语义与 UI / UX。

validation 按日期保留验证证据，不作为当前模块路径索引。带 handoff 的文件是历史交接快照，不覆盖当前架构。移动文件时更新当前文档与代码引用；历史命令、日志路径和提交信息保留原样。

## 目录与依赖

```text
App.tsx                     NSH 默认挂载入口
ui/app/                     启动恢复、窗口、菜单、快捷键和应用接线
ui/workspace/               工作区状态、快照、存储、侧栏、分组和平铺
ui/git/                     Git 变更 / diff、History、分支 / worktree 与提交车道
ui/settings/                设置界面
ui/notifications/           通知历史订阅与标题栏面板
ui/terminal/                原生终端展示和查找
ui/shared/                  通用控件、图标、字体、文字和主题映射
src/                        Rust 应用 crate
  git_discovery.rs          可取消的后台仓库扫描
  git.rs                    只读 Git 查询：status、refs、worktree、log、commit、diff
  git_ops/                  显式 Git 操作：fetch、fast-forward pull、push、checkout、建分支、worktree 管理
  sessions/                 显式会话 API 与进程生命周期测试
  terminal/                 Ghostty 应用适配测试
  session_view/             显示控制、输入策略、场景与平台后端
crates/ghostty-vt/           独立安全封装
crates/ghostty-vt-sys/       官方 C ABI、头文件、绑定和预编译库构建
tests/                     测试代码，详见下文
scripts/                    固定版本工具与验证入口
assets/                     提交的产品资源
artifacts/                  忽略的本地日志、截图与报告
```

TSX 的应用接线位于 app；工作区状态与操作位于 workspace；终端视图位于 terminal；无业务状态的公共表现放 shared。组件用 PascalCase，状态与工具文件用小写及连字符。沿用当前两空格 TSX 缩进，避免目录整理时混入整文件格式重写。

app 可以组合 workspace、settings、notifications、terminal 和 shared。workspace 的状态模块不导入 app 或 terminal 展示组件；TilingCanvas 是工作区到终端展示的组合点。terminal 可消费 workspace 状态接口，但不创建应用窗口或接管进程生命周期。shared 不导入上述业务模块。当前 workspace 状态使用 NSH signals 和原生服务，不宣称是纯函数领域层；tiling 的树变换和 snapshot 的解析各自保留独立入口。

Rust 的 lib.rs 只做导出注册。SessionManager 拥有 PTY、进程与工作线程；terminal 负责 Ghostty 到应用的适配；session_view 负责显示与交互；平台对象只放 platform 下。两个 Ghostty crate 不依赖 NSH、PTY 或应用。跨线程传 owned 数据，句柄同步和回调约束见 crates/ghostty-vt/THREADING.md。

新增模块按职责归属；共享机制应有唯一所有者。源代码文件最多 500 行，超过时按职责拆分：Rust 使用目录和 `mod.rs`，TypeScript / TSX 使用目录和 `index.ts`；保持公开入口清晰，不为每个组件增加 crate。

## 测试与验证

- `tests/workspace/`：工作区交互、布局、持久化及其 fixtures。
- `tests/terminal/`：内核绑定、真实会话、查找及其 fixtures。
- `tests/native/*.acceptance.tsx`：需要前台 AppKit 窗口的验收，不进入默认 headless 发现。
- `tests/probes/`：验收启动的 Python 协议与负载探针，不属于产品运行依赖。
- Rust 单元测试跟随模块，独立 crate 的集成测试留在各 crate 的 tests 下。

测试文件命名为 `*.test.tsx`，fixture 不使用此后缀。NSH 的 `open()` 相对测试模块解析。原生 acceptance 经 scripts/test-native 临时复制到仓库根目录运行，所以其 open 路径以根目录为基准；Python 命令同样以仓库根目录为工作目录。临时文件退出时删除；fixture 邻近的 .nsh 缓存不得提交。

常规验证：

```sh
./scripts/nsh check
./scripts/nsh test --junit artifacts/headless-tests.xml
./scripts/nsh build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

主题修改先 build 再 check，以更新生成的 token 类型。仅改文档可检查链接及 diff；目录移动必须验证类型、测试发现和构建。修复应验证用户行为或所有权约束，不写只复述实现的测试。

原生验收按涉及能力选择，串行运行，避免窗口焦点与共享 Cargo target 互相干扰：

```sh
./scripts/test-native-input
./scripts/test-native-scroll
./scripts/test-native selection
./scripts/test-native-tiling
./scripts/test-native-protocols
./scripts/test-native-ghostty
./scripts/test-native status
./scripts/test-native notifications
./scripts/test-native osc
./scripts/test-native gitlive
./scripts/test-native history
./scripts/test-native settings
```

这些入口保持兼容，也可用 `./scripts/test-native <类别>`。完整 headless 发现不得与 scripts/test-native 并行：后者在仓库根目录生成临时 *.test.tsx，会被前者误发现。原生测试需要前台窗口，不能由类型检查或 headless 成功替代。剪贴板 helper 的编码与管道由 Rust 单元测试验证；系统剪贴板 Unicode 和 OSC 52 使用 `./scripts/test-native protocols --filter 'OSC52 preserves Chinese'` 验证，该原生用例保存并恢复剪贴板。系统输入法候选窗、拖动手感、选区和长期负载的人工验收另记。

验证记录注明 revision 或未提交状态、命令、结果、日志以及未覆盖项。预览、真实 PTY、headless、原生自动化、人工验收分别说明。遇到已有失败先保留具体失败与历史基线，不通过改断言或超时来掩盖。持久化提交曾记录 126/128；本次目录整理复验为 128/128。历史失败仍保留在 validation 对应章节，不推断间歇性问题已根治。

## 工具与生成文件

NSH 统一经 scripts/nsh 调用；scripts/bootstrap 安装固定版本并验证校验和。Rust 版本要求见 README。日常 Ghostty 构建使用固定校验和的官方预编译库；离线覆盖和绑定再生成流程分别见两个 crate 的 README。

Cargo.lock 随代码提交。theme.json 和 assets 是源文件；.nsh、.tools、target、node_modules、artifacts 是本地生成物。不要手改生成类型或在源码目录保存截图与日志。
