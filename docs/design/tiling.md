# Tiling Workspaces

The core interaction is implemented; sustained daily manual acceptance is still pending. Ownership is described in the [workspace architecture](../architecture/workspace.md), the terminal boundary in the [terminal architecture](../architecture/terminal.md), visual rules in [ui.md](ui.md), and evidence in the [workspace validation record](../validation/workspace.md).

## Goals and scope

Users can run and view shells, development services, tests, agents and similar sessions at the same time, always know which terminal receives keyboard input, and adjust the layout with mouse or keyboard.

The current implementation provides:

- Splitting the current pane to the right or down, and placing existing sessions in either direction.
- Directional focus, position swapping, resizing and equalizing the layout.
- Temporary zoom and restore, collapsing a pane and closing a terminal.
- Dragging existing sessions in from the sidebar, and drag-and-drop between panes.
- Titles, exit status, bell, input method and focus isolation with multiple panes.

It does not add automatic main-and-stack layout, a scrolling infinite canvas, floating terminal windows, nested tab bars inside a pane, multiple windows sharing one session, broadcast input or layout preset management. Layout structure is persisted with the workspace; live sessions are not restored after an application restart.

The UI continues to use the NSH 0.6.0 macOS native backend. The PTY, terminal output, grid and high-frequency painting remain managed by Rust.

## Interaction model

The design follows tiled window managers for directional splitting, directional focus, position swapping and temporary zoom. [i3's usage and layout documentation](https://i3wm.org/docs/userguide.html#_using_i3) is the concrete reference; shortcuts and in-application interactions are redesigned for terminals. Existing vertical tabs and groups are retained. Directional splitting is the first-version model because it keeps user-chosen positions and proportions stable.

A typical layout keeps the main working terminal on the left, a service in the upper right and tests in the lower right:

```text
┌───────────┬────────────────────┬───────────────┐
│ Sessions  │ Shell              │ Dev server    │
│           │                    │               │
│ Shell     │                    ├───────────────┤
│ Server    │                    │ Tests         │
│ Tests     │                    │               │
└───────────┴────────────────────┴───────────────┘
```

This is one layout inside one window. A tab group is one tiled workspace: the canvas shows only the current group's layout tree, and groups never appear on the same screen. Ungrouped and pinned tabs share another tree. Collapsing, renaming and in-group reordering do not change the group's layout; switching to another group shows that group's own tree, and switching back preserves proportions, zoom and panes.

Two or more panes combine into one split unit in the sidebar, styled as a rounded card. The tabs in the card are the panes; their count cannot exceed the number of splits that fit, and no hidden tabs are collected there. A tab that is still on screen but not focused shows a dot vertically centered at the row end; on hover or keyboard focus the dot gives way to the close button, without occupying the top-right corner. The unit can stand alone in the sidebar or move as a whole into a tab group from a tab's "Move to group"; the card has no separate menu, and moving in does not create a group. When already in a group, the split stays in that group's card while tabs not participating in the split remain outside it. Dragging another group's tab onto the current screen adds it to the current group and splits or replaces according to the drop point. The menu entries "Show existing terminal on the right" and "Show existing terminal below" list only current-group sessions; choosing a tab from another group switches to that group instead of joining the two groups. When ungrouping, if the ungrouped workspace has no layout it takes over this group's tree; if it already has a layout, only the currently viewed terminal is placed into it.

## Session, tab, pane and focus

| Concept | Identity and responsibility |
|---|---|
| Session | PTY, process, grid, output and status owned by Rust, with a stable session ID |
| Tab | Session entry in the sidebar; continues to use the session ID and is not a layout container |
| Pane | Display position in the layout with an independent, stable pane ID referencing a session ID |
| Split | Binary layout node storing direction, two children and a ratio |
| Layout focus | `focusedPaneId`, deciding the current session, menu action target and focus-restoration target |
| Native input focus | Whether the current AppKit Input actually holds the keyboard; it may temporarily be absent while a menu, popup or another application is active |

Core invariants:

1. A session appears at most once in the current window's layout tree.
2. Multiple panes can display and accept mouse interaction at once; at most one terminal receives keyboard input at a time.
3. A non-empty layout has exactly one logical focus pane. A popup may temporarily own actual input focus.
4. The selected session in the sidebar is derived from the focus pane; a second, potentially stale selection state is not maintained.
5. Splitting, swapping, resizing, zooming and collapsing never rebuild or end an existing Rust session.
6. Only closing a terminal, closing its group, closing the main window, quitting the application, or a shell exiting with code 0 enters the existing session-close path.
7. The layout tree stores no terminal byte stream, cell grid, process PID or output copy.

## Sidebar and session switching

| Entry | Result |
|---|---|
| Click a tab in the current group that is already in the layout | The whole right side becomes this group's split, and the corresponding pane is focused |
| Click a tab in the current group that is not in the layout | The whole right side becomes this terminal. If the group has a split, its tree stays; clicking a tab in the split again brings the whole block back |
| Click a tab in another group | The whole right side becomes that group: its full split if the tab is in it, otherwise only this terminal |
| Search and select a session | Same rule as clicking its tab |
| `⌘1…⌘9`, Ctrl+Tab and other existing session switching | Same selection action; never implicitly adds a pane |
| `⌘T` and existing global new-terminal entries | Create and select a session; the whole right side becomes the new terminal. Existing splits stay in the sidebar |
| Select or create a session when there is no layout | Create one pane and focus it |

"New terminal" and "split new terminal" are distinct actions. Existing global new-terminal grouping rules are unchanged; a split-created terminal belongs to the current pane session's group and does not inherit pinned state.

Paths use the current session's live directory. OSC 7 / ConEmu CurrentDir / iTerm2 CurrentDir update `Tab.liveCwd`; the toolbar, search, new terminal from that tab and the next save use it, falling back to the startup directory when no live value exists. Group paths still use the group's own `cwd`.

The sidebar continues to highlight only the current focus session. Other displayed sessions use a lightweight pane marker and the tooltip "already shown in layout" without a second selection layer. The collapsed state expresses this through a small marker beside the icon and an accessibility description; the distinction is verified natively. Group collapse remains, and selecting a member follows the existing auto-expand rule.

## Splits and sizing

"Split right" turns the current pane into a left/right pair with the old session on the left and the new session on the right; "Split down" places the old session above and the new one below. The new pane takes focus and the two share the space equally by default.

The toolbar or menu also offers "Show existing terminal on the right…" and "Show existing terminal below…". If the selected session is already in the layout, it is focused directly; its position is changed by drag-and-drop, avoiding two views contending for the same PTY size.

Size is validated before a session is created. If `Sessions.open` returns a launch-failure record, the new pane keeps a clear error state and allows closing or choosing another session; a second shell is never created silently.

Sizing rules:

- The pane gap uses `gap-pane`, currently 8 pt; no extra nested rounded container is added.
- Dragging a gap adjusts the nearest split boundary, not the sidebar width.
- The ratio belongs to the Split and is stored as a floating-point value; layout allocation uses actual native bounds for final sizes.
- The candidate minimum body size is 40 columns × 8 rows, computed from the font's actual cell size and adding the pane toolbar plus left, right and bottom padding. This value is a prototype starting point.
- If a new split cannot satisfy both subtree minimums, the operation is disabled with an explanation that the current session can be replaced or another pane collapsed first.
- Double-clicking a separator restores the corresponding Split to the closest feasible 1:1 ratio. "Equalize layout" applies the same operation recursively.

Shrinking the window, increasing the font or expanding the sidebar may make an existing layout violate the minimums. The preferred response is to adjust actual ratios within constraints without overwriting the user's saved ratio. If the layout still cannot fit, only the focused pane is temporarily shown, with the explanation "Window too narrow, showing the current terminal only". The tree and other sessions remain, and the layout is restored when space returns. The implementation never silently deletes panes or shrinks a terminal until it is unusable.

## Focus and keyboard

Clicking a pane title or body updates logical focus and gives input to that terminal. Mouse movement over other panes does not change focus. Scrolling an unfocused pane scrolls only its content and never steals the keyboard; native text selection and context-menu behavior still require separate verification.

Directional navigation searches neighbors by actual geometry: it prefers candidates in the requested direction whose projection perpendicular to travel overlaps the current pane, then compares boundary distance and center distance, using stable pane ID as the final tie-break. With no neighbor, focus stays put and does not wrap to the other side.

Directional swap uses the same neighbor rule and only swaps the session references; pane IDs and split ratios stay unchanged. Focus follows the original session to its new position.

The current macOS shortcuts are below. The menu keeps every action entry. AppKit gives keys to the input method during composition, so workspace shortcuts may not execute then.

| Action | macOS shortcut |
|---|---|
| Split right and create terminal | `⌘D` |
| Split down and create terminal | `⌘⇧D` |
| Focus adjacent pane | `⌘⌥arrow` |
| Swap sessions with adjacent pane | `⌘⌥⇧arrow` |
| Temporary zoom / restore | `⌘⇧Enter` |
| Close current terminal | existing `⌘W` |

Keyboard resizing enters "Adjust size" mode through the menu: arrow keys adjust the corresponding ancestor Split, and Enter or Escape exits. A visible hint is shown during the mode, arrow keys do not go to the shell, and each step is 4% of the relevant split ratio.

Control combinations remain reserved for the terminal first; the window manager's shortcut table is not copied directly. Dialogs, menus and composition each have their own focus priority.

## Drag and drop

A pane can be dragged only from its title bar; dragging the terminal body still selects text. Dragging inside the sidebar continues to reorder tabs and manage groups; layout drop hints appear only after the pointer enters the right-hand pane area.

| Source and target | Result |
|---|---|
| Tab of a hidden session in the current group → pane edge | Split the target pane and place the existing session in the indicated direction |
| Tab of a hidden session in the current group → pane center | Replace the target pane's session; the old session is retained |
| Tab from another group → pane edge or center | The terminal joins the current group, then splits or replaces per the drop point; the original group's layout removes it |
| Displayed session's tab / pane title → another pane center | Swap the two sessions; focus follows the dragged session |
| Displayed session's tab / pane title → another pane edge | Remove the source leaf, merge the original branch, then split the target pane and insert the source pane |
| Drop on itself or outside the window | No layout change |

Moving a pane out of the layout preserves its pane ID. Edge insertion must recompute the target constraints after removing the source, so the old size is not used to decide incorrectly whether a split fits. Dragging itself does not modify a session's group, pinned state or sidebar order.

Preview requirements:

- The center shows a swap or replace hint; an edge shows the actual half-region the pane is expected to occupy.
- Invalid positions show disabled feedback and never suddenly degrade to replace on release.
- The drag changes only the preview; release commits the tree, focus and session allocation once.
- Escape, dragging outside the window and losing drag capture all cancel and restore the original layout.
- In zoomed or too-narrow temporary display, the full displayable layout must be restored before a pane drag can start; when space is insufficient, the menu replace entry remains available.

The current implementation supports only the target pane's four edges, not "insert at the whole canvas edge" or dragging a parent container. Edge hit testing uses 24% of the target pane's width or height as the normalized band; corners take the nearest edge and the remainder is the center region.

## Zoom, collapse and close

Temporary zoom is a display mode inside the window content area, not macOS full screen. The layout tree, ratios and pane IDs stay unchanged, and restoring returns panes to their original positions. Focusing another existing pane moves the zoom target with focus. Layout-changing split operations exit zoom first and then execute at the original pane position; if space is insufficient the operation is rejected.

While zoomed, the logical positions of other panes are retained but those panes are temporarily hidden, have hit testing disabled and are removed from accessibility traversal. Hidden views never send a 0×0 resize to the PTY and keep their last valid grid size.

"Collapse pane" removes only the corresponding layout leaf and merges its parent Split; the session stays in the sidebar and keeps running. Collapse is disabled when only one pane remains, avoiding an unexplained state with sessions but no content.

"Close terminal" keeps the existing process-termination semantics:

1. Explicitly close the corresponding session, remove the tab and its leaf in the tree.
2. If the focused pane was closed, prefer the nearest pane in the merged sibling subtree, then other panes.
3. If the tree is empty but the sidebar still has sessions, select the most recently used remaining session and create a pane.
4. If all sessions are closed, show the existing empty workspace and do not automatically start a shell.

Closing a non-focused session does not steal focus. Closing a group removes its sessions from the layout as one workspace operation and normalizes the tree, avoiding a transient selection of a session that is about to close.

A shell exiting with code 0 closes its tab and pane together, the same as `⌘W`. Ctrl+D on an empty line follows this path. A non-zero exit or launch failure keeps the pane and output, shows the exit code or error, and never auto-merges or creates a new pane. The pane title-bar remove action is explicitly labeled "Collapse pane"; "Close terminal" lives in the session menu, so one × never means two different things.

## Visual and state feedback

The design uses the current Fleet Dark / Light palette, 12 pt pane corners and 8 pt gaps. Every terminal pane has its own compact toolbar; no additional generic terminal card is wrapped around it.

- The toolbar shows the session name and necessary directory hints, with the full current directory in a tooltip or menu. A narrow pane prioritizes a distinguishable session name.
- Only the focused pane uses a 1 pt theme focus-color stroke, painted without occupying layout size; other panes are not globally dimmed.
- A pane belonging to a tab group shows that group's color marker before its title. In the sidebar, the group owning the current layout has a light background and its accessibility description includes "current layout".
- The terminal body adds no top padding; its 16 pt left, right and bottom padding continues from the existing design.
- Separators are normally hidden; the gap shows low-contrast resize feedback on hover and drag.
- The zoomed state exposes a discoverable "Restore layout" action. Too-narrow temporary display and user-requested zoom use different explanations.
- When a non-focused terminal rings, the sidebar and that pane's toolbar show a marker; focusing clears it. Being visible does not mean the user has attended to it.
- Theme changes update default colors and controls in every visible pane while preserving program ANSI / truecolor semantics.

The window top bar remains solely responsible for window-level actions and window dragging. A pane title handles focus and pane drag-and-drop; the two never share `windowDrag` behavior.

## State model and implementation boundaries

The pure layout module `ui/workspace/tiling.ts` owns tree transforms, constraints, directional selection and normalization; `ui/workspace/state/index.ts` combines session operations and layout transforms into one state commit.

```ts
type LayoutNode =
  | { kind: "pane"; id: string; sessionId: string }
  | {
      kind: "split";
      id: string;
      axis: "row" | "column";
      ratio: number;
      first: LayoutNode;
      second: LayoutNode;
    };

type TilingState = {
  root: LayoutNode | null;
  focusedPaneId: string | null;
  zoomed: boolean;
};
```

Too-narrow temporary display is derived from the tree's minimum sizes and the actual content area and is not written into `zoomed`. Visibility must distinguish sessions assigned in the tree from sessions actually shown; both are derived from state, with no second list that can drift.

The original single-terminal implementation used `activeId` to control visibility, hit testing, accessibility hiding and `SessionTerminal.active`. Multiple panes split those responsibilities:

- Pane displayed: can paint, scroll, select and receive hits.
- Pane logically focused: decides the workspace's current session, menu target and focus stroke.
- Native Input actually focused: decides whether that session can receive keyboard input.
- Popup or menu open: input transfers temporarily per existing native priority, and other terminals do not automatically reclaim focus.

`TerminalSurface` position and size are determined by pane bounds; all visible surfaces can no longer use the same `inset: 0`. The existing semantics of `SessionTerminal.active` must be checked and narrowed to input authorization rather than stopping refresh for other visible panes.

The native mount approach prioritizes a stable surface layer keyed by session ID. The layout tree computes or measures pane rectangles and assigns them to the corresponding surface, avoiding rebuilds of Input, selection and scroll views during swaps and tree normalization. Split dragging should prefer NSH native split real-time layout. If the framework cannot provide both a stable surface and real-time geometry synchronization, perform a native technical verification first and record whether scrolling, selection and marked text survive before choosing a mount method; "the Rust session was not lost" is not evidence that all UI state was preserved.

Native layout size changes continue directly into the Rust session resize path. The terminal byte stream, refresh timers and grid painting do not pass through TSX. The preview transport stays explicitly marked and executes no commands.

## Atomic operations and recovery

Before committing any structural operation, check that the session exists, pane IDs are unique, each session is referenced once, both children of a Split are non-empty, the ratio is valid and focus points to a valid leaf. After deletion, a Split with one child collapses into that child.

Split creation has a boundary between creating the session and committing the layout: size prechecks pass before `Sessions.open` is called; if creation succeeds but the subsequent layout commit fails, the workspace must still register the new session and offer a recoverable entry rather than leaving an orphan process invisible in the sidebar.

Layout transforms never call `Sessions.close`. A UI remount rebinds the same Rust session. Persisted layouts store session references but never PIDs; on application restart the workspace restores structure and opens new shells instead of restoring old processes.

## Current implementation

- Terminals stay in a unique tiled surface layer keyed by session ID, with position and size computed from the current tab group's tree. Other groups' trees are retained but not projected. The header toolbar is draggable and the body remains available for text selection. A pane in a group shows that group's color marker.
- A shell exit code of 0, including the preview and a real session receiving Ctrl+D on an empty line, closes the corresponding tab and pane. A non-zero exit leaves a status bar.
- Pane resizing uses native View pan/responder events to update TSX geometry in a controlled way. The terminal is not placed in a recursive `ResizablePanelGroup`, preserving the Input, scroll-view and marked-text instances. The sidebar still uses native splits. Layout updates during a drag pass through TSX, so this is not claimed to bypass JS entirely.
- `TerminalMetrics` uses the same font measurement function as the Rust grid. Constraints are 40 columns × 8 rows plus the toolbar and host padding, and are re-measured after a font change.
- Temporary zoom changes only the projection; the too-narrow hint stays visible and the layout restores automatically when space returns. Only panes actually displayed participate in hit testing and accessibility traversal.
- `scripts/test-native-tiling` covers AppKit views, composition text moving with a pane, candidate anchor, commit attribution and layout restoration. The marked-text move test invokes the same workspace swap operation through a fixture; normal directional shortcuts are asserted separately, so the two kinds of evidence are not conflated.
- A fixture can inject the same workspace model into `WorkspaceApp` to control sessions and test actions; the production entry point still creates its own workspace. Layout structure is persisted, and remounting the workspace rebinds sessions by ID.

## Remaining acceptance

The following criteria are not yet fully verified.

### Layout and workspace

- Splitting right and then splitting the right pane down produces the expected three-pane layout; changing one ratio does not rearrange unrelated subtrees.
- After repeated splitting, swapping, edge moves, collapse and close, there are no duplicate session references, empty Splits, invalid focus targets or orphaned sessions.
- Selecting a tab in the split shows the whole group split on the right with focus on that tab. Selecting a tab outside the split replaces the right side with that terminal while the split tree remains. Group collapse, in-group reordering and renaming do not change the group layout. Cross-group drop onto the canvas joins the current group.
- Closing a non-focused session does not steal focus; closing a whole group performs one final focus selection; closing the last session reaches the empty state.
- Zoom / restore and too-narrow / restore preserve the tree and user ratios. A disabled split is rejected before creating a process.
- Drag-and-drop at the center, four edges, onto itself, outside the window, at infeasible sizes and with Escape cancel all produce the defined result.

### Rust and real sessions

- Two or more PTYs producing sustained output refresh their panes correctly, and output in a non-focused pane never stops.
- Ordinary input, paste, Ctrl+C and composition enter only the target session, including after zoom, swap and sidebar replacement.
- Each pane's rows and columns match its real usable body area; hidden and mode-switched views never send 0×0 to the PTY.
- Collapse, replacement, tree normalization and view rebinding do not end a process, replay input or clear output.
- Explicitly closing a session, closing a group and quitting the application still satisfy the existing termination and reclamation contract.

### Native GUI

- Expanding, collapsing and hover-covering the sidebar match the layout size and input-focus rules; overlay mode does not unexpectedly resize terminals.
- Switching between panes sends the real system input-method candidate window to the correct cursor; composition in progress still belongs to the original session.
- After resizing, moving panes and zoom / restore, the grid and Input share a coordinate origin; check scrolling, selection and copy.
- In dark and light modes and at high DPI, focus borders, bells, drag previews, resize hit areas and accessibility labels are distinguishable.
- Compare sustained-output performance and resource use between single-pane and multi-pane layouts, recording window size, session count, output load and measurement method.

Remaining improvement candidates:

1. Whether 40 columns × 8 rows is a sufficient minimum, and the exact presentation of the too-narrow hint.
2. The style of the sidebar "already shown" marker in expanded and collapsed states.
3. macOS shortcut conflicts, the keyboard-resize step and the exit feedback.
4. Stable mounting with existing marked text and anchor automation evidence; selection, scroll position and the real system candidate list still need daily manual acceptance.
5. The pane-drag hit band, narrow-pane title information priority, and how many persistent action buttons are needed.
