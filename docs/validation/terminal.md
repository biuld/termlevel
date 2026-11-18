# Validation: Terminal

## Entry index

| Date | Entry |
|---|---|
| 2026-11-12 | First Alacritty core integration |
| 2026-11-12 | Alacritty + real PTY integration complete |
| 2026-11-12 | Rust grid view and native composition input |
| 2026-11-14 | Terminal content search and scrollback limit |
| 2026-11-14 | libghostty-vt prebuilt Rust crates |
| 2026-11-14 | Ghostty wired into the default app |
| 2026-11-14 | Fixed terminal grid replacement |
| 2026-11-14 | Application-owned AppKit renderer embedded in NSH |
| 2026-11-14 | Per-session SurfaceController |
| 2026-11-15 | Presentation starvation under continuous synchronized output |
| 2026-11-15 | Run seams in CJK highlight backgrounds |
| 2026-11-15 | Platform boundary cleanup for the terminal renderer |
| 2026-11-15 | Broken graphic glyphs and missing blank-cell backgrounds |
| 2026-11-15 | Mojibake when copying Chinese through OSC52 |
| 2026-11-15 | Local images and file drops into the terminal |
| 2026-11-15 | Terminal identity setting |
| 2026-11-15 | Input queue backpressure and warning recovery |
| 2026-11-15 | History scrollbar and cross-screen drag selection |
| 2026-11-15 | Codex pet disappearance and the synchronized-refresh watchdog |
| 2026-11-15 | Input echo stalls during chunked image transfer |
| 2026-11-16 | Bundled terminal fonts and programming ligatures |
| 2026-11-16 | Clipped fallback glyphs for circled digits |
| 2026-11-16 | Clipboard test boundary |
| 2026-11-18 | Junction thickness in thin terminal table lines |
| 2026-11-18 | Built-in terminal graphic glyphs |

## 2026-11-12 · First Alacritty core integration

Prior UI adjustments were committed first: `ea2d600` (Refine sidebar hierarchy and use solid UI surfaces). On top of that commit this round integrated crates.io `alacritty_terminal =0.26.0`, added `nsh-base =0.5.0` (only `term`), and kept all NSH dependencies pinned at 0.5.0. The native `TerminalEngine` at the app root installs an emulator factory, and the existing preview terminal now actually uses the Rust Alacritty parser/grid; this is not merely adding a dependency. The transport is still a deterministic demo transport that creates no PTY or shell. `SessionManager` and the production lifecycle separated from the view are not yet implemented.

| Check | Result |
|---|---|
| `cargo fmt --check` | Passed |
| `./scripts/nsh check` | Passed |
| `./scripts/nsh build` | Passed; generated native declarations and the app bundle |
| `./scripts/nsh test --junit artifacts/alacritty-tests.xml` | 24 passed, 0 failed: 8 Rust + 14 workspace + 2 core binding interaction |
| `git diff --check` | Passed |
| `./scripts/nsh test App.tsx --target macos` | 1 native step scenario passed; preview transport only |

Rust tests verified cross-byte UTF-8/escape sequences and combining characters, wide-character placeholder conversion, ANSI/truecolor and styles, background blanks, bounded history order, alternate screen restore, width-scaling reflow, title/bell/cursor query replies, input modes, palette modification and reset, multi-emulator isolation, and reset. The NSH binding test sends REP (`CSI b`), which the default VT does not support, and verifies repeated characters, so a factory that failed to take effect cannot fall back silently; it also verifies truecolor, Chinese combining characters, the alternate screen, resize, application cursor keys, bracketed paste, title and protocol reply paths.

The native step entered `echo alacritty-native-output` (a preview command, not a shell), collapsed the sidebar, hovered to switch tabs and switched back, then switched to light. Screenshots are in the ignored `artifacts/alacritty-expanded.png`, `artifacts/alacritty-collapsed.png` and `artifacts/alacritty-light.png`, all reviewed. Compared with the earlier solid-surface screenshot at the same window size, preview content no longer kept narrow-column wrapping; this is reflow evidence in this scenario and cannot be generalized to all Unicode/font scenarios.

Explicit gaps this round: NSH's `TerminalEmulator` only expresses two input modes, a simple cursor and line styles, and cannot fully carry Alacritty's mouse reporting, focus reporting, modern keyboard protocols, cursor shapes, hyperlinks and multiple underline styles; the latter temporarily degrade to a plain underline. Color settings overrides can be displayed, but color queries and pixel-size query replies have no complete path. OSC52 and the Kitty keyboard protocol are explicitly disabled. These unconnected capabilities must not be treated as supported.

The native host still reported an `SF Pro Text` fallback to a system font, plus the existing link warning. Not verified: real zsh/fish, Neovim/tmux, IME, selection copy, high-volume output performance, process close and leaks, and Rust session rebinding after a hot update.

## 2026-11-12 · Alacritty + real PTY integration complete

Based on UI commit `ea2d600`, the current workspace opens the user's real login shell by default. Preview is only enabled explicitly by `TERMLEVEL_PREVIEW=1`, and the preview interpreter still executes no system commands. Rust `SessionManager` strongly owns the PTY, the Alacritty parser/grid and the child process; the native view binds by stable session ID, unmounting does not end the process, and a closed ID is not implicitly restarted. All traffic stays in Rust: reader → parser/grid → native row view, native input → writer → PTY; TSX receives only low-frequency metadata and bell events.

Code is organized in the NSH mixed style: `App.tsx` only mounts `ui/WorkspaceApp.tsx`; `src/lib.rs` only registers native exports; Runtime, DialogKeys, session management, API, core adaptation and the native view are separate modules. The structure references [NSH Project structure](https://nsh.dev/docs/project-structure/) and [Building with Rust](https://nsh.dev/docs/rust/), without introducing multiple crates or a web frontend.

### Current workspace checks

| Command | Current result |
|---|---|
| `cargo fmt --check` | Passed |
| `./scripts/nsh check` | Rust + TSX passed |
| `./scripts/nsh build` | Passed |
| `./scripts/nsh test --junit artifacts/alacritty-live-tests.xml` | **40 passed, 0 failed**: 21 Rust + 19 JS interaction |
| `./scripts/nsh test terminal/Session.test.tsx --target macos --junit artifacts/alacritty-native-tests.xml` | **3 passed, 0 failed** |
| `git diff --check` | Passed |
| `cargo tree --prefix none --offline` | nsh-base/core/macros/updater all 0.5.0 |

Parsing the two JUnit reports confirmed 40 and 3 testcases respectively, with no failure/error. Reports and screenshots are in the ignored `artifacts/`, and generated directories are not committed.

### Behavior evidence

- Rust core tests: cross-chunk UTF-8/escape sequences, combining and wide characters, colors/styles/background blanks, bounded history order, alternate screen, reflow, cursor/input modes, title/bell/reply, palette modification/reset, multi-emulator isolation, and atomic commit and timeout of synchronized output.
- Real PTY tests: Chinese text under sh and zsh, actual `stty size`, title and bell, Ctrl+C ending a foreground `sleep` while keeping the shell, exit codes and output retention, session isolation and rebinding, startup errors for invalid cwd/program, bounded history and completion marker after 4000 lines of output, closing foreground tasks and ordinary background jobs, and worker-thread exit after manager shutdown. The zsh tests used `-f` and `/dev/null` history to avoid depending on personal configuration.
- Protocol replies were received by real child processes and read back with `od` for verification: DSR cursor position and the OSC default background color query. The default color is set to the UI theme before the reader starts, so a query before the first draw does not get a wrong color. The physical pixel size is still unknown; pixel-size query replies are 0.
- Synchronized output was sent by a real child process as an incomplete mode 2026 sequence; while the process was still blocked waiting for input the screen could commit on timeout, and EOF also forced a commit of residual output. During resize the parser lock covers the PTY ioctl and grid adjustment, so output at the new size does not land in the old grid first.
- Three native AppKit scenarios: (1) Chinese paste, shrinking the viewport, unmounting/remounting the same session, continuing input and the exit code; (2) creating/closing tabs in the production workspace, collapsing the sidebar and switching appearance while output is preserved; (3) destroying/rebuilding the whole TSX workspace and rebinding the same session from the Rust snapshot to continue executing commands. The fixture explicitly selects a real `/bin/sh`, so a normal `./scripts/nsh test` does not need to change the user's SHELL and does not write to personal fish history. When closing the last window the native host can disconnect before the close request arrives; the test accepts this expected disconnect only in an explicit close operation.
- The default `App.tsx` was additionally run with the user's real fish: `fish_history='' ./scripts/nsh test App.tsx --target macos`, executing `printf` and a real `pwd`, verifying Chinese output, rename, sidebar collapse/expand, creating a new terminal and closing another, switching to light, and finally closing the session to show an empty workspace. Reviewed `artifacts/alacritty-fish-live.png`, `alacritty-fish-collapsed.png`, `alacritty-fish-light.png`; real shell and demo output are clearly distinguished.
- Native window-close cleanup was probed separately: a real `/bin/sh` wrote its own PID to `artifacts/native-shell.pid`; the native main window was closed; the driver scenario passed. Subsequently `os.kill(pid, 0)` returned ProcessLookupError, confirming test shell PID 38371 had exited. The Rust reaper is serialized with close: on Unix it first HUP's the shell to clean ordinary jobs, and after 50 ms force-terminates if still alive; app exit waits for `Sessions.settled()` within the NSH 200 ms budget.

### Compatibility boundaries

This round completed the core and the real-session data/lifecycle path, which is not the same as the Alacritty application renderer or full terminal compatibility. NSH 0.5.0 still needs M3 to fill in mouse reporting, focus reporting, complete function keys/modifiers, modern keyboard protocols, cursor shapes and hyperlink metadata. The Kitty keyboard protocol and OSC52 are explicitly disabled, and underline variants project to a plain underline. The toolbar directory is the initial startup directory and does not pretend to be the live shell cwd.

Chinese input/paste automation does not prove acceptance of the real IME candidate window, complex emoji, or selection copy; the 4000-line Rust output test does not prove GUI throughput and frame times. No nvim/tmux executable was found on the machine, so they were neither installed nor claimed fully compatible this round. Windows/Linux were not accepted. The native host still has the existing `SF Pro Text` system-font fallback notice and link warning, with no new compiler warnings.

App restart ends sessions; layout/settings persistence remains M1.1. What is confirmed is rebinding of views and workspaces within the same native host, not background persistence or restoration across an app restart. The integration changes are still in the working tree and were not mixed with the prior UI commit.

## 2026-11-12 · Rust grid view and native composition input

Real sessions switched from the default `nsh-base::Terminal` to `SessionTerminal`'s own Rust grid row view and native Input. Preview continues to use the original NSH Terminal. The NSH CLI and all `nsh-*` remain pinned at 0.5.0; no upstream upgrade was awaited or introduced. The lifecycles of the session manager, PTY and parser remain independent.

The input control stays stably mounted; its position uses the Alacritty current cursor, scroll offset and bottom alignment margin; text, styles and character widths are displayed by grid column number. During composition the native control shows marked text and pauses terminal key bindings. Ordinary characters first confirm the platform value, then clear it in NSH `after_answer`, avoiding an identical empty-attribute diff being skipped. Composition end sends only the final delta; cancel deletes the marked text without writing to the PTY. When switching tabs, AppKit's blur confirmation still enters the original session.

| Command | Result |
|---|---|
| `./scripts/nsh check` | Rust + TSX passed |
| `./scripts/nsh test --junit artifacts/native-input-tests.xml` | 46 passed: 27 Rust + 19 JS |
| `./scripts/nsh test terminal/Session.test.tsx --target macos --junit artifacts/native-session-tests.xml` | 3 passed |
| `./scripts/test-native-input` | 1 complete AppKit scenario passed, report `artifacts/native-ime-tests.xml` |
| `./scripts/nsh build` | Passed |
| `cargo fmt --check` / `git diff --check` | Passed |

The native input script uses NSH's macOS development host, a real `/bin/sh` PTY, and the `NSTextInputClient` interface of the actual AppKit field editor:

- `setMarkedText` injects pinyin and the converted Chinese; in the Rust parser snapshot both are verified not to have entered the PTY, while the native editor keeps the marked text.
- `insertText` confirms and then runs `printf`, verifying the actual shell output contains the complete Chinese and was committed only once; the input control ends up empty. Cancel replaces the marked range with an empty string, verifying pinyin did not leak into the PTY, after which ordinary commands still execute.
- `firstRectForCharacterRange` verifies the candidate anchor moves horizontally with the command prefix and vertically with two lines of shell output; covers scrollback and viewport resize. These coordinates come from the native input interface, not TSX-simulated coordinates.
- Switching between two still-mounted real sessions during composition: the new session does not contain the old session's Chinese; the new session can execute commands, and after switching back the original session can continue input.
- After changing the font, `stty size` is executed and the real PTY's rows/cols verified to shrink accordingly; the size is driven jointly by the viewport, font size/character width and line height, and does not depend on the window happening to perform another layout.
- Rust tests additionally verify rapid consecutive input, delayed clearing, repeated Han characters, cancel with an old prefix; explicit column positions for CJK and combining characters; and input anchor retrieval while a TUI cursor is hidden.

The NSH 0.5.0 test driver has no `compose()`, so `scripts/test-native-input` temporarily registers a native test file and deletes it at the end; the case source stays in `terminal/fixtures/Input.acceptance.tsx` and is not mixed into the default headless tests. `src/session_view/native_probe.rs` is registered only in macOS debug builds and does not expose input-injection interfaces to the production terminal API.

Reviewed `artifacts/native-ime-preedit.png` and `native-ime-after.png`: the first shows Chinese marked text at the command-line cursor, the second shows the history, cursor and font-size changes after shrinking the window. Reports and screenshots remain in the ignored artifacts directory.

Boundaries: this round verified the native input protocol, display and PTY commit; it did not operate a real system input-method candidate list. Actual pinyin/Japanese/Korean candidate windows, full interaction of Escape/arrow keys with the input method, long pre-edit at line end, complex emoji, and cross-line selection/copy still require manual acceptance. Mouse reporting, complete function keys/modifiers and modern keyboard protocols are still missing, and Neovim/tmux are not claimed fully compatible. The native host still has the existing SF Pro Text fallback and link warning.

### Real-device feedback fix: production padding pushed the input method up and left

The user's production window screenshot exposed `padding-terminal: 16px`, which the earlier native fixture did not include. The grid list previously participated in the padding layout, while the absolutely positioned Input was positioned from the outer origin; earlier tests only compared the relative movement of the candidate anchor against the actual grid cursor, not its page coordinates, so this error was missed.

The fixture was first given the same padding as `TerminalSurface`, and the actually mounted Input and grid cursor were read through the debug probe's `ElementRef.measure()`: before the fix `(446.5,19)` versus `(462.5,35)`, 16 pt off both horizontally and vertically, and the assertion failed. The fix establishes a shared content container inside the host padding so the list and Input share the origin, without hardcoded compensation for theme values.

After the fix, across four stages — ordinary input, 65 lines of scrollback, shrunken window, increased font size — the page coordinates were `(462.5,35)`, `(252,585)`, `(252,245)` and `(319.5,241)`, with Input and grid cursor exactly matching each time. The expanded `./scripts/test-native-input` passed, and `artifacts/native-ime-preedit.png` with production padding was reviewed again, with marked text on the same baseline as the command line. This native layout/input-protocol evidence is still distinguished from manual acceptance of the real system candidate list.

### Removing the terminal top margin

In the production window, the clipping line of full-screen output stopped about 16 pt below the toolbar, with empty terminal background in between. This was the top edge of `padding-terminal`. The terminal now keeps only left, right and bottom padding, and scrolled content clips at the lower edge of the toolbar. Input and grid still share this content container's origin.

Native window screenshot pixels at 1100×740, scale 2: toolbar text at about 13–22 pt, the terminal's first line of pixels starting at 36 pt, line spacing 21 pt. Before the change the first line at the same position started at 52 pt.

### Sidebar drag grouping

Dragging a tab vertically reorders it within the sidebar, or drags it onto a group header to place it in that group. Dropping onto a tab in an existing group adds it to that group; dropping onto a tab without a group forms a new group, with the group name and path taken from the tab that was dropped on, without opening the group editor. A terminal created inside the group starts from the group path. During the drag, a highlighted group header means the drop targets that group, a highlighted tab means grouping with that tab, and a line between tabs means the insertion position. Releasing outside the sidebar, or sliding horizontally away, leaves groups unchanged. Dropping into a collapsed group expands it. Gaps between tabs do not insert an unpinned tab into the pinned area. Dragging does not change the session ID, and terminal output is preserved.

`./scripts/nsh test App.test.tsx` passed 16 items, two of which cover dropping into an expanded group, dropping into a collapsed group, and horizontal drag cancel. `./scripts/nsh check` passed. This is headless pointer driving; hit-testing and tracking in a native window by hand have not yet been accepted.

## 2026-11-14 · Terminal content search and scrollback limit

⌘F searches within the current terminal content, case-insensitively. In real sessions, matching, scrolling and highlighting stay in Rust and the grid is not sent into JS. Preview only shows the hit count. The scrollback limit is 10,000 lines, and the Terminal page in settings states this number. ⌘K still finds terminals by title, directory and group.

`./scripts/nsh check`, `./scripts/nsh build` and `git diff --check` passed. `./scripts/nsh test --filter search` passed 3 (including two Rust match tests and the existing "search opens a terminal…"). `./scripts/nsh test --filter scrollback_limit` passed 1. `./scripts/nsh test terminal/Find.test.tsx` passed 2: in preview, searching `preview` yields "N of N" and the find bar disappears after closing; the Terminal settings page contains `10,000`. This round did not rerun the full test suite and did not perform native AppKit acceptance. Highlighting and scrolling to a hit only exist on the real-session path and are invisible to preview tests.

## 2026-11-14 · libghostty-vt prebuilt Rust crates

Added two independent workspace packages, `termlevel-ghostty-vt-sys` / `termlevel-ghostty-vt`, pinned to official libghostty-vt **1.0.1**. The official XCFramework's SHA256 is `c0f9631277f726154c8f521954d17cdc41aac4626fc0df8cf925a19ee58e829b`; the sys layer downloads and verifies it and then extracts the macOS target-architecture static library. The original C headers and bindgen 0.72.1 generated declarations are both committed as source; the build does not run Zig or bindgen. Detailed interfaces and boundaries are in the [crate README](../../crates/ghostty-vt/README.md).

The verification environment was macOS arm64, Rust 1.99.0. Tests of both native crates call the real prebuilt library: 9 tests cover chunked UTF-8/escape sequences, independent terminals, the alternate screen, resize/reflow/history, reply/title/cwd/bell/reset, usability after snapshot release, wide characters/combining characters/truecolor/underline/cursor, legacy/Kitty keys, mouse/focus/paste encoding and invalid parameters. One more uses Clang to compare the size and alignment of 7 key C structs against Rust, and 3 doctests include `!Send` / `!Sync` compile-failure constraints.

The build path additionally verified:

- After clearing the archive/offline environment overrides, running `cargo test -p termlevel-ghostty-vt-sys -p termlevel-ghostty-vt` automatically downloads the official library, verifies it and links statically.
- With the package missing, `GHOSTTY_VT_OFFLINE=1 CARGO_NET_OFFLINE=true` correctly refuses to download; with a specified corrupted zip it correctly fails with a SHA256 mismatch.
- Specifying an already verified zip via `GHOSTTY_VT_ARCHIVE` lets the build complete in a network-restricted environment. The cache lives in each Cargo build's `OUT_DIR`; different build configurations may need the archive supplied again or downloaded again.
- Regenerating bindings and helpers produced exactly identical content; 36 vendored headers match the official XCFramework byte for byte.
- `cargo package --allow-dirty -p termlevel-ghostty-vt-sys` passes standalone packaging and verification compilation; the source package is about 153 KiB compressed and contains no prebuilt binaries. The package is not yet published.

Project regression: `./scripts/nsh check` and `./scripts/nsh build` passed; `./scripts/nsh test --junit artifacts/ghostty-crates-headless.xml` passed **80 / 80**, including the new crates' native/doctests and the existing Rust/TSX tests. Under restricted network, the already downloaded zip was used via `GHOSTTY_VT_ARCHIVE` with `CARGO_NET_OFFLINE=true`. Both new packages passed `cargo clippy --all-targets -- -D warnings`, targeted `cargo fmt --check` and `git diff --check`.

The first full run found that the existing `large_output_stays_bounded_and_finishes_before_view_attaches` still asserted the old 3,000-line default. The test now explicitly configures 3,000 lines, checks the boundary by that limit and the actual screen line count, and keeps the oldest-line eviction and newest-line retention assertions; the production default of 10,000 lines is unchanged. After the fix the full rerun produced the above 80 / 80.

This round did not wire Ghostty into the production `SessionManager` / NSH surface and did not perform native GUI acceptance. The x86_64 target is configured, but the corresponding Rust target is not installed, so no Rust build or run for that architecture has been performed; Windows/Linux are not integrated yet. Safety-layer capabilities such as search, selection, hyperlinks, images, clipboard and synchronized-output timeout still need to be filled in, so production terminal compatibility cannot be claimed complete on this basis.

## 2026-11-14 · Ghostty wired into the default app

Both the default `SessionManager` and the explicit preview provider switched to official libghostty-vt 1.0.1, and the app's Alacritty dependency and old adapter implementation were removed. Rust continues to own the session/PTY/reader/writer/reaper, and the UI still binds by stable session ID. The active screen uses a Rust RenderState frame cache, with history lines read on demand; the complete grid or output stream is not sent to JS.

The safety layer gained native search, color defaults and palette, size query replies, history lines and exact logical windows, boundary snapshots for synchronized output, and active commit. Beyond native page-granular history, each screen's tracked reference preserves the exact logical origin; shrinking and growing again does not resurrect old lines. Synchronized output is handled on normal end, 150 ms timeout, EOF and resize; regression also covers that after a hold snapshot consumes dirty state, another renderer must not miss earlier modifications, and that the logical window stays strictly bounded when the same chunk produced history before the hold began.

After review the threading contract was adjusted to Send / !Sync, and the app serializes parsing, rendering, queries and input encoding through the same `Session.emulator` mutex; the contract source and description are in [THREADING.md](../../crates/ghostty-vt/THREADING.md). Cross-thread moves of the native terminal and renderer have separate tests. Keys, final IME commit text, paste and focus events are encoded by Ghostty; the native Input's composition commit/cancel and stable-mount mechanism are retained. The view now consumes block / bar / underline cursor geometry.

### Automated regression

- `GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip CARGO_NET_OFFLINE=true ./scripts/nsh check` and `./scripts/nsh build` passed. The path is a locally verified official zip; another path to the same version may be substituted.
- `./scripts/nsh test --junit artifacts/ghostty-integration-headless.xml`: **85 / 85** passed, using the same archive/offline environment. Includes the standalone C ABI / safe crate, all old terminal regressions migrated to Ghostty, real PTY, UI rebinding, multi-pane input isolation and all TSX interaction.
- New real raw PTY byte comparison: the app-encoded SS3 arrow keys, Ctrl+Shift+F5, Chinese, bracketed paste preserving newlines and focus reporting totaled 42 bytes, byte-for-byte identical to what the child process actually read via `dd | od`. The first test version wrote od's double space as a single space and timed out; it now waits for the reader to drain, parses hex tokens and compares the complete byte array, without changing encoding behavior for the test.
- `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, `git diff --check` and `sh -n` on the new scripts passed.

### macOS AppKit

The following native tests all connect to a real PTY and do not use the preview transport. The first restricted sandbox prevented the local dev server from listening; they passed after the local development server and AppKit host were allowed.

- `./scripts/test-native-input`: 1 / 1 passed. Chinese marked text, commit once, cancel, rapid input, commit attribution after switching sessions, Input/cursor coordinate alignment after scrolling and font-size changes. Report `artifacts/native-ime-tests.xml`.
- `./scripts/test-native-tiling`: 1 / 1 passed. Real dual PTY, continuing composition while swapping panes, confirming text still goes to the original session, directional focus, divider resizing, maximize/restore and narrow-window restore. Report `artifacts/tiling-native.xml`; the two-pane screenshot was reviewed.
- Newly added repeatable `./scripts/test-native-ghostty`: 2 / 2 passed, fixture `terminal/fixtures/Ghostty.acceptance.tsx`. It opens the default `App.tsx` directly, uses the machine's default fish, and under Dark / Light confirms Chinese command execution, real search count/highlight/jump, and `seq 1 200 | /usr/bin/less -R` jumping to the end, showing 200 / (END), and restoring shell content after exit. Report `artifacts/ghostty-production-native.xml`. Screenshots include `artifacts/ghostty-production-{dark,light}{,-find,-less}.png`; the dark search and light less screenshots were reviewed. The script's temporary registration file is cleaned up on exit; screenshots/reports are ignored local artifacts.

### Not yet verified and not connected

The above proves the core migration, real input and specified native flows on macOS arm64, and does not represent full daily-use compatibility. The mouse encoder is not yet connected to the UI; images/hyperlinks/terminal selection gestures and system clipboard protocols are not yet integrated; fine underline variants still degrade to a plain underline. NSH bindings lack complete key release / repeat action / physical layout information, so the full Kitty keyboard protocol cannot be claimed. Real system IME candidate lists, complex emoji, cross-line selection copy and sustained-output / many-session performance still need separate acceptance. Neovim / tmux were not found on the current PATH, so no actual compatibility testing of these TUIs was done; the limited less flow above passed. x86_64 and other platforms are not yet accepted.

## 2026-11-14 · Fixed terminal grid replacement

Working directory `/Users/biu/Projects/termlevel`, `main` based on an uncommitted working tree at `4e8c7ff`. Existing handoff changes were kept; nothing was committed or pushed. NSH remains pinned at 0.5.0, libghostty-vt is 1.0.1; the machine's `/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip` and `CARGO_NET_OFFLINE=true` were used.

The production surface removed virtual_list, per-line canvas, PaintedScreen, image_strip, the wheel overlay and the list scroll API, replacing them with two canvases, three image layers, an independent cursor and the original field editor. The measurement box is separate from the full-cell drawing rect, and history is positioned with integer windows and Ghostty tracked anchors. Inside the lock only owned-in-memory Frame/history cells/images and the color context are taken; outside the lock, runs are merged, semantic text is assembled and the Painter is recorded. Search became a native ASCII stepped feed/tick, Unicode in 64-line batches, with SearchChange providing asynchronous counts; hidden panes do not capture frames on new output. Design and limits are in [the terminal design](../design/terminal.md).

### Automation

- `./scripts/nsh check` and `./scripts/nsh build` passed.
- `./scripts/nsh test --junit artifacts/terminal-grid-tests.xml`: **102 / 102** passed. Includes real PTY, workspace rebinding, multi-pane input isolation, Unicode asynchronous search and replacement of old queries, plus standalone C ABI / safe crate tests.
- New real-library evidence: history anchors follow narrowing/widening reflow, alternate screen and eviction; a held window freezes screen identity and history; selection invalidates snapshots; active images share Frame pixels; pure-window following and sub-line trackpad accumulation; independent begin_path/clip per run; the out-of-lock second stage still emits the old snapshot after the terminal was modified and destroyed.
- On this machine with pinned 1.0.1, **resize ends the hold**. A new test first proves switching to the alternate screen still preserves the hold, then proves that after resize the new screen and size are committed; this corrected the old handoff assumption that "the hold is still kept after resize".
- `cargo clippy --workspace --all-targets --offline -- -D warnings`, `cargo fmt --all -- --check` and `git diff --check` passed.

### macOS AppKit itemized acceptance

- `./scripts/test-native-input`: 1 / 1 passed; the actual field editor's marked text, commit/cancel, session switching, and input and cursor coordinates after scrolling/resize/font-size changes. Report `artifacts/native-ime-tests.xml`. This is not acceptance of the user's actual system IME candidate window.
- `./scripts/test-native-protocols`: final full re-verification **3 / 3** passed. Actual Kitty press/repeat/release, PNG, OSC8 text, drag-select copy, Select All, OSC52 and SGR mouse reporting/reading settings. Report `artifacts/protocols-native.xml`. Reviewed `native-terminal-graphics.png` and `native-terminal-selection.png`; three-layer overlapping images and animation stress still need separate acceptance.
- `./scripts/test-native-ghostty`: 2 / 2 passed. Default production app, default user shell, 1200×780, Dark/Light, covering stepped search, less to the end and exit, and ED2 clear. Asserts that after clear the visible area contains GRID_CLEAR_OK and does not contain old GHOSTTY_PROD_ lines. Report `artifacts/ghostty-production-native.xml`. Reviewed `ghostty-production-dark-clear.png` and `ghostty-production-light-less.png`; the former has no thin history strip at the top, the latter shows 200 / (END).

The first native script did not reach interaction because the sandbox could not listen for the local development server; acceptance completed after permission to start the local NSH server and AppKit window was granted. Early text assertions depended on per-line Text views and now read canvas semantic content; CanvasNode provides visible-line hit metadata but does not mount NSH row views. Custom unmarked NSEvents were filtered by the NSH test host, so that temporary debugging method was removed; the final drag-selection acceptance uses NSH's own driver and actually reads the copy result. Test failures were not bypassed by setting the emulator selection directly.

### Not yet proven

Codex continuous scrolling FPS, input latency and native CoreText replay were not measured; performance compliance cannot be inferred from 102 tests or Painter recording time. Debug sampling once recorded individual presentation updates over 8 ms, after which host run merging was moved outside the lock; this is not a claim that the overall frame budget is met. Native capture still copies the viewport.

The actual system IME candidate list, complex emoji/fallback baselines/ligatures, three-layer image overlap and animation, multi-pane sustained output, real trackpad feel and accessibility daily use still need manual verification. Users can run `./scripts/nsh run --window-size 1200x780` for daily acceptance. Kitty file/temp/shared-memory remain off, the pane toolbar is still the startup path, and OSC7 is not implemented. Screenshots and reports are all under the ignored artifacts directory.

## 2026-11-14 · Application-owned AppKit renderer embedded in NSH

Continued on the same uncommitted working tree on `main` / `4e8c7ff`; existing dirty changes were kept, nothing committed or pushed. NSH source was not modified, the CLI and all `nsh-*` remain pinned at **0.5.0**, and libghostty-vt remains **1.0.1**.

### Implementation boundaries

Using NSH's public `#[native_components(macos)]` + `NativeComponent`, `TermlevelTerminalSurface` is registered, and Termlevel Rust itself draws the grid and three image layers with CoreText / Core Graphics. NSH keeps the workspace layout, containers, independent cursor and the original single Input / field editor; it does not use the Ghostty application's internal embedding ABI, and it is not a full frame converted to RGBA and handed to NSH. The two drawing canvases and the NSH image subtree were removed; the remaining canvas **only carries CanvasNode semantic metadata with no drawing operations**, so the NSH test driver can locate visible lines.

The engine publishes the latest `Arc<Scene>`, and the native view takes the scene slot lock only briefly while cloning that Arc; drawing does not hold the emulator lock. Frame attributes crossing into NSH contain only the Rust scene slot ID / revision. The native component does not become first responder and `hitTest` returns nil, so cell events and the original Ghostty input encoding stay on the same path. Fonts / CTLines have a bounded cache, images are cached by owned pixel identity, and the mount lifecycle does not own the PTY.

### Verification performed

Same archive / offline environment:

```sh
GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip
CARGO_NET_OFFLINE=true
```

- `./scripts/nsh check` and `./scripts/nsh build` passed, logs `artifacts/native-surface-{check,build}.log`.
- `./scripts/nsh test --junit artifacts/native-surface-tests.xml`: final **106 / 106** passed. New tests cover eviction and most-recent-use retention when the cache reaches 2048 entries; native bitmap tests cover the three image layer order, source-region clipping and explicit backgrounds; font and CTLine caches are reused across redraws/themes and invalidated by font changes; latest-frame overwrite and slot release; and the semantic adapter recording no drawing operations.
- `cargo clippy --workspace --all-targets -- -D warnings` passed, log `artifacts/native-surface-clippy.log`.
- `./scripts/test-native-input -v`: **1 / 1** passed, log `artifacts/native-surface-ime.log`. Actual marked text commit/cancel, session switching and input anchor; records input/grid coordinates both as `(319.5, 234)`. Not manual acceptance of the actual system IME candidate list.
- `./scripts/test-native-protocols -v`: **3 / 3** passed, log `artifacts/native-surface-protocols.log`. Actual Kitty keys, PNG, OSC8 text, drag-select / Select All copy, OSC52, SGR mouse reporting. Reviewed `artifacts/native-terminal-graphics.png` drawn by the new view.
- `./scripts/test-native-ghostty -v`: **2 / 2** passed, log `artifacts/native-surface-ghostty.log`. Production workspace default shell, Dark / Light, search, less, ED2 clear; reviewed `artifacts/ghostty-production-dark-clear.png` from the new renderer, with no history strip.

### Native scroll performance comparison

`scripts/test-native-scroll` uses `terminal/fixtures/scroll_probe.py` in a real PTY: 1200×780, Menlo 13 pt, 21 pt line height, alternate screen, 1000/1006 mouse reporting, 2026 synchronized output, Chinese/English and colored runs. The wheel enters the actual grid's NSH wheel handler through the debug AppKit driver, then the original Ghostty encoder turns it into mouse bytes; no background direct feed bypasses the wheel and PTY. The fixture's JS interval is only an automated input generator; production data and drawing do not go through JS.

- Old Canvas path: `artifacts/scroll-before.log` and `artifacts/scroll-before.sample.txt`. The NSH frame meter showed about **38–44 / 60 FPS**, Canvas native replay averaged about **24–26 ms** with p95 about **44 ms**; CPU sampling hotspots included `Replay::text -> make_font` and CoreText font matching.
- Own NSView first round: `artifacts/scroll-native-surface.log`, native test **1 / 1** passed. Steady-state draw averaged about **2.6–2.9 ms**, the NSH frame meter mostly **54–59 / 60 FPS**, with some intervals around 50 FPS. The native component recorded from 8 up to 496 actual drawRect calls; it was not only updating test text without drawing. Reviewed `artifacts/native-scroll-end.png`; Chinese/English, colors and full-line positions were normal.
- 25-second scroll full re-verification: `artifacts/scroll-native-surface-sustained-final.log`, **1 / 1** passed. After the cache reached **2048 entries**, average draw was about **2.7–3.1 ms**, the later-stage NSH frame meter about **54–60 / 60 FPS**, and the native component accumulated **905** actual draws. The fixture explicitly asserts the actual native view draw count is greater than 10, so an empty component cannot pass.
- The first 25-second stress round already reached the 2048-entry cache limit and native drawing was still about 2.7–2.8 ms, but the test plus startup exceeded the default 30 seconds and was terminated by the tester; the long load was then given an explicit 60-second test budget, keeping the failure log `artifacts/scroll-native-surface-sustained.log`.
- Both groups are local macOS arm64 debug builds. NSH's `canvas=0` only means Canvas replay is no longer used; the new draw timings come from the native renderer's own statistics and cannot be used to claim drawing has zero cost. NSH FPS and terminal snapshot publication counts are not the same metric. The first-round performance numbers are observations of a controlled fixture, not a strictly isolated machine benchmark, and not a recording of actual Codex.

### Not yet proven

Real Codex continuous trackpad scrolling, input latency, the actual system IME candidate window, complex emoji / fallback / bold italic, animated image stress, multi-pane sustained output and fine-grained AX daily use still need manual acceptance. The current native renderer only implements macOS; other platforms need their own backend. The CoreText path is not called the Ghostty application's Metal / atlas renderer.

Run `./scripts/nsh run --window-size 1200x780` for daily verification. Kitty file/temp/shared-memory remain off; the pane title is still the startup path and OSC7 is not implemented; there is no footer, wheel cancel or framework patch.

## 2026-11-14 · Per-session SurfaceController

On an uncommitted working tree after `main` / `57a846e`, implemented [the terminal Surface controller](../design/terminal.md): one `SurfaceController` per session, a single 16 ms frame schedule, a single sampling entry point, removing the second clock in `view.rs` and `Grid`'s per-event capture. The input path does not queue: keys, IME, paste, press/release and reported wheel all encode synchronously on arrival; hover and host drag selection take the latest value and apply at most once per round; history wheel only updates the integer window. Reported wheel cancels the 32-line truncation, all integer displacement is encoded and the fractional remainder is kept. All mouse entry points (including AppKit middle/side buttons) go through `Session::mouse_reports`, deciding the mode and encoding under the same lock. Release acts as a barrier that applies the final cell synchronously, so a click that clears the selection cannot be re-selected by a pending endpoint.

### Automation

- `./scripts/nsh test --junit artifacts/surface-controller-tests.xml`: **112 / 112** passed. New surface state-machine tests cover: following while already at the bottom does not trigger sampling, following while not at the bottom samples only once, sub-line wheel accumulation, at most one capture per round, user-origin marking, and hover/drag selection keeping the latest value.
- `view.rs` pure-function tests: 65 / 33 line wheel displacement is not truncated; release uses the final cell rather than an intermediate endpoint.
- `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` and `git diff --check` passed.
- `./scripts/nsh check` and `./scripts/nsh build` passed.

### macOS AppKit itemized acceptance

- `./scripts/test-native-input -v`: **1 / 1** passed. Marked text commit / cancel; grid cursor coordinates and input anchor still agree.
- `./scripts/test-native-protocols -v`: **3 / 3** passed. Kitty keys, PNG, OSC8, drag-select copy / Select All, OSC52, SGR mouse reporting and reading settings.
- `./scripts/test-native-ghostty -v`: **2 / 2** passed. Production workspace default shell, Dark / Light, search, less, ED2 clear.
- `./scripts/test-native-tiling -v`: **1 / 1** passed. Multi-pane geometry, IME attribution and state preserved after splitting / moving.
- `./scripts/test-native-scroll -v`: **1 / 1** passed. Controlled mouse-reporting TUI scroll for 34 s; this run had native draw about 3.4–3.7 ms, NSH frame meter 55–57 / 60, cache 2048 lines. The values come from a single run, are not an isolated benchmark, and no causal conclusion is drawn against the variation in the 25-second round.

### Not yet proven

Real Codex feel, the actual system IME candidate window, cross-line selection, three-layer image animation and multi-pane sustained output still need user manual acceptance. `TERMLEVEL_SURFACE_STATS` counters only confirm sampling convergence and do not replace input latency or end-to-end frame budget measurements. A full writer queue keeps the existing visible error semantics and has not been turned into a separate transient state.

## 2026-11-15 · Presentation starvation under continuous synchronized output

Logs saved after the user actually ran Codex and Claude are in `artifacts/codex-claude-stutter-before.log` (originally `/tmp/termlevel-codex-stutter.log`). Line 44: within about 2 seconds there were 111 reported wheel events, 115 revision changes and 126 ticks, but 125 holds were skipped and only one was published; continuous wait for a publish reached 2100.8 ms. The log has no program labels, so it cannot be attributed separately to Codex / Claude; that duration also does not represent the duration of a single 2026 transaction.

The fix lets the bottom-following window advance to the complete owned boundary frame before the current hold begins, reusing the same Arc; the history window is frozen and does not read unfinished live cells / images / screen identity. The dirty bit is kept until the transaction ends or times out, to commit the last frame. See [the terminal design](../design/terminal.md).

This round's verification (macOS arm64 debug):

- `./scripts/nsh check` and `./scripts/nsh build` passed.
- `./scripts/nsh test --junit artifacts/sync-boundary-tests.xml`: **114 / 114**. New real Ghostty test with 160 consecutive "end previous transaction + start next transaction" cycles, each sampling still in hold, requiring a completed frame to be published every time; also verifies same-boundary reuse, unfinished text and screen-switch isolation. Existing history-window freeze and resize-commit cases remain passing.
- `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` and `git diff --check` passed.
- `./scripts/test-native-scroll -v`: **2 / 2**. Keeps the 25-second regular scroll and adds a continuous-transaction scenario with a real PTY / AppKit wheel, requiring the visible frame number to advance six consecutive times at 500 ms intervals while input continues. Log `artifacts/sync-boundary-native-scroll.log`, screenshot `artifacts/native-scroll-consecutive-sync.png`.
- The three roughly 2-second windows in the continuous-transaction scenario published **121 / 116 / 115** times, with the longest wait for advancement about **22 ms**; native draw averaged about **2.6–2.8 ms**. This is controlled reproduction verification, not an equal-load before/after benchmark of actual Codex / Claude, nor an end-to-end input latency measurement.
- Native regressions passed serially: `./scripts/test-native-input -v` **1 / 1** (composition commit / cancel, session switching and input anchor); `./scripts/test-native-protocols -v` **3 / 3** (keyboard, images, mouse reporting and clipboard); `./scripts/test-native-ghostty -v` **2 / 2** (production Dark / Light, shell, search, less and clear). Logs are all `artifacts/sync-boundary-native-{input,protocols,ghostty}.log`.

The first native test failed because the sandbox prohibited the dev server from listening; it passed on rerun after granting the permissions the native test needs. The test host link still has the existing `__eh_frame` too-large warning. The user's actual Codex / Claude feel after the fix and sustained multi-pane load have not been re-verified.

The user later retried and confirmed "it works"; this round's Codex / Claude scroll stutter fix received manual confirmation. That confirmation does not extend to sustained multi-pane, complex input methods, or full terminal compatibility acceptance.

## 2026-11-15 · Run seams in CJK highlight backgrounds

In the user's screenshot the blue highlight showed thin vertical seams at Chinese text and style boundaries. The old renderer filled the background per text run, and fractional cell widths plus geometric antialiasing left insufficient coverage where rectangles met. Now consecutive same-color backgrounds within each line are merged independently (including inverse and host selection), shared boundaries are rounded uniformly in CGContext device space, geometric antialiasing is disabled only during the background phase, and the drawing state is restored afterward. Text layout and image layering keep their existing responsibilities.

This round's verification (macOS arm64 debug):

- `./scripts/nsh check` and `./scripts/nsh build` passed.
- `./scripts/nsh test --junit artifacts/background-seams-tests.xml`: **116 / 116**, log `artifacts/background-seams-tests.log`. New native RGBA bitmap cases check pixel by pixel at 1× / 2× backing scale, fractional cell width/height and origin offset for solid coverage across Chinese / ASCII segmentation, adjacent lines, explicit background / inverse / selection and adjacent rectangles of different colors; existing three-layer image and source-clipping cases pass.
- `./scripts/test-native-protocols -v`: **4 / 4**. Added a mixed Chinese / ASCII / bold highlighted list in a real PTY; screenshot `artifacts/native-continuous-backgrounds.png` reviewed: the blue band is continuous with no segmented vertical seams from the original screenshot. Existing Kitty keyboard, image, selection copy, mouse and OSC52 regressions passed; log `artifacts/background-seams-native.log`.
- `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` and `git diff --check` passed.

The native test used the local test service permission needed outside the sandbox; the host link still has the existing `__eh_frame` warning. The above is controlled bitmap and real native window verification; the actual Codex resume screen from the user's screenshot has not yet been re-verified by the user.

## 2026-11-15 · Platform boundary cleanup for the terminal renderer

The old `native_surface.rs` was split into platform-independent `presentation.rs` (latest scene slot / weak registry / attribute binding), `platform/macos/view.rs` (NSView host) and `platform/macos/renderer.rs` (CoreText / Core Graphics drawing and caches). The former `keyboard.rs` and `native_probe.rs` moved into the macOS backend as well. The generic `view.rs` reaches the surface and input through `platform` and no longer contains macOS conditional compilation or references to a concrete backend; only the crate-root host registration entry needs platform types. The concrete contract for a new backend is in [the terminal architecture](../architecture/terminal.md).

Source comparison confirmed the renderer body, keyboard and native probe keep their original logic, with only module visibility and reference paths changed. Shared publication tests moved out of the macOS renderer, and token parsing, host slot ownership and token invalidation after the last release were added. Session lifecycle, the synchronized-output fix, background pixel boundaries, NSH 0.5.0 and target-platform dependency versions were all unchanged.

This round's verification (macOS arm64):

- `./scripts/nsh check` and `./scripts/nsh build` passed.
- `./scripts/nsh test --junit artifacts/platform-boundary-tests.xml`: **116 / 116**, log `artifacts/platform-boundary-tests.log`.
- Native scripts passed serially: Input **1 / 1**, Protocols **4 / 4**, production Ghostty Dark / Light **2 / 2**, Scroll **2 / 2** (including consecutive sync transactions). Logs are `artifacts/platform-boundary-native-{input,protocols,ghostty,scroll}.log`. Reviewed the updated `artifacts/native-continuous-backgrounds.png`; the highlight band is continuous.
- `cargo clippy --workspace --all-targets -- -D warnings`, fmt and diff check passed.
- `cargo check --release -p termlevel-native` passed; current diagnostic code still produces unused imports / dead code warnings, and release is not claimed warning-free.

Only the macOS backend passed the build and native acceptance this time. The non-macOS `unsupported.rs` is an explicit placeholder and has not been compiled for a target platform; Ghostty prebuilt extraction still supports macOS only. The system clipboard, native workspace shortcuts and builds/packaging for other platforms remain follow-up work.

## 2026-11-15 · Broken graphic glyphs and missing blank-cell backgrounds

The user provided comparison screenshots of Termlevel and a normal terminal for Codex / Claude: block-character icons were broken vertically, drawn lines showed seams, and the prompt-area background followed only the text length, forming a staircase. Diagnosis used a real PTY outputting the same Unicode characters plus `EL` / `ED` sequences with background colors; screenshot `artifacts/glyph-audit-before.png` confirmed the reproduction.

The background root cause was in the safe wrapper: it read only the raw style and missed the RGB / palette background content tag of blank packed cells. A new real-library test failed before the fix at row 0, column 1 of the active frame (actual Default, expected RGB), log `artifacts/erased-background-red.log`. Now the active frame, hold callback frames and history lines share `Style::with_cell_background`, and palette indices are kept so they follow OSC4 updates; the renderer no longer has to guess the color of blank regions.

Graphic characters are no longer drawn on the ordinary text baseline and line spacing. The macOS renderer reuses vector outlines selected from system monospace fonts (including CoreText fallback), mapped to the full pixel cell by a shared reference plane for the block / line-drawing families, avoiding raster hinting shrinking the edges; half blocks and quadrants keep their relative proportions. Ordinary text continues to use the user-selected font and the original text path. No hand-written character shape table was introduced, and the Ghostty application's internal sprite ABI was not integrated; the existing libghostty-vt 1.0.1 does not export that renderer. Reference planes and outlines are reused in the existing bounded renderer cache.

This round's verification (macOS arm64 debug):

- `./scripts/nsh check` and `./scripts/nsh build` passed.
- `./scripts/nsh test --junit artifacts/graphics-background-tests.xml`: **120 / 120**, log `artifacts/graphics-background-tests.log`. New coverage: RGB / palette blank backgrounds, EL / ED, active / held / history snapshots and OSC4; native bitmaps verify pixel by pixel the complete background, solid block coverage, half-block proportions, and thin / thick / double border connections across cells and rows. Graphic proportions are preserved when the ordinary font selection switches to Menlo / Monaco / monospace.
- Native regressions passed serially: Protocols **5 / 5**, production Ghostty Dark / Light **2 / 2**, Input **1 / 1**, Scroll **2 / 2** (keeping the consecutive sync transaction regression). Logs are `artifacts/graphics-background-native-{protocols,ghostty,input,scroll}.log`.
- Added `terminal/fixtures/glyph_probe.py`; native screenshot `artifacts/native-terminal-glyphs.png` reviewed: icons connect, block color bands and horizontal lines are continuous, and the prompt background includes blank lines and the part erased to end of line, forming a complete rectangle. This fixture starts no model and sends no API requests.
- `cargo clippy --workspace --all-targets -- -D warnings`, fmt and diff check passed.

User re-verification of the actual Codex / Claude fix has not been done. Other Unicode graphic families in font fallback, emoji, arbitrary fonts and full terminal compatibility on all platforms remain outside this round's acceptance scope; no claim of full Ghostty application renderer equivalence is made from controlled screenshots.

## 2026-11-15 · Mojibake when copying Chinese through OSC52

The user reported garbled text after copying Chinese in Claude. The cause was that SystemClipboard started `pbcopy` / `pbpaste` inheriting the host locale, and both choose byte encoding based on locale. The Rust / Ghostty interface passes UTF-8; a C or unrecognized locale makes pbcopy interpret those bytes under an old encoding. The fix pins `LANG`, `LC_CTYPE` and `LC_ALL` to macOS-supported `en_US.UTF-8` only in the helper child process, without changing the app, shell or user environment.

Added `tests/probes/clipboard_utf8_probe.py` and a native protocol case: a real PTY sends OSC52 with Chinese punctuation, accented text, combining characters, emoji and newlines, then queries OSC52 back; the complete Unicode string in the system clipboard is independently verified through AppKit `NSPasteboard.stringForType`, and the clipboard is saved / restored around the test. Checking only the helper round trip is insufficient, because both ends using the wrong encoding can still restore the original bytes.

Before the fix: `LC_ALL=C LANG=C LC_CTYPE=C ./scripts/test-native-protocols` was **5/6**; the new case failed, with AppKit actually reading `Claude ‰∏≠Êñá…`, while the OSC52 round trip still reported OK. Log artifacts/clipboard-utf8-before.log.

After the fix the same command was **6/6**, the Unicode string matched character by character, and the existing read-permission denied / allowed cases passed; log artifacts/clipboard-utf8-after.log. `./scripts/nsh check`, `./scripts/nsh build`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `git diff --check` passed.

This round did not rerun the full headless suite; the previous directory-cleanup record was 128/128. The fix was reproduced and verified through the same OSC52 system path; the user's current Claude session was not operated directly, and copying within real Claude still awaits user re-verification.

## 2026-11-15 · Local images and file drops into the terminal

`SessionTerminal` uses NSH 0.5.0's native `on_file_drop`; Rust decodes the local file URL and escapes it into a shell literal path, then feeds it through the existing `Session.paste` in drop order, without automatic Enter and without reading or uploading images. A visible inactive pane requests focus and then receives input; hidden, modal-blocked, composing and exited sessions ignore the drop. A new `inputBlocked` property distinguishes the blocked state from pane focus. Remote URLs and paths with control characters are rejected as a whole batch. The UI preview does not integrate file drops.

- `./scripts/nsh test tests/terminal/FileDrop.test.tsx --junit artifacts/file-drop-focused.xml`: **2/2**, headless + real /bin/sh, verifying Chinese spaces, multi-file order, single quotes and command substitution characters staying literal, no automatic submission, inactive pane focus, modal blocking and remote URL ignoring. Log artifacts/file-drop-focused.log.
- `cargo test -p termlevel-native session_view::file_drop::tests -- --nocapture`: **2/2**, verifying URL single decoding, order, escaping and rejection of invalid mixed batches; log artifacts/file-drop-rust.log.
- `./scripts/nsh check`, `./scripts/nsh build`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `git diff --check` passed.
- This round did not rerun the full suite. NSH 0.5.0's macOS test driver explicitly does not support dropFiles (only headless / Linux), so native Finder drops or Claude attachment recognition are not claimed accepted. Manual checks should cover dropping one / multiple images from Finder, Chinese and space-containing file names, split targets and dialog blocking; browser images without a local file URL and clipboard bitmaps are outside this implementation.

## 2026-11-15 · Terminal identity setting

Based on uncommitted changes on `55a0afdc335d13a5befd053c5fa7f11346986500`. Settings → Terminal adds two choices, Termlevel default / Ghostty compatibility; saved with the workspace v1 document, falling back to the default when the old field is missing or unknown. The identity is passed into Rust on every explicit open and only affects new processes; Ghostty mode removes the version variable and keeps xterm-256color / truecolor. The user's Codex configuration and private pet assets were not changed.

Commands used `GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip CARGO_NET_OFFLINE=true`:

- `./scripts/nsh test --junit artifacts/terminal-identity-headless.xml`: 136/136, passing Rust, document and headless TSX tests, including real PTY identity isolation, old session retention, unknown-value fallback and settings storage. Log artifacts/terminal-identity-headless.log. A first development check omitted a new parameter already called by existing Rust tests; it passed after the parameter was added. A new UI test initially targeted multi-session input at the whole panel and was changed to locate by session ID, after which it passed.
- After the full run, a test for restoring shells was added and the selected-state styling adjusted. `./scripts/nsh test tests/terminal/Session.test.tsx --filter 'terminal identity' --junit artifacts/terminal-identity-focused.xml`: 2/2, real /bin/sh verifying the old process environment is unchanged after switching identity, new sessions use the selected identity, and a saved Ghostty identity takes effect before the restored shell is created; log artifacts/terminal-identity-focused.log.
- With final styling, `./scripts/nsh test tests/workspace/Persistence.test.tsx --filter 'terminal identity' --junit artifacts/terminal-identity-persistence.xml`: 1/1, log artifacts/terminal-identity-persistence.log.
- Native AppKit automation: copied Session.test.tsx temporarily to the root, corrected the fixture relative path and took a screenshot after selecting the identity, running `./scripts/nsh test .native-identity.test.tsx --filter 'terminal identity' --target macos --junit artifacts/terminal-identity-native.xml`: 2/2. The temporary driver was deleted; the committed case can be rerun with `./scripts/nsh test tests/terminal/Session.test.tsx --filter 'terminal identity' --target macos`. Log artifacts/terminal-identity-native.log; reviewed artifacts/terminal-identity-ghostty-native.png, with a clear selected state and no clipping of controls or description. The sandbox first prohibited the dev server from listening; native acceptance passed after the permission flow. The host build still has one link warning.
- Final `./scripts/nsh check`, `./scripts/nsh build`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `git diff --check` passed; the corresponding check/build/clippy logs are in artifacts/terminal-identity-*.log.

Preview executes no real commands and accesses no real workspace storage. The real PTY and AppKit results above do not represent Codex Pets acceptance: no real Codex was started to check the picker, continuous animation, resize, pane switching and cleanup; the user's shell startup files and multiplexer environment may still affect the client's final detection. Manual daily acceptance is recorded separately.

## 2026-11-15 · Input queue backpressure and warning recovery

Based on the uncommitted state on `55a0afdc335d13a5befd053c5fa7f11346986500`, plus the same-day terminal identity change. The user's screenshot showed `Terminal input unavailable: sending on a full channel`. Code confirmed that the old queue was limited to 128 messages and that a rejected `try_send` permanently set state=error; the screenshot itself is not bound to a triggering event, so whether that instance was a keyboard, mouse or protocol-reply burst cannot be determined.

Rust Session's exclusive InputQueue changed to a FIFO total-byte budget: 4 MiB pending (including in-flight chunks), 1 MiB per single input, 8 KiB merged write blocks. The producer does not wait for the PTY and will not block the UI / reader to eliminate queueing errors. Over-limit input is rejected as a whole before copying, and a separate `inputWarning` clearly states it was not sent, without overwriting real errors; the warning clears only after input accepted after the rejection is successfully written to the PTY, so old in-flight writes cannot clear it by mistake. Real read/write failures still report errors. Close / exit clears the queue and wakes the writer, keeping the Session lifecycle and default identity setting.

All verification commands used `GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip CARGO_NET_OFFLINE=true`:

- `cargo test -p termlevel-native sessions:: -- --nocapture`: 18/18, passing real PTY session and queue tests. Log artifacts/input-queue-sessions.log. Deterministic tests cover 20,000 single-byte inputs preserving order, 4 MiB saturation (including in-flight bytes), rejection without partial writes, old writes not clearing a new warning, recovery of subsequent input, and close waking the writer.
- Added tests/probes/input_backpressure_probe.py: a raw PTY continuously outputs 20,000 cursor queries, reads input only after sending is finished, and verifies all 120,000 reply bytes and their order byte by byte. Both the direct Rust probe and the UI launch passed. This controlled client is not the real Codex from the user's screenshot.
- `./scripts/nsh test --junit artifacts/input-queue-headless.xml`: 144/144, passing Rust / doc tests / headless TSX, including two InputBackpressure tests: an over-limit paste warning and subsequent input recovery with a real /bin/sh, and the query burst above. Log artifacts/input-queue-headless.log. The first focused test was 1/2: the workspace shell started in home and could not find the relative probe path; using the ProtocolsFixture started from the project directory made the full run pass, and the first log is kept as artifacts/input-queue-ui.log.
- `./scripts/nsh test tests/terminal/InputBackpressure.test.tsx --target macos --junit artifacts/input-queue-native.xml`: 2/2, the same two behaviors passing in a native AppKit window with a real PTY. Log artifacts/input-queue-native.log. The host build still has one link warning.
- `./scripts/nsh check`, `./scripts/nsh build`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `git diff --check` all passed. The check/build/clippy logs are artifacts/input-queue-{check,build,clippy}.log.

The terminal the user was using was not closed or restarted. The Rust change takes effect after an app restart; the running instance is not hot-swapped. Sustained daily use of real Codex and the specific event source in the screenshot have not been reproduced; the automation above is not treated as manual acceptance. Finite memory cannot guarantee unlimited acceptance for a client that never reads input; input truly over budget is still explicitly rejected and requires a retry, without silent dropping or unbounded growth.

## 2026-11-15 · History scrollbar and cross-screen drag selection

Based on uncommitted changes on `0f8fe2b`. The real terminal previously had no history scrollbar, the drag selection endpoint was clamped within the visible grid, and there was no edge continuous scrolling. Code also confirmed that motion's history line already included the origin, yet the controller added the origin again, affecting history drag selection and link hit-testing.

Local cells and absolute history lines are now separated; after movement the NSH responder captures an already-started host gesture, keeps the initial press event and receives drag / release outside the grid. SurfaceController updates the single Window, edge scrolling and the Ghostty tracked selection endpoint within the existing frame schedule, sampling once per round. The new scrollbar uses WindowSnap's origin / count / rows directly; the reserved gutter no longer changes column count when history appears or the alternate screen switches. Navigation does not clear the selection; mouse release / cancel / modal / hide / resize stop tracking. Scrollbar clicks and capture release are each finished separately, and a framework-cancelled click cannot be treated as a physical mouse release. The synchronized-output hold decision and selection updates share the emulator lock, so the PTY cannot start a new transaction between them.

- Pure Rust covers scrollbar top/bottom bounds / midpoint inverse calculation and clipping, degeneration with no history, edge continuous speed, no catch-up jump after a pause, consecutive motion preserving sub-line accumulation, and reverse clearing of the remainder; Window cases distinguish local rows from absolute rows when the history origin is nonzero.
- tests/terminal/Selection.test.tsx uses a real /bin/sh outputting 800 lines containing Chinese, verifying continued scrolling while resting at the top edge, stop after release, selection preserved after the scrollbar reaches the top / bottom, automatic scrolling at the bottom edge, and the alternate screen hiding the history scrollbar. Reading the selection uses only the debug `NativeInputProbe.selection` and does not modify the system clipboard; the production path does not send selection or grid data to JS.
- The two native AppKit acceptances in `./scripts/test-native selection` verify upward cross-screen drag selection (visible sample about 140 lines), stop on release, selection preserved by slider navigation, and downward cross-screen drag selection from the top of history. Reviewed artifacts/native-selection-autoscroll.png and native-selection-history-top.png, confirming highlight, history content and slider position correspond; the other screenshots are native-selection-history-bottom.png and native-selection-autoscroll-down.png.
- `./scripts/test-native protocols`: 6/6, log artifacts/selection-protocols-native.log; Kitty keyboard, program SGR mouse reporting, OSC8 / selection copy / images and OSC52 behavior preserved. The native build still has one link warning.

Failures during development verification are kept: the first headless drag located the outer SessionTerminal, and only after switching to the real grid did the case cover the correct entry point; natively delivered self-built mouse events were filtered by NSH test isolation, that probe was deleted and NSH's own AppKit drag was used. One full run ran in parallel with scripts/test-native, mistakenly discovered a temporary native test in the root directory and changed a synthetic input switch; that invalid run was stopped (exit 143), log artifacts/selection-headless-contaminated.log; development now explicitly forbids this kind of parallelism. A later full run was 157/159: when the click was cancelled by the framework the scrollbar stopped tracking too early; after the fix to a real mouse-up / responder-release the focused test passed, keeping artifacts/selection-headless-before-release-fix.{log,xml}.

The automation does not replace long-term feel acceptance with a real mouse / trackpad. Double-click word selection and triple-click line selection semantics are still in use and have not been extended to word / whole-line drag selection; concurrent history eviction, out-of-window focus loss and long multi-pane load still need daily re-verification. Neovim / tmux are not claimed fully compatible. The user's terminal was not committed or restarted.

Final verification used `GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip CARGO_NET_OFFLINE=true`:

- `./scripts/nsh test --junit artifacts/selection-headless.xml`: 159/159, log artifacts/selection-headless.log. After the full run, applying the final coordinates when the slider is released inside the viewport was added; the focused tests below cover that version.
- `./scripts/nsh test tests/terminal/Selection.test.tsx --junit artifacts/selection-focused.xml`: 3/3, log artifacts/selection-focused.log.
- `./scripts/test-native selection`: 2/2, log artifacts/selection-native.log, covering cross-screen selection in both directions and viewport navigation / selection preservation. Headless additionally verifies continued scrolling while the mouse rests at the edge; native verifies sustained dragging and release through NSH's own drag path.
- `./scripts/nsh check`, `./scripts/nsh build`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` and `git diff --check` passed. Logs artifacts/selection-check.log, selection-scroll-build.log, selection-clippy.log. One link warning in the native build remains.

### Same-day fix: scrollbar changed to an overlay

As confirmed by the user, the history scrollbar changed from an in-grid side-by-side gutter to an overlay element outside the terminal. The default hit width is 10px and the pane right margin 2px, provided by theme.json; text padding remains independent, and column calculation no longer subtracts scrollbar width. It appears on scroll / viewport change / entering the right hit area, stays visible during hover, slider drag or text drag selection, and fades out after 180ms idle over 180ms. The fade only updates the Rust slider's opacity and does not change the grid, PTY size or tracked selection. The existing uncommitted drag-selection changes were kept; the user's terminal was not automatically committed to or restarted.

Final verification all used the fixed Ghostty archive and offline environment described above:

- `./scripts/nsh test tests/terminal/Selection.test.tsx --junit artifacts/scrollbar-overlay-focused.xml`: 4/4, log artifacts/scrollbar-overlay-focused.log. New real PTY column-count checks: the column count is the same with no history, when history appears, on hover, after idle fade and before/after slider navigation; existing cross-screen drag selection / selection preservation / alternate screen cases continue to pass. The fixture now uses the same horizontal and bottom text padding as production.
- `./scripts/test-native selection`: 3/3, log artifacts/scrollbar-overlay-native.log. Reviewed artifacts/native-scrollbar-overlay-hover.png and native-scrollbar-overlay-idle.png; the slider sits near the pane's right edge, appears on hover and disappears after the pointer leaves, and the text position and rows/columns stay consistent; the native sample is always 110 columns. The debug getter's unused-import warning in that run was removed, and final strict Clippy passed; the native host's existing link warning remains.
- `./scripts/nsh test --junit artifacts/scrollbar-overlay-headless.xml`: 160/160, log artifacts/scrollbar-overlay-headless.log. Run serially with the native script, with no temporary root-directory test interference.
- `./scripts/nsh check`, `./scripts/nsh build`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` and `git diff --check` passed; logs artifacts/scrollbar-overlay-{check,build,clippy}.log.

The overlay approach supersedes the reserved-gutter approach in the history above; current behavior is defined by the architecture and the code. Manual long-term feel acceptance remains separate from the automation above.

## 2026-11-15 · Codex pet disappearance and the synchronized-refresh watchdog

Based on uncommitted changes on `4090aa6`. Real Codex 0.169.0's bottom-right Kitty pet deletes the old image first, then transfers the new image in chunks; the old watchdog started a fixed 150 ms timer at mode 2026 and would commit the in-transaction state even while the transfer was still ongoing. Native per-frame logs actually captured `SYNC_TIMEOUT force=false` followed by `DRAW images=0 frameimages=0`, then recovery to 1. Early commit could also expose a cursor temporarily hidden during the transaction.

The watchdog now commits only after the last non-empty PTY output has been silent for 150 ms, with a single hold lasting at most 1 s. The normal end marker, EOF and resize still release through the original paths; boundary snapshots and session lifecycle are unchanged.

- Deterministic regression: during a 250 ms chunked Kitty replacement, the old red pixels and the visible cursor are preserved, and green pixels appear after commit; silence release and the 1 s cap under continuous output are also verified. Temporarily restoring the deadline to the old fixed 150 ms makes `active_chunked_image_transfer_survives_the_initial_sync_deadline` fail during transfer; it passes after restoring the fix. Old-policy failure log: `artifacts/pets-watchdog-regression-before.log`.
- Real PTY / native automation: a temporary native case opened the production `WorkspaceFixture.tsx`, ran `TERM_PROGRAM=ghostty codex`, and observed idle for 20 s; no prompt was committed. Temporary per-draw image count and timeout diagnostics were removed. Before the fix, among 89 draw frames the pet appeared and then 2 frames had no image, with 23 timeouts; after the fix, among 110 draw frames there were no empty-image frames or timeouts. Logs `artifacts/pets-watchdog-{before,after}-draw.log`, `artifacts/pets-watchdog-native.log`; screenshot `artifacts/pets-trace.png`. The counts are bounded measurements and not equal to long-term daily acceptance.
- `cargo test -p termlevel-native terminal::tests -- --nocapture`: 31/31. Includes consecutive image replacement, chunked replacement, cursor visibility, idle timeout, continuous-output cap, adjacent synchronized transactions and history window freezing.

The installed `/Applications/Termlevel.app` was not replaced, and user manual acceptance is still pending; normal periodic cursor blinking is not used as evidence of pet disappearance. The first investigation mistook a test without cell pixel dimensions set for a hold snapshot losing images; after adding the dimensions that was ruled out; the final fix is based on the real native empty frames correlating with timeouts, and on the chunked regression that failed under the old watchdog.

Final checks: `./scripts/nsh check`, `./scripts/nsh build`, `cargo fmt --all -- --check` and `git diff --check` passed; `./scripts/nsh test --junit artifacts/pets-watchdog-tests.xml` ran outside the sandbox, 197/197 (`artifacts/pets-watchdog-tests.log`). `cargo clippy --workspace --all-targets -- -D warnings` initially could not download the official library due to sandbox network restrictions; it passed after setting `GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip CARGO_NET_OFFLINE=true` (`artifacts/pets-watchdog-clippy.log`). The native development host keeps one existing link warning.

## 2026-11-15 · Input echo stalls during chunked image transfer

Based on uncommitted changes on `ea48542`. The user reported in the actual Codex composer that after the previous synchronized-refresh watchdog fix, typing English or Chinese still took a while to appear. `ea48542` only adjusted the synchronized commit deadline; this round located the PTY reader's fixed `thread::sleep(4 ms)` after `WouldBlock`. While the Codex pet / image outputs in chunks, every gap between chunks can enter that branch, and the delay accumulates with the number of chunks.

The reader changed to `poll(POLLIN, 4 ms)`: it wakes immediately when the next chunk is readable, and when there is no data it still checks session close and the synchronized commit deadline with a 4 ms upper bound. The non-Unix branch keeps the original sleep.

Added `tests/terminal/fixtures/GraphicsInputFixture.tsx` and `tests/probes/graphics_input_probe.py`: a real PTY continuously outputs a chunked Kitty RGB image while the actual AppKit field editor performs marked text and Chinese/English commits, recording composition call and character echo times separately. The native case requires 24 marked calls under 150 ms and 8 echoes under 300 ms.

- Control (only restoring the reader to the fixed 4 ms sleep): the 8 echoes were 612 / 746 / 897 / 692 / 715 / 733 / 662 / 736 ms, failing the 300 ms assertion; log `artifacts/ime-reader-baseline-final.log`.
- After restoring `poll`: echo was 14–24 ms, and the maximum of 24 marked calls was 6 ms; `./scripts/test-native input -v` passed **2 / 2**, logs `artifacts/ime-reader-native-final.log`, report `artifacts/native-ime-tests.xml`.

The same investigation found that `native AppKit composition commits once…` already failed on clean `ea48542`: `firstRectForCharacterRange` for Chinese marked text returned `(0,0,0,0)`, while ASCII was normal; with the reader changes stashed it also failed, confirming it was unrelated to the reader. The previous "expand the field editor by composition content" version measured width only with the controlled `value`, while marked text exists inside the native field editor and the field was still 1 cell during Chinese composition. It now measures `value` and `composition_text` separately and uses the larger; after the fix the Chinese candidate anchor is `(845.5,937)`, on the same baseline as before composition, and the case passes. The first failure record is kept in `artifacts/ime-reader-composition.log`.

Regular checks: `./scripts/nsh check`, `./scripts/nsh build`, `./scripts/nsh test --junit artifacts/ime-reader-tests.xml` **197 / 197**, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` and `git diff --check` passed; logs `artifacts/ime-reader-{check,tests,build,clippy}.log`. The native host still has one existing `__eh_frame` link warning.

The controlled probe is not the actual Codex load and is not equal to manual acceptance of the system IME candidate window; the user's real-device feel, real IME candidate window position and other platforms have not been re-verified.

## 2026-11-16 · Bundled terminal fonts and programming ligatures

Based on the uncommitted workspace at `6fdd3a0`. The final choice is standard JetBrains Mono 2.304 (not NL): regular, bold, italic, bold italic; Chinese and missing symbols use Sarasa Term SC 1.0.43 regular/bold. Font files remain exactly as distributed upstream; versions, download URLs, SHA-256 and file sizes are in `assets/fonts/manifest.json`; both OFL licenses and the JetBrains attribution notice ship with the bundle.

- A native CoreText test confirmed all four Latin styles come from JetBrains Mono, and Chinese, the Chinese comma and ①②③ all come from Sarasa Term SC, with no extra fit needed at the default 13 pt. Inside the Sarasa font, ①②③ advance is 500/1000 em while Han characters are 1000/1000 em; Ghostty still allocates one column and two columns respectively.
- Ligature tests confirmed that `->`, `!=`, `=>`, `==`, `===`, `<=`, `>=` have actual glyph IDs different from independent per-character shaping, while total advance still equals the character count × single-cell width; covers all four primary font styles and trailing characters.
- `./scripts/nsh build` and `./scripts/nsh check` passed. `./scripts/nsh package --target macos --out artifacts/font-package` produced `artifacts/font-package/Termlevel.app` (about 117 MiB). The first iconutil conversion inside the sandbox failed; a retry outside the sandbox succeeded.
- The actual `.app`'s `Contents/Resources/nsh/assets/@termlevel-native/fonts/` contains six font files, licenses, the attribution notice and the source manifest; all files verified against the source manifest, confirming no leftover NL files. `codesign --verify --deep --strict` passed.
- The final installer was launch-checked with `TERMLEVEL_PREVIEW=1`, with no font loading errors, log `/tmp/termlevel-bundled-fonts-package-smoke.log`; this is preview-launch evidence only and not shell acceptance.
- The final full headless test `./scripts/nsh test --junit artifacts/bundled-fonts-headless.xml` was 245/246; the only failure was still the existing sidebar split assertion at `tests/workspace/Tiling.test.tsx:154`. Log `/tmp/termlevel-bundled-fonts-headless-final.log`. Rust font fallback and ligature tests, the default-font related UI and real PTY headless tests all passed.
- Final `cargo clippy --workspace --all-targets -- -D warnings` still reports only the unmodified `src/clipboard.rs:86` `items_after_test_module`; the lint was not suppressed.
- Real PTY native automation: `./scripts/test-native glyphs` 1/1, `./scripts/test-native input --filter 'native AppKit composition commits once'` 1/1, `./scripts/test-native selection` 3/3. Checked consecutive circled digits, Chinese/English adjacency, the last column, and regular/bold/italic ligatures in `artifacts/glyphs-native.png`. That screenshot now shows the final bundled font combination; no separate screenshot after the earlier temporary Menlo adaptation was kept; the original clipping reproduction is still in `artifacts/glyphs-before.png`.
- New workspaces default to the bundled fonts; existing font settings are kept. System fonts remain selectable. Ligatures are currently limited to same-style ASCII spans and do not cross color, selection or hyperlink boundaries; the user's real daily font preference and long-term load manual acceptance are still not done.

## 2026-11-16 · Clipped fallback glyphs for circled digits

Based on the uncommitted workspace at `6fdd3a0` (which also contains the repository discovery fix and group editor overlay restoration). Current host macOS arm64, NSH 0.5.0; test fonts were Menlo / Monaco / Courier New / Andale Mono / System Mono at sizes 11, 13 and 18 pt. The specific font configuration from the user's screenshot was still not obtained.

- Reproduced on a real `/bin/sh` PTY: `①②③`, ASCII / Chinese adjacency and the last column all showed right-side clipping. `artifacts/glyphs-before.png` is before the fix, `artifacts/glyphs-native.png` after; `TERMLEVEL_PREVIEW=1` was not used.
- CoreText measurement: Menlo 13 pt `0` advance is about 7.827 pt; ①②③ actually use HiraginoSans-W3, with ink x about 0.273 pt and width about 12.454 pt. Ghostty allocates one column per circled digit and two for Chinese. Temporary measurement output is in `/tmp/termlevel-glyph-metrics.log`.
- `renderer/shaping.rs` proportionally fits only glyphs that actually fall back and whose advance and ink both exceed the allocated range; the primary font baseline, terminal column width and selection data keep their original values. Adjacent blank cells are not borrowed. The specific source and trade-offs are in [the terminal design](../design/terminal.md).
- Regression tests cover five fonts, three font sizes, bold/italic, circled digits mixed with Chinese, and preservation of ordinary italic and combining accents.
- Native automation: `./scripts/test-native glyphs` 1/1, `./scripts/test-native selection` 3/3, `./scripts/test-native input --filter 'native AppKit composition commits once'` 1/1. The latter records that input/grid cursor coordinates agree, and covers Chinese composition commit, cancel and resize.
- `./scripts/nsh check`, `./scripts/nsh build`, format / diff checks passed. Strict Clippy was blocked by the unmodified `src/clipboard.rs:86` `items_after_test_module`; the lint was not relaxed.
- The final `./scripts/nsh test --junit artifacts/glyphs-headless.xml` was 243/244; the only failure was the previously recorded `tests/workspace/Tiling.test.tsx:154` (`a split is its own sidebar unit and can move into a tab group`). Log `/tmp/termlevel-glyphs-headless-final.log`; all renderer / glyph / real PTY tests passed. After the overlay restoration, the two cases that still called the old group menu were changed to the actual overlay buttons; behavior assertions were not changed.
- Native window screenshots were checked; manual GUI acceptance under the user's real configuration is still pending. Long-running load and complete glyph fitting for other complex emoji are outside this evidence.

## 2026-11-16 · Clipboard test boundary

- Targeted Rust clipboard helper tests 2/2, verifying UTF-8 bytes, the child-process encoding environment, size limits and invalid UTF-8 rejection through a stub process; the system clipboard is not accessed.
- Native automation: `./scripts/test-native protocols --filter 'OSC52 preserves Chinese'` 1/1; rerun with `LC_ALL=C LANG=C LC_CTYPE=C` also 1/1, verifying the real clipboard Unicode through AppKit and saving and restoring the clipboard. No manual GUI acceptance was performed.
- `./scripts/nsh check` and `./scripts/nsh build` passed. The full headless regression was stopped at the user's request, and no final result was obtained.

## 2026-11-18 · Junction thickness in thin terminal table lines

Based on uncommitted renderer changes on `9dea7e1`. The user-provided `┌─┬┐` / `├─┼┤` table was reproduced in a real PTY and native window: with antialiasing off, the straight segments measured 4 solid device pixels in the screenshot while junctions measured 2 solid pixels plus a pale edge. The horizontal cross-section of the same font-source outline was consistent; enabling antialiasing only for the line-drawing outline made both show 2 solid pixels plus a pale edge. The fix keeps font outlines, enables antialiasing for line drawing, keeps block characters off, and reverts the earlier temporary patch that drew line segments for 11 characters.

- `cargo test -p termlevel-native session_view::platform::macos::renderer`: 13/13, including box line connections, line widths, block characters, backgrounds and font cases.
- `./scripts/nsh check` and `./scripts/nsh build` passed; the build still has the existing Grok SVG `foreignObject` notice.
- `./scripts/test-native glyphs --filter 'native Unicode table'`: 1/1; reviewed `artifacts/native-table-border.png`, where sampled straight segments and junctions on the same row are both 2 solid device pixels plus a pale edge. Native screenshot comparisons are also kept in `artifacts/native-table-border-before.png` and `artifacts/native-table-border-aa.png`.
- `./scripts/test-native protocols --filter 'native block glyphs'`: 1/1. Used the verified `GHOSTTY_VT_ARCHIVE` offline library; the full regression was not run, manual acceptance at different scales, font sizes and themes was not done, and real Claude Code was not rerun.

## 2026-11-18 · Built-in terminal graphic glyphs

Based on uncommitted changes on `af33ec5`: box drawing, block characters and other supported terminal graphic characters use Alacritty's pixel glyph algorithm, while ordinary text goes through CoreText; the source and porting scope are in the built-in glyph module's NOTICE.

- `cargo test -p termlevel-native session_view::platform::macos::renderer`: 16/16, covering character ranges and multiple cell sizes, mask directions, half blocks, line connections, gradient blocks, fractional cell width line overlap and existing renderer behavior.
- `./scripts/nsh check`, `./scripts/nsh build` and `cargo clippy -p termlevel-native --lib -- -D warnings` passed; `./scripts/nsh test tests/terminal/Engine.test.tsx` 2/2. Used `GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip CARGO_NET_OFFLINE=true`. The build has the existing Grok SVG `foreignObject` notice.
- Confirmed that the build output `.nsh/out/assets/@termlevel-native/licenses/alacritty/` contains the upstream Apache-2.0 license and source notice.
- After the Termlevel app declared Apache-2.0, the license text is provided uniformly by the repository-root `LICENSE`; the license in the build output is byte-for-byte identical to the root file and preserves Alacritty's source and copyright notice. The two independent Ghostty wrappers continue to declare MIT.
- `./scripts/test-native glyphs`: 3/3; `./scripts/test-native protocols --filter 'native block glyphs'`: 1/1. Reviewed `artifacts/native-table-border.png` and `artifacts/native-builtin-glyphs.png`, including thick lines, double lines, rounded corners, diagonals and block characters. The real PTY is controlled shell output and is not equal to long-term real Claude Code use acceptance; complete manual native acceptance across font sizes / display scaling was not done.

Entries were translated from the consolidated validation log; failures above remain part of the record and must not be erased.
