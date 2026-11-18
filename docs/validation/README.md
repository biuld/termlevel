# Validation

This directory holds dated evidence for Termlevel. It records what was checked, how, and what remained unverified. It is not a current architecture or a feature list; link to [architecture](../architecture/README.md), [design](../design/ui.md) or the [roadmap](../roadmap.md) for those.

## Rules

1. **Append, do not rewrite.** Add a new dated entry to the matching topic file. Never edit an older entry to hide a failure or make the current state look green.
2. **Record the baseline.** State the revision (`master` or a commit hash) or that the change was uncommitted, and name any unrelated changes in the same working tree.
3. **Record the command and the result.** Keep exact test counts, exit status, failure names, log paths and screenshot names. Do not replace a real failure with a summary.
4. **Separate evidence types.** Preview mode, headless tests, real PTY tests, native AppKit automation and manual GUI acceptance are distinct. One does not substitute for another.
5. **Name what was not covered.** IME candidate windows, selection feel, drag responsiveness, VoiceOver, Reduce Motion, sustained load and similar manual checks belong in the entry when they were skipped.
6. **Keep existing failures visible.** If a known failure is unrelated to the change under test, say so and keep the exact failure and count.
7. **Prefer the topic owner.** Put terminal/renderer evidence in `terminal.md`, workspace UI and persistence in `workspace.md`, and so on. Link to the design or architecture document instead of restating the design.
8. **Do not add preview evidence as shell evidence.** `TERMLEVEL_PREVIEW=1` runs deterministic fixture data and never executes commands or opens real workspace storage.

Each entry uses `## <date> · <title>`, with `###` subsections for commands, native automation, manual acceptance and unverified items. Files sort entries chronologically, oldest first.

## Topic index

| File | Covers |
|---|---|
| [terminal.md](terminal.md) | Terminal core, PTY, Ghostty wrapper, fixed-cell renderer, input/IME, search, scrollback, selection, fonts and glyphs, images, clipboard |
| [workspace.md](workspace.md) | Sidebar, tabs, groups, window layout, theme surfaces, settings UI, notifications panel, tiling, first-run and persistence |
| [git.md](git.md) | Repository discovery, Changes, History, worktrees, read isolation and explicit Git operations |
| [osc-status.md](osc-status.md) | OSC capability inventory, PWD/OSC 22/OSC 133, progress and program status, agent icons |
| [packaging.md](packaging.md) | Application bundle and asset packaging |
| [development.md](development.md) | Baseline checks, environment, directory conventions and development-host recovery |

There is no dedicated `files.md` yet: the Files view has no standalone dated entry in the log. That is a documentation gap, not evidence that the view has been accepted. Record new Files evidence in a new `files.md` and add it to this table.

## Environment

The log began on 2026-11-11 on macOS arm64 with Rust/Cargo 1.99.0 and NSH 0.5.0 (`f4d68f7`). The original preface also stated that new records appear at the end of the log. The project now pins NSH 0.6.0. Individual entries name the toolchain, archive override and revision actually used; do not assume they share one environment.

Generated logs and screenshots referenced by entries live under `artifacts/`, which is ignored by git. Treat missing artifacts as missing evidence, not as a pass.

## Related

- [Development guide](../development.md): how to run the checks.
- [Roadmap](../roadmap.md): outstanding acceptance work.
- [AGENTS.md](../../AGENTS.md): documentation maintenance rules.
