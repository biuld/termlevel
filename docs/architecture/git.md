# Git architecture

Git views are powered by the system `git` CLI; the application does not embed a Git implementation. The minimum version is Git 2.20. A version that is too old, a missing `git`, or an unreadable repository returns an explanatory error that the UI displays. Reads are centralized in `src/git/` and explicit write operations in `src/git_ops/`. Repository discovery and directory watching live in `src/git_discovery/`.

`GitWorkspace` coordinates repository selection and background tasks. A single scheduler scoped to the view refreshes data every 5 seconds, scans repositories every 30 seconds, and fetches the selected repository every 5 minutes while the application is active. Manual refresh also scans. Product preview runs no background tasks. A shared per-repository operation gate prevents overlapping Git mutations across view remounts. Automatic fetch failures are displayed without repeated notifications.

## Data path

```text
TSX GitPanel      ── status / diff ───────────────────────┐
TSX HistoryPanel  ── branches / worktrees / log / commit ──┤ Rust Git (read-only)
                                                           │ blocking (outside the engine thread)
                                                           │ git -C <repo> ...
                                                           ▼
                                    RepoStatus / RepoBranches / RepoWorktrees
                                    RepoLog / RepoCommitDetail / unified patch

TSX HistoryPanel  ── fetch / pull / push / checkout / createBranch / worktree* ── Rust GitOps
                                                           │ explicit operations, 60-second timeout
                                                           ▼
                                    git checkout / branch / worktree ...
```

## Read commands and limits

- Status uses `git status --porcelain=v2 --branch -z --untracked-files=all` and parses only the documented machine format; a rename uses the two NUL-separated paths in the `2` record, conflicts use `u`, and untracked files use `?`.
- Diffs use `git diff --no-ext-diff --no-textconv --no-color --unified=3 [--cached] -- <path>`. For untracked files, Rust reads bounded text and generates a new-file patch.
- Branches are read with `git for-each-ref` over `refs/heads`, `refs/remotes` and `refs/tags`, skipping `origin/HEAD`; an annotated tag's commit target comes from the peeled object. Rust retains a complete refs snapshot, HEAD and a version, and the UI receives at most 256 entries at a time. The same version returns only an unchanged marker without re-transferring the array or rebuilding the Tree. More than 32 MiB is an explicit error that keeps the last successful snapshot.
- Worktrees use `git worktree list --porcelain`, parsing worktree / HEAD / branch / bare / detached / locked / prunable. `current` is determined by matching the canonical path of the group directory against each worktree path by component prefix, taking the longest match.
- History uses `git log --topo-order --max-count=<limit+1> --skip=<n>`. When the first screen is read, the reachable commit tips are fixed from the refs snapshot; later pages reuse the same set and pass it through `--stdin`, avoiding duplicates or missing entries when refs move and avoiding argument-length limits. When HEAD is detached, the current HEAD is included. Commit metadata does not expand all refs through `%D`; Rust attaches at most 8 decorations and the full count per commit, and full names remain available in the branch / tag lists. Commit details separately read `git show -s` metadata, changed paths through `--name-status`, and a patch through `--first-parent --unified=3`.
- All commands start with an argument array and do not go through a shell. Reads set `GIT_OPTIONAL_LOCKS=0`, `GIT_TERMINAL_PROMPT=0`, `GIT_NO_LAZY_FETCH=1`, `GIT_PAGER=cat` and `LC_ALL=C`. Write operations do not set `GIT_OPTIONAL_LOCKS=0`, allowing the index lock and the repository's own hooks.
- Read commands time out after 3 seconds and history after 5 seconds; every command terminates its own process group. Status, diff, log and commit patches are each limited to 2 MiB, single-file previews to 512 KiB, and entries to 5,000; a commit page defaults to 200 entries and loads the next page through Load more at the bottom. The timeout covers draining pipes, so a helper still holding a pipe is cleaned up with the process group even if Git itself has exited.
- Branch names first pass through `git check-ref-format --branch`, and start points through `git rev-parse --verify <start>^{commit}`. Worktree removal and locking accept only paths from `git worktree list`. Write operations do not use `--force`.
- The repository root is located from the group path first, and commands then run against the repository root, so paths are repository-root-relative; when the group path is a repository subdirectory, the worktree `current` value still matches the worktree that owns it.
- While an operation is in progress, merge / rebase / cherry-pick / revert / bisect are detected through a marker in the worktree's git dir; read-only queries do not execute hooks.

## Supported and unsupported operations

Git queries remain read-only. History's explicit actions are fetch, fast-forward-only pull, push, checkout, create / delete branch, and worktree management (add, remove, lock and prune). Explicit write operations time out after 60 seconds.

- Fetch uses the default remote configured in Git. Pull updates the current branch of the current worktree; Git refuses a diverged branch, and remote refs are still refreshed after a failed operation.
- Push pushes only the current branch of the current worktree to its configured upstream remote branch. It pushes no other branches or tags and never force-pushes; a missing upstream or detached HEAD prompts the user. Network operations use system Git and the configured credential helper; interactive terminal prompts are disabled.
- Branch deletion uses `git branch -d` without force; Git refuses the current branch, an unmerged branch, or a branch checked out in a worktree.
- The application does not stage, unstage or commit, create merge commits, rebase, cherry-pick, reset or stash, and does not resolve conflicts.

Product preview provides no Git data and starts no `git`; test fixtures can inject a deterministic Git stand-in.

## Threading and read lifecycle

The NSH 0.6.0 AppKit main thread handles windows, system input and platform drawing submission; a separate engine thread runs QuickJS, signals, components and layout. The synchronous part of `async` still executes on the engine. NSH already provides a Tokio runtime, and the application uses `blocking` to put Git CLI execution, parsing, sorting, version computation and lane layout on its blocking thread pool. It does not create another runtime and does not depend on Tokio directly.

Each group / worktree session has its own read lease, reused when Changes / History mounts. The asynchronous admission future in `requests` suspends on the engine, and at most 4 read tasks execute application-wide; queued work does not occupy a blocking thread. Each lease and each query type accepts only the latest request. Switching repositories, replacing a query or unmounting cancels old requests, and a running read-only Git process group is terminated and reaped; generation / disposed checks also prevent late results from updating the UI. Final release of large caches also runs in the background, avoiding synchronous release of an entire refs table during close or engine future finalization. Cancelling a read does not cancel explicit Git writes and does not affect the PTY. Writes continue to use the cross-view repository mutex gate. Discovery uses an independent single-thread worker with cancellable Git probes and does not consume the four foreground read slots.

## Multi-repository discovery and selection

The design separates discovery, activation and selection, working-tree identity and name disambiguation with reference to Zed's [Repository Activation](https://github.com/zed-industries/zed/blob/main/docs/src/git.md), [GitStore](https://github.com/zed-industries/zed/blob/main/crates/project/src/git_store.rs) and [RepositorySelector](https://github.com/zed-industries/zed/blob/main/crates/git_ui/src/repository_selector.rs); Termlevel uses the terminal `liveCwd` instead of an editor's file-activation context.

Discovery, selection and loading Git data are managed separately. Discovery checks only the group directory and its direct child directories using `git rev-parse --show-toplevel`, accepts only working-tree roots, publishes incremental results, and is cancelled when the view is disposed or replaced. Rust `git_discovery/` shares a directory source per group root. It identifies the nearest ancestor repository, the root's direct child directories, and repositories owning the terminal `liveCwd` or explicitly added paths, without recursively traversing the whole project tree. Creating, deleting or moving a directory, or a `.git` change, triggers a merged background rescan; macOS uses non-recursive kqueue directory watching instead of scanning every repository every 30 seconds. Each source watches at most 1024 directories; a watch failure or limit overflow shows an explicit error and offers Rescan. Watching stops when there is no subscription, at most 8 idle sources are retained, and cancelling one view does not affect other subscribers to the same source.

Each worktree gets a stable in-process ID from its canonical root / git dir; linked worktrees are shown independently, and submodules are identified through Git metadata. The common dir is used separately for Rust / TSX write mutual exclusion, preventing two linked worktrees from modifying shared refs at the same time. Directory snapshots carry a revision and at most 256 entries per page; a version change restarts assembly, and unchanged directories are not re-sent. The selector uses a searchable native virtual List; duplicate names are disambiguated by the shortest parent path, long names are middle-truncated, and the full path appears in the subtitle and tooltip. Inactive repositories show only directory metadata and an existing status summary; status / fetch is not run per list item.

Changes and History share the group's selection. By default it follows the active terminal directory; a manual selection pins it, and only an explicit Follow terminal directory restores automatic following. If the selected repository becomes temporarily unavailable, the selection is kept and a message is shown instead of jumping to another repository. The selected canonical root, manual mode and at most 32 explicit paths are saved in the workspace snapshot; runtime IDs and Git data are not persisted.

Runtime state is retained per group / root for search, scope, selected commit / file, expanded directories, diff layout and scroll position. Heavy data caches at most three sessions, each holding at most 5000 commits / 32000 refs; beyond the limit only lightweight preferences are kept. Restoring a session must verify that the Rust history scope / count cursor is consistent; if cancelled pagination causes an inconsistency, the first page is reloaded so the cache and native lanes do not misalign. Eviction or deleting a group releases the read lease and caches; closing the Git panel does not change PTY lifecycle.

## History view internals

Changes and History are group views opened on demand; closing them does not affect terminals or splits. The workspace snapshot saves which views are open and which one is selected. History's left pane uses a virtualized Tree for Local / Remote / Tags, and the middle pane uses a native recycling List for commits, 200 per page; only commit rows near the viewport are created. Rows keep the data accessor provided by the List and are identified by SHA, with selection state updated inside the recycled row for the current SHA. Resizing panes, updating refs and loading more preserve the list container and scroll position.

Rust `graph` assigns fixed slots across pages by parent relationship, using a target-to-slot index and reusing empty slots without shrinking other slots. It records real incoming markers; a new branch tip has no incoming line above it. Merging into an existing parent commit connects to the parent slot, and through-lines keep their positions. Each row transmits at most the first 24 through-lines; overflow lanes share the last column and show a simplified hint, while the commit record is retained. The Canvas compresses lane spacing according to the actual column width, and the graph column occupies at most one quarter of the commit panel width. Search filters only loaded commits and keeps the original lane data; full ref-name matching runs against Rust's refs snapshot, the UI merges matching SHAs, and branches absent from a chip preview remain searchable. When intermediate commits are hidden, only dots are drawn, avoiding lines that imply adjacent relationships that do not exist. The layout functions in `commit-graph.ts` serve only injected test / preview transports; production obtains geometry from Rust.

A commit row shows at most 2 width-limited ref chips plus an additional count, and all chips together occupy no more than 45% of the title area. Names are middle-truncated to preserve the prefix and the distinguishing suffix, with the full name on hover; the left pane and the top current branch / scope use the same truncation. Full refs remain in the Tree, and details show at most 8 decorations, avoiding tens of thousands of views for a single commit's decorations. The title-bar summary can shrink, while global Git actions and search always retain space.

The right pane shows the commit message, changed paths and DiffView; when the canvas is narrower than 1000 pt, details move below the commit list. A selected file shows only its own patch; patches are not `trimEnd`-ed, preserving trailing context lines. The Changes directory tree, the worktree / changed-files list and the selection overlay keep their own data paths.

Status, refs and worktrees refresh every 5 seconds. Log is reloaded only when the Rust refs version or HEAD attached / detached state changes; unchanged refs / worktrees do not update the corresponding signals. A failed query keeps the last successful snapshot and displays an error. SHA identifies only row identity, while decorations on rows and details come from the latest log snapshot. Explicit operations are still rejected by Git for dirty conflicts, divergence or illegal worktree removal; the application does not automatically stash or force.
