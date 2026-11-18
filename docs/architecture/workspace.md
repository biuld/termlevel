# Workspace architecture

The TSX workspace owns workspace UI and low-frequency metadata. Rust owns terminal streams, grids, parsing and high-frequency rendering; session and process lifecycle is described in [terminal.md](terminal.md). This document covers workspace composition, native UI integration, tab and group behavior, tiling with stable mounting, and workspace persistence.

## Composition

`App.tsx` is the mount entry. `WorkspaceRoot` reads the persisted document before mounting the workspace. `WorkspaceApp` wires windows, menus, shortcuts, dialogs and session operations. The sidebar renders vertical tabs, groups and bell markers. `ui/workspace/workspace-snapshot.ts` owns the workspace document schema, validation and session ID restoration, and `ui/workspace/persistence.ts` reads and saves it through NSH Storage. `ui/workspace/state/index.ts` owns sessions, Git view selection, tiling operations and the single logical focus; `groups.ts` owns groups, moving and in-group pinning; `initial.ts` restores the workspace and layout; `tiling.ts` provides layout-tree transforms, constraints, geometry and directional navigation.

`TilingCanvas` is the composition point from workspace state to the terminal display, mounted by session ID. The terminal surface itself and the native input/rendering boundary are covered in [terminal.md](terminal.md). Multi-pane interaction and acceptance boundaries are in [multi-terminal tiling](../design/tiling.md).

## Local file drop

Local file drop uses NSH's native `on_file_drop`. The target `SessionTerminal` receives file URLs; Rust `session_view/file_drop.rs` decodes them and generates shell literal arguments (escaping single quotes and internal quotes), then sends them through the existing `Session.paste` / Ghostty bracketed-paste path into that session without sending Enter. Multi-path input of images and other local files is supported; images are not read and attachments are not uploaded. Hidden sessions, modal input blocking, IME composition and exited sessions ignore the drop. A visible but inactive pane first requests workspace focus and then receives the input. Remote URLs and paths containing control characters are rejected as a whole batch. `inputBlocked` is independent of `active`, so an inactive pane and a dialog block are not conflated.

## Native dialog keys and group editor

`DialogKeys` gives dialogs Escape priority at the native layer; the interface and dialog state remain in TSX. `GroupEditor` anchors a small group-editing overlay using NSH `Overlay` and the group header's `ViewRef` for the group name, color and group operations. Edits are written to the workspace immediately, and closing the editor does not roll them back.

## Window bar, sidebar and measurement

The unified window bar is fixed at the top. Below it, the sidebar and terminal are independent rounded panes, with outer margins and gaps controlled by `gap-pane`. The sidebar exists as a single instance absolutely positioned at the window root, avoiding clipping by the 44 pt native split.

Each side of the split boundary is allocated half the gap; the root-level sidebar subtracts that half gap so its top and bottom align with the terminal pane. A measurement View inside the split synchronizes the actual width during a drag through `onLayout`; `onResize` saves only the final width. When the sidebar is collapsed, hovering expands it as an overlay covering the terminal without changing the terminal's allocated width or mounting position. It stays expanded while a group is being edited.

## Session titles, bells and exit status

A session title comes from low-frequency metadata. After the user renames a session, the program title no longer overwrites it. When an inactive session rings its bell, a marker appears and is cleared when the session is selected. A non-zero exit or startup failure shows a brief status and a new-terminal entry in the pane title bar; existing output is retained and details go to notification history. Exit code 0 closes the terminal.

## Tab drag and drop

Dragging a tab vertically reorders it within the sidebar or puts it into a group. The saved tab-drop setting chooses whether the center of another tab groups the terminals or creates a split. Grouping joins the target group, or creates a group from the target directory. Splitting adds the source to the right of the target in its workspace. Dropping into a collapsed group expands that group. A group saves its own path, terminals created inside it start from that path, and the group name defaults to the path name. Cancelling a drag does not change grouping, and the session ID is unchanged.

The toolbar directory uses the live cwd reported by OSC 7 / ConEmu / iTerm2, falling back to the startup directory when no live value exists; the tooltip states the current directory.

## Tiling state and stable mounting

Tiling uses a TSX-owned binary tree, pane IDs, ratios and `focusedPaneId`. Each tab group has its own tree, and ungrouped sessions share one tree; the canvas projects only the current group. When a tree has two or more panes, the sidebar collapses those sessions into one split unit whose members correspond one-to-one with panes. The unit can stay outside a group or move as a whole into a tab group.

Sidebar `activeId` is derived from the focused leaf of the current canvas. Selecting a session that is already in the group's split replaces the entire right side with that split and focuses it. Selecting a session outside the split replaces the right side with that single terminal while leaving the split tree unchanged. Selecting another group switches to that group's canvas. Closing a group removes its sessions at once and discards that group's tree. Collapsing and in-group reordering do not change the tree. Dragging another group's terminal into the canvas changes its group and inserts it into the current tree. The pane menu can exchange a split session with any terminal outside the current split, including one in another group's layout. Cross-group exchange swaps the terminals' group assignments and layout references while preserving both trees.

`TilingCanvas`'s surface layer is keyed only by session ID; terminals are not placed inside a recursive Split component. The tree generates absolute positions and sizes, and swapping, moving and zooming modify only host geometry, avoiding rebuilds of Input and scroll views when the tree is reflowed. Pane separators update TSX ratios through native View pan / responder events; this is workspace geometry and does not carry terminal bytes or grids. The sidebar still uses NSH `ResizablePanelGroup`. An undisplayed surface keeps its last valid rectangle, and Rust's non-positive viewport check prevents sending a 0×0 size to the PTY.

`TerminalMetrics` measures cell width in the native view scope and supplies it through a low-frequency event so TSX can compute the 40-column × 8-row minimum body size; it shares the font measurement function with the Rust renderer. When the window cannot fit the whole tree, only the focused pane is displayed temporarily; the tree and user ratios are preserved and restored when space returns. User-initiated zoom and this temporary projection are managed separately.

Tab switching, group collapsing, renaming, reordering, selection changes, view remounts and theme changes preserve the same session ID. The workspace keeps terminal displays mounted by session ID; the layout tree decides which sessions are displayed simultaneously, and sessions not displayed are hidden, have hit testing disabled and are removed from accessibility traversal. The focused pane determines the single input target, while other visible panes keep refreshing.

`SessionTerminal.active` controls input authorization and focus; the refresh timer does not depend on it. Clicking or actually focusing Input propagates workspace selection through `FocusRequest`. Dialogs and editing overlays temporarily prevent the terminal from taking focus automatically. Pane keyboard zoom uses a native `ResizeKeys` mounted after the terminal and unloaded when the mode ends, so arrow keys and Enter return to the terminal. During marked text, AppKit gives keys to the input method, and workspace shortcuts cannot be assumed to execute.

## Persistence and restore

Workspace structure and settings are persisted through NSH Storage. The document carries a schema version and saves tab groups, tab order and pin state, the split tree, the selected tab, sidebar width and expansion state, font size, scrollback, font, appearance and tab-drop action. It also records which group tool views are open and which one is selected. On macOS the storage file is `~/Library/Application Support/app.termlevel.desktop/storage.json`; a corrupt document makes the application start with an empty workspace.

Session IDs do not restore across processes. Old session IDs in the layout map to new sessions during restore, and process PIDs are never written. On application start, tabs reopen in their saved order, groups and splits are restored, and each tab starts a new shell in its saved directory; a first start with no document stays blank and does not create a shell automatically. Application restart still ends old sessions, so PID cannot be used as a recovery credential. Process opening and closing is described in [terminal.md](terminal.md).

Clipboard read permission is not persisted and resets on restart. OSC 7501 status, OSC 9;4 progress, OSC 22 mouse shape and OSC 133 prompt state are runtime metadata and are not written to the workspace document; they are empty after an application restart, and UI hot reload restores them from current sessions through `Sessions.list()`. The live directory is the exception: saving writes the tab's `cwd` as `liveCwd ?? cwd`, so the next start opens a new shell in the last directory while the schema stays unchanged.
