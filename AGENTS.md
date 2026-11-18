# Termlevel

- Read [development.md](docs/development.md) for directory ownership, dependency rules and validation. Read [architecture.md](docs/architecture.md) and [roadmap.md](docs/roadmap.md) before changing ownership boundaries.
- UI is TSX on NSH with the native macOS backend. Do not introduce a web frontend.
- Keep the CLI and all `nsh-*` crates on the same pinned version (currently 0.6.0).
- The default transport runs real PTY sessions. Rust owns processes and terminal state independently of UI mount lifetime. Only explicit session operations create or close processes.
- Session ID is stable within the host process. Collapse, rename, reorder, selection and view remount preserve it and terminal state. Restart restores workspace structure by opening new sessions, not by restoring old processes.
- Keep terminal streams, grids, parsing and high frequency rendering in Rust; TSX owns workspace UI and low frequency metadata.
- `TERMLEVEL_PREVIEW=1` is deterministic preview only: it never executes commands or accesses real workspace storage. Do not treat preview evidence as real shell evidence.
- Theme values belong in `theme.json`; generated `.nsh/types` must not be edited.
- A source code file must not exceed 500 lines (hard limit). Split oversized Rust modules into a directory with `mod.rs`, and TypeScript / TSX modules into a directory with `index.ts`; keep responsibilities and public exports clear.
- Use `./scripts/nsh check`, `./scripts/nsh test`, and `./scripts/nsh build` for validation. Record native automation and manual GUI acceptance separately; report existing failures explicitly.
- Do not commit `.tools/`, `.nsh/`, `target/`, `node_modules/` or local artifacts.
