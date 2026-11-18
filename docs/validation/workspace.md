# Validation: Workspace

## Entry index

| Date | Entry |
|---|---|
| 2026-11-12 | Sidebar surface and hierarchy adjustment |
| 2026-11-12 | Earlier failed native acceptance record |
| 2026-11-12 | Remove all glass effects |
| 2026-11-13 | Independent panes and unified window bar |
| 2026-11-13 | Fleet Dark / Light palette mapping |
| 2026-11-13 | Flat groups |
| 2026-11-13 | Guide line separation from tabs |
| 2026-11-13 | Remove terminal body top padding |
| 2026-11-13 | Directed multi-terminal tiling |
| 2026-11-13 | Title bar global buttons |
| 2026-11-13 | Tab group workspaces and clean exit |
| 2026-11-14 | Scrollback limit in settings |
| 2026-11-14 | Pane toolbar becomes search entry |
| 2026-11-15 | Workspace and settings persistence |
| 2026-11-15 | In-app notification panel |
| 2026-11-15 | Notification type scale unified to Interface body |
| 2026-11-16 | New terminal entry points and group context |
| 2026-11-16 | Ungrouped section and History module split |
| 2026-11-16 | Group-pinned terminals and status module cleanup |
| 2026-11-16 | Collapsed sidebar group buttons |
| 2026-11-16 | Ungrouped pinned terminal divider |
| 2026-11-16 | Collapsed sidebar Ungrouped selection hint |
| 2026-11-16 | Merge Appearance and Interface settings |
| 2026-11-16 | Settings window test fixture |
| 2026-11-16 | Collapsed sidebar selection background spacing |

## 2026-11-12 · Sidebar surface and hierarchy adjustment

The pinned expanded and collapsed sidebar switched to an opaque background close to the terminal, and the collapsed state removed the divider fill; the hover overlay keeps glass, right rounded corners and a shadow. Group names use a neutral color, group badges keep their color, guide lines have lower contrast, and the bottom separator was removed. The sidebar toggle uses the same circular, background-free normal state in both states.

- `./scripts/nsh build`, `./scripts/nsh check` and `git diff --check` passed.
- `./scripts/nsh test --junit artifacts/headless-tests.xml`: 14 passed, 0 failed. The group editor test was adjusted to check the accessible group badge color names, with the group name color staying neutral; it continues to verify name updates, session output and instance retention.
- Two `./scripts/nsh test App.tsx --target macos` native steps passed, and the dark expanded, dark collapsed, dark hover, light expanded and light collapsed screenshots were reviewed. The first restricted environment still reported the local server as `Operation not permitted`; it succeeded after the local development server was allowed. Screenshots are in the ignored `artifacts/sidebar-quiet-*.png`.
- The screenshots confirm the pinned sidebar has no glass top bright edge and that hover-expanded content is not clipped by the 44 pt split. The native host was running in the background, so the screenshot window buttons were unfocused; the light and hover screenshots used the default narrow viewport and are not acceptance for all window widths.
- The native host reported `SF Pro Text` as unavailable and fell back to a system font; links still have the existing warning. Preview output in the screenshots showed narrow-column wrapping; this round did not change terminal layout or reflow and did not locate the cause.

This round does not mean title bar dragging, IME, selection copy, high DPI, activation switching, different desktop backgrounds or the system "Reduce Transparency" setting have passed. The following is a historical record of earlier acceptance limitations; this round obtained the native screenshots described above.

## 2026-11-12 · Earlier failed native acceptance record

`./scripts/nsh test App.tsx --target macos` compiled the native host, but when starting the test the environment prohibited listening on the local development server and reported `cannot start a dev server: Operation not permitted (os error 1)`. Therefore no native test screenshot was obtained.

A process that loaded the bundle directly with the compiled host could start, but macOS system service connection logs appeared, and no inspectable window screenshot was obtained. Visual or native interaction acceptance cannot be claimed on this basis.

This must be run in a normal terminal:

```sh
./scripts/nsh run --window-size 1200x780
```

Manually check title bar dragging, sidebar resizing and collapsing, the group menu, mouse selection and copy, the input method, fonts and high DPI. There is currently no real PTY, so Neovim / tmux / shell behavior are outside this round's scope.

After the theme redesign, the native screenshot test was attempted again and still failed due to the same local development server permission restriction. Semantic tokens, appearance switching and state retention have headless evidence; native final visuals have not been accepted.

On 2026-11-12 the chosen full-height sidebar-through layout was adopted: full-height sidebar, 28 pt single-line tabs, a 36 pt session toolbar and a full-bleed terminal; and a Firefox-style group editor overlay was implemented. Type checking, the build and 10 interaction tests passed. Native screenshots are still blocked by the development server permission restriction; system window buttons, the collapsed state and the actual position of the group overlay need acceptance on a real machine window.

## 2026-11-12 · Remove all glass effects

At the user's request, the hover sidebar uniformly uses solid `sidebar`, with hierarchy expressed by a thin border, right rounded corners and a light shadow; the toolbar removed its capsule base, with directories and buttons arranged directly; the group editor and tooltip changed to solid `popover`, a thin border and a light shadow. The glass helper module, IconButton's glass branch and the glass tokens in both appearances were deleted. The app TSX and theme no longer contain `glassEffect`, glass tint, background blur or platform backdrop configuration; the modal dialog's uniform scrim is kept.

`./scripts/nsh build`, `./scripts/nsh check` and `git diff --check` passed; `./scripts/nsh test --junit artifacts/headless-tests.xml`: 14 passed, 0 failed.

The `./scripts/nsh test App.tsx --target macos` native screenshot step passed, obtaining dark expanded, collapsed, hover, group editor and light expanded screenshots in the ignored `artifacts/sidebar-solid-*.png`; the hover, editor overlay and light screenshots were reviewed, confirming the background stays solid, the toolbar has no capsule base and the editor content is fully visible. The native host still reports a system font fallback and links have the existing warning; preview narrow-column wrapping is still present. These screenshots do not replace IME, selection copy, activation switching and long-term daily acceptance.

## 2026-11-13 · Independent panes and unified window bar

The layout changed to a 40 pt unified window bar, with the sidebar and terminal below as independent rounded panes. Pane tops meet the window bar's lower edge directly, and 8 pt is kept on the left, right, bottom and between the two panes; the extra 8 pt of top whitespace from the first version was removed. The sidebar resize boundary sits in the gap; hover expansion still overlays the terminal without changing the terminal's allocated width, and collapsed and temporary expansion prohibit resizing. The real terminal and preview use the same body padding: 8 pt at the top, 16 pt left, right and bottom.

Verification: `./scripts/nsh build` and `./scripts/nsh check` passed. `./scripts/nsh test --junit artifacts/floating-panes-headless.xml` covers the Rust core/PTY, workspace interaction and mount restoration, 50 items in total.

Native AppKit used a temporary `nsh/test` script, running `./scripts/nsh test .pane-visual.test.tsx --target macos --junit artifacts/floating-panes-native.xml` under `TERMLEVEL_PREVIEW=1`; the dark and light items passed. Reviewed the 1100×720 pt scale 2 expanded, collapsed, hover overlay and light screenshots: `artifacts/panes-expanded.png`, `artifacts/panes-collapsed.png`, `artifacts/panes-hover.png`, `artifacts/panes-light.png`. The screenshots confirm the pane top is at 40 pt and the hover overlay is not clipped by the narrow split. The temporary script was removed; screenshots and JUnit are ignored local artifacts.

This native acceptance used explicit preview sessions and proves only layout and overlay display; real PTY workspace operations and restoration are covered by the headless integration tests above. The system candidate window, selection copy, title bar dragging and long-term daily use retain separate manual acceptance boundaries.

## 2026-11-13 · Fleet Dark / Light palette mapping

The complete dark and light semantic tokens in `theme.json` changed to the Fleet Islands UI palette mapping; the fixed source and usage table is in [the UI design](../design/ui.md). The workspace window, panes, popovers, controls, groups and the terminal's default foreground/background/cursor are all covered; NSH's fixed ANSI 16 colors and program truecolor are not modified. Transparent neutral colors in source are pre-composited into opaque surfaces.

`./scripts/nsh build` and `./scripts/nsh check` passed. All 50 tests of the headless regression `./scripts/nsh test --junit artifacts/fleet-palette-headless.xml` passed. The theme switching test still checks that colors update with appearance, that explicit appearance overrides the system choice, and that terminal instances and output are preserved; the earlier assertion requiring terminal text and tab text colors to differ was removed, because under Fleet both use the same color for primary text.

Native AppKit executed a temporary `.palette-visual.test.tsx` in explicit preview mode; the dark and light items passed (`artifacts/fleet-palette-native.xml`), capturing and reviewing the workspace, settings panel and sidebar hover state respectively: `artifacts/fleet-dark.png`, `artifacts/fleet-light.png`, and the corresponding `-settings.png` / `-hover.png`. The temporary script was deleted, and all screenshots and reports are local ignored artifacts. This is a native visual and interaction automation record and does not replace manual acceptance of the actual system input method and long-term daily use.

## 2026-11-13 · Flat groups

Removed the whole-block colored background and rounded outer frame of expanded groups and the hover fill of group header buttons, keeping row positions, colored group badges, member guide lines and a single neutral background layer for the active terminal. Group header hover still shows the more button, and the keyboard focus ring and drag target feedback are kept.

`./scripts/nsh build` and `./scripts/nsh check` passed. The native preview script hovered group headers and checked the sidebar overlay state under Dark / Light; both passed, with screenshots `artifacts/flat-groups-dark.png`, `artifacts/flat-groups-light.png` and the corresponding `-hover.png`, and report `artifacts/flat-groups-native.xml`. The temporary script was removed. All 50 headless tests passed; interaction regression report: `artifacts/flat-groups-headless.xml`. Native screenshots do not replace manual acceptance of the real system input method, selection, and so on.

## 2026-11-13 · Guide line separation from tabs

In the expanded state the member container uses `gap-group-guide: 6px` to leave a gap from the guide line to the tab's left edge, and the duplicate colored short vertical bar inside the active tab is deleted; the selection background and colored terminal icon are kept. The collapsed state does not add this indent. `./scripts/nsh build` and `./scripts/nsh check` passed. The native preview Dark / Light items passed, with screenshots of the expanded, collapsed and hover overlay states (`artifacts/group-guide-{dark,light}.png` with the corresponding `-collapsed.png` and `-hover.png`; report `artifacts/group-guide-native.xml`); the gap and selection marker in the expanded, collapsed and hover states were reviewed. The temporary native script was removed. The headless regression had 49 passed and 1 Rust PTY test, `real_shell_utf8_resize_title_and_exit_status`, timing out waiting for state (report `artifacts/group-guide-headless.xml`); rerunning it alone with `./scripts/nsh test --filter real_shell_utf8_resize_title_and_exit_status --junit artifacts/group-guide-pty-retry.xml` passed. No Rust or timeout thresholds were changed this time, and a single-item rerun is not treated as a fully green round.

## 2026-11-13 · Remove terminal body top padding

Both the real and preview terminals removed `pt-(--padding-terminal-top)`, and the now-unused top padding token was removed from the theme as well; left, right and bottom remain 16 pt. The scrolled grid and Input continue to share the original content container. `./scripts/nsh build` and `./scripts/nsh check` passed. The native default entry used a real fish session screenshot (`artifacts/terminal-no-top-inset.png`); the temporary native test passed (`artifacts/terminal-no-top-inset-native.xml`) and the script was removed. All 50 headless tests passed, report: `artifacts/terminal-no-top-inset-headless.xml`.

## 2026-11-13 · Directed multi-terminal tiling

The implementation is in [multi-terminal tiling](../design/tiling.md). Layout, directional navigation, constraints and drag and drop use `ui/tiling.ts`; `ui/workspace.ts` composes session and layout operations; `ui/TilingCanvas.tsx` keeps surfaces by stable session ID and only updates native host geometry. Split does not own the PTY. Pane resizing updates TSX proportions through the native View pan/responder, and is not claimed to be a fully JS-bypassing native split chain.

### Automated evidence

- `./scripts/nsh check`, `./scripts/nsh build`, `cargo fmt --check` and `git diff --check` passed.
- Full command: `./scripts/nsh test --junit artifacts/tiling-all-tests.xml`, 58 / 58 passed. Covers the original 50 items plus new pure layout, tiling interaction and real dual PTY tests.
- The 7 items in `ui/Tiling.test.tsx` cover geometry and neighbor selection, split and replace, maximize restore, narrow-window restore, directional focus and swap, resize and key return, group close, sidebar drag-in, title edge movement, center swap and size pre-check. The drag and drop tests read tree snapshots through an independent fixture, confirming edge insertion changes direction, center swap changes session assignment, and Escape leaves the whole tree completely unchanged. It is not enough to see only that the preview disappeared or that the session count is unchanged.
- `terminal/TilingSession.test.tsx` uses two real `/bin/sh` PTYs, verifying that left and right output update at the same time, click input does not leak into the other session, `stty size` changes on maximize, the PID is unchanged after collapse and re-display, and explicit close removes the session. The test reads real Rust session metadata, not preview output.
- Two existing UI tests were changed to select Color test before typing, so the test driver no longer injects text directly into a hidden terminal and bypasses the actual focus rules. Preview input sending is also controlled by unique focus and modal state.

### Native AppKit

`./scripts/test-native-tiling` passed, report `artifacts/tiling-native.xml`. The script temporarily registers `terminal/fixtures/Tiling.acceptance.tsx` as a native test and cleans it up on exit, using a real shell rather than the preview transport.

Acceptance covers:

- Both panes have real output, and the native directional focus shortcut updates the current pane.
- `NativeInputProbe.geometry` compares the page coordinates of the actually mounted Input and the grid cursor for both sessions; after swap, drag resize, maximize and restore, each error is under 0.5 pt.
- While marked text exists, the fixture calls the same workspace swap operation: the text stays marked, the candidate anchor moves more than 100 pt horizontally, confirming input still commits to the original session and the other session received no Chinese.
- AppKit gives all keys to the input method first during composition; no claim is made that the swap shortcut necessarily executes while marked text is present. The test proves ordinary shortcuts and layout changes during composition separately.
- Three-pane, 640×480 narrow-window temporary focus, original layout restore and sidebar hover overlay all have screenshots. Reviewed `artifacts/tiling-native-two-panes.png`, `tiling-native-ime-swap.png`, `tiling-native-three-panes.png`, `tiling-native-narrow.png` and `tiling-native-sidebar-overlay.png`.
- The native driver does not support `dragAndHold`; AppKit acceptance uses a complete `drag`. Mid-drag preview and cancel are proven by headless tree snapshot assertions.

The existing `./scripts/test-native-input` also passed (`artifacts/native-ime-tests.xml`), covering composition commit once, cancel, rapid consecutive input, scrolling, font size and native switching. After the shared font measurement changes, Input / cursor coordinates still agree.

A temporary production-entry screenshot test was also run with the default `App.tsx` and real fish; the dark and light items passed (`artifacts/tiling-production-native.xml`), and `artifacts/tiling-production-dark.png` and `artifacts/tiling-production-light.png` were reviewed. The test waited for the window layout to stabilize before capturing, avoiding treating a window transition frame as the interface result. The temporary production screenshot test was deleted; screenshots and reports are local ignored artifacts.

### Acceptance boundaries

What has been implemented and verified is layout, session retention, input isolation and the native input protocol. The actual system input method candidate list, complex composition sequences, cross-line selection and copy, CPU / memory / frame time under long sustained output and many sessions, and the tracking feel of manual dragging still require independent manual acceptance. Layout is not yet persisted; a UI workspace rebuild falls back to a single pane from the existing Rust sessions, and an app restart does not restore live processes.

## 2026-11-13 · Title bar global buttons

New group, search terminals and settings moved from the bottom of the sidebar to the top-right of the window title bar, and the button area disables window dragging. When the sidebar is expanded, only the session count remains at the bottom; when collapsed, the bottom toolbar is no longer shown.

`./scripts/nsh check`, `./scripts/nsh build` and `git diff --check` passed. `./scripts/nsh test App.test.tsx --junit artifacts/global-actions-tests.xml` passed 18 / 18. A temporary native test took screenshots of the expanded and collapsed states under Dark / Light and verified that after collapsing the title bar buttons open settings, search and the new group editor respectively, passing 2 / 2 (`artifacts/global-actions-native.xml`). Reviewed `artifacts/global-actions-dark-collapsed.png` and `artifacts/global-actions-light.png`; the temporary test was removed.

## 2026-11-13 · Tab group workspaces and clean exit

Each tab group stores its own tiling tree, and the canvas shows only the current group. Switching to another group swaps in that group's layout; coming back keeps the split. Dragging tabs from other groups into the view adds them to the current group and places them at the drop point; "Show existing terminals" in the menu lists only the current group. Ctrl+D on an empty line and exit code 0 close the tab and pane. A nonzero exit code leaves a status bar. Two or more panes collapse into one split card in the sidebar, with members in one-to-one correspondence with panes; the card can stay outside groups or move as a whole into an existing tab group, and the split itself does not create a group. When the sidebar selects a tab outside the split, the whole right side switches to that terminal and the split tree is retained; selecting a tab inside the split brings the whole split back into view.

`./scripts/nsh check` and `./scripts/nsh build` passed. `./scripts/nsh test --junit artifacts/group-layout-tests.xml` had 58 passed, 1 failed: the failure was the newly added real dual PTY step typing into an already collapsed terminal. After correcting the step, `./scripts/nsh test --filter "two real PTYs"` passed. That run confirmed that `/bin/sh`'s `exit 3` keeps the tab and status bar, and Ctrl+D on an empty line closes another still-running session. Ctrl+D and `exit 1` in preview are covered by `ui/Tiling.test.tsx`. Native AppKit tiling acceptance was not rerun this time.

## 2026-11-14 · Scrollback limit in settings

The Terminal settings can adjust scrollback in steps of 1,000 lines, from 1,000 to 100,000, defaulting to 10,000. Changes apply immediately to already open real sessions, and shrinking discards the oldest lines; sessions created afterward use this value. The preview terminal rebuilds its grid to the new limit. The shortcuts page adds Close terminal, Previous terminal, New group, the first nine terminals and Settings. Appearance, font size and fonts already take effect immediately. These values are still not written to disk.

`./scripts/nsh check` passed. `./scripts/nsh test --filter set_scrollback` passed. `./scripts/nsh test terminal/Find.test.tsx` passed 2: settings changed from 10,000 to 9,000 and back, and the shortcuts page shows Close terminal, Previous terminal and Find in terminal. The full suite was not rerun, and no native AppKit acceptance was performed.

## 2026-11-14 · Pane toolbar becomes search entry

The new button was removed from the single-pane toolbar. Non-split terminals are still created by ⌘T, the window menu and "New terminal" in the sidebar; the empty-state button when there is no terminal is kept. The first action on the left of each pane toolbar changed to search, which selects that pane and opens the existing content find bar.

`./scripts/nsh check` passed. `./scripts/nsh test terminal/Find.test.tsx` passed 2: after closing preview search, the toolbar has no `titlebar-new-terminal`, and pressing `pane-find` brings the find bar back. The full suite was not rerun, and no native AppKit acceptance was performed.

## 2026-11-15 · Workspace and settings persistence

The first launch no longer creates a shell automatically: after the window opens the canvas shows a blank guide, and a terminal can be opened directly in home or a new group created. Saved tab groups, tab order, pinned state, split trees, selected tab, sidebar width and expanded state, font size, scrollback, interface / terminal fonts and appearance are written to NSH Storage at `~/Library/Application Support/app.termlevel.desktop/storage.json`. On app restart, tabs restart shells by their saved directories, and old session IDs map to new sessions; process PIDs and terminal output are not written to the document. Clipboard read permission still resets after restart by design.

The document carries a schema version. Corrupt, unknown-version or illegal layout nodes are dropped and the app continues to start with safe defaults; a tab referencing a nonexistent group returns to the ungrouped area, and unmappable panes in the layout are trimmed. `ui/workspace-snapshot.ts` handles parsing and validation, `ui/persistence.ts` handles NSH Storage reads and writes, and `ui/WorkspaceRoot.tsx` applies fonts and appearance before mounting the workspace, avoiding a first-frame flash.

This round's verification (macOS arm64 debug):

- `./scripts/nsh check`, `./scripts/nsh build`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `./scripts/nsh test --junit artifacts/persistence-tests.xml`: **126 / 128**. The new 5 items in `ui/Persistence.test.tsx` and 3 items in `ui/Snapshot.test.tsx` all passed: first-launch blank guide, home terminal startup and save, new home group from the blank guide, group / pinned / split restore, collapsed-state retention, fallback to home after the directory disappears, fallback for corrupt documents and documents containing only groups, schema parsing and reference normalization, and layout dropping for duplicate pane / session IDs.
- Two failures are unrelated to this change's paths and were not bypassed by modifying tests:
  - `sessions::tests::real_shell_utf8_resize_title_and_exit_status` timed out waiting for PTY state. This item already has a record in this workspace validation file of timing out and then passing when rerun alone; this round a solo rerun also timed out, and the Rust PTY or timeout threshold was not modified.
  - The Chinese paste assertion in `terminal/Session.test.tsx` produced `--------` instead of the expected `PASTE_` value followed by Chinese text under headless. This test and `SessionFixture` were not modified this round; the historical pass was through a native `--target macos` run.
- Preview mode explicitly does not read or write real storage, and `App.test.tsx` still uses the deterministic demo workspace.
- Persistence tests use the headless test's own NSH Storage, and `app.storage()` can read back the saved document without touching the user's real `storage.json`.

Manual acceptance by restarting a real window has not been performed: the native host must confirm the first blank page, the number of shells and their directories after rebuilding groups and quitting and reopening, sidebar width / appearance / font restoration, and that the last save lands on disk when closing the window and quitting the app. Headless evidence is not native GUI acceptance.

## 2026-11-15 · In-app notification panel

Based on uncommitted changes on `bc3c8f7`. The earlier terminal identity and input queue fixes were committed as `bc3c8f7`; the existing docs/codex-pets-handoff.md remains untracked and unmodified. The title bar adds a bell, an unread indicator and a history panel, collecting terminal OSC 9 / OSC 777, BEL, and app errors / warnings. Rust is the sole owner of runtime history, with at most 100 entries, merging and counting identical notifications within two seconds; it does not disappear when the terminal view unmounts or the source closes, and is not saved across app restarts. Supports removing individual items, clearing, and returning to the source by existing session ID; clearing history does not clear the original session error. Preview does not call the real notification service.

Verification used `GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip CARGO_NET_OFFLINE=true`:

- `cargo test --workspace notification -- --nocapture`: 1 item in the safe crate and 3 items in the app passed; log artifacts/notifications-rust.log. Covers OSC chunking and owned strings surviving terminal destruction, bounded history eviction, unread / duplicate counting, IDs not being reused, real PTY notifications with no mounted view, and history retained after the source closes.
- `./scripts/nsh test --junit artifacts/notifications-headless.xml`: 152/152, including three new real PTY notification interactions and preview empty panel / collapsed sidebar entry cases. Log artifacts/notifications-headless.log. The final focused headless notification run was 3/3; log artifacts/notifications-focused.log.
- Native AppKit automation: temporarily copied Notifications.test.tsx to the root as .native-notifications.test.tsx, corrected the relative fixture path and inserted screenshots, then ran `./scripts/nsh test .native-notifications.test.tsx --target macos --junit artifacts/notifications-native.xml`: 3/3. Log artifacts/notifications-native.log. The temporary driver was deleted; the in-repo case can be rerun with `./scripts/nsh test tests/terminal/Notifications.test.tsx --target macos`. Verified background shell notifications, jumping back to the source, history after the source closes, history retention after an app input warning recovers, removing individual items, five-entry history scrolling and clearing. The host still has one link warning.
- Reviewed artifacts/notifications-source-native.png and artifacts/notifications-history-native.png: a single item shrinks to its content, and the panel still opens when the sidebar is collapsed; in a narrow 640×420 window at most three entries are visible, with separators, scroll hint and clear button visible. Panel height is computed from the root window's available space and the measured height of the first three entries; long bodies can be scrolled.
- `./scripts/nsh check`, `./scripts/nsh build`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `git diff --check` passed. Logs artifacts/notifications-{check,build,clippy}.log.

During development the focused test once depended on the wrong notification number because tests shared Rust history; the fixture now explicitly clears its own history, and cases locate by the specific notification's accessibility label. Native also received an extra real BEL notification, and the initial generic close label was not unique; the label now includes the notification title and verifies removal by individual item, without assuming history is empty after removing one. The final regression above covers these corrections.

This is in-app notification history and does not send system desktop pushes; OSC 99, cross-restart history and automatic command-completion inference from shell integration are not implemented. Real Codex notification policy and long-term user daily use have not been manually accepted, and the controlled OSC / BEL cases cannot replace that. Rust changes require rerunning the app to take effect; the user's current sessions were not closed.

### Same-day addition: remove the terminal bottom status bar (amend the notification panel commit)

At the user's request, the bottom status bar was removed from the real TerminalSurface and the deterministic PreviewTerminal. Input rejection and detailed errors are shown uniformly by the notification panel; the pane title bar keeps a short Exited / Error status and an "Open new terminal" entry, with existing output and the original session ID preserved, and a new session is created explicitly only when the entry is clicked. Clearing notification history does not clear the exit status or automatically restart the old process. The current architecture has been synchronized; the status bar descriptions in the older validation sections above are kept as historical records.

- `./scripts/nsh test tests/terminal/Notifications.test.tsx --target macos --junit artifacts/notification-status-removal-native.xml`: 4/4. Added real /bin/sh verification that a nonzero exit preserves output, the bottom does not repeat the error, notification history contains the exit reason, the title bar still shows the exit status after clearing history, and input works after explicitly opening a new terminal. Log artifacts/notification-status-removal-native.log. The host still has one link warning; this is not long-term manual daily acceptance.
- Updated the InputBackpressure and real / preview tiling regressions to verify the notification entry and title bar status respectively, no longer relying on the bottom status bar.
- `./scripts/nsh check`, `./scripts/nsh build` and `git diff --check` passed. Logs artifacts/notification-status-removal-{check,build}.log. There were no Rust code changes this time.
- `./scripts/nsh test --junit artifacts/notification-status-removal-headless.xml`: 153/153, log artifacts/notification-status-removal-headless.log.

## 2026-11-15 · Notification type scale unified to Interface body

The previous notification primary text used the dialog title type scale, 15 / 20, inconsistent with the ordinary interface body of 12 / 16. This change unifies the primary text and context to body in NotificationRow, expressing emphasis only through weight, color and icons; expanding long content only toggles weight, without changing size or line height. The font family continues to read `interfaceFont` through Type, source / time continue to use the existing caption scale; the global dialog title token is not modified.

`./scripts/nsh check`, `./scripts/nsh build` and `git diff --check` passed (logs `artifacts/notification-type-{check,build}.log`); the focused notification run was 6/6 (`artifacts/notification-type-focused.{log,xml}`); `./scripts/test-native notifications` 2/2 (`artifacts/notification-type-native.log`, `artifacts/notifications-native.xml`). The native dark notification and light 360 pt narrow-window screenshots were reviewed again, and the primary text type scale matches the sidebar / ordinary interface text. The native host still has the existing link warning; the full Rust / workspace suites unrelated to the type scale were not rerun this time, and the earlier notification-row run's 194/194 remains a prior record.

## 2026-11-16 · New terminal entry points and group context

Based on uncommitted changes on `bcf08b7`. `./scripts/nsh check`, `./scripts/nsh build` and `git diff --check` passed. `./scripts/nsh test --junit artifacts/headless-tests.xml` was 217 / 218: `clipboard::tests::osc52_bytes_reach_the_pasteboard_as_unicode` failed with `pbcopy rejected the sample`; the headless interaction of the group, ungrouped and overview entry points passed this round. The JUnit record is at `artifacts/headless-tests.xml`.

Native window automation and manual GUI acceptance were not performed; headless preview cannot replace real window interaction acceptance.

## 2026-11-16 · Ungrouped section and History module split

Based on uncommitted work on `7dd7c8c`. Ungrouped is distinguished from ordinary groups by a separate area and divider line; the History branch action bar uses the same `IconButton` style and keeps a fixed height. The previous 1410-line History file was split into 7 files under `HistoryPanel/` (1–389 lines), with `index.ts` as the public entry point. `AGENTS.md` and the development guide add the 500-line hard limit for source files and the module split convention; the repository still had 13 non-generated Rust / TS source or test files over 500 lines, and this round did not refactor those existing files.

`./scripts/nsh check`, `./scripts/nsh build` and `git diff --check` passed; `./scripts/nsh test --junit artifacts/headless-tests.xml` was 227 / 229, with failures from the existing `pbcopy rejected the sample` and the new test selector `text:main` matching two places. After correcting the selector, `./scripts/nsh test tests/workspace/History.test.tsx` was 4 / 4. During the run the History public entry changed from `index.tsx` to `index.ts` and focused re-verification was completed; the full suite was not rerun. `./scripts/test-native history` 2 / 2, and `artifacts/history-branch-actions-all.png` and `artifacts/history-branch-actions-remote.png` were checked; `./scripts/test-native gitlive` first reported `no context menu is open`, and a rerun was 1 / 1, with expanded and collapsed sidebar screenshots checked. Manual GUI acceptance was not performed.

## 2026-11-16 · Group-pinned terminals and status module cleanup

- `./scripts/nsh check` and `./scripts/nsh build` passed. Focused tests for pinning within a group, moving across groups, split preservation and snapshot restoration passed.
- Native automation: `./scripts/test-native tiling` 1/1, covering real PTY and split operations; manual GUI operation acceptance was not performed.
- The complete `./scripts/nsh test --junit artifacts/headless-tests.xml`: 232/234; the existing clipboard test failed with `pbcopy rejected the sample`, and `sessions::tests::closing_allows_shell_to_hang_up_background_jobs` reported background tasks still running. The latter passed when individually re-verified with `./scripts/nsh test --filter closing_allows_shell_to_hang_up_background_jobs`, which does not establish that the problem is fixed.

## 2026-11-16 · Collapsed sidebar group buttons

- `./scripts/nsh check` and `./scripts/nsh build` passed; the focused sidebar interaction tests passed 25/25.
- `./scripts/nsh test --junit artifacts/headless-tests.xml`: 233/234, with the existing clipboard test failing with `pbcopy rejected the sample`.
- Native automation `./scripts/test-native gitlive` 1/1 passed, and the collapsed sidebar screenshot was checked; manual GUI operation acceptance was not performed.

## 2026-11-16 · Ungrouped pinned terminal divider

- `./scripts/nsh check` and `./scripts/nsh build` passed; the full test run was 233/234, with the existing clipboard test failing with `pbcopy rejected the sample`.
- Native automation `./scripts/test-native tiling` 1/1 passed, and the pinned terminal screenshot was checked; manual GUI operation acceptance was not performed.

## 2026-11-16 · Collapsed sidebar Ungrouped selection hint

- `./scripts/nsh check` and `./scripts/nsh build` passed; native automation `./scripts/test-native gitlive` 2/2 passed, and the spacing between the hint bar and the panel rounded corner was checked in `artifacts/ungrouped-compact.png`. Manual GUI operation acceptance and the complete headless suite were not performed.

## 2026-11-16 · Merge Appearance and Interface settings

- `./scripts/nsh check` and `./scripts/nsh build` passed; focused persistence tests 11/11 and the appearance switching test 1/1, covering theme, working indicator and interface font.
- Native automation `./scripts/test-native settings` 1/1; reviewed screenshots of the merged page and of switching settings pages after scrolling. Manual GUI operation acceptance and the complete headless suite were not performed.

## 2026-11-16 · Settings window test fixture

- The native settings test switched to a real terminal fixture without probes; `./scripts/nsh check` and `./scripts/test-native settings` (1/1) passed, and the screenshot confirmed the split probe area is no longer shown at the bottom. The split native test still uses its dedicated probe; manual GUI acceptance and the complete headless suite were not performed.

## 2026-11-16 · Collapsed sidebar selection background spacing

- `./scripts/nsh check`, `./scripts/nsh build` and the focused collapsed-sidebar Git navigation test 1/1 passed.
- Native automation `./scripts/test-native gitlive` 2/2 passed, and the collapsed sidebar selection screenshot was checked; manual GUI operation acceptance and the complete headless suite were not performed.

Entries were translated from the consolidated validation log; failures above remain part of the record and must not be erased.
