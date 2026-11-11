# Termlevel

- UI is TSX on NSH. Use the native macOS backend first; do not introduce a web frontend to approximate it.
- Keep the CLI and all `nsh-*` crates on the same pinned version (currently 0.5.0).
- Read `docs/architecture.md` and `docs/roadmap.md` before changing ownership boundaries.
- Tab identity is a stable session ID. Collapse, rename, reorder and selection must preserve terminal state.
- The current preview transport never executes commands. Do not present it as a real shell.
- Future real PTY sessions belong to Rust, independently of UI mount lifetime.
- Keep terminal streams and high frequency rendering in Rust; TSX owns workspace UI.
- Use `./scripts/nsh check`, `./scripts/nsh test`, and `./scripts/nsh build` for validation. Record native GUI acceptance separately.
- Do not commit `.tools/`, `.nsh/`, `target/`, `node_modules/` or local artifacts.
