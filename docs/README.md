# Documentation

This directory documents how Termlevel is built, designed and verified. The [product README](../README.md) is the user-facing introduction; this page is the map for contributors and agents.

## Status

- **Current** documents describe code and behavior as they exist today. They are updated in the same change as the code.
- **Evidence** documents are dated validation records. They are append-only and are not rewritten when behavior later changes.
- **Gallery** images are captured from the real app with preview or controlled fixture data.
- **Roadmap** records work that is not finished. It is not a changelog.

Historical handoff snapshots and completed plans are not kept in the tree. Use git history to inspect an earlier design.

## Map

| Document | Status | Purpose |
|---|---|---|
| [development.md](development.md) | Current | Setup, directory ownership, dependency rules, tests, validation commands and tools |
| [architecture/README.md](architecture/README.md) | Current | System overview, ownership boundaries and top-level data flow |
| [architecture/terminal.md](architecture/terminal.md) | Current | PTY sessions, terminal state, OSC handling, renderer integration, lifecycle and concurrency |
| [architecture/workspace.md](architecture/workspace.md) | Current | TSX workspace state, native window UI, tiling and persistence |
| [architecture/git.md](architecture/git.md) | Current | Repository discovery, Git read pipeline, Changes, History and explicit operations |
| [architecture/files.md](architecture/files.md) | Current | Group directory browsing, text editing, Git status and directory watching |
| [architecture/notifications.md](architecture/notifications.md) | Current | In-app notification history, sources and panel behavior |
| [design/ui.md](design/ui.md) | Current | Layout, surfaces, typography, color, dialogs, settings and theme rules |
| [design/terminal.md](design/terminal.md) | Current | Fixed-cell renderer, input, search, selection, SurfaceController and frame scheduling |
| [design/osc-status.md](design/osc-status.md) | Current | Supported OSC protocols, terminal status model and status UI/UX |
| [design/tiling.md](design/tiling.md) | Current | Multi-terminal split panes, focus, drag/drop and pane actions |
| [roadmap.md](roadmap.md) | Roadmap | Outstanding platform, compatibility, packaging and acceptance work |
| [screenshots.md](screenshots.md) | Gallery | Captured workspace, Git, Files, settings and terminal views |
| [validation/README.md](validation/README.md) | Evidence | Validation policy, environment baseline and topic index |

## Adding or changing a document

Follow the rules in [AGENTS.md](../AGENTS.md):

1. Choose the owning document. Do not create a second description of the same mechanism.
2. Link to the owner from related documents instead of copying facts.
3. Update this map for any added, removed or renamed document.
4. Update the owning document in the same change as the code or behavior it describes.
5. Keep current documents free of handoff snapshots, completed plans and stale module paths.
6. Check relative links and confirm the document is in English before finishing.
