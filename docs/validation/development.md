# Validation: Development

| Date | Entry |
|---|---|
| 2026-11-11 | Passed checks |
| 2026-11-11 | Environment record |
| 2026-11-15 | Directory structure and development conventions |
| 2026-11-15 | Recovering from insufficient disk space on the macOS development host |

## 2026-11-11 · Passed checks

| Check | Result |
|---|---|
| Official CLI distribution package SHA-256 | Matches the distribution package's checksum file |
| `cargo check --all-targets --offline` | Passed |
| `cargo fmt --check` | Passed |
| `sh -n scripts/nsh scripts/bootstrap` | Passed |
| `CARGO_NET_OFFLINE=true ./scripts/nsh check --tsc /path/to/tsc` | Rust + TypeScript passed; this run used an existing TypeScript compiler |
| `CARGO_NET_OFFLINE=true ./scripts/nsh build` | Passed; produced `.nsh/out/bundle.nshc` |
| `CARGO_NET_OFFLINE=true ./scripts/nsh test --junit artifacts/headless-tests.xml` | 14 passed, 0 failed |
| macOS native development host | Compiles |

Headless tests used the real NSH engine, the default Terminal, and QuickJS, with a desktop viewport of 1200 × 780. They covered:

1. Terminal output preserved after switching tabs and collapsing groups.
2. Output preserved after creating a group and renaming a tab.
3. Tabs preserved after moving across groups and ungrouping.
4. Selecting the remaining tabs after closing the active group; tabs can be recreated after closing all.
5. Finding a tab inside a collapsed group; the compact sidebar can still switch and create tabs.
6. Current session output preserved after pinning and menu reordering.
7. Light / dark / follow-system switching: verified that the toolbar, terminal default text, and group colors actually update, explicit theme overrides system changes, and terminal output and instances are preserved.
8. Session menu still operable with the path bar hidden in a narrow window; Escape closes the rename dialog, and the path bar returns after collapsing the sidebar so typing can continue.
9. The group edit overlay shows nine colors; directly changing the name and color and creating a terminal in the group preserve existing terminal output.
10. Right-click opens the edit overlay when the sidebar is collapsed; pressing Enter after choosing a color closes it, clicking outside closes it, and the more button can be clicked again to close.
11. A collapsed group in the collapsed sidebar keeps the active tab; the hidden count updates from +1 to +2 with selection, and clicking the count expands the group and preserves terminal output.
12. Hovering into the collapsed sidebar expands it and moving out retracts it; it stays expanded while the group edit overlay is open; the terminal toolbar in a narrow window keeps its original width layout, and existing terminal output and instances are preserved. Native overlay drawing and mouse hit-testing still require local acceptance.

13. While the divider is held during dragging, the narrow-window toolbar already hides the path for the new content width; after release, collapse, and hover, the group edit can still be entered and terminal output is preserved.
14. Dragging the divider while the sidebar is collapsed or in the hover overlay state changes neither the content layout nor the saved width; after manual expansion the adjustable width is restored.

2026-11-12 overlay clipping fix: a real-machine screenshot from the user confirmed that placing Sidebar inside the 44 pt split clipped the expanded content. The current Sidebar is back at the window root, and when pinned expanded it synchronizes the actual width through `onLayout` of a measuring View inside the split; `onResize` only saves the size at the end of the drag. 13 interaction regressions passed; the overlay's full native drawing, hit-testing, and per-frame drag following still require real-machine acceptance, and headless interaction passing cannot prove the absence of native clipping.

2026-11-12 platform materials: the pinned sidebar changed to the `sidebar` visual effect, and the hover sidebar, session toolbar, group edit overlay, and tooltip changed to `regular` glass with blur fallback. `./scripts/nsh check` passed and 14 headless items passed. Liquid Glass draws only on macOS 26; material sampling, contrast, and window active state still require real-window acceptance.

Local artifacts are in the ignored `.nsh/out/` and `artifacts/`. The debug runner had 1 linker warning at link time: `__eh_frame section too large (max 16MB) to encode dwarf unwind offsets in compact unwind table`, indicating that exception-handling performance may be affected; build and tests both succeeded. Whether a release build has the same warning has not been verified.

## 2026-11-11 · Environment record

At initialization the environment ran out of disk space; after space was recovered the full toolchain was re-extracted and the build completed successfully. Cargo downloads hit DNS failures in restricted commands; after caching dependencies with a separate `cargo fetch`, checks completed with `CARGO_NET_OFFLINE=true`.

The project files were created; this environment forbids creating `.git` metadata for this directory, and `git init` returned `Operation not permitted`, so the Git repository was not initialized and there are no commits. `git init` can be run in a normal terminal.

## 2026-11-15 · Directory structure and development conventions

Uncommitted reorganization based on `717918f`: TSX production code was grouped into ui/app, workspace, settings, terminal, and shared; interaction tests and fixtures were centralized under tests/workspace and tests/terminal, native acceptance under tests/native, and Python probes under tests/probes. The 50 migrated source files were compared one by one with HEAD, and the differences were only import, open, and probe path strings. Rust ownership and implementation were unchanged. File paths and commands in the old sections are kept as historical evidence; current paths are in development and architecture.

Updated the AGENTS notes on real PTY and restart restoration; added the development guide; the five native script groups now share scripts/test-native while keeping the original entry points, output report names, and the scroll fps option. `.nsh` caches adjacent to fixtures were included in the ignore scope.

macOS validation this round:

- `./scripts/nsh check`, `./scripts/nsh build`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `git diff --check` passed.
- `./scripts/nsh test --junit artifacts/directory-headless.xml`: **128 / 128**; log `artifacts/directory-headless.log`. The discovered test count stayed at 128, including workspace, persistence, real sessions, and rebinding. The two failures recorded earlier in persistence did not reproduce this round; this is not taken to mean the intermittent problem is fixed.
- `./scripts/test-native-protocols`: **5 / 5**; log `artifacts/directory-native-protocols.log`, report still `artifacts/protocols-native.xml`. Verified the new acceptance / fixture / Python paths and the shared script, covering AppKit keyboard, background, glyphs, images, selection, mouse, and OSC52. The first restricted run could not listen on the local dev server, and passed after that restriction was lifted.
- All native entry points passed shell syntax checks; the other four native scenarios were not run this round, and no manual GUI acceptance was performed.

## 2026-11-15 · Recovering from insufficient disk space on the macOS development host

The user's `./scripts/nsh run` failed at the native host link stage with the fatal error `ld: write() failed, errno=28 (No space left on device)`; the macOS minimum version and duplicate library messages were warnings. The remaining space was 341 MiB; target was about 40 GB, of which target/debug/incremental was about 3.7 GB, and there was another .nsh/rust/target of about 4.9 GB.

Only these two regenerable caches were deleted, keeping source, artifacts, tools, and the Ghostty download cache. `CARGO_TARGET_DIR=/Users/biu/Projects/termlevel/target cargo build --manifest-path .nsh/rust/macos-host/Cargo.toml` then succeeded (including native host linking, about 5 seconds); log artifacts/macos-host-disk-recovery.log, with about 7.1 GiB free afterwards. The user's workspace was not launched and GUI acceptance is not claimed; the existing link warnings remain.

Entries were translated from the consolidated validation log; failures above remain part of the record and must not be erased.
