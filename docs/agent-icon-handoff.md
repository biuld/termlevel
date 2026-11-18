# OSC 7501 Agent 图标交接

## 目标与范围

用户希望在侧边栏一眼看出哪个 tab 运行着哪个常用 agent：当 agent 处于 idle 等中性状态时，以 agent 图标代替默认 terminal / pin 图标。此文是后续实现的交接，不代表功能已经实现。交接基于 `a96c125`，开始前重新检查 checkout、工作区和项目指令。

## 已有基础

- [OSC 集成与终端状态设计](osc-support.md) 描述现有协议、聚合、确认和生命周期；先读 [开发指南](development.md)。如调整所有权边界，还要读架构和路线图。
- Rust 在 `src/sessions.rs` 保存 7501 记录并聚合到 `SessionInfo.activity`，包括 `app`；idle 可参与聚合，已确认的 done/error 不参与聚合但记录仍保留。每次报告整体替换同 id 的记录，app 可以从祖先继承。
- `ui/workspace/Sidebar.tsx` 的状态图标目前在 idle / 无报告时回退到 terminal 或 pin。`TerminalSurface` 和 workspace 状态负责传递会话摘要；检查当前代码后再决定新增字段与更新路径。
- OSC 133 新 prompt 当前只保留结果记录，实际也会清除 idle；clear 删除对应子树，reset 清全部，进程退出有独立清理。没有 shell integration 时不能假设一定收到 prompt。

## 建议实现方向

1. 从程序显式报告的 `app` 映射已验证的 agent，先核实各 agent 具体版本、真实输出和图标资源许可；`codex` 与 `claude-code` 是值得优先验证的候选。未知或缺失 app 回退原有图标，不从标题、输出速度或静默时间推断。
2. 将 agent 身份与用于展示的聚合活动状态区分：确认 done/error 后聚合可能为空，但底层结果记录仍在。设计清楚身份来源、记录替换与多记录冲突，不要简单缓存“最后见过的 app”并无限保留。协议 app 是自报信息，不是经过验证的进程身份。
3. working、blocked、未读 done/error、宿主故障和退出状态保留既有视觉优先级；中性状态才显示 agent 图标。确认结果后只有在仍有足够证据证明 agent 身份有效时才回退到其图标。tooltip / 无障碍应同时表达 agent 身份和状态，收起侧栏、固定标签及深浅主题也要可辨认。
4. 仔细设计 clear、同 id 替换、祖先继承、prompt、reset、退出、关闭、重启和 UI 热重载。尤其是 agent 退出后结果记录可以跨 prompt 保留：不能把已确认的旧 done/error 永久当成活跃 agent；同时也不能把 agent 内部子命令误判为 agent 退出。对于没有可靠结束信号的情况，应明确保守退化和限制，而不是猜测性超时。
5. 遵守现有 Rust 持有运行期会话状态、TSX 持有低频 UI 元数据的边界，不将运行期身份持久化。保持 7501 聚合、通知、确认版本和进度规则不变；如确需调整规则，先更新设计并说明理由。

## 验证与待确认问题

- 使用真实 PTY 的受控协议序列覆盖已知 / 未知 app、idle、状态切换、确认、多个记录和 app 继承、clear 子树、prompt、reset、退出和 UI 重挂。验证现有通知、状态优先级及进度未回归。
- 按开发指南运行适当的 `./scripts/nsh check`、`./scripts/nsh test`、`./scripts/nsh build`，并记录结果和现有失败。针对视觉变化做原生窗口验收，分别记录自动化与人工结果；检查固定 / 普通 tab、侧栏展开 / 收起、主题、tooltip 和无障碍。
- 在真实 Codex / Claude Code 中分别记录版本和实际序列，观察启动、空闲、工作、确认、退出；必要时单独记录 tmux / SSH 的转发结果。受控脚本和 preview 不能替代真实 agent 验收。
- 图标需核实来源、分发许可及小尺寸表现。若无法可靠判断 agent 是否仍运行，应向用户明确限制，再讨论是否引入 hooks 或其他识别方式，不默认安装或修改用户配置。

协议背景：[Program Status Protocol](https://www.superlogical.com/rex/docs/build/program-status)；已报告的实际行为示例：[Codex](https://github.com/openai/codex/issues/60296)、[Claude Code](https://github.com/anthropics/claude-code/issues/104195)。这些示例不能替代本项目的真实版本验收。
