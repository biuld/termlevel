# Validation: OSC and terminal status

| Date | Entry |
|---|---|
| 2026-11-15 | Sidebar terminal status and progress (OSC 7501 / OSC 9;4) |
| 2026-11-15 | OSC support inventory and PWD / OSC 22 / OSC 133 mapping |
| 2026-11-15 | OSC status UI / UX redesign landed |
| 2026-11-15 | Notification row icons and content hierarchy |
| 2026-11-15 | Configurable working indicator |
| 2026-11-18 | OSC 7501 idle agent icons |
| 2026-11-18 | OSC 7501 agent identity across status updates |

## 2026-11-15 · Sidebar terminal status and progress (OSC 7501 / OSC 9;4)

Uncommitted changes based on `2f1ae0c`. Sidebar tabs now show three layers of information at once: the title still comes from OSC 0 / OSC 2; the aggregate status from OSC 7501 replaces the default terminal / pin icon (working uses a three-dot breathing icon, not a rotating ring); the progress from OSC 9;4 is a 2 pt line below the title column, shrinking below the icon when the sidebar is collapsed. When working has both a 9;4 and a 7501 percentage, 9;4 wins; the 7501 percentage is only a fallback when 9;4 is missing; working without an explicit percentage does not synthesize an indeterminate line, and blocked / error keep a full state line. See [OSC integration and terminal status design](../design/osc-status.md) for visuals, priority, and lifecycle. The complete id record table for 7501 exists only in the Rust `Session`, and `SessionInfo` sends an aggregate summary; each transition into blocked / error writes one notification, while working, done, and 9;4 do not. Selecting the tab calls `Sessions.acknowledge` to clear done / blocked / error; the initial selection when the app restores does not count as user acknowledgement. Runtime fields do not enter the workspace document.

Validation used `GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip CARGO_NET_OFFLINE=true`:

- `cargo test -p termlevel-native`: 86/86, log artifacts/terminal-status-rust.log. Added cases for aggregate priority and app inheritance, subtree clearing, 9;4 / 7501 entering `SessionInfo` under a real `/bin/sh`, acknowledge, and clearing working on exit.
- `cargo test -p termlevel-ghostty-vt --test native`: 23/23, log artifacts/terminal-status-wrapper.log. Added the five 9;4 states and missing percentage, chunked input, consecutive progress merging, 7501 blocked decoding, child record clear, `OSC 7501;?` supported reply, RIS clearing before reset, and 9;4 not generating notifications.
- `./scripts/nsh test --junit artifacts/terminal-status-headless.xml`: 179/179, log artifacts/terminal-status-headless.log. Added four real PTY cases in tests/terminal/Status.test.tsx: 42% / indeterminate / remove progress line; working → done → blocked icons and accessible labels, 9;4 taking priority over the 7501 percentage, blocked unread notification and selection acknowledgement; working without a percentage not synthesizing a progress line; error notification and acknowledgement. A Snapshot case also confirms that `progress` / `activity` in the persisted JSON are dropped.
- `./scripts/nsh test tests/terminal/Status.test.tsx --target macos --junit artifacts/terminal-status-native.xml`: 4/4, log artifacts/terminal-status-native.log. The native host still has one existing link warning. After the final adjustment of the progress line track height and the 9;4 / 7501 percentage priority, `./scripts/nsh test tests/terminal/Status.test.tsx --junit artifacts/terminal-status-focused.xml` re-verified 4/4, log artifacts/terminal-status-focused.log.
- `./scripts/test-native status`: 1/1, log artifacts/terminal-status-native-acceptance.log. Reviewed artifacts/native-status-progress.png, native-status-working.png, native-status-blocked.png, native-status-collapsed.png: the 42% progress line sits below the title text; working uses the three-dot breathing icon and no longer shows a rotating ring; blocked uses a circle exclamation mark with a full emphasis line, and an unread notification appears in the title bar; after collapsing the sidebar a short progress line remains below the icon. The native host still has one existing link warning.
- `./scripts/nsh check`, `./scripts/nsh build`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, `git diff --check` passed; logs artifacts/terminal-status-{check,build,clippy,fmt,diff}.log.

Unfinished items and boundaries: static screenshots cannot prove the actual breathing effect of the three-dot pulse, degradation under the system Reduce Motion setting, or the readability of the 2 pt line at different font sizes and narrow sidebar widths, and no full manual keyboard / screen-reader walkthrough was done; the 7501 policy was not validated with a long task from real cargo / terraform / a coding agent, and long-term CPU / redraw behavior under high-frequency 9;4 updates was not validated. Preview still does not connect these two protocols and is not used as evidence. OSC 133 prompt lifecycle is not connected, so working / blocked currently clear automatically only on process exit and RIS; the record list, per-app icons, and window-level aggregation are left for later. Real program support and long-term daily use still require manual acceptance.

## 2026-11-15 · OSC support inventory and PWD / OSC 22 / OSC 133 mapping

Uncommitted changes based on `2f1ae0c` (including the sidebar status and progress from the previous section). Added [OSC integration and terminal status design](../design/osc-status.md), listing item by item which OSCs libghostty-vt 1.0.1 recognizes, handles internally, requires an application callback for, or does not support / connect. The application adds three mappings:

- OSC 7 / ConEmu 9;9 / iTerm2 1337 CurrentDir: decode `file://` URIs and bare absolute paths, rejecting remote hosts, query/fragment, control characters, and overlong values; write to `SessionInfo.currentCwd`, and the UI uses the runtime `Tab.liveCwd` to update the toolbar, Cmd-K search, group path suggestions, new terminals created from that tab, and the next save. `Tab.cwd` remains the startup directory, and the group cwd is unchanged.
- OSC 22: the safe wrapper reads `DATA_MOUSE_SHAPE`, and `GhosttyEmulator` publishes only when the shape changes; `SessionInfo.mouseShape` reaches the host cursor of `SessionTerminal`, and the grid and scrollbar each keep their own cursor.
- OSC 133: register `SEMANTIC_PROMPT`, and in the callback copy kind, prompt kind, exit code, command, error, and `CURSOR_AT_PROMPT`; `PROMPT_START` clears 7501's working/blocked, while `done`/`error` are kept until acknowledged; `SessionInfo.atPrompt` is runtime metadata only, with no new visible UI.

Validation:

- `cargo test -p termlevel-native`: 89/89, log artifacts/osc-rust.log. Added PWD URI / bare path validation, the PWD / OSC 22 / OSC 133 lifecycle under a real `/bin/sh`, and backend mapping cases.
- `cargo test -p termlevel-ghostty-vt --test native`: 25/25, log artifacts/osc-wrapper.log. Added all key OSC 22 shapes, empty value / reset, and the four OSC 133 kinds plus `cursor_at_prompt`.
- `./scripts/nsh test --junit artifacts/osc-headless.xml`: 186/186, log artifacts/osc-headless.log. Added two real PTY cases in tests/terminal/Osc.test.tsx: OSC 7 updating the toolbar / save / new terminal directory; OSC 133 clearing working and exposing prompt state, and OSC 22 exposing the shape.
- `./scripts/nsh test tests/terminal/Osc.test.tsx --target macos --junit artifacts/osc-native.xml`: 2/2, log artifacts/osc-native.log; the native host still has one existing link warning.
- `./scripts/test-native osc`: 1/1, log artifacts/osc-native-acceptance.log, JUnit artifacts/osc-native-acceptance.xml. Reviewed artifacts/native-osc-live-cwd.png (toolbar updated from the project directory to `/private/tmp`) and native-osc-prompt.png (working icon gone after OSC 133 A).
- `./scripts/nsh check`, `./scripts/nsh build`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, `git diff --check` passed; logs artifacts/osc-{check,build,clippy,fmt,diff}.log.

Unfinished items and boundaries: the actual NSCursor shape for OSC 22 was not verified in native screenshots (screenshots do not include the pointer), and NSH may update the cursor only on the next pointer move, so a person needs to run `printf '\033]22;pointer\007'` in the terminal and then move the pointer to confirm; isolation of the scrollbar / toolbar cursor likewise requires manual acceptance. OSC 1, 99, ConEmu non-progress keys, OSC 66/72/3008, other OSC 1337 keys, and the unsupported OSC 5522 authorization / MIME details remain unconnected per the inventory document. OSC 133 requires shell integration; the local `/bin/sh` case uses printf to simulate the protocol and does not mean the user's shell is configured. The historical record in the earlier validation section that "the toolbar is still the startup path / OSC 7 is not done" is kept as evidence of that time; this section and the current [architecture](../architecture/terminal.md) / [terminal tiling](../design/tiling.md) are authoritative. No user session was committed or restarted.

## 2026-11-15 · OSC status UI / UX redesign landed

Uncommitted worktree based on `c2f9488`, retaining the earlier uncommitted OSC and directory-cleanup changes. The design is consolidated in [OSC integration and terminal status design](../design/osc-status.md); the earlier status / OSC sections still record the historical evidence of the old implementation.

This change separates what the program reports from what the user has read: 7501 records carry a status generation, and `Sessions.acknowledge(id, observedRevision)` acknowledges only the presented version; blocked remains after viewing, read done/error no longer cover working, and repeated reports of the same state do not mark it unread again. Notifications changed to record-level transitions carrying record identity / generation, so different tasks with identical text are not missed just because the aggregate remains blocked. `TerminalStatusInfo` unifies lifecycle, activity, and real progress in Rust, and the Sidebar, tooltip, and accessibility share the summary; the fake full line for blocked/error and unknown pause is removed. A new prompt clears the previous round's 9;4, and shellPhase distinguishes unknown from a known phase. Native FocusRequest distinguishes user actions from automatic input focus, and remounting does not acknowledge results.

Automated evidence (real PTYs are all controlled `/bin/sh`, not preview, and not acceptance of actual daily agent use):

- `./scripts/nsh test --junit artifacts/status-redesign-headless.xml`: 192/192, log `artifacts/status-redesign-headless.log`. Includes 93 Rust application tests, 25 safe-wrapper native tests, ABI, doc tests, and TSX; the Rust native test names here do not represent foreground AppKit GUI acceptance.
- Six status TSX tests: progress consistency across both protocols, 9;4 remove / indeterminate, blocked remaining after viewing, error not covering new working after acknowledgement, unknown pause/error not inventing a percentage, and a production TerminalSurface remount preserving unread done. Log `artifacts/status-redesign-focused.log`, report `artifacts/status-redesign-focused.xml`.
- Rust added record-level notifications, an old-version acknowledgement not swallowing a new generation, acknowledgement preserving the original record, generations not reused after reset, prompt / exit clearing, 100% not equal to done, whole-record replacement, and 256-entry eviction; the full suite passed. An earlier standalone `cargo test -p termlevel-native --lib` was 92/92 (before the capacity test was added), log `artifacts/status-redesign-rust-unsandboxed.log`; the 93 tests in the final full run are the final code evidence.
- `./scripts/nsh check`, `./scripts/nsh build`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, `git diff --check` passed. Logs `artifacts/status-redesign-{check,build,clippy}.log`; fmt / diff were run directly with no separate logs. Local documentation link checks passed.

Foreground AppKit native automation, all run serially:

- `./scripts/test-native status`: 1/1, log `artifacts/status-redesign-native.log`, report `artifacts/status-native.xml`. Reviewed `native-status-progress.png`, `native-status-working.png`, `native-status-blocked.png`, `native-status-collapsed.png`, `native-status-blocked-no-progress.png`, `native-status-tooltip.png`, `native-status-blocked-read.png`, `native-status-error.png`, `native-status-done.png`, `native-status-paused.png` (all under artifacts). Confirmed expanded / collapsed states in a dark native window, real progress short line, blocked with no fake full line and the icon still present after viewing, failure / done icons, the static unknown pause track, and a readable two-line tooltip.
- `./scripts/test-native osc`: 1/1, log `artifacts/status-redesign-native-osc.log`, report `artifacts/osc-native-acceptance.xml`. Live cwd and new-prompt clearing of the active tab re-verified. The actual NSCursor shape for OSC 22 still needs manual inspection; screenshots cannot prove it.
- The native host build still has one existing link warning; a summary without detailed warning text was not inferred to be a new problem.

Failures during validation and their handling:

- The first Cargo build could not resolve `release.files.ghostty.org`. Subsequent commands used `GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip CARGO_NET_OFFLINE=true`, build.rs verifies the pinned SHA256, and library verification was not skipped.
- In the sandbox the existing `clipboard::tests::osc52_bytes_reach_the_pasteboard_as_unicode` failed with `pbcopy rejected the sample`: standalone Rust 91/92, first full run 189/190. After accessing the system pasteboard outside the sandbox the case passed; `artifacts/status-redesign-rust.log` is kept as a record and its assertions were not modified.
- During one validation round the FocusRequest event signature was changed while running, so the already-started old runner did not match the new JS and 9 items produced `Cannot convert undefined or null to object`; production source edits were stopped and after a rebuild the error disappeared. Log `artifacts/status-redesign-headless-before-focus-rebuild.log`.
- The new remount fixture only disposed at the end, and the leftover PTY caused duplicate `Terminal 1` in subsequent tiling tests; an explicit close was added, and the final state and consecutive tiling runs passed. Failure log `artifacts/status-redesign-headless-before-fixture-cleanup.log`.
- The first native status run wrongly queried an empty Text diagnostic node observable only in headless; it now queries the actual product label and takes a screenshot; read semantics are checked directly by Rust / headless, and the native screenshot confirms the icon remains. The first native OSC run still queried the old working copy; it was updated to the new readable label. Failure logs are `artifacts/status-redesign-native-fixture-diagnostic.log` and `artifacts/status-redesign-native-osc-old-label.log`; neither was fixed by raising timeouts.

Manual acceptance not done: continuous reporting from real cargo / terraform / agents; the system Reduce Motion switch and actual VoiceOver listening; full combinations of light/dark themes / multiple font sizes / long messages and pinned grouped tabs; long-term feel of window activation and split switching; high-frequency progress and multi-session long-running load. Native screenshot review was done this round, but the daily-use items above are not claimed to pass.

## 2026-11-15 · Notification row icons and content hierarchy

Uncommitted changes based on `efafe22`. Previously the notification row title, body, and actions had similar weight; this change extracts `ui/notifications/NotificationRow.tsx`: the actual message / body uses 15 pt semibold primary text; the task / event title is context, and source, program name, category, and time are reduced to secondary information. On the left a 32 pt tinted backing with a semantic icon is added (approval, question, sign-in, failure, ordinary message), and the bottom separately holds source jump, expand / collapse, and remove. Programs have no image attachments, so application brand icons are not fabricated. 7501 history appends Rust-sanitized kind/app/title/message metadata; the UI does not judge task category by parsing the generated title.

Long content provides a bounded summary and Show more / Show less; when expanded, long primary text uses 12 pt regular weight, and the full history text is retained. History still sizes the viewport from the measured height of the first three entries, keeping separators and a scroll hint; the panel width is capped at window width minus 16 pt, with a default width of 420 pt and history height up to 384 pt. Opening / clearing / returning to the source does not change existing session ownership or program state acknowledgement rules.

Validation (real PTYs are controlled `/bin/sh`; the "128 tests" in the screenshots is a test message sample, not this round's test count):

- `./scripts/nsh test --junit artifacts/notification-row-headless.xml`: 194/194, log `artifacts/notification-row-headless.log`. Includes Rust checks for structured kind/app/message, existing status and workspace regressions, and six notification TSX tests. Afterwards only the weight / size of expanded long primary text was adjusted, and the relevant six tests and native acceptance were re-verified separately.
- `./scripts/nsh test tests/terminal/Notifications.test.tsx --junit artifacts/notification-row-focused.xml`: 6/6, log `artifacts/notification-row-focused.log`. Covers background source jump, closed source, input warning, scrolling with more than three entries, exit information, program-requested primary text / semantic icon, and long text expand / scroll / collapse. In the first extended case the body fit fully in the viewport and should not scroll; changed to long body text that actually exceeds the viewport to verify scrolling, without raising timeouts or relaxing product assertions.
- `./scripts/test-native notifications`: 2/2, log `artifacts/notification-row-native.log`, report `artifacts/notifications-native.xml`. Covers ordinary message, task failure, needs approval, question, and sign-in in dark and light; 1000×720, 640×420, 360×420 layouts, empty state, long body expansion, and collapse at the end. Reviewed `native-notifications-{dark,light}.png`, `native-notifications-{dark,light}-compact.png`, `native-notifications-light-narrow.png`, `native-notifications-{dark,light}-empty.png`, `native-notifications-expanded.png`, `native-notifications-expanded-end.png` under artifacts; the actual message stands out, semantic icons are distinguishable, and the panel and actions are usable in narrow windows.
- `./scripts/nsh build`, `./scripts/nsh check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, `git diff --check` passed. Logs `artifacts/notification-row-{build,check,clippy}.log`; fmt / diff were run directly.

Build and tests used the verified offline library: `GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip CARGO_NET_OFFLINE=true`. The full suite ran outside the sandbox for the existing system pasteboard case; the native host still reports one existing link warning, and no new Rust / TSX check failures were introduced. Native screenshot review is not equivalent to VoiceOver listening, large system fonts, or real agent daily-use acceptance.

## 2026-11-15 · Configurable working indicator

The working state was previously fixed as a three-segment line breathing animation. After viewing the candidates the user judged the three-segment line to have poor visual legibility, and it was removed from the options; Spectrum and Equalizer were too similar, so Spectrum was replaced with Vinyl (a turntable). Reduce Motion shows a static version of the selected shape. Settings → Interface now offers `Working indicator`: Equalizer (three bars, default), Vinyl (turntable), Wave (three dots), Beat (beat ring). The choice is written to the workspace storage as `workingIndicator`; missing, unknown, old `dashes` / `spectrum`, or old documents all fall back to `bars`, without bumping the snapshot version. The indicator color follows the folder color of the group the tab belongs to, falling back to `primary` when ungrouped. It plays only when `working` and there is no progress; with progress it still shows the static icon and progress line, and blocked / done / error are unchanged.

`ui/shared/WorkingIndicator.tsx` uses NSH `createMotion` `scaleY` / `translateY` / `rotate` keyframes at 30 fps, with no image assets; each sub-animation calls `stop()` on unmount. The four options in the settings page are themselves live previews, and animation pauses when the dialog closes.

- `./scripts/nsh test --junit artifacts/working-indicator-tests.xml`: **199 / 199**, log `artifacts/working-indicator-tests.log`. Added snapshot tests covering missing / unknown / valid values and old `dashes` / `spectrum` fallback, and persistence tests covering rewriting after unknown-value fallback and writing `vinyl` / `beat` after interface selection.
- Native screenshots: `artifacts/working-indicator-{bars,vinyl,wave,beat}.png` and `artifacts/working-indicator-settings.png`. A temporary fixture with a purple group confirmed that the sidebar indicator uses the group purple rather than primary; the four settings options and live previews were visible. The temporary fixture and test were deleted, log `artifacts/working-indicator-native.log`. `artifacts/working-indicator-{dashes,spectrum}.png` are historical screenshots of the candidates before removal and no longer represent current options.
- `./scripts/nsh check`, `./scripts/nsh build`, `git diff --check` passed; the native host still has the existing `__eh_frame` link warning.
- The settings page preview is fixed to primary and does not follow the active tab's group color; the actual sidebar renders with each tab's own group color. Animation feel, real Codex working round-trips, and long-term CPU usage have not been manually accepted by the user.

## 2026-11-18 · OSC 7501 idle agent icons

Uncommitted changes based on `a96c125` (the workspace also has separate terminal renderer changes). The sidebar shows the corresponding product icon for explicitly idle Codex, Claude Code, Grok, Pi, and OpenCode; other active states keep the previous priority. The icon does not prove the process is alive, and cleanup depends on protocol events and the existing lifecycle.

- `./scripts/nsh check`, `./scripts/nsh build`: passed. Used `GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip CARGO_NET_OFFLINE=true`.
- `./scripts/nsh test tests/terminal/Status.test.tsx`: 7/7, controlled `/bin/sh` real PTY covering agent mapping, working, unknown app, prompt, and clear.
- `./scripts/nsh test`: 272/273; `tests/workspace/Tiling.test.tsx:154` expects `Shown in the layout`, which the current UI source does not contain, and an individual rerun also failed. The assertion is unrelated to the agent icons, but a clean baseline was not rerun this time, so it cannot be asserted whether the historical baseline passed or failed.
- `./scripts/test-native status --filter 'native sidebar shows idle agent icons'`: 1/1; starting the local service inside the sandbox was denied, and it passed outside the sandbox. Reviewed `artifacts/native-agent-{codex,claude,grok,pi,opencode}.png`, covering the dark theme; the test also covers the collapsed sidebar.
- Grok Build 1.0.88 used an isolated temporary configuration and a signed-out state: with `[ui.notifications] program_status = "auto"`, the native Termlevel real PTY completed the query and showed `blocked/auth` and `Sign in to Grok`; a separate PTY probe captured `app=grok`. The initial probe that did not capture the report had written the configuration in the wrong place and cannot be used as evidence that the protocol is unsupported. The full lifecycle of real Grok idle, working, done, exit, and post-sign-in remains unverified.
- Not completed: the full interactive lifecycle of the five real agents, light theme, pinned tabs, actual VoiceOver listening, and long-term daily-use manual acceptance. Real programs may not send idle, or an intermediate layer may not forward the report; without clear / prompt the report may go stale. Pi's use of OSC 133 may also clear valid state; see the design.

Entries were translated from the consolidated validation log; failures above remain part of the record and must not be erased.


## 2026-11-18 · OSC 7501 agent identity across status updates

Uncommitted changes based on `8a60213`. See [agent icon behavior](../design/osc-status.md#scan-hierarchy). Builds used `GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip` and `CARGO_NET_OFFLINE=true`.

### Commands and real PTY tests

- `./scripts/nsh check` and `./scripts/nsh build` passed. Logs are `artifacts/osc-agent-identity-check.log` and `artifacts/osc-agent-identity-build.log`.
- `cargo test -p termlevel-native sessions:: --lib` passed 30/30. Log is `artifacts/osc-agent-identity-rust.log`.
- `./scripts/nsh test tests/terminal/Status.test.tsx` passed 9/9. Controlled real PTY reports switched the same tab from Codex to Claude Code. The suite verified identity through a missing app, an unknown app, result acknowledgement and an OSC 133 prompt. It also checked scoped clear, terminal reset and a foreground job returning to the shell without clear. Log is `artifacts/osc-agent-identity-headless.log`.

### Native automation

- `./scripts/test-native status --filter 'native sidebar shows idle agent icons'` passed 1/1 after the sandbox denied the local dev server with `Operation not permitted (os error 1)`. Both runs are recorded in `artifacts/osc-agent-identity-native.log`.
- Reviewed `artifacts/native-agent-codex.png` and `artifacts/native-agent-claude.png`. The same native tab showed the Codex mark and then the Claude Code mark. The existing native linker and Grok SVG warnings were present.

### Unverified

Interactive agent binaries, SSH and multiplexer forwarding, and manual GUI acceptance were outside these controlled tests.
