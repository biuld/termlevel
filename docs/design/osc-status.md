# OSC Integration and Terminal Status

OSC integration is the broader scope; status is one user experience within it. OSC 0 / 2 provides titles, 7 the working directory, 9 / 777 notifications, 9;4 progress, 7501 program status and 133 shell lifecycle. These share the parsing and session path but are never collapsed into one status field. Process exit and host I/O failure come from `SessionManager` and form a separate class of terminal status input.

Ownership is described in the [terminal architecture](../architecture/terminal.md), notification history in the [notifications architecture](../architecture/notifications.md), visual rules in [ui.md](ui.md), and evidence in the [OSC status validation record](../validation/osc-status.md). This design adds no web UI, changes neither NSH nor the terminal library, and does not alter PTY lifecycle.

## Design conclusions

- Keep the 28 pt tab row: a status icon on the left, title in the middle, an optional real progress line below, and the existing layout marker / close button on the right. The collapsed sidebar keeps the same icon and a short progress line.
- The working icon animation is selectable in Settings → Interface: three-bar equalizer (default), vinyl record, three-dot waveform or beat ring. Its color follows the folder color of the tab's group; ungrouped tabs use primary. It plays only while working and there is no progress, and Reduce Motion shows the same shape statically. The choice is saved with the workspace. Old `dashes` / `spectrum` values, unknown values and a missing field fall back to the equalizer.
- The icon answers "how is this terminal now"; the progress line answers "how far has the task advanced"; unread records answer "which results have not been seen". The three are modeled separately.
- Viewing by the user cannot clear program blocked state. Viewing only acknowledges existing results and reminders; it does not delete a program fact.
- blocked / error never fabricate a full progress bar; 100% does not automatically become done; a running shell does not mean a task is working.
- Tooltip, accessibility label and the screen use the same display summary, avoiding a 42% line with a 40% description.

## How OSC data reaches the app

libghostty's independent OSC parser exposes only two data classes: title strings and raw bytes for unknown sequences. Other command types are recognized, but their effect comes from terminal options, callbacks or data. "Recognized" therefore does not mean "visible to the application".

- **Internal handling**: the library performs the work; no application callback is needed. Query replies still require write-PTY.
- **Callback**: the application must register the option for the effect to occur.
- **Recognized only**: the parser knows the command, but the terminal discards it or exposes no C API.
- **Not integrated**: Termlevel intentionally does not map it, usually because there is no data, there is a security risk, or it has no UI semantics.

## OSC capability inventory

| OSC | Protocol | libghostty-vt behavior | Termlevel mapping |
|---|---|---|---|
| 0 / 2 | Window title | Stores the title; `TITLE_CHANGED` / `DATA_TITLE` | Integrated: `SessionInfo.title` → tab title |
| 1 | Window icon name | Parser recognizes, terminal ignores; no data key | Not integrated: no readable value, and the macOS sidebar does not need it |
| 4 / 5 / 10–19 / 104 / 105 / 110–119 | Colors and dynamic colors | Applies overrides, answers queries and handles resets; fg/bg/cursor/palette have data keys, while most special colors are not queryable | Used: theme defaults plus program overrides are painted directly by the renderer; query replies go through `WRITE_PTY` |
| 7 | PWD (`file://` URI) | Stores raw PWD; `PWD_CHANGED` / `DATA_PWD` | Integrated: decoded into `currentCwd`, updating toolbar, search, new terminal and save |
| 8 | Hyperlink | Writes the cell hyperlink; query API is on the grid ref | Used: hover tooltip; Cmd+click opens http/https/mailto |
| 9 | iTerm2 desktop notification | `DESKTOP_NOTIFICATION` callback | Integrated: notification history |
| 9;1–9;8, 9;10, 9;11 | ConEmu sleep / message box / tab title / wait input / GuiMacro / run process / env / xterm emulation / comment | Terminal discards; parser type only | Not integrated: no callback; GUI and process types carry security risk |
| 9;4 | ConEmu progress | `PROGRESS_REPORT` callback | Integrated: tab progress line |
| 9;9 | ConEmu CurrentDir | Routes through PWD; `PWD_CHANGED` / `DATA_PWD` | Integrated: same as OSC 7 |
| 21 | Kitty color protocol | Internal color stack; no C API | Not integrated: the app cannot see it |
| 22 | Mouse shape | Stores `GhosttyMouseShape`; `DATA_MOUSE_SHAPE`, no change callback | Integrated: polled and mapped to `mouseShape`, applied to the terminal grid's pointer cursor |
| 52 | Clipboard | Write/read callback; standard locations, text MIME | Integrated: text-only, 1 MiB, reads controlled by the global setting |
| 66 | Kitty text sizing | Parser recognizes, terminal discards | Not integrated: no data or callback |
| 72 | Kitty drag-and-drop | The pure Zig terminal handles it internally, but it has no effect through the C API | Not integrated: the app cannot see it |
| 99 | Kitty desktop notification | Parser recognizes, terminal discards; the notification callback declares only OSC 9 / 777 | Not integrated: no callback |
| 133 | Semantic prompt | Updates cell/row semantics and `CURSOR_AT_PROMPT`; `SEMANTIC_PROMPT` callback | Integrated: prompt start clears 7501 working / blocked, and `atPrompt` enters `SessionInfo` |
| 777 | Desktop notification | `DESKTOP_NOTIFICATION` callback | Integrated: notification history |
| 1337 `Copy` | iTerm2 clipboard write | Normalized to the clipboard write callback | Integrated: standard locations, text-only |
| 1337 `CurrentDir` | iTerm2 PWD | Routes through `PWD_CHANGED` | Integrated: same as OSC 7 |
| 1337 other keys | SetMark / StealFocus / SetProfile / SetUserVar / OpenURL etc. | Upstream recognizes and returns invalid command; no unknown sequence is produced | Not integrated: no C API, and not reported as unknown |
| 3008 | Hierarchical context | Parser recognizes, terminal discards | Not integrated: no data or callback |
| 5522 | Kitty clipboard | Full read/write callback, MIME, authorization, state; write defaults to a 64 MiB limit | Partially integrated: text-only adapter retained; authorization, MIME list and paste events are not implemented |
| 7501 | Program status | `PROGRAM_STATUS` callback; once set, answers `OSC 7501;?` | Integrated: status icon, progress fallback, notifications |
| Unknown number | Any | `UNKNOWN_SEQUENCE` + `UNKNOWN_MAX_BYTES`; reports only numbers the parser does not implement | Not integrated: the callback stays off; recognized-but-discarded numbers do not appear here |

## PWD, OSC 22 and OSC 133 mapping

### PWD

- `crates/ghostty-vt` already emits `Event::WorkingDirectoryChanged`; the application layer previously discarded it.
- The application decodes `file://` URIs and ConEmu / iTerm2 bare paths. It accepts only absolute local paths and rejects control characters, query/fragment parts and remote hosts; the limit is 4096 bytes.
- `SessionInfo.cwd` remains the startup directory; the new `currentCwd` holds the live value.
- The UI adds a runtime `Tab.liveCwd`. The toolbar path, Cmd-K search, group path suggestions and new terminals from that tab use `liveCwd ?? cwd`. Saving the document writes `liveCwd ?? cwd`, so the next launch starts new sessions in the last directory; the group's own `cwd` is unchanged.
- Reset clears the live value; process exit keeps the last value for recovery and saving.

### OSC 22

- The safe wrapper reads `GHOSTTY_TERMINAL_DATA_MOUSE_SHAPE` and maps 34 `GhosttyMouseShape` values; the initial value and an empty OSC 22 are both `text`.
- There is no change callback, so `GhosttyEmulator` reads the value after every `backend_events` and publishes only when it changes.
- `SessionInfo.mouseShape` → `Tab.mouseShape` → the host `style.cursor` of `SessionTerminal`. NSH cursor resolution walks the ancestor chain, so the terminal grid and its scrollbar keep independent cursors.
- Known limitation: `nsh-platform-macos` may apply a new cursor only on the next pointer move. Immediate switching for a stationary pointer needs a follow-up native NSCursor refresh.

### OSC 133

- The safe wrapper registers `SEMANTIC_PROMPT`, copies kind, prompt kind, exit code, command and error, and reads `CURSOR_AT_PROMPT` inside the callback.
- `PROMPT_START` clears 7501 `working` / `blocked` records; `done` / `error` keep their original records and stop participating in sidebar aggregation once acknowledged. `atPrompt` updates from the callback and `CURSOR_AT_PROMPT`; `shellPhase` distinguishes unknown, prompt, input, output and commandEnd. A new prompt also clears the previous round's 9;4 progress.
- `SessionInfo.atPrompt` is runtime metadata and adds no visible UI; the sidebar still prioritizes `blocked` / `error`.
- OSC 133 requires shell integration. Receiving no event is not an error.

## Security boundary

- OSC 52 / 5522 / 1337 reads remain denied by default and are readable only when the setting explicitly allows them. Writes are always allowed but are text-only and size-limited.
- Title reporting (CSI 21 t) stays off, preventing a program from injecting the title back into the PTY.
- Unknown-sequence reporting stays off.
- ConEmu GUI macros, process execution and environment variables are only recognized by the parser; the terminal does not execute them, and the application exposes no entry point.
- 7501 / 133 text is untrusted program input. It is sanitized and truncated before display and is never treated as markup.

## Status protocol contract

- OSC 9;4 states are `0=remove`, `1=set`, `2=error`, `3=indeterminate`, `4=pause`. The percentage is optional, 0–100; missing or `-1` means there is no definite value and is not 0 or 100. BEL / ST termination and chunk parsing belong to the lower layer; the application keeps the latest report and maintains no progress event list.
- OSC 7501 carries `id/app/state/kind/title/message/progress`; state is idle, working, done, blocked, error or clear; blocked kind is permission, question or auth. 7501 does not need shell integration, but the sending program must adopt the protocol; OSC 133 needs the shell to report lifecycle.
- Registering the `PROGRAM_STATUS` callback is itself a capability declaration: libghostty answers `OSC 7501;?` states/kinds through write-PTY. The application cannot display only some states and leave the rest without business handling.
- The safe wrapper copies bounded owned data inside the callback; future unknown enums are skipped without panicking. RIS root clear arrives before the reset callback. Before text enters the display layer, control characters, bidirectional overrides and invisible formatting characters are removed, and the text is truncated to one line as plain text; it is never treated as a command or markup.
- Status and progress do not own each other: 9;4 remove does not clear 7501, and 7501 clear does not clear 9;4. Shared lifecycle cleanup is described below.

## Status business model

### Five independent dimensions

| Dimension | Source / owner | Product meaning |
|---|---|---|
| Session lifecycle | Rust `SessionManager`: running / exited / error / closed | Whether the shell still exists and whether a host failure occurred |
| Shell phase | OSC 133 | Prompt / input / output / command end; unknown without integration |
| Program activity | OSC 7501, one record per id | idle / working / blocked / done / error |
| Task progress | OSC 9;4 plus the optional percent of a representative working record | Numeric / indeterminate / paused / progress error |
| Attention and read state | Rust record version plus the user acknowledgement cursor | Whether this reminder or result has been actively viewed |

`SessionInfo.state=running` only says the session process is alive. Without 7501, the application does not guess whether the program is busy, and it does not infer status from output speed, title spinners, keyboard input or silence. `atPrompt=false` may simply mean there is no OSC 133 and must not display "executing". `shellPhase` distinguishes unknown from known phases; the `atPrompt` boolean alone is not sufficient.

### OSC 7501 records and aggregation

Preserve the raw protocol semantics: an empty id is the root; a report wholly replaces the same id and missing fields do not inherit old values; app may be inherited from the nearest ancestor; clear deletes that id and the `id/` subtree, and an empty id clears everything. At most 256 records are kept; exceeding that evicts the least recently updated record. A same-state update to title, message or percent is not a new task transition.

Each record maintains a status generation and an acknowledged generation beside it, without modifying program reports to express read state. The same id moving from working → blocked creates a new reminder generation; repeated blocked text updates do not remind again; blocked → working → blocked is a new reminder. done / error produce result generations in the same way. Acknowledge does not advance the program generation, and repeating the same report does not set it unread again.

Aggregation selects candidates in this order, newest first among equals. Read acknowledgement excludes acknowledged done / error but never excludes a still-valid blocked:

```text
host failure / retained non-zero exit
  > blocked (still visible after read)
  > unread error
  > working
  > unread done
  > idle / default icon
```

- Acknowledged done / error raw records remain until the same id is replaced, cleared, reset, closed or evicted by capacity; they no longer suppress current working.
- The representative record's title, message, app, kind and percent must all come from the same record. Percentages of several subtasks are never averaged or summed.
- Multiple pending tasks occupy one icon, and the tooltip appends `N more pending`. The count covers other blocked records and unread errors, not ordinary working records.
- Working again on the same id replaces the old result. Whether a subtask ended is expressed by protocol events; the application does not guess parent-child process trees.

### Acknowledgement and clearing

| User or program action | Result |
|---|---|
| User actively selects a tab, or actively moves focus to a visible pane | Acknowledges reminder / result versions before the published summary; keeps the blocked fact |
| Clicking the source notification to jump to the terminal | Also acknowledges the presented version; notification-history read state is handled independently |
| Clicking the currently selected tab | Can acknowledge a later result without switching away first |
| blocked / error / done arrives while the terminal is already focused | Display it first; do not auto-acknowledge merely because it is active; a later active view action acknowledges |
| Automatic restore, hot reload, remount, window activation, hover expansion, or being shown only in a split | Do not acknowledge |
| User starts typing after blocked | Do not guess it is resolved; wait for a program update, clear or new prompt |
| User clears notification history | Does not change active status, progress or session errors |

`Session::acknowledge` and workspace state selection acknowledge only the status generations within `observedRevision`; they keep every protocol record, so blocked remains visible after viewing. The acknowledgement API is `acknowledge(sessionId, observedRevision)`: it acknowledges only the snapshot the user has seen, not events that arrive while the click is in flight. The UI returns the currently displayed version to Rust, and Rust decides which records may be acknowledged. "Delete the record" must not be used to implement read state.

Host failure and retained non-zero exit are always displayed; selection cannot turn an exited process into a normal terminal. Details go to the existing notification panel. No persistent status bar is added.

## Status UI and UX

### Scan hierarchy

The title prefers a user rename, otherwise OSC 0 / 2. The 7501 task title and message go to the tooltip and reminder details, avoiding a tab name that jumps on every task update. The directory remains in the pane toolbar and search and does not crowd the tab row.

| State | Icon / color | Behavior and user expectation |
|---|---|---|
| No report / idle | terminal icon, or pin for a pinned tab; follows group color | Neutral default; does not claim the shell is waiting for input |
| working | three-dot busy, primary | Processing; with a progress line the icon is static, without one it may pulse at low intensity |
| blocked | circled exclamation, primary | Needs my action; remains after viewing, static |
| Unread done | check, notification-success | A completion result is waiting; after acknowledgement returns to default or another active state |
| Unread program error | xCircle, destructive | A failure result is waiting; after acknowledgement it does not cover later working |
| Host failure / retained non-zero exit | xCircle, destructive | The session itself failed or exited; the tooltip states the source and exit code |

A recognized OSC 7501 `app` establishes the session's agent identity in any reported state. Supported IDs are `codex`, `claude-code`, `grok` / `grok-build`, `pi` and `opencode`; icon sources are listed in [the agent assets](../../assets/agents/README.md). The latest recognized report selects the idle icon, tooltip and accessibility name. Omitted or unknown app values, result acknowledgement and OSC 133 prompt markers leave that identity intact. Working, blocked, unread results and host failures keep their icon priority.

Identity is runtime-only. A clear covering the report's ID, a terminal reset, and session exit, failure or close remove it. When Termlevel can associate the report with a distinct local foreground process group, a change of foreground group also removes it. A reporter inside SSH or a multiplexer can share the outer client's foreground process group, so its own exit or pane change requires an explicit clear. An OSC 133 prompt can also mark an in-app transcript, as described in [the Pi issue](https://github.com/earendil-works/pi/issues/11017).

Status semantic colors take precedence over group color, and shape distinguishes states beyond color. No second unread badge is added: the existing BEL dot remains independent and the title-bar bell owns notification unread state. An active pinned tab still lets the status icon take over; the pinned relationship is expressed by position and actions. Error text must not say `Terminal failed` for a program task; use `Task failed`. Only host failures are called terminal failures.

Expanded-state sketch (the icon occupies the existing 13 pt slot and the progress line stays within the title column):

```text
 ⋯  build all           ×       working + 42%
    ━━━━━━━╸──────────

 !  deploy              ×       waiting for approval; no progress, no line

 ×  api tests           ×       failure result waiting; no fake full line

 ✓  tests               ×       completion result waiting
```

Keep the 28 pt row height, selection background and the close button on hover / keyboard focus. Expanded progress is a 2 pt rounded line under the title; collapsed progress is a 12 pt short line under the icon. Hover expansion only restores the same content and does not acknowledge. The line does not participate in hit testing, does not cross the tab radius and does not move the group guide. In the collapsed state, the tooltip still provides the title and action hints.

### Progress has one meaning

Resolve the source in the display summary first, then feed the line, tooltip and accessibility from the same value:

1. Session exited / host failure: show no running progress.
2. OSC 9;4 set, pause or error with 0–100: use that state and percentage.
3. Otherwise, the representative 7501 record is working and has an explicit percent: use that value.
4. Otherwise draw no line. A missing percentage, `-1` and indeterminate are not measurements. blocked, error and done do not generate progress themselves.

| resolvedProgress | Presentation |
|---|---|
| set + percent / 7501 percent | Fill by 0–100; 100% still waits for the program to report done |
| set without percent, `-1`, indeterminate | No line; working continues to show the Working indicator |
| pause + percent | Existing ratio static, secondary color |
| pause without percent | No line |
| error + percent | Real ratio, destructive color, description "progress report error" |
| error without percent | No line |
| remove | Clear the 9;4 source; if a 7501 working percentage remains, fall back by the rules above |

blocked / error and a 9;4 percentage can coexist: the icon expresses activity and the line continues to express the raw percentage. Without a percentage there is no empty track, and unknown progress is not painted as a filled segment. 7501 neither rewrites 9;4 as pause nor clears it. The two protocols cannot prove they belong to the same subtask, so descriptions say "terminal progress", not task progress.

A progress line appears only with a percentage and is static. Without a line, the working Working indicator is the only activity hint; it is also static under Reduce Motion. Normal progress and the working icon follow the group color, or primary when ungrouped; blocked uses warning, error destructive, done notification-success and paused progress subtle-foreground. Theme tokens come from `theme.json`; semantic colors are never hardcoded.

### Details and accessibility

Default information stays inline. Hover or keyboard focus shows the same tooltip of at most two lines: the first has tab name, activity and terminal progress; the second has program name and message (or title). Each line truncates and renders as plain text. Examples:

- `build all · Working · Terminal progress 42%` / `cargo · Compiling dependencies`
- `deploy · Needs approval` / `agent · Apply these changes?`
- `api tests · Task failed` / `pytest · 2 tests failed`
- `shell · Exited (code 1)`, distinct from program activity error

blocked.kind UI text is permission → Needs approval, question → Needs answer, auth → Sign in required; enum names are never shown instead of readable text. Without a kind it is Needs input. Full details for long messages are carried by existing notification history; no automatic popup or forced focus is added.

The tab accessibility label and tooltip share the resolved status and real percentage; pause / unknown are never announced as 100%. Only real values provide a progress range value, while pause / error with only a track use the status description and no invented value. Keep the `tab-status-${id}` and `tab-progress-${id}` test entries. Percentage updates do not use interruptive announcements; blocked / error must be semantically readable, but each progress tick is not announced.

## Notifications and lifecycle

7501 notifications are decided by record-level state transitions, not by whether the aggregate icon changed. Entering blocked or error writes history; changing percentage or text within the same state does not write again; working, done and 9;4 do not write notifications. The existing notification module still owns capacity and short-window content merging. Acknowledge produces no new notification and does not clear history.

Notifications carry the session ID and record identity / generation to locate the source; user-facing text includes the terminal name, program name and sanitized message. Titles use `Needs approval / Needs answer / Sign in required / Needs input · <terminal>` or `Task failed · <terminal>`. Only an action back to the existing session is provided; program messages never generate approval / sign-in buttons, and the user completes interaction in the terminal. After a session closes, history is retained and source jumping is disabled. A notification row uses the structured message (otherwise task title) as its main text and kind as its semantic icon; app, category and time are secondary information. The generated history title is still used for locating and for accessibility action labels. Long content expands in place; the hierarchy is in [ui.md](ui.md#notification-content).

| Event | Action |
|---|---|
| OSC 9;4 report | Later value overwrites the previous one; remove clears only its own source |
| OSC 7501 report / clear | Replace the record / clear the subtree; recompute the summary and generate record-level reminders |
| OSC 133 new prompt | End old working / blocked, keep done / error and read state; clear the previous round's 9;4 progress so an old line does not remain on the new command |
| OSC 133 command end | Update the shell phase; do not synthesize 7501 done / error; the exit code can remain shell metadata |
| No OSC 133 | Rely on program remove / clear, exit or reset; set no speculative timeout |
| Process exit | Clear progress and non-result records; update session lifecycle; non-zero exit messaging takes priority |
| Clean exit | Keep the existing automatic tab close; the done icon is not guaranteed to remain, and notification history stays independent |
| RIS / full reset | Clear progress, records, acknowledgement cursor and related runtime summary; handle the title by existing reset behavior |
| Explicit close | Remove the session and its runtime state; do not clear history notifications |
| App restart | A new session has no old progress, activity or read state |
| UI hot reload | `list` returns the current summary and version; restoring selection does not acknowledge |

Clearing 9;4 on a new prompt is an application lifecycle policy, not a protocol behavior guaranteed by libghostty. The prompt is treated as the previous command boundary; programs that need progress to span prompts must report it again.

## Implementation boundaries

```text
libghostty callbacks → ghostty-vt owned Event → terminal BackendEvent
  → Rust Session (records, progress, shell phase, read generations, notification transitions, display summary)
  → SessionInfo (bounded summary + revision, no record table)
  → session_view StatusChange / Sessions.list
  → TerminalSurface patchTab → Sidebar
```

Rust is the sole owner of program facts, acknowledgement state and semantic priority. The UI stores only the runtime summary and owns icons, layout and animation. The summary includes the representative record identity, acknowledgeable version, pending count and `resolvedProgress` source / state / value. `TerminalStatusInfo` provides display semantics, `ActivityInfo` provides record identity and `statusRevision` provides the acknowledgement upper bound. There is no separate JS state machine; tooltip and accessibility are generated from the same summary. `SessionManager` still independently owns the PTY and never creates or closes a process because of collapse, selection, remount or acknowledgement.

Percentage changes aggregate in Rust and reach TSX through the existing 16 ms poll; suspending frame capture for a hidden surface does not block this metadata poll. UI hot reload recovers from `Sessions.list()`. Progress, program status, prompt state and mouse shape are not persisted; live cwd is the exception and is projected as the next startup directory when saving. `TerminalSurface.tsx` forwards the bounded summary. A clean exit still auto-closes the tab, while notification history remains independent.

The existing PTY and rendering path is preserved: `SessionInfo` sends raw progress / aggregated activity plus `status: TerminalStatusInfo`, and `statusRevision` increases monotonically with 7501 reports; reset never reuses an old generation. Rust acquires `control` before `status` when reading foreground ownership, and acquires `status` before `info`, and record modification, acknowledgement and lifecycle cleanup are published in the same critical section so an acknowledged result cannot be overwritten by an old aggregate. Reader tail output arriving after exit accepts only done / error / clear and cannot restore working or progress.

Before an active selection, the UI captures the current summary's `statusRevision`; initialization restore skips acknowledgement. Native `FocusRequest` distinguishes user clicks / drops from incidental input focus, so remount and window-activation focus events only synchronize logical focus and do not acknowledge. Reduce Motion uses NSH `reduceMotion`; indeterminate lines and the busy icon use at most one animation region. Runtime snapshot projection still excludes activity, progress, status, `statusRevision` and `shellPhase`.

Every non-output event that changes the summary must bump the revision. Continuous progress updates keep only the latest value and do not send a cross-layer message per percentage tick. The record table and acknowledgement metadata share the 256-record limit. Updates in one input batch are processed before publishing, and the UI receives only the final summary.

## Acceptance criteria

Evidence lives in [../validation/osc-status.md](../validation/osc-status.md). Old implementation evidence does not prove the current UX; covered and uncovered items are recorded separately.

- Real PTY: 7501 capability query, BEL / ST and chunked reports, whole-record replacement, ancestor app, clear subtree, capacity eviction and RIS clear / reset order; all 9;4 states and missing percentages.
- Business behavior: blocked remains visible after viewing and does not re-notify; a same-state repeated report does not reset read state; blocked → working → blocked reminds again; two blocked records both enter history even when the aggregate does not change; acknowledging an old version cannot swallow a new result.
- Lifecycle: acknowledged done / error does not cover working; a new prompt clears old working / blocked and 9;4; no integration means no busy guessing; clean exit closes and non-zero exit is retained; hot reload restores without acknowledging; persistence drops runtime fields.
- Text and graphics agree: with 9;4=42% and 7501=40%, the line, tooltip and accessibility all use terminal progress 42%; blocked without progress draws no line; pause / error without a percentage draw no full line and do not animate; 100% does not become done automatically.
- Native window: expanded / collapsed / hover-expanded states, pinned and grouped tabs, keyboard selection, split focus and notification source jumping; the close button is not squeezed and the BEL dot does not overlap; the 2 pt line is readable at multiple font sizes.
- Manual UX: working, needs-action, complete and failure are distinguishable without reading the tooltip; focus is not stolen; long messages are viewable from history; Reduce Motion, color recognition and stability under long multi-session progress are recorded separately.

Validation commands follow [../development.md](../development.md): `./scripts/nsh check`, `./scripts/nsh test` and `./scripts/nsh build`; native runs use `./scripts/test-native status` and `./scripts/test-native osc` as needed and are not run in parallel with a full headless run. Results and logs belong in the validation record; manual acceptance cannot be replaced by automated passes.

A complete task list, application icon, window-level task aggregation, system desktop push and OSC 99 are outside this scope. The protocol capability boundary is retained, and the UI does not claim that an underlying capability which is not exposed has been integrated.
