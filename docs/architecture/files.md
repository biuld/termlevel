# Files

The Files view browses the current group directory and edits its text files. It is a group tool view like Changes and History: it covers the split canvas while open, but terminals keep running underneath and no session is created or closed. This document describes the implemented behavior. Related ownership is in [terminal](terminal.md) and [Git](git.md); visual rules are in [UI design](../design/ui.md). The evidence policy is in [validation](../validation/README.md); dated Files evidence is in [files](../validation/files.md).

## Purpose and non-goals

Files is rooted at the group's `cwd`. It lists one directory level at a time, opens UTF-8 text files up to 512 KiB, saves them explicitly, and shows the same Git status letters as Changes. It also exposes a small set of system file actions.

It does not provide rename, create, multi-select, drag-to-terminal, recursive tree search, a language server, remote or SSH filesystems, Quick Look, media preview or permanent deletion. It does not follow the terminal's live working directory to change the root, and it does not watch directories recursively. PDF, video, archives, images and other binary files are reported as non-text rather than previewed. Directory watching is implemented only on macOS; other platforms keep manual refresh.

## Placement

Files is opened from the title-bar Files button, the File menu, or a group's options. It requires a selected group. When the workspace has no group, the menu reports that a group must be selected. Once opened, a Files tab appears in the sidebar next to Changes and History.

Opening a changed file from Changes also opens Files. Return or a double press on a file in the Changes tree, and the Open in Files control beside its patch, select that file, expand its parent directories, and replace Changes on the canvas. A hidden path turns on showing hidden files. The repository path is mapped onto the group directory, including a `~` group path and the `/tmp` / `/private/tmp` aliases. A change outside the group directory stays in Changes.

`openFiles` selects the group, clears that group's selected Git view, and sets the group's Files state to open and showing. `openGitView` hides Files before selecting Changes or History. Open Git tabs remain available; the two tool views are mutually exclusive only as the canvas-covering view.

While Files is showing, `TilingCanvas` overlays it on the split area. Terminal surfaces remain mounted, hidden and outside hit testing. Closing Files reveals the group's previous layout. Session IDs, PTYs and process state are unchanged by opening, switching, hiding or closing the view. Hiding or closing Files when the buffer is dirty is gated by a save / discard dialog; quitting the application uses the same gate.

## Data path

```text
TSX FilesPanel
  list(root, directory) ── Rust Files (read-only)
                            blocking off the engine thread
                            std::fs::read_dir + symlink_metadata
                            ▼
                     DirectoryListing { entries, truncated, unreadable }

  expand / refresh ── list one level
  read / save ─────── Rust Files ── bounded UTF-8 text, whole-file write

  expanded folders ── watchOpen / watchSet ── notify kqueue, non-recursive
  coalesced events ── watchPoll ── list the changed level again

  reveal / openWithDefaultApp / trash
      ── nsh_core::element::{reveal_path, open_path, move_to_trash}
  Open terminal here
      ── workspace addTab → Sessions.open
```

Enumeration and file access stay in Rust. TSX stores only the listed levels, the expanded set, the selected path and the current buffer. Files tasks do not enter the terminal frame scheduler. They use a separate admission limit of two concurrent tasks; Git's four read slots are unaffected.

Preview mode has no native Files transport. `TilingCanvas` shows "Files are unavailable in preview.", and no disk or Git access occurs.

## Listing contract

```rust
struct FileEntry {
    name: String,          // one path component, no separator
    kind: String,          // "directory" | "file" | "symlink" | "other"
    hidden: bool,          // name starts with '.'
    bytes: Option<u64>,    // regular file length; None for directories, links or unreadable metadata
}

struct DirectoryListing {
    entries: Vec<FileEntry>,
    truncated: bool,       // more than 5_000 entries were found
    unreadable: Option<String>,
}
```

- `root` is canonicalized. `directory` must be empty or a relative path below it; absolute paths, parent components and NUL are rejected before any filesystem call.
- Listing uses `std::fs::read_dir` and `DirEntry::file_type`, which does not follow symlinks, and never goes through a shell.
- Entries sort by rank, then Unicode lower-case name, then raw bytes. Rank order is directory, symlink, file/other. The result is locale-independent and differs from Finder's localized order.
- At most 5,000 entries are returned per level. Collection stops at 50,000 entries; `truncated` is true when either bound is exceeded. The tree shows "Showing the first 5000 entries".
- Hidden entries are returned and marked; the UI filters them with the per-group `showHidden` switch, so toggling the switch does not read the disk again.
- A permission or read failure yields empty entries and an `unreadable` message for that level. Other expanded levels keep their last successful result.
- The root is listed when the view opens or the root changes. Other levels are listed when expanded, manually refreshed, or reported changed by a directory watch.
- Names that are empty or contain NUL, `/` or `\` are dropped.

## Paths and symlinks

Every API takes `root` plus a path relative to it. Rust reconstructs the absolute path; TSX cannot request an arbitrary absolute read.

`locate` canonicalizes the root, then walks each component. Intermediate symlinks must resolve inside the root; a regular directory must canonicalize inside the root. A final symlink is returned as the link itself for actions, while read and save resolve it. `within` compares path components, so `/proj` does not match `/proj-other`.

Listing a symlink resolves its target. If the target leaves the root, that level reports "The link leaves this folder."; if it is not a directory, "The link cannot be opened."; otherwise the target is listed while the tree keeps the link's own path as its ID. A read or save through a symlink requires the target to stay inside the root and to be a regular file. Broken links are reported as unavailable.

`terminal_directory` returns the entry's own directory, the parent for a file, the link path for an in-root directory symlink, and otherwise the parent clamped to the root. A root of `~` or `~/...` expands to the home directory before canonicalization.

## Tree model

The root path appears in the toolbar and is not a Tree row. Top-level rows are the root's direct children. Every ID is a root-relative path joined with `/`; the root's own ID is the empty string and appears only in state.

An unloaded directory or symlink row receives one disabled child marker `parent/\0pending` labelled "Loading…", so `Tree` still treats it as a folder and keeps its chevron. Expanding the row lists that level and replaces the marker with real children. A level with no visible children gets `parent/\0empty` labelled "Empty folder" for the same reason. NUL cannot occur in a real path.

Additional disabled rows carry state:

- `parent/\0truncated` with "Showing the first 5000 entries".
- `parent/\0error` with the level's unreadable message.
- `parent/watch` with a per-directory watch failure message. This row is disabled but, unlike the other placeholders, its ID contains no NUL.

A side table `Map<id, FileEntry>` holds the listed entries. The row renderer reads kind, icon and Git state from that table; the payload is not placed in `TreeItem`. Any ID containing NUL is a marker.

The expanded set is capped at 64 paths (`MAX_EXPANDED`). `rememberExpanded` keeps the newest paths and drops the least recently used; the watch set is re-armed from the capped list. Snapshot parsing applies the same 64-entry cap.

## Reading, editing and saving

`Files.read` accepts only a regular file. The limit is 512 KiB (`MAX_BYTES`). A file whose metadata length exceeds the limit returns `truncated: true` with empty text and the file's byte length. Otherwise Rust reads at most 512 KiB + 1 byte. A NUL in that buffer marks the file binary; a longer buffer marks it truncated. Valid UTF-8 is returned directly. Invalid sequences are decoded with replacement characters and `lossy: true`.

`FileText` carries `text`, `binary`, `truncated`, `lossy`, `bytes` and `modifiedMs`. The panel edits only when the file is text within the limit. Binary, oversized and lossy files are read-only and show a centered explanation: "This file is binary. Open it with the default app.", "This file is larger than 512 KiB." or "This file is not valid UTF-8."

The panel keeps one buffer per group. `text() !== saved()` is the dirty state. When the buffer is dirty, selecting another file, hiding or closing Files, or quitting prompts to keep editing, discard or save. Deleting the group shows its own destructive confirmation and warns that unsaved edits will be discarded. Save is offered for an editable selection and writes only when the buffer is dirty.

`Files.save` re-checks the path and the regular-file constraint. It rejects the save when the file's modification time in milliseconds or its length differs from the values returned when the file was opened: "The file changed on disk. Reload it before saving." It also refuses files over 512 KiB. The write is a whole-file `std::fs::write` of the UTF-8 buffer, followed by a fresh read that becomes the new saved state. There is no temporary-file rename.

If a watched parent directory changes while the selected buffer is clean, the panel reloads the file. A dirty buffer is kept; a later save is rejected by the modification check when the disk changed.

## Git status

Files uses the same system `git status` data as Changes. When a Git transport is present, `startGitStatus` polls every three seconds while the application is active. A non-repository or an error response clears the overlay; the file tree remains usable and unmarked. A rejected poll keeps the last successful marks.

`gitOverlay` converts repository-relative changes to group-relative paths and drops changes outside the group directory. It handles canonical `/tmp` and `/private/tmp`, and `/var` and `/private/var`, variants. Each `PathGit` mark records `status`, `staged`, `unstaged`, `untracked`, `conflicted` and `deleted`.

File rows show the Changes status letter (A, M, D, R, C, T, U or ?) with the same colors; a primary dot marks a staged change. A directory shows a warning dot when a descendant changed. Rows already on screen, including the first level under the group root, pick up a mark when a later status arrives and drop it when the path becomes clean. The editor title repeats the selected file's letter and one of "Staged", "Unstaged", "Staged + Unstaged", "Untracked" or "Conflict". A deleted file that is no longer in `read_dir` gets a row under its parent so the status remains visible; selecting it reports "This file was deleted."

Line marks appear only when the selected file has a Git mark, is editable, is not conflicted or deleted, and the buffer matches disk. `watchGitHunks` asks Git for `untracked` for an untracked file, otherwise for `head` (working tree against HEAD, including staged changes). `diffHunks` parses added-side line numbers. The gutter and overview ruler show added lines in green, modified lines in the warning color, and deletions in the destructive color, with a deletion mark above or below the affected line. Marks clear when the file changes, the buffer becomes dirty, the file is conflicted or deleted, or the diff read fails.

## Directory watching

Watching is macOS-only and uses `notify` with the `macos_kqueue` backend. It is a separate watcher and quota from Git discovery's 1,024 watched directories.

One subscription is opened per mounted panel. `watchSet` receives the root and the expanded paths; duplicates are removed and the set is capped at 65 directories (the root plus 64 expanded paths). Excess directories receive "This folder is not watched." Every path is checked off the engine thread; a symlink target must stay inside the root and be a directory. Each directory is watched with `RecursiveMode::NonRecursive`.

Access events and metadata-only modifications are ignored so listing a directory does not loop. Other events, and rescan events, mark the owning directory due. Events coalesce in a 200 ms window; the first deadline for a directory is retained. The UI polls every 200 ms, re-lists changed directories and refreshes the Git overlay.

A failed `watch` call puts "This folder cannot be watched." on that directory. If the watcher itself cannot be created, every requested directory reports "Directory watching is unavailable." Messages appear as disabled state rows, and manual Refresh remains available. Collapsing a directory drops its watch when the set is re-armed; closing Files closes the subscription; deleting the group forgets the state. On other platforms the install call reports watching unavailable and only manual refresh is effective.

## Actions

| Action | Behavior |
|---|---|
| Refresh | Clears listings and generations, lists the root and every expanded level, and refreshes Git status. |
| Show hidden files | Filters the existing listings and stores the preference for the group. |
| Reveal in Finder | Checks the relative path, then calls `reveal_path`. Failure enters notification history. |
| Open with default app | Checks the relative path, then calls `open_path`. Failure enters notification history. |
| Open terminal here | Resolves the entry's directory with `terminalDirectory`, then calls the workspace `addTab` path and `Sessions.open`. The new tab belongs to the current group. |
| Copy path | Resolves the absolute path and writes it to the clipboard. |
| Move to Trash | Asks for confirmation, then calls `move_to_trash`. If the selected file was moved, selection clears; the parent level is re-listed and Git is refreshed. Failure enters notification history. No permanent delete exists. |

Expand and collapse are Tree interactions, not actions. Selecting a file loads it; selecting a directory does not read a file. Row context menus run the same actions on that row without replacing the open editor first.

## Snapshot

The workspace schema remains version 1. `files` is optional and additive:

```ts
files?: Record<string, {
  open: boolean;
  showing: boolean;
  showHidden: boolean;
  expanded: string[];       // at most 64, relative to the group root
  selected: string | null;  // relative to the group root
}>
```

`open` means the sidebar tab exists; `showing` means Files currently covers the canvas. Both are saved so an open but hidden tab survives. Listings, file contents, unsaved buffers, watch handles and absolute expansion state are not persisted. The root comes from the group's current `cwd` at each launch.

Parsing keeps only groups that still exist. `open`, `showing` and `showHidden` are true only when the stored value is exactly `true`, and `showing` is forced false unless `open` is true. Expanded and selected paths pass a lexical check: a non-empty string, no NUL, not absolute and no `..` component; duplicates are removed and the first 64 expanded paths are kept.

The parser does not compare saved paths with the group's new root. A retained expanded path that no longer exists produces an unreadable marker when listed; a retained selected path that no longer exists leaves the editor empty. An absent `files` field means no Files view has ever been opened for that group. Unknown fields elsewhere in the document are ignored, so adding `files` does not require a schema bump.

## Security and privacy

All filesystem access goes through the `src/files/` APIs with a root and a relative path. TSX cannot request an arbitrary absolute path, and Rust rebuilds and checks the path before every operation. Root confinement uses canonical paths and component-boundary checks. Symlinks cannot lead listing, reading, saving or watching outside the root.

The workspace snapshot stores only view state, never file content. Unsaved text stays in group memory. Saving writes only the selected path after the root and modification checks. Move to Trash requires confirmation and acts on one path; there is no directory clearing or permanent deletion. File names and contents are not sent to the terminal or to notifications. Action failures write a short message to the in-app notification history, not file content. Preview mode never reads the disk or Git.

## Observability

There is no metrics system. Listing, read and save failures stay in the panel: a root error state, a per-level unreadable marker, or a file note. Git status failure keeps the tree usable and either clears or preserves the last overlay as described above. Reveal, default-app, open-terminal, copy and Trash failures go to the existing notification history because the result is needed after the user leaves the panel.

Watch degradation is a marker on the affected directory plus a working Refresh button; it does not emit a notification per event. Closing the view drops the watcher. Late listing and read results are discarded by their generation and root checks.

## Related documents

- [Terminal architecture](terminal.md): session ownership, PTY lifetime and the native surface.
- [Git architecture](git.md): status, diffs, Changes and explicit operations.
- [UI design](../design/ui.md): typography, surfaces, colors and control behavior.
- [Validation index](../validation/README.md): dated evidence across topics. Files evidence is in [files](../validation/files.md).
- [Product README](../../README.md): user-facing behavior and requirements.
