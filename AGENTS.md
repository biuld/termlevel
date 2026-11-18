# Termlevel

## Engineering

- Read [development.md](docs/development.md) for directory ownership, dependency rules and validation. Read [architecture](docs/architecture/README.md) and [roadmap.md](docs/roadmap.md) before changing ownership boundaries.
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

## Documentation

Documentation is written in English.

- [README.md](README.md) is the product page: positioning, features, requirements, install/run, shortcuts, limits and license. Do not put internal architecture, directory trees or validation workflow in it.
- [docs/README.md](docs/README.md) is the documentation map. Add or remove entries there whenever a document is added or removed.
- Each document has one owner and one purpose:
  - `docs/development.md`: setup, directory ownership, tests, validation commands and tools.
  - `docs/architecture/`: current code ownership, data paths, lifecycle and boundaries. Describe the code as it is.
  - `docs/design/`: intended UI, interaction, rendering and visual rules, including known limitations.
  - `docs/roadmap.md`: work that is not finished. Do not keep completed checklists or a changelog here.
  - `docs/validation/`: dated evidence. Follow the rules in [docs/validation/README.md](docs/validation/README.md).
- Current documents must not contain handoff snapshots, completed PR plans, "will remove" sections, design-time "current gaps" lists or old module paths. Git history is the archive.
- Update documentation in the same change as the behavior, path, command, version or ownership change it describes. A document must not point at a file, flag or command that no longer exists.
- Do not duplicate facts across documents. Link to the owner instead: architecture links to design, roadmap links to validation, and validation records decisions without restating the architecture.
- Keep a document focused and split by topic when it grows past roughly 300 lines. Use a directory with an `index`/`README` entry for a family of documents, matching the existing `docs/architecture/` layout.
- Validation entries are append-only evidence: record the revision or uncommitted state, commands, results, logs and unverified items. Never rewrite or delete an old failure to make the current state look green. Put each entry in the matching topic file and update the index when a topic file is added.
- Before finishing a documentation change, check that every relative link resolves and that no non-English prose remains.
