# Validation: Git

| Date | Entry |
|---|---|
| 2026-11-16 | Branch marker update for same-SHA commits after push |
| 2026-11-16 | Group overview History, branches and worktrees |
| 2026-11-16 | Git group tabs |
| 2026-11-16 | History Fetch / Pull |
| 2026-11-16 | History toolbar and Git notifications |
| 2026-11-16 | History split handle escaping its boundary |
| 2026-11-16 | Git directory cleanup, preview isolation and History loading state |
| 2026-11-16 | Changes side-by-side diff |
| 2026-11-16 | Changes diff layout button position |
| 2026-11-16 | History Push |
| 2026-11-17 | Multi-repository directories, selection and runtime cache |
| 2026-11-17 | Git large-repository isolation, commit lanes and long names |

## 2026-11-16 · Branch marker update for same-SHA commits after push

Uncommitted fix based on `0dff5c2`. The root cause was that keyed commit rows stored the initial row object, so when the SHA did not change they still showed the old refs; the old commit metadata in the details area likewise was not re-read when the SHA stayed the same. Another omission was that periodic refresh compared only ref targets, not whether HEAD was attached/detached or the current marker.

- Added regressions for push marker movement on the same SHA and for HEAD detaching in place; both failed before the fix. After the fix the list and details share the latest log decorations, keep the selected old commit, and do not re-read the patch because markers changed.
- `./scripts/nsh test tests/workspace/GitCommitRefresh.test.tsx` 4/4, passing new cases for an added SHA, refs-only change, HEAD-only change, and removal of a marker from a selected old commit.
- `./scripts/nsh test tests/workspace/History.test.tsx` 6/6, `GitActions.test.tsx` 1/1; the local bare remote's `push_updates_only_the_current_upstream_and_rejects_divergence` 1/1. Nothing was pushed to any external remote.
- The native window run of `GitCommitRefresh.test.tsx --target macos --foreground` passed its first three cases; its Git data is a simulated snapshot and does not represent a real remote push. Checked screenshot `artifacts/git-ref-refresh-after-push.png`, confirming that both the top row and the details show origin/main, and that the old row has lost the marker.
- `./scripts/nsh check`, `./scripts/nsh build`, `git diff --check` passed. This run targeted Git History and its action tests; the full headless suite was not rerun. Existing failure records for the earlier sidebar split test and strict Clippy remain.

## 2026-11-16 · Group overview History, branches and worktrees

Uncommitted changes based on `bf27737`. Added `HistoryPanel`, `commit-graph.ts`, and `src/git_ops.rs`, and added a Changes / History switch to the group overview header. The validation environment was macOS arm64, Rust / Cargo 1.99.0, NSH 0.5.0; window 1200 × 780.

Implementation boundaries:

- Read-only queries remain in `src/git.rs`: porcelain v2 status, diff, `for-each-ref` branches / tags, `worktree list --porcelain`, `git log`, single-commit metadata / files / patch. Reads do not set optional locks and do not run hooks.
- Explicit writes are centralized in `src/git_ops.rs`: checkout, branch creation, branch deletion (`git branch -d`, no force), worktree add / remove / lock / unlock / prune. Write operations may take the index lock and run repository hooks, but do not use `--force`; branch names go through `check-ref-format --branch`, starting points through `rev-parse --verify`, and worktree paths must match `worktree list`. The current branch, unmerged branches, and branches already checked out in a worktree are rejected by Git / pre-checks.
- Commit lanes are computed from parent relationships by `ui/dashboard/commit-graph.ts`, with the fixed first column drawn using `Canvas`; commit rows are full-row `Button`s inside `Scroll` + `Each`, 200 per page with Load more at the bottom; the header and rows are sized from actual column widths, the Commit column fills the remaining space, dragging a split or resizing the window adds or removes Author / Date / Hash columns, and when the column set changes the list is remounted so old row widths are not reused. The selected row is drawn by the content layer as a primary overlay absolutely positioned at the selected index, without relying on row component re-render; hover uses native `hover:bg-tab-selected`. At canvas widths ≥1000 pt there are three columns; when narrower the right column is hidden and an adjustable details / patch split appears below the commit list. Changed files rows are clickable, with a selection overlay and hover; the first file is selected by default, and the `DiffView` on the right shows only that file's patch, split by the commit patch's file sections. `DiffView` no longer receives a `trimEnd()`-ed patch: deleting the trailing empty context line leaves the hunk count short, which is why `45aba59` showed nothing at all. The panel lightly refreshes status / branches / worktrees every 5 seconds, and reloads commits only when the HEAD target changes or the scope becomes invalid.
- Merge, rebase, cherry-pick, reset, stash, conflict resolution, and MR / PR are all unimplemented; there is no stage / unstage / commit.

Validation:

- `./scripts/nsh check`, `./scripts/nsh build`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `git diff --check` passed.
- `cargo test --lib git`: **13 / 13**, covering version parsing, porcelain v2, refs, worktrees, log, name-status, temporary-repository log / branch / commit, files and patch for a merge commit against its first parent, and branch creation, switching, deletion, worktree add / remove.
- `./scripts/nsh test --junit artifacts/headless-tests.xml`: **218 / 218**, log `artifacts/history-panel-tests.log`. Added `History.test.tsx` covering Changes / History switching, branch and worktree lists, commit and patch, details updating after clicking a commit, Changed files defaulting to the first file, patch switching after clicking another file, switching to the narrow layout and hiding Hash when the viewport shrinks to 880 pt, returning to the wide layout after enlarging to 1600 pt, creating a branch (without checkout), the delete-branch dialog, and the dialog flow for creating and deleting a worktree; `commit-graph.test.ts` covers linear history and merge lanes. The first full run produced one `sessions::tests::closing_allows_shell_to_hang_up_background_jobs` failure ("background job survived closing its terminal"), a test unrelated to these changes; an individual rerun and two subsequent full reruns all passed, kept as a record of existing instability under parallel load.
- Native automation: `./scripts/test-native history` **1 / 1**, screenshots `artifacts/history-merge-graph.png`, `artifacts/history-selection.png`, `artifacts/history-hover.png`, `artifacts/history-file-hover.png`, `artifacts/history-file-diff.png`, `artifacts/history-worktrees.png`; `./scripts/test-native gitlive` **1 / 1**, viewing History and Worktrees on a real repository, shrinking the window to 920 × 680 to check the narrow layout and enlarging to 1600 × 900 to check column filling, screenshots `artifacts/dashboard-history-live.png`, `artifacts/dashboard-history-worktrees.png`, `artifacts/dashboard-history-920.png`, `artifacts/dashboard-history-wide.png`. The native host retained its existing `__eh_frame` link warning.
- The preview fixture does not run git; the real-repository screenshot shows master as the current branch, 27 changed files, commit lanes, commit details, and the selected file's patch. The native hover screenshot confirmed by pixel sampling that hovered rows are `tab-selected` and the selected row is a primary tint; file-list hover is also visible. The native host runs in the background and this is not manual GUI acceptance.
- After the last batch of UI adjustments, only targeted checks were run as requested: `./scripts/nsh check`, `./scripts/nsh build`, targeted `History.test.tsx`, `./scripts/test-native history`, `./scripts/test-native gitlive`, `cargo fmt --check`, `git diff --check` all passed; **the full 218 tests were not rerun**. The 218 / 218 and 13 / 13 above are the most recent full results before this batch of UI adjustments.
- Found and fixed during user acceptance: the architecture-document patch from `45aba59` did not display, because `patch().trimEnd()` removed the empty context line at the end of a hunk; both History and Changes now pass it to `DiffView` unchanged. After enlarging a small window the Author column appeared but the author was empty in rows, because list rows did not re-render with split width; the commit list was changed to `Scroll` + `Each`, remounted when the column layout changes, and a headless case verifies that Author appears from 700 → 1600. After clicking a commit the details updated but the row background did not; changed to an absolutely positioned overlay by selected index; Changed files and worktrees got the same treatment. Targeted re-validation: `./scripts/nsh check`, `History.test.tsx`, `./scripts/test-native gitlive` passed; the real-repository screenshot `artifacts/dashboard-history-45aba59.png` confirms the selected row background, file selection, and patch display. The full suite still was not run.

Not yet verified: the behavior of running shells after switching / deleting branches from the UI in a real repository, Git rejection messages with a dirty working tree, group and shell lifecycle after creating / deleting worktrees from the UI, long-history scrolling, the narrow-window three-column layout, and real IME and selection regressions. The Rust test for branch deletion and the headless dialog flow passed, but merged / unmerged / already-checked-out branches were not individually accepted by hand. Preview screenshots and headless interaction cannot replace this real-machine acceptance.

## 2026-11-16 · Git group tabs

Uncommitted changes based on `7dd7c8c`. `./scripts/nsh check`, `./scripts/nsh build`, `git diff --check` passed. The full `./scripts/nsh test --junit artifacts/headless-tests.xml` was 218 / 221: the clipboard test failed with `pbcopy rejected the sample`; the History test fixture's buttons had no layout size, and the split test still operated the old overview navigation; after fixing, `./scripts/nsh test tests/workspace/History.test.tsx` and `./scripts/nsh test tests/workspace/Tiling.test.tsx --filter 'each tab group keeps'` passed separately. During the full run an empty-group deletion boundary fix and test were added; the full suite was not rerun after the fix.

Native automation: `./scripts/test-native gitlive` and `./scripts/test-native history` each 1 / 1, passing real-repository queries and menu switching and producing screenshots such as `artifacts/dashboard-git-live.png`; the first was initially denied starting a dev server in the sandbox and passed when run in a permitted environment. Preview, headless, and native automation are not manual GUI acceptance; manual acceptance was not performed.

The same uncommitted change later fixed Git tab hover: `./scripts/nsh check`, `./scripts/nsh build`, `git diff --check`, and `./scripts/nsh test tests/workspace/App.test.tsx --filter 'Git menu opens'` passed; `./scripts/test-native gitlive` 1 / 1, and the native window screenshots `artifacts/git-tab-hover-title.png` and `artifacts/git-tab-hover-close.png` were checked. The full test suite was not rerun, and manual GUI acceptance was not performed.

The Git tab icon then changed to the theme color of its group: `./scripts/nsh check`, `./scripts/nsh build`, `git diff --check`, the targeted Git menu headless case, and `./scripts/test-native gitlive` passed. Checked the native screenshot of a purple group under the dark theme, `artifacts/git-tab-hover-title.png`; light theme and other group colors were not manually accepted natively.

Compact sidebar Git tab visibility fix: `./scripts/nsh check`, `./scripts/nsh build`, `git diff --check` passed; `./scripts/nsh test tests/workspace/App.test.tsx --filter 'compact'` was 6 / 6, covering Git tab switching and the collapsed group hidden count; `./scripts/test-native gitlive` was 1 / 1, and Changes / History switching after collapsing the sidebar passed, with screenshot `artifacts/git-tabs-compact.png` checked. The full test suite was not rerun, and manual GUI acceptance was not performed.

Ungrouped workspace entry (uncommitted changes based on `7dd7c8c`): `./scripts/nsh check`, `./scripts/nsh build`, `git diff --check` passed; `./scripts/nsh test tests/workspace/App.test.tsx` 23 / 23, `./scripts/nsh test tests/workspace/Persistence.test.tsx` 11 / 11, covering switching when there is no ungrouped terminal, title-bar creation, and restart restoration. `./scripts/test-native gitlive` 1 / 1, verifying the compact sidebar entry and group view switching, with screenshot `artifacts/ungrouped-compact.png` checked; after the screenshot the duplicate decorative icon in the compact sidebar title was removed, and check / build passed again. The full test suite was not rerun, and manual GUI acceptance was not performed.

## 2026-11-16 · History Fetch / Pull

Uncommitted changes based on `7dd7c8c` (including the earlier Git tab / Ungrouped work). `./scripts/nsh check`, `./scripts/nsh build`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `git diff --check` passed. `./scripts/nsh test --junit artifacts/headless-tests.xml`: 225 / 226; the only failure was `clipboard::tests::osc52_bytes_reach_the_pasteboard_as_unicode` with `pbcopy rejected the sample`, consistent with earlier records. During the full run the remote test's dirty-working-tree assertion was expanded, and targeted re-validation then passed.

`cargo test --lib git_ops::tests::fetch_and_fast_forward_pull_use_the_current_worktree_upstream` used a local temporary bare remote and two worktrees, verifying that fetch only updates remote refs, pull fast-forwards, conflicting uncommitted files are not lost, and HEAD stays unchanged when diverged; the History headless tests verified success, failure, and refresh after failure. `./scripts/test-native history` 1 / 1, verifying the buttons and feedback through a simulated transport; checked the narrow-window screenshot `artifacts/history-fetch-pull-narrow.png`. No acceptance was performed against a real network remote or a remote requiring interactive authentication; manual GUI acceptance was not performed.

## 2026-11-16 · History toolbar and Git notifications

Uncommitted changes based on `7dd7c8c`. Fetch / Pull use icon buttons; commit search expands on demand and reports the number of matches in loaded data, and Load more is still available when there are no matches. Git operation results are written to the host notification history, and a 5-second toast appears in the top-right of the app; results still arrive after switching away from History.

`./scripts/nsh check`, `./scripts/nsh build`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `git diff --check` passed. `./scripts/nsh test --junit artifacts/headless-tests.xml` was 227 / 228, and the only failure was still `clipboard::tests::osc52_bytes_reach_the_pasteboard_as_unicode` with `pbcopy rejected the sample`. During the full run, Load more when search has no matches and the display of app information notifications were added; `History.test.tsx` 3 / 3 and `GitActions.test.tsx` 1 / 1 then passed targeted re-validation. `./scripts/test-native history` 2 / 2, and the native screenshots `artifacts/history-git-toast.png`, `artifacts/history-search-open.png`, `artifacts/history-search-compact.png` were checked; the final no-match hint and notification row display adjustments were not rerun through native automation. Manual GUI acceptance and real network remote authentication acceptance were not performed.

The same uncommitted change later adjusted terminal tab icons: the normal terminal icon for a grouped terminal uses the group theme color both selected and unselected; running, done, blocked, and error states still use their original status colors. `./scripts/nsh check`, `./scripts/nsh build`, `git diff --check`, and the workspace appearance targeted test 1 / 1 passed; the full suite and native window acceptance were not rerun.

## 2026-11-16 · History split handle escaping its boundary

Uncommitted changes based on `7dd7c8c`. NSH 0.5.0's `ResizableState::resize_panel_at_handle` directly increases the size of the following panel when shrinking the previous one, without truncating to the following panel's `maxSize`; the actual flex layout is constrained by the limit, but the handle model keeps moving. In the native reproduction the handle was at screenshot x=2044 while the details panel boundary was at x=2104 (2× screenshot, 30 pt apart). Removed the upper limits on the right-hand details panel in the wide History layout and the bottom details panel in the narrow layout, keeping each panel's minimum size, to avoid triggering that upstream defect; the drag range is therefore no longer limited by the former 540 / 480 pt caps.

`./scripts/nsh check`, `./scripts/nsh build`, `git diff --check`, and `./scripts/nsh test tests/workspace/History.test.tsx` 4 / 4 passed. `./scripts/test-native history --filter 'history pane divider'` 1 / 1, dragging in a real macOS window and capturing the normal position, both limits, and the narrow layout; pixel scanning of the dark screenshots `artifacts/history-divider-drop.png`, `artifacts/history-divider-limit-left.png`, `artifacts/history-divider-limit-right.png` confirmed only two panel boundary lines, and the narrow-layout screenshot `artifacts/history-divider-narrow.png` was inspected visually. NSH's macOS test target does not support `dragAndHold`, so no native screenshot mid-drag-hold was obtained; the full test suite and manual GUI acceptance were not rerun.

## 2026-11-16 · Git directory cleanup, preview isolation and History loading state

- `./scripts/nsh check`, `./scripts/nsh build` passed.
- `./scripts/nsh test --junit artifacts/headless-tests.xml`: 230/231 passed; the existing `clipboard::tests::osc52_bytes_reach_the_pasteboard_as_unicode` failed with `pbcopy rejected the sample`. The History loading state and product preview isolation targeted tests passed.
- Native automation: `./scripts/test-native gitlive` 1/1, `./scripts/test-native history` 3/3 passed; the former verified a real repository and a collapsed sidebar. Checked native screenshots of the expanded and collapsed sidebar.
- Manual GUI operation acceptance was not performed. The product preview does not start Git, a shell, or real workspace storage; simulated Git and terminals are injected only for test fixtures.

## 2026-11-16 · Changes side-by-side diff

- `./scripts/nsh check`, `./scripts/nsh build`, and `./scripts/nsh test tests/workspace/Git.test.tsx` (2/2) passed, covering mode switching, file and staged state, and switching back to the unified view in a narrow window.
- Native automation `./scripts/test-native gitlive` 2/2 passed; checked wide and narrow window screenshots of the side-by-side diff on a real repository, where long lines in the narrow window wrap automatically. Manual GUI operation acceptance was not performed; the full headless suite was not run.

## 2026-11-16 · Changes diff layout button position

- The switch button moved to the top toolbar of Changes to avoid overflowing the file title bar. `./scripts/nsh check`, `./scripts/nsh build`, and the targeted Git test 2/2 passed.
- Native automation `./scripts/test-native gitlive` 2/2 passed; checked the narrow-window screenshot. Manual GUI operation acceptance and the full headless suite were not performed.

## 2026-11-16 · History Push

- `./scripts/nsh check`, `./scripts/nsh build` passed; the local bare-repository Push targeted test passed, covering the current upstream, rejection of non-fast-forward, no upstream configured, and detached HEAD, and not being affected by wildcard push configuration. The targeted History test 6/6, and the targeted Git notification test 1/1.
- Native automation `./scripts/test-native history` 3/3, checking buttons and notifications through a test fixture; no real remote was pushed, no manual GUI acceptance was performed, and the full headless suite was not run.

## 2026-11-17 · Multi-repository directories, selection and runtime cache

Based on `master` / `238d795`, retained as uncommitted changes together with the Git performance fixes described below. Referring to Zed's separation of discovery / activation / selection, Termlevel uses the terminal liveCwd as context; specific ownership and reference sources are in the "Multi-repository directories and selection" section of [architecture](../architecture/git.md).

- A single Rust background worker shares the directory source, identifying parent repositories, direct child repositories, and deep repositories in explicit / terminal contexts; canonical worktree identity distinguishes linked worktrees from submodules, and the common dir is used to serialize write operations. Each directory transfer carries at most 256 entries, reassembled when the revision differs; unchanged data is not resent. The foreground runs at most four Git read tasks, directory probing queues independently, and no additional Tokio runtime is added.
- A macOS non-recursive kqueue watch replaces periodic full scans; events are coalesced and stale probes cancelled; at most 1024 watched directories, with a Rescan prompt on error or overflow. When watching is unavailable, updating the terminal context does not repeatedly scan all direct child directories. Watches are released when there are no subscribers; cancelling one subscription does not affect others. The first-round FSEvents backend could not receive newly created repository events in controlled tests; the final kqueue case passed.
- Each group persists a manual selection and an explicit directory; manual mode is not overridden by cwd changes, and when the selected repository disappears the selection is kept and shown as unavailable. The picker uses a virtual List / path search, disambiguation of duplicate names, full-path hints, and worktree / submodule markers; Git data is loaded only for the current repository.
- Three bounded runtime sessions retain History data and native cursors; on switching back the scope / count is checked, and the cursor is reloaded if it does not match. Search, selected commit / file, expanded directories, diff layout, and scroll position are isolated per group / root; over the limit only lightweight preferences are kept, and deleting a group or evicting a cache releases the native lease. No PTY is created or closed because of switching groups, hiding Git, or discovering repositories.

### Automated results

- `CARGO_NET_OFFLINE=true cargo test -p termlevel-native git -- --nocapture`: **29/29**. Added validation for real parent / deep contexts, path aliases, linked worktree identity / common dir, directory-creation watching, shared subscription cancellation, pagination revision switching, and the shared write gate. Log: `artifacts/multi-repository-rust.log`.
- `./scripts/nsh test <file> --junit <report>`: MultiRepository, GitRepositories, History, GitCommitRefresh, GitActions, GitScheduler, Snapshot, Persistence, **35/35** in total. Includes viewport row counts and search with 1500 repositories, per-group manual / automatic selection, not jumping when a repository is lost, History search / selection / cursor restoration and continued pagination, and snapshot path validation. Logs / reports: `artifacts/multi-repository-<file name>.log` / `.xml`.
- `CARGO_NET_OFFLINE=true ./scripts/nsh build`, `CARGO_NET_OFFLINE=true ./scripts/nsh check`, `cargo fmt --all -- --check`, `git diff --check`: passed. New and modified source files all remained under 500 lines. Build / check logs: `artifacts/multi-repository-build.log`, `artifacts/multi-repository-check.log`.
- `cargo clippy --workspace --lib -- -D warnings`: passed. The full all-targets run still fails only on the unmodified `src/clipboard.rs:86` / `:136` `items_after_test_module`, so full clippy is not recorded as passing. Logs: `artifacts/multi-repository-clippy-lib.log`, `artifacts/multi-repository-clippy.log`.

Dependency cache was complete; crates.io DNS was initially unavailable, and subsequent key checks used `CARGO_NET_OFFLINE=true`, without modifying the project's network / Cargo configuration. The full terminal / PTY regression was not run.

### Native window

`CARGO_NET_OFFLINE=true ./scripts/test-native repositories` used two repositories with 600 real commits each and one linked worktree; the test fixture injects only session metadata and creates no PTY. The directory itself is inside the Termlevel repository, so automatic recognition of the parent Termlevel repository is also verified; no real remote fetch / push was performed.

Native acceptance **1/1 passed**, and the final screenshot was reviewed (including the dark input field text color). The flow verified switching between alpha / beta, reusing the first page when switching back to alpha (resume succeeded and log count did not increase), continuing to read the second page, recognizing alpha when the group root is `alpha/src`, keeping the selection when returning to the original group, and the worktree marker. Reports / logs: `artifacts/multi-repository-native.xml`, `artifacts/multi-repository-native.log`. Screenshots: `artifacts/multi-repository-native-restored.png`, `artifacts/multi-repository-native-picker.png`.

Manual acceptance with a daily-use repository was not performed; network filesystem watching, the manual Rescan flow with more than 1024 directories, and the load of long-term frequent directory changes still need separate validation and are not replaced by this controlled native automation result.

## 2026-11-17 · Git large-repository isolation, commit lanes and long names

Uncommitted changes based on `master` / `238d795`. Reads were split from the oversized `src/git.rs` into `src/git/`, and all Git source files modified here are under 500 lines. CLI / NSH crates remain 0.5.0; no new Tokio dependency or runtime was added.

### Implementation and boundaries

- Changes / History each own a read lease; globally at most 4 read tasks, and waiting for admission does not occupy a blocking worker. Replacing a query, switching repositories, or unmounting cancels the old task; the read-only subprocess group is terminated and reaped in the background, and pipe draining is also deadline-bound. Final cache release moved to a cleanup thread to avoid synchronously releasing the entire refs table when the engine future ends. Explicit writes and the PTY lifecycle are unaffected by read cancellation.
- refs are sorted / versioned in the Rust background, delivered to JS in batches of at most 256; when unchanged only a version marker is returned. The limit was adjusted from 2 MiB to 32 MiB, with an explicit error when exceeded instead of silently returning an incomplete branch list. Building a complete Tree for the first time still has engine cost, and the stress results below do not mean the 16 ms frame budget has been met.
- Commits use a recycling native List, with Rust maintaining lanes across pages; the first screen pins immutable revision tips, queries later pages through stdin, and avoids duplicates, missing entries, and misconnected lanes caused by moving branches. Fixed stray line jumps after slot contraction, false incoming edges for new tips, and misleading connections after filtering hides intermediate commits. At most 24 independent lanes are displayed, with extras sharing the last column and showing a hint; commits are still fully retained.
- Each commit carries at most 8 refs previews plus the full count, preventing `%D` with tens of thousands of branches pointing at the same commit from blowing up log / show metadata. Full ref name matching is performed in the Rust snapshot, so branches not shown in the chip preview are still searchable.
- Long branches / tags use middle elision and a width-capped full tooltip; the refs in a commit row occupy at most 45% of the title area in total, showing at most 2 chips plus an extra count. Details refs scroll within the split and do not cross the Changed files boundary; the top summary shrinks to preserve the search and action buttons.

### Automated results

- `cargo test -p termlevel-native git -- --nocapture`: **26/26**, including real 20,000 long remote refs, annotated tag peeled target, bounded pagination / unchanged, full ref search, fixed tips after branches move, bounded load with a wide graph, lane continuity across pages, read admission / replacement / close cancellation, and the deadline while a helper holds the pipe. Log: `artifacts/git-review-rust.log`.
- `./scripts/nsh build`, `./scripts/nsh check`, `cargo fmt --all -- --check`, `git diff --check`: passed. Logs: `artifacts/git-review-build.log`, `artifacts/git-review-check.log`.
- Ran History, GitCommitRefresh, GitActions, GitRepositories, GitScheduler, Git, commit-graph, and GitScale separately via `./scripts/nsh test <file> --junit <report>`: **22/22** (6 + 4 + 1 + 1 + 2 + 2 + 3 + 3). Reports / logs: `artifacts/git-review-<file name>.xml` / `.log`. Covers viewport row counts with 4,000 loaded commits, unchanged refresh after scrolling, search among loaded commits, late-result isolation, and lease release on unmount; the full terminal / PTY regression was not run.
- `cargo clippy --workspace --lib -- -D warnings`: passed. Full `cargo clippy --workspace --all-targets -- -D warnings` still fails on the existing `src/clipboard.rs:86` / `:136` (`items_after_test_module`); the file is unmodified, and full clippy is not recorded as passing. Logs: `artifacts/git-review-clippy-lib.log`, `artifacts/git-review-clippy-all-targets.log`.

### Native automation and screenshots

`./scripts/test-native gitscale`: **2/2**. `scripts/prepare-git-scale` created a reproducible repository with an ownership marker under the ignored artifacts: **10,000 real commits (including forks / merges), 20,000 remote refs, plus main and a tag, for 20,002 refs in total**. The repository is for controlled local acceptance only, with no real remote network operations.

In a macOS native window using a 20 ms JS heartbeat probe, the maximum interval during first-screen loading in the final run was **103 ms**; during the unchanged refs refresh phase it was **35 ms**, and the log call remained 1. Also verified the second page load, a search hit for the `19099` branch not among the 8 preview refs, and the narrow window / full tooltip. This measures engine heartbeat intervals, including scheduling and commit overhead, not AppKit frame time; it cannot be used to claim sustained 60 FPS or that cold-cache / million-commit repositories have passed.

Reports / logs: `artifacts/git-scale-native.xml`, `artifacts/git-review-native.log`. The following native screenshots were reviewed:

- `artifacts/git-scale-native-loaded.png`: two parent edges of a real merge, fixed pass-through lane slots, and details refs scrolling within the split.
- `artifacts/git-scale-native-page-two.png`, `artifacts/git-scale-native-ref-search.png`: pagination and full ref name search.
- `artifacts/git-long-refs-wide.png`, `artifacts/git-long-refs-narrow.png`, `artifacts/git-long-ref-tooltip.png`: long-name elision, title space, and the full tooltip.
- `artifacts/git-long-refs-search.png`: only matches shown among 4,000 commits, hiding broken graph connections.

The newly added large-list fixture once showed no filter update on the headless per-keystroke input path; the headless stress case now verifies the same result by pasting the path, and per-keystroke input is separately verified by the native case above — the headless per-keystroke path is not recorded as passing. The native script's initial 30-second total deadline was insufficient for the long-distance scrolling, screenshot, and search steps; the final script uses a 60-second total deadline, and the heartbeat acceptance limit remains 500 ms without being relaxed.

Manual acceptance boundaries: screenshot inspection is complete; long-term scrolling in a real daily-use repository, continuous fetch, manual typing feel, a million commits / more refs, and a strict 16 ms frame budget still need separate measurement.

Entries were translated from the consolidated validation log; failures above remain part of the record and must not be erased.
