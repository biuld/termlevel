# Architecture

## 当前边界

```text
App.tsx                         原生窗口、菜单、快捷键、对话框
  ui/Sidebar.tsx                vertical tabs 与 tab groups
  ui/GroupEditor.tsx            锚定组头的名称、颜色与组操作浮层
  ui/workspace.ts               标签与分组状态，稳定的会话 ID
  ui/theme.ts                   分组颜色标识到主题 token 的映射
  terminal/TerminalSurface.tsx   NSH 默认 Terminal 的显示与焦点
    preview-session.ts          确定性演示输入输出，不执行 shell

src/lib.rs                      termlevel/native，版本信息与对话框键盘作用域
```

采用 NSH mixed 模板，TSX 在 QuickJS 执行，组件经 NSH Rust 引擎由平台绘制。`Runtime.version()` 是真实的 TSX → Rust 绑定；不是网络 RPC。原生类型声明由 NSH 生成到 `.nsh/types`，不手工维护。

默认 Terminal 的原生按键绑定优先于 JS 绑定。打开对话框时挂载轻量的 Rust `DialogKeys`，在原生层取得 Escape 的优先级，将 Dismiss 事件交给 TSX 关闭对话框；对话框卸载时释放绑定。界面和对话框状态仍在 TSX。

分组编辑状态由 App 持有。GroupEditor 使用 NSH `Overlay` 锚定组头的 `ViewRef`，不通过手工屏幕坐标定位；名称与颜色即时写入工作区，浮层关闭只结束编辑。它复用原生 Escape 绑定，并在选色后将焦点交回名称输入框。

侧栏固定展开状态由工作区持有；悬停状态与分栏测量宽度由 App 持有。唯一的 Sidebar 实例在窗口根层绝对定位，避免被 44 pt 原生分栏裁剪。第一个 ResizablePanel 内保留铺满面板的测量 View，`onLayout` 将拖动期间的实际宽度同步给固定展开态的 Sidebar；`onResize` 只在拖动结束时保存最终宽度，不驱动实时布局。收起时面板保持 44 pt，鼠标移入将 Sidebar 临时展开到已保存宽度并覆盖终端，移出收回，不改变终端分配宽度或挂载位置。分组编辑期间保持展开，允许指针进入侧栏外的编辑浮层；手动切换侧栏会清除临时悬停状态。

## 生命周期

标签 ID 是终端显示实例的身份；标题、排序、分组和选中状态都不是身份。`TerminalSurface` 在工作区层按 ID 挂载，不嵌在组的折叠条件中。未选中的终端保留布局与实例，但隐藏、禁用命中并从辅助功能树中移除。删除标签才卸载组件。

预览阶段会话与组件同寿命。M2 的真实会话由 Rust `SessionManager` 持有，UI 视图可挂载 / 卸载，关闭会话必须调用独立的显式操作。生产会话不能继承预览 transport 的生命周期。M2 还需定义热更新后的重新绑定与事件补发。

## 后续原生接口

Rust 负责 PTY、进程、终端解析、输入编码和高频数据；TSX 负责应用导航及用户操作。终端视图应通过 session ID 绑定 Rust 会话，向 TSX 报告标题、状态、cwd、退出等低频事件。输出与解析、绘制之间避免每个 chunk 都经过 JS。

先在 NSH 默认 Terminal 上验证 PTY，再引入 Ghostty。默认 Terminal 的 emulator 适配接口不代表完整的键盘、鼠标、图形和 renderer 接口；替换内核前必须确认其表达能力。

## 目前的限制

- 只有模拟命令；没有 PTY、SSH 或真实 shell。
- 重启重置全部预览数据；支持浅色、深色和跟随系统。
- 菜单支持排序 / 跨组移动；拖拽尚未实现。
- 实机输入法、选区和性能需要原生窗口验收。

## 视觉主题

配色、字体、主要尺寸和圆角集中在 `theme.json`；组件用语义类和 `token()` 引用。固定侧栏、收起图标栏、悬停侧栏和编辑浮层统一使用实色主题背景；浮层通过细边框与轻阴影表达层级，工具栏控件直接放置，不使用玻璃或背景模糊。主题切换不拥有终端生命周期。分组数据保存颜色标识，具体色值按当前外观解析。设计说明见 [design.md](design.md)。
