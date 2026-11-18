# Development Guide

This guide covers directory ownership, dependency rules, tests, validation commands and generated files. The [documentation map](README.md) is the entry point for the document set; [AGENTS.md](../AGENTS.md) defines maintenance rules; the [product README](../README.md) covers installation, running and user-facing features.

## Source layout

Ownership is defined at the directory level. The code tree is the authoritative record of individual files; do not keep a per-file inventory here.

| Path | Owns |
|---|---|
| `App.tsx` | NSH mount entry. It only mounts the workspace. |
| `ui/app/` | Startup restore, window, menus, shortcuts, dialogs and application wiring. |
| `ui/workspace/` | Workspace state, snapshot and persistence, sidebar, groups, tiling and native window UI. |
| `ui/git/` | Repository selection, Changes, History, branch and worktree views, and commit lanes. |
| `ui/files/` | Group file tree, file previews and text editing, Git status overlays and directory-watch subscriptions. |
| `ui/settings/` | Settings UI. |
| `ui/notifications/` | Notification-history subscription, panel and rows. |
| `ui/terminal/` | Native terminal display and find bar. |
| `ui/shared/` | Stateless controls, icons, fonts, text roles and theme mapping. |
| `src/` | Rust application crate: native exports, runtime, session ownership, terminal adaptation and platform integration. |
| `src/files/` | Single-level listings, bounded text reads and saves, expanded-directory watches and file actions. |
| `src/git_discovery/` | Shared repository directories, context discovery and directory watches. |
| `src/git/` | Read-only Git CLI, read leases and per-view read state. |
| `src/git_ops/` | Explicit Git operations: fetch, pull, push, checkout, branch and worktree management. |
| `src/sessions/` | Session API, PTY and process ownership, and the bounded input queue. |
| `src/session_view/` | Display control, input policy, rendering and the platform backend. |
| `crates/ghostty-vt/` | Independent memory-safe wrapper around libghostty-vt. |
| `crates/ghostty-vt-sys/` | Official C ABI, headers, bindings and prebuilt-library build. |
| `tests/` | Headless tests, native acceptance tests and protocol probes. |
| `scripts/` | Pinned tooling and validation entry points. |
| `assets/` | Committed product assets. |

The stable entry points are `App.tsx`, `ui/app/WorkspaceApp.tsx`, `src/lib.rs`, `src/sessions/mod.rs` and `src/session_view/`. Their contracts and data flow belong in [architecture](architecture/README.md), not here.

TSX application wiring lives in `app`; workspace state and operations live in `workspace`; the terminal view lives in `terminal`; stateless presentation lives in `shared`. Components use PascalCase; state and utility modules use lower-case and hyphens. Keep the existing two-space TSX indentation and avoid whole-file formatting rewrites during moves.

## Dependency rules

TSX:

- `app` may compose `workspace`, `settings`, `notifications`, `terminal` and `shared`.
- `workspace` state modules do not import `app` or terminal presentation components. `TilingCanvas` is the composition point from workspace to the terminal surface.
- `terminal` may consume workspace state contracts, but it does not create application windows or take over process lifecycle.
- `shared` does not import any business module.
- Workspace state uses NSH signals and native services; it is not a pure functional domain layer. Tiling tree transforms and snapshot parsing keep separate entry points.

Rust:

- `src/lib.rs` only registers exports and native components.
- `SessionManager` in `src/sessions/manager.rs` owns the PTY, child process and worker threads. Only explicit session operations create or close processes.
- `src/terminal.rs` adapts Ghostty state to the application. `src/session_view/` owns display and interaction. Platform objects live only under `session_view/platform/`.
- `crates/ghostty-vt` and `crates/ghostty-vt-sys` do not depend on NSH, the PTY layer or the application. Pass owned data across threads; handle synchronization and callback rules are in [crates/ghostty-vt/THREADING.md](../crates/ghostty-vt/THREADING.md).
- Add a module where its responsibility belongs; a shared mechanism has one owner. Do not add a crate per component.

## The 500-line rule

A source code file must not exceed 500 lines. Split an oversized Rust module into a directory with `mod.rs`; split TypeScript or TSX into a directory with `index.ts`. Keep responsibilities and public exports clear.

## Testing layout

- `tests/workspace/`: workspace interaction, layout, persistence, Files entry and Git views, with deterministic fixtures.
- `tests/terminal/`: engine bindings, real sessions, find, file drop, input backpressure, notifications, OSC, selection, status and tiling.
- `tests/files/`: directory tree, Git status overlay, diff hunks and row actions, with fixtures.
- `tests/native/*.acceptance.tsx`: AppKit acceptance that needs a foreground window; these files are excluded from default headless discovery.
- `tests/screenshots/*.acceptance.tsx`: opt-in captures for the documentation gallery. Run `tests/screenshots/setup.sh` first and follow its README; these files are also excluded from default discovery.
- `tests/probes/`: Python protocol and payload probes launched by acceptance tests. They are not product runtime dependencies.
- Rust unit tests live beside their modules. Integration tests for an independent crate stay under that crate's `tests/` directory.

Test files use the `*.test.tsx` suffix; fixtures do not. NSH resolves `open()` relative to the test module. `scripts/test-native` copies an acceptance source to the repository root and runs it there, so acceptance `open()` paths are root-relative; Python probes also run with the repository root as the working directory. Temporary files are removed on exit, and `.nsh` caches next to fixtures must not be committed.

## Standard validation

```sh
./scripts/nsh check
./scripts/nsh test --junit artifacts/headless-tests.xml
./scripts/nsh build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

A theme change needs `build` before `check` so generated token types update. A documentation-only change needs link and diff checks; a directory move must also verify types, test discovery and the build. Validation should exercise user behavior or an ownership constraint, not restate the implementation.

## Native acceptance

Run these with a foreground AppKit window, serially, to avoid window-focus and shared Cargo target interference:

```sh
./scripts/test-native input
./scripts/test-native scroll
./scripts/test-native selection
./scripts/test-native tiling
./scripts/test-native protocols
./scripts/test-native ghostty
./scripts/test-native glyphs
./scripts/test-native status
./scripts/test-native notifications
./scripts/test-native osc
./scripts/test-native gitlive
./scripts/test-native gitscale
./scripts/test-native repositories
./scripts/test-native history
./scripts/test-native settings
```

The compatibility wrappers `scripts/test-native-input`, `-scroll`, `-protocols`, `-tiling` and `-ghostty` remain valid. `./scripts/test-native <category> [NSH test options]` passes options through, for example a `--filter` for one case. `gitscale` prepares its local repository first; `repositories` prepares its fixture repositories.

Do not run full headless discovery in parallel with `scripts/test-native`: the latter creates a temporary `*.test.tsx` file at the repository root that the former would discover. Native success cannot substitute for manual GUI acceptance, and type-check or headless success cannot substitute for native acceptance.

Record validation with the revision or uncommitted state, the command, the result, log paths and what was not covered. Keep preview, real PTY, headless, native automation and manual GUI acceptance distinct. Do not hide an existing failure by changing an assertion or timeout. The evidence policy and topic index are in [validation/README.md](validation/README.md); dated entries live in the topic files.

## Tooling and generated files

Call NSH through `./scripts/nsh` only. `./scripts/bootstrap` installs the pinned NSH release and verifies its SHA-256 checksum. The CLI, `nsh-core`, `nsh-base` and `nsh-platform-macos` are pinned to 0.6.0 and must stay aligned. Rust, macOS and Git requirements are listed in the [product README](../README.md).

Daily Ghostty builds use the official prebuilt library with a fixed checksum and do not need Zig. Offline overrides and binding regeneration are documented in [crates/ghostty-vt-sys](../crates/ghostty-vt-sys/README.md) and [crates/ghostty-vt](../crates/ghostty-vt/README.md).

`Cargo.lock` is committed. `theme.json` and `assets/` are sources; `.nsh/`, `.tools/`, `target/`, `node_modules/`, `artifacts/` and temporary `.native-*` acceptance runners are local generated or ignored files. Do not edit generated `.nsh/types` by hand, store screenshots or logs in source directories, or commit local artifacts. Theme values belong in `theme.json`.
