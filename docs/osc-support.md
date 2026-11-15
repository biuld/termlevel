# OSC 集成与终端状态设计

日期：2026-11-15。本文合并原 terminal-status 文档，统一说明 OSC 能力、业务语义和 UI / UX。协议盘点和状态模型依据当前工作树；状态重设计已落地代码，自动化与人工验收分别见文末及 validation。

OSC integration 是范围，status 是其中一个用户体验：OSC 0/2 提供标题，7 提供目录，9/777 提供通知，9;4 提供进度，7501 提供程序状态，133 提供 shell 生命周期。它们共享解析和会话通路，但不能混成一个 status 字段。进程退出和宿主 I/O 故障则来自 SessionManager，属于终端状态展示的另一类输入。

目录和所有权见 [架构](architecture.md)，主题规范见 [UI 与主题](design.md)，已有验证见 [验证记录](validation.md)。本文不新增 web UI、不更换 NSH 或终端库、不改变 PTY 生命周期。

## 设计结论

- 保留 28 pt 标签行：左侧一个状态图标，中间标题，下方可选真实进度线，右侧沿用布局标记 / 关闭按钮。收起侧栏保留同一个图标和短进度线。
- 图标回答“这个终端现在怎么样”；进度线回答“任务推进多少”；未读记录回答“哪些结果还没看过”。三者分别建模。
- 用户查看不能替程序解除 blocked。查看只确认已有结果和提醒，不删除程序事实。
- blocked / error 不再伪造满格进度；100% 不自动变 done；shell 在运行不等于任务在 working。
- tooltip、无障碍标签和画面使用同一份展示摘要，避免图上 42%、说明里 40%。

## 本次实现修正

| 代码 | 已实现行为 |
|---|---|
| `src/sessions.rs::aggregate` | blocked > 未读 error > working > 未读 done > idle；同级取最新；携带代表 id、状态代次、已读与其余待处理数量 |
| `Session::acknowledge`、`workspace.ts::select` | 只确认 observedRevision 以内的状态代次；保留全部协议记录，blocked 查看后继续显示 |
| `resolve_status`、`TerminalStatusInfo` | Rust 统一会话故障、活动和进度来源，Sidebar、tooltip、无障碍消费相同摘要 |
| `Sidebar.tsx` | 真实数值才有确定进度条；未知暂停 / 错误只有静态轨道；blocked/error 不再画伪满格线 |
| `Session::apply_program_status`、`notifications::record_program` | 按记录级状态跃迁通知，附带记录身份和代次；同文案的不同任务不会合并为同一个事件 |
| `Session::apply_semantic_prompt` | 已知 shell_phase 与 unknown 区分；新 prompt 清掉旧 working/blocked 与上一轮 9;4 进度 |
| `TerminalSurface.tsx` | 转发有界摘要；干净退出继续自动关闭标签，通知历史独立保留 |

当前百分比变化经 Rust 聚合、既有 16 ms 轮询发给 TSX；隐藏 surface 的抓帧暂停不阻断此处的元数据轮询。UI 热重载从 `Sessions.list()` 恢复；进度、程序状态、prompt、鼠标形状均不持久化。实时 cwd 是例外：保存时投影为下次启动目录。

## 读法

libghostty 的独立 OSC parser 只暴露两类数据：标题字符串和未知序列原始字节。其余命令类型只能识别，具体效果由 terminal 的 option/callback/data 提供。因此“识别”不等于“应用能看见”。

- **内部处理**：library 自己完成，不需要应用回调；查询回复仍需要 write-PTY。
- **回调**：应用必须注册 option 才能产生效果。
- **仅识别**：parser 认识，但 terminal 丢弃或没有 C API。
- **不接入**：Termlevel 有意不映射，通常因为无数据、安全风险或没有 UI 语义。

## 当前 OSC 能力清单

| OSC | 协议 | libghostty-vt 行为 | Termlevel 映射 |
|---|---|---|---|
| 0 / 2 | 窗口标题 | 存储标题，`TITLE_CHANGED` / `DATA_TITLE` | 已接：`SessionInfo.title` → tab 标题 |
| 1 | 窗口图标名 | parser 识别，terminal 忽略；无数据键 | 不接：没有可读值，macOS 侧栏也不需要 |
| 4 / 5 / 10–19 / 104 / 105 / 110–119 | 颜色与动态颜色 | 应用覆盖、回答查询、处理 reset；fg/bg/cursor/palette 有 data key，特殊色多数不可查询 | 已用：主题默认值 + 程序覆盖由 renderer 直接显示，查询回复经 `WRITE_PTY` |
| 7 | PWD（`file://` URI） | 存储原始 PWD，`PWD_CHANGED` / `DATA_PWD` | 已接：解码后写入 `currentCwd`，更新工具栏/搜索/新终端/保存 |
| 8 | 超链接 | 写入 cell hyperlink；查询 API 在 grid ref | 已用：hover tooltip、Cmd+click 打开 http/https/mailto |
| 9 | iTerm2 桌面通知 | `DESKTOP_NOTIFICATION` callback | 已接：通知历史 |
| 9;1–9;8, 9;10, 9;11 | ConEmu sleep / message box / tab title / wait input / GuiMacro / run process / env / xterm emulation / comment | terminal 丢弃；只有 parser 类型 | 不接：无回调；GUI/进程类有安全风险 |
| 9;4 | ConEmu 进度 | `PROGRESS_REPORT` callback | 已接：tab 进度线 |
| 9;9 | ConEmu CurrentDir | 走 PWD，`PWD_CHANGED` / `DATA_PWD` | 已接：同 OSC 7 |
| 21 | Kitty 颜色协议 | 内部颜色栈；无 C API | 不接：应用看不到 |
| 22 | 鼠标形状 | 存储 `GhosttyMouseShape`；`DATA_MOUSE_SHAPE`，无 change callback | 已接：轮询后映射为 `mouseShape`，应用到终端 grid 的 pointer cursor |
| 52 | 剪贴板 | 写入/读取 callback；标准位置、text MIME | 已接：text-only、1 MiB、读取受全局设置控制 |
| 66 | Kitty 文本尺寸 | parser 识别，terminal 丢弃 | 不接：无数据/回调 |
| 72 | Kitty 拖放 | 纯 Zig terminal 内部有处理，但 C API 无效果 | 不接：应用看不到 |
| 99 | Kitty 桌面通知 | parser 识别，terminal 丢弃；通知 callback 只声明 OSC 9/777 | 不接：没有回调 |
| 133 | semantic prompt | 更新 cell/row 语义与 `CURSOR_AT_PROMPT`；`SEMANTIC_PROMPT` callback | 已接：prompt 开始时清除 7501 working/blocked，`atPrompt` 进入 `SessionInfo` |
| 777 | 桌面通知 | `DESKTOP_NOTIFICATION` callback | 已接：通知历史 |
| 1337 `Copy` | iTerm2 剪贴板写入 | 归一化到 clipboard write callback | 已接：标准位置、text-only |
| 1337 `CurrentDir` | iTerm2 PWD | 走 `PWD_CHANGED` | 已接：同 OSC 7 |
| 1337 其他键 | SetMark / StealFocus / SetProfile / SetUserVar / OpenURL 等 | 上游识别后返回 invalid command，不产生 unknown | 不接：没有 C API，也不会作为未知序列上报 |
| 3008 | Hierarchical context | parser 识别，terminal 丢弃 | 不接：无数据/回调 |
| 5522 | Kitty 剪贴板 | 完整读写 callback、MIME、授权、状态；写入默认上限 64 MiB | 部分接：沿用 text-only 适配器；授权/MIME 列表/paste 事件未实现 |
| 7501 | 程序状态 | `PROGRAM_STATUS` callback；设置回调后回答 `OSC 7501;?` | 已接：状态图标、进度兜底、通知 |
| 未知编号 | 任意 | `UNKNOWN_SEQUENCE` + `UNKNOWN_MAX_BYTES`；只报告 parser 不实现的编号 | 不接：回调保持关闭；已识别但被丢弃的编号不会出现在这里 |

## 当前 PWD、OSC 22 与 OSC 133 的映射

### PWD

- `crates/ghostty-vt` 已经产生 `Event::WorkingDirectoryChanged`，此前应用层丢弃。
- 应用解码 `file://` URI 和 ConEmu/iTerm2 裸路径：只接受绝对本地路径，拒绝控制字符、query/fragment 和远程 host；上限 4096 字节。
- `SessionInfo.cwd` 仍是启动目录；新增 `currentCwd` 保存实时值。
- UI 新增运行期 `Tab.liveCwd`。工具栏路径、Cmd-K 搜索、分组路径建议和从该 tab 新建终端使用 `liveCwd ?? cwd`。保存文档时写入 `liveCwd ?? cwd`，下次启动以最后目录为新会话的启动目录；分组自己的 cwd 不变。
- reset 清除实时值；进程退出保留最后值，方便恢复和保存。

### OSC 22

- 安全封装读取 `GHOSTTY_TERMINAL_DATA_MOUSE_SHAPE`，映射 34 个 `GhosttyMouseShape` 值；初始和空 OSC 22 都是 `text`。
- 没有 change callback，`GhosttyEmulator` 在每次 `backend_events` 后读取并仅在变化时发布。
- `SessionInfo.mouseShape` → `Tab.mouseShape` → `SessionTerminal` 的 host `style.cursor`。NSH 的 cursor 解析会沿祖先链查找，所以 terminal grid 和它的 scrollbar 各自保持独立光标。
- 已知限制：`nsh-platform-macos` 可能只在指针下一次移动时应用新 cursor；静止指针的即时切换需要后续原生 NSCursor 刷新。

### OSC 133

- 安全封装注册 `SEMANTIC_PROMPT`，复制 kind、prompt kind、exit code、command、error，并在回调内读取 `CURSOR_AT_PROMPT`。
- `PROMPT_START` 清除 7501 的 `working` / `blocked` 记录；`done` / `error` 保留原始记录，确认后不再参与侧栏聚合。`atPrompt` 根据 callback 和 `CURSOR_AT_PROMPT` 更新；`shellPhase` 区分 unknown / prompt / input / output / commandEnd。新 prompt 同时清除上一轮 9;4 进度。
- `SessionInfo.atPrompt` 目前只作为运行期元数据，未新增可见 UI；侧栏仍以 `blocked` / `error` 优先。
- 需要 shell integration 才会收到 OSC 133；没有事件不是错误。

## 安全边界

- OSC 52 / 5522 / 1337 读取仍默认拒绝，只有设置里显式允许才可读；写入始终允许但受 text-only 和大小上限约束。
- 标题回报（CSI 21 t）保持关闭，避免程序把标题注入回 PTY。
- 未知序列回报保持关闭。
- ConEmu GUI 宏、运行进程、环境变量等只被 parser 识别，terminal 不执行；应用也不提供入口。
- 7501 / 133 的文本是不可信程序输入，显示前清洗、截断，不当作标记语言。

## 状态协议契约

- OSC 9;4 的 state：`0=remove`、`1=set`、`2=error`、`3=indeterminate`、`4=pause`。百分比为可选的 0–100；缺失或 -1 表示没有确定值，不等于 0 或 100。BEL / ST 终止和分块解析交给底层；应用保留最新报告，不维护进度事件列表。
- OSC 7501 包含 `id/app/state/kind/title/message/progress`；state 为 idle、working、done、blocked、error、clear；blocked kind 为 permission、question、auth。7501 不需要 shell integration，发送程序需主动采用协议；OSC 133 则需要 shell 报告生命周期。
- 注册 `PROGRAM_STATUS` 回调本身就是能力声明：libghostty 经 write-PTY 回答 `OSC 7501;?` 的 states/kinds。不能只显示部分状态而对其余状态无业务处理。
- 安全封装在回调内复制有界 owned 数据；未来未知枚举跳过，不 panic。RIS 的根 clear 在 reset 回调之前到达。文字进入展示层前移除控制字符、双向覆盖和不可见格式字符，作为纯文本单行截断；不当作命令或标记语言。
- 状态与进度互不拥有：9;4 remove 不清 7501，7501 clear 不清 9;4；共同生命周期清理另见下文。

## 状态业务模型

### 五个独立维度

| 维度 | 来源 / 所有者 | 产品含义 |
|---|---|---|
| 会话生命周期 | Rust SessionManager：running / exited / error / closed | shell 是否仍存在，是否发生宿主故障 |
| shell 阶段 | OSC 133 | 提示符 / 输入 / 输出 / 命令结束；缺少 integration 时是未知 |
| 程序活动 | OSC 7501，每个 id 一条记录 | idle / working / blocked / done / error |
| 任务进度 | OSC 9;4 + 代表 working 记录的可选 percent | 进度数值 / 不确定 / 暂停 / 进度错误 |
| 注意与已读 | Rust 记录版本 + 用户确认游标 | 本次提醒或结果是否被用户主动查看 |

`SessionInfo.state=running` 只说明会话进程存活。没有 7501 时不猜程序忙闲；不根据输出速度、标题 spinner、键盘输入、静默时间推断状态。`atPrompt=false` 也可能只是没有 OSC 133，不得显示“正在执行”。`shellPhase` 区分阶段未知与已知，不能单用 `atPrompt` bool 推断。

### 7501 记录与聚合

保留协议原始语义：空 id 是根；报告整体替换同 id，缺失字段不继承旧值；app 可从最近祖先继承；clear 删除该 id 和 `id/` 子树，空 id 清空全部。最多 256 条，超限淘汰最久未更新记录。同状态更新 title/message/percent 不视为新一次任务跃迁。

记录旁维护状态代次和已确认代次，不修改程序报告来表达已读。相同 id 从 working → blocked 产生新提醒代次；blocked 连续更新文字不重新提醒；blocked → working → blocked 是新提醒。done/error 同理产生结果代次。acknowledge 不推进程序代次，重复相同报告也不重新置未读。

聚合候选按以下顺序选择，同级取最近更新；读取确认会排除已读 done/error，但不会排除仍成立的 blocked：

```text
宿主故障 / 保留的非零退出
  > blocked（已读仍可见）
  > 未读 error
  > working
  > 未读 done
  > idle / 默认图标
```

- 已读 done/error 原始记录仍保留，直到同 id 替换、clear、reset、关闭或容量淘汰；不再压住当前 working。
- 代表记录的 title/message/app/kind/percent 必须来自同一记录；不把多个子任务的百分比平均或相加。
- 多个同时待处理的任务只占一个图标，tooltip 附 `另有 N 项待处理`；数量指其余 blocked 和未读 error，不包含普通 working。
- 同 id 再次 working 会替换旧结果；子任务是否结束由协议事件表达，不通过猜测父子进程树补写状态。

### 确认与解除

| 用户 / 程序动作 | 结果 |
|---|---|
| 用户主动选择标签，或主动把焦点切到可见 pane | 确认本次已发布摘要之前的提醒 / 结果版本；保留 blocked 事实 |
| 点击来源通知跳到终端 | 同样确认已呈现版本；历史通知已读独立处理 |
| 点击当前已选标签 | 可确认后来到达的新结果，无需先切走 |
| 当前终端正聚焦时到达 blocked/error/done | 先显示；不因“恰好 active”自动确认，后续主动查看动作才确认 |
| 自动恢复、热重载、重挂、窗口激活、hover 展开、仅显示在分屏 | 不确认 |
| blocked 后用户开始输入 | 不猜是否已解决，等程序更新 / clear / 新 prompt |
| 用户清空通知历史 | 不改变活动状态、进度或会话错误 |

确认 API 为 `acknowledge(sessionId, observedRevision)`：只确认用户已看到的快照，不确认点击途中到达的新事件。UI 将当前展示的版本交回 Rust；Rust 决定哪些记录可确认。不得继续使用“删除记录”实现已读。

宿主故障和保留的非零退出始终显示，选择不能把退出进程变成正常终端；详情进入已有通知面板。此处不新增常驻状态栏。

## 状态 UI 与 UX

### 扫视层级

标题保持用户重命名优先，否则使用 OSC 0/2。7501 的任务 title/message 放 tooltip 和提醒详情，避免任务更新使标签名不断跳变。目录继续属于 pane 工具栏和搜索，不挤入标签行。

| 状态 | 图标 / 颜色 | 行为与用户预期 |
|---|---|---|
| 无报告 / idle | terminal，固定标签用 pin；沿用分组色 | 中性默认态；不声称 shell 正在等待输入 |
| working | 三点 busy，primary | 正在处理；有进度线时图标静态，无进度线时可低强度 pulse |
| blocked | 圆圈感叹号，primary | 需要我操作；已查看后仍保留，静态 |
| 未读 done | check，subtle-foreground | 有完成结果待查看；确认后回默认或其他活动状态 |
| 未读程序 error | xCircle，destructive | 有失败结果待查看；确认后不遮挡后续 working |
| 宿主故障 / 保留的非零退出 | xCircle，destructive | 会话本身异常 / 已退出；tooltip 明确来源与退出码 |

状态语义色优先于分组色，形状提供颜色之外的区分。不新增第二个未读角标：现有 BEL 圆点保持独立，标题栏铃铛负责通知未读。固定标签遇到活动时仍由状态图标接管，固定关系由位置和操作表达。不要把 error 显示为 `Terminal failed`，程序任务失败应写 `Task failed`；只有宿主故障才称终端故障。

展开态示意（图标占既有 13 pt 槽，进度线只在标题列内）：

```text
 ⋯  build all           ×       working + 42%
    ━━━━━━━╸──────────

 !  deploy              ×       等待批准，无进度就没有线

 ×  api tests           ×       失败结果待查看，无伪造满格线

 ✓  tests               ×       完成结果待查看
```

保留 28 pt 行高、选中背景、hover / 键盘焦点关闭按钮和布局圆点让位规则。展开态进度为标题下方 2 pt 圆角线，收起态为图标下 12 pt 短线；hover 展开只恢复同一内容，不触发确认。线不参与命中、不越过 tab 圆角，不移动分组引导线。收起态仍可通过 tooltip 获得标题和动作提示。

### 进度只有一种含义

先在展示摘要中解析来源，再统一供线、tooltip、无障碍消费：

1. 会话已退出 / 宿主故障：不展示运行进度。
2. 存在非 remove 的 OSC 9;4：使用它的 state 和 percent。
3. 否则代表 7501 记录为 working 且显式有 percent：使用该数值。
4. 否则不画线。blocked/error/done 自身不生成进度。

| resolvedProgress | 表现 |
|---|---|
| set + percent / 7501 percent | 按 0–100 填充；100% 仍等程序报告 done |
| set 无 percent / indeterminate | 不确定光带；明确 indeterminate 时忽略百分比 |
| pause + percent | 已有比例静止、次级色 |
| pause 无 percent | 仅静态轨道，说明“已暂停 · 进度未知”，不填满、不动画 |
| error + percent | 真实比例、危险色，说明“进度报告错误” |
| error 无 percent | 静态危险色轨道，无填充、不动画 |
| remove | 清除 9;4 来源；如仍有 7501 working 百分比，则按规则回退 |

blocked/error 与 9;4 可同时存在：图标表达活动，线继续表达原始进度，不覆盖成满格。如果 9;4 仍报告 indeterminate，blocked/error 下将光带静止，避免“等待我操作”仍显得正在推进；tooltip 保留“进度未知”。7501 不把 9;4 改写为 pause，也不清除它。两个协议无法证明属于同一个子任务，因此说明中称“终端进度”，不把它说成代表任务的进度。

全行最多一个动画区域：有动态进度时 busy 图标静态；无进度时才 pulse busy。减少动态效果时全部静态；不确定进度以固定短段表达，不用假百分比。无 JS 动画定时器。主题 token 来自 theme.json，先复用 primary / destructive / subtle-foreground，不硬编码语义颜色。

### 详情与可访问性

默认信息留在行内；hover 或键盘焦点显示同一个至多两行 tooltip：第一行标签名、活动、终端进度；第二行程序名与 message（否则 title）。单行截断，按纯文本渲染。例子：

- `build all · Working · Terminal progress 42%` / `cargo · Compiling dependencies`
- `deploy · Needs approval` / `agent · Apply these changes?`
- `api tests · Task failed` / `pytest · 2 tests failed`
- `shell · Exited (code 1)`，与程序活动 error 区分。

blocked.kind 的 UI 文案分别为 permission → Needs approval、question → Needs answer、auth → Sign in required；不暴露枚举名代替可读文案。没有 kind 时为 Needs input。长消息的完整可用详情由已有通知历史承载，不新增自动弹窗或强制切焦点。

tab 无障碍标签与 tooltip 共享已解析状态和真实百分比；暂停 / 未知不宣读为 100%。真实数值才提供 progress 范围值，只有轨道的暂停 / 错误使用状态说明，不挂虚构 value。沿用 `tab-status-${id}`、`tab-progress-${id}` 测试入口。百分比更新不使用打断式播报；blocked / error 需要语义可读，但不对每个进度 tick 重复宣布。

## 通知与生命周期

7501 的通知按记录级状态跃迁决定，不按聚合图标是否变化决定。blocked 和 error 进入时各写历史；相同状态内修改百分比 / 文案不重复写；working、done 和 9;4 不写通知。已有通知模块仍负责容量和短时内容合并。acknowledge 不产生新的通知，也不清空历史。

通知携带 session ID 和记录身份 / 代次以便定位来源；用户文案带终端名、程序名和清洗后的消息。标题使用 `Needs approval / Needs answer / Sign in required / Needs input · <terminal>` 或 `Task failed · <terminal>`。只提供返回已有会话的动作，不根据程序 message 生成批准 / 登录按钮；用户在终端里完成交互。会话关闭后保留历史，禁用来源跳转。通知行以结构化 message（否则任务 title）为主文，kind 对应语义图标；app、类别与时间为次级信息，生成的历史标题仍用于定位和无障碍操作标签。长内容可原地展开，具体层级见 [UI 与主题](design.md#通知内容)。

| 事件 | 动作 |
|---|---|
| OSC 9;4 报告 | 后值覆盖前值；remove 只清除自身来源 |
| OSC 7501 报告 / clear | 替换记录 / 清除子树；重算摘要，记录级产生提醒 |
| OSC 133 新 prompt | 结束旧 working/blocked，保留 done/error 及已读信息；清除上一轮 9;4 进度，避免旧线留在新命令上 |
| OSC 133 命令结束 | 更新 shell 阶段；不合成 7501 done/error，退出码可留作 shell 元数据 |
| 没有 OSC 133 | 依赖程序 remove/clear、退出或 reset，不设置猜测性超时 |
| 进程退出 | 清进度与非结果记录；更新会话生命周期；非零退出的提示优先 |
| 干净退出 | 沿用自动关闭标签；不保证保留 done 图标，通知历史仍独立 |
| RIS / full reset | 清进度、记录、确认游标及相关运行期摘要；标题按既有 reset 行为处理 |
| 显式关闭 | 移除会话及其运行期状态，不清历史通知 |
| 应用重启 | 新会话没有旧进度、活动和已读状态 |
| UI 热重载 | list 返回当前摘要与版本；恢复选中不确认 |

新 prompt 清除 9;4 是本次实现的应用生命周期策略；并非 libghostty 自动保证的协议行为。这里以 prompt 作为上一命令边界；需要跨 prompt 长期展示的程序应重新报告进度。

## 实现边界

```text
libghostty callbacks → ghostty-vt owned Event → terminal BackendEvent
  → Rust Session（记录、进度、shell 阶段、已读代次、通知跃迁、展示摘要）
  → SessionInfo（有界摘要 + revision，不发送记录表）
  → session_view StatusChange / Sessions.list
  → TerminalSurface patchTab → Sidebar
```

Rust 是程序事实、确认状态和语义优先级的唯一所有者；UI 只保存运行期摘要并负责图标、布局和动画。摘要包含代表记录身份、可确认版本、待处理数量、resolvedProgress 来源 / 状态 / 数值。`TerminalStatusInfo` 提供展示语义，`ActivityInfo` 提供记录身份，`statusRevision` 提供确认上界；不另建 JS 状态机；tooltip 和无障碍由同一摘要生成。`SessionManager` 仍独立拥有 PTY，不因折叠、选择、重挂或确认创建 / 关闭进程。

实现保留原有 PTY 和渲染通路：`SessionInfo` 发送原始 progress / 聚合 activity 以及 `status: TerminalStatusInfo`，`statusRevision` 只随 7501 报告单调增加，reset 不复用旧代次。Rust 锁顺序为 status → info，记录修改、确认和生命周期清理在同一临界区发布，防止已读结果被旧聚合覆盖。退出后到达的 reader 尾部输出仅接纳 done/error/clear，不恢复 working 或进度。

UI 在主动选择前捕获当前摘要的 statusRevision；初始化恢复跳过确认；原生 FocusRequest 区分用户点击 / 拖放和 incidental input focus，重挂与窗口激活产生的焦点事件只同步逻辑焦点、不确认。减少动态效果使用 NSH 的 `reduceMotion`；不确定线和 busy 图标至多一个动画区域。运行期快照投影仍排除 activity、progress、status、statusRevision 和 shellPhase。

每次非输出事件改变摘要时必须 bump revision；进度连续更新保留最新值，不为每个百分比 tick 发跨层消息。记录表和确认元数据一起受 256 条上限约束；同一输入批次的更新先处理再发布，UI 只拿最终摘要。

## 验收标准

已有证据见 [验证记录](validation.md) 的状态 / OSC 及重设计章节；旧实现证据不能证明新 UX。验收覆盖与未覆盖项分别记录，检查标准如下：

- 真实 PTY：7501 能力查询、BEL/ST 和分块报告、整体替换、祖先 app、clear 子树、容量淘汰、RIS clear/reset 顺序；9;4 所有状态和缺省百分比。
- 业务：blocked 查看后仍显示且不重新通知；同状态重复报告不重置已读；blocked → working → blocked 再提醒；两个 blocked 记录分别进入历史，聚合未变也不漏；确认旧版本不能吞掉新结果。
- 生命周期：done/error 确认后不遮挡 working；新 prompt 清掉旧 working/blocked 与 9;4；无 integration 不猜测 busy；干净退出关闭、非零退出保留；热重载恢复且不确认；持久化丢弃运行期字段。
- 图文一致：9;4=42%、7501=40% 时，线、tooltip、无障碍均用终端进度 42%；blocked 无进度不画线；pause/error 无百分比不画满且不动画；100% 不自动 done。
- 原生窗口：展开 / 收起 / hover 展开、固定与分组标签、键盘选择、分屏焦点和通知来源跳转；不挤压关闭按钮、不重叠 BEL / 布局圆点；多种字号下 2 pt 线可读。
- 人工 UX：无需读 tooltip 能分清工作 / 待操作 / 完成 / 失败；不抢焦点；长消息可从历史查看；减少动态效果、色彩辨识和长时间多会话进度下的稳定性单独记录。

验证命令沿用 [开发指南](development.md)：`./scripts/nsh check`、`./scripts/nsh test`、`./scripts/nsh build`；原生按需运行 `./scripts/test-native status` 和 `./scripts/test-native osc`，不得与全量 headless 并行。本次执行结果和日志见 validation，人工验收不得由自动化通过代替。

完整任务列表、应用图标、窗口级任务聚合、系统桌面推送和 OSC 99 不在本轮范围；保留协议能力边界，不因 UI 设计宣称底层未暴露的能力已接入。
