# Roadmap

Termlevel aims to be a native terminal workspace organized with Firefox-style vertical tabs and tab groups. The order is macOS AppKit first, then Windows and Linux after the core behavior and platform acceptance are complete. NSH's cross-platform claims do not replace Termlevel's own platform tests.

This document records unfinished work; it is not a changelog. Current behavior belongs in the [architecture overview](architecture/README.md), [Files architecture](architecture/files.md) and the [design documents](design/ui.md). Completed work is summarized below, and acceptance evidence lives under [validation](validation/README.md).

## Current state

- **Workspace:** vertical tabs with groups, colors, pinning, drag reordering, cross-group moves, per-group working directories, collapse, search, persisted sidebar width, fonts and appearance.
- **Tiling:** split right or down, place an existing session, directional focus, session swap, divider and keyboard resize, equalize, panes hidden or closed explicitly, zoom, and a focused-pane projection for narrow windows.
- **Terminal:** real PTY sessions with the login shell; the official Ghostty 1.0.1 core behind an independent Rust wrapper; Rust-owned parsing, grid, scrollback, search and session state; native AppKit fixed-cell rendering and IME composition.
- **Status and notifications:** OSC 7501 program status, OSC 9;4 progress, OSC 7 live working directory, OSC 22 pointer shape, OSC 133 prompt state, OSC 9 / OSC 777 notifications, BEL alerts and an in-app notification history.
- **Git:** per-group Changes and History, multi-repository discovery and selection, status and diffs, commit lanes, branches, remotes, tags and worktrees; fetch, fast-forward pull, push, checkout, branch creation and deletion, and worktree management.
- **Files:** per-group directory tree rooted at the group directory, bounded UTF-8 text editing, Git status letters and relative-to-HEAD editor marks, file actions, and non-recursive macOS directory watching for expanded folders.
- **Persistence:** workspace structure, layout, group and tab state, tool views, settings, fonts, scrollback limit and terminal identity survive restart. Restart opens new shells in saved directories; it does not restore old processes.
- **Automation:** headless suites and macOS AppKit acceptance cover the implemented behavior. Manual GUI acceptance, cross-platform acceptance, packaging and several real-world compatibility scenarios remain open.

## Manual GUI acceptance on macOS

Native automation does not replace sustained human use. Record each item in the matching validation topic.

- [ ] Workspace window: title-bar drag, context menus, sidebar resize, focus restore, copying a selection, and high-DPI behavior.
- [ ] Tiling interaction: tab and pane drag/drop, insertion feedback, divider feel, keyboard resize, hide/close/zoom, and layout restoration after the window grows again.
- [ ] Input and IME: the real system candidate window, complex composition and cancellation, switching keyboard layouts, and emoji entry.
- [ ] Selection: cross-screen drag, edge auto-scroll, release, copy, double-click word, triple-click line, Alt rectangle selection, and coordination with the history scrollbar.
- [ ] Performance under sustained load: many sessions, continuous output, large scrollback, memory growth and frame timing. Record measurements instead of impressions.
- [ ] Accessibility: VoiceOver reading order and labels, Reduce Motion with static working indicators, long titles, multiple font sizes, and pinned / grouped combinations.
- [ ] State and history: long notification histories, high-frequency agent status reports, unread/read transitions, closed-source navigation, and status behavior at prompt boundaries.

See the [terminal design](design/terminal.md), [tiling design](design/tiling.md), [OSC and status design](design/osc-status.md) and [UI design](design/ui.md) for the intended behavior.

## Real-world terminal compatibility

Protocol support is not the same as daily compatibility. Verify each application in a real session and keep per-application evidence.

- [ ] Neovim, tmux, less, top and common coding-agent CLIs, including alternate screen, mouse reporting, bracketed paste, focus reporting and reflow.
- [ ] Keyboard layouts, modifier combinations, physical key / layout text / repeat / release handling, and IME interaction in full-screen applications.
- [ ] Chinese, combining characters, emoji sequences, wide cells, fallback fonts and cross-style shaping.
- [ ] Kitty image animation, multiple image layers, large transfers and sustained image output.
- [ ] Long-running output, synchronized output boundaries, history limits, search over large scrollback, and multi-session responsiveness.

## Windows and Linux

The current non-macOS platform entry is an explicit placeholder, not a working renderer. Cross-platform support requires all of the following per target:

- [ ] Native terminal renderer and surface backend.
- [ ] Native input adaptation, IME, clipboard and system services.
- [ ] Ghostty build artifacts for the target, with checksum verification and a documented build path.
- [ ] PTY and session behavior, process cleanup and shell integration.
- [ ] Build, packaging, installation and a platform acceptance matrix.
- [ ] Manual daily-use acceptance on each platform. Compilation alone does not close this milestone.

## Session lifecycle and recovery

- [ ] Complete the session model: distinguish restoring workspace structure, reopening a shell, and restoring a living session. Decide explicitly whether a background process model is required.
- [ ] Crash recovery and its user-visible behavior.
- [ ] Close prompts while foreground or background work is running, including group close and application quit.
- [ ] Configuration errors and shell start failures, with a retry path that does not silently create a replacement session.

Only explicit session operations may create or close processes; a view remount, group collapse or selection change must not do so.

## Session configuration, search history, shell integration and links

- [ ] Session configuration for starting new shells and terminals.
- [ ] Search history, distinct from the existing find-in-scrollback behavior.
- [ ] Shell integration beyond the current OSC 133 prompt state.
- [ ] Link handling beyond the current OSC 8 HTTP(S) / mailto targets.

## Git acceptance and boundaries

- [ ] Manual History acceptance: real branch switching, refusal with a dirty working tree, worktree creation and removal with the group and shell lifecycle, long-history scrolling, and narrow-window layouts.
- [ ] Manual multi-repository acceptance: real network remotes and credential helpers, rescan after exceeding the 1,024-directory quota, network filesystems, and sustained directory churn.
- [ ] Verify every explicit operation against a real repository, not only a fixture transport.

Explicitly out of scope: merge, rebase, cherry-pick, stash and conflict resolution. Termlevel also does not stage, unstage, commit, create merge commits, reset, force-push, or push tags and other branches. These boundaries are product decisions, not missing UI.

## Packaging and distribution

- [ ] macOS application bundle, icon, signing, notarization, update channel and release verification.
- [ ] Installable artifacts for Windows and Linux after their platform acceptance.
- [ ] Reproducible release evidence: checksums, bundled fonts and licenses, Ghostty wrapper license, and archive validation.

Ad-hoc local packaging is available today; distribution signing, notarization and updates are not configured.

## How work closes

A checkbox closes only when the behavior is implemented or accepted and the evidence is recorded in the corresponding validation topic. Headless tests, preview fixtures, native automation and manual GUI acceptance are separate evidence types, and one does not substitute for another. Existing failures stay visible instead of being hidden by changed assertions or timeouts.
