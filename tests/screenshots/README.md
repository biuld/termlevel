# Screenshot acceptance

Opt-in captures of the real AppKit app, written to `docs/assets/screenshots/`. These files use the `.acceptance.tsx` suffix and are not part of default headless discovery.

Every capture is privacy-safe:

- Workspace, tabs, Git and Files use the deterministic preview or a fixture transport. They never read the user's workspace storage, Git repositories or files.
- Status, notifications and the split-pane CLI capture use a temporary `/bin/sh`/bash in `/tmp`, started by `setup.sh` with no user startup files and a neutral prompt.
- Demo text, authors and paths are generated. Do not add real personal data to a fixture.

## Capture

```sh
./tests/screenshots/setup.sh

./tests/screenshots/run.sh preview
./tests/screenshots/run.sh status
./tests/screenshots/run.sh clis
```

`run.sh` copies the selected acceptance file to a temporary root `*.test.tsx`, because NSH resolves `open()` relative to the test module. Do not run full headless discovery at the same time: it would discover that temporary file.

`Preview` captures the workspace, search, group editor, Git, Files, sidebar and settings. `Status` captures agent icons, progress and notifications from a controlled shell that emits the OSC 7501 reports itself; no agent binary, account or session is used. `Clis` captures the hero image: UNGROUPED split panes running `vim`, `less` and `cmatrix`, with an Agents tab group below whose sessions report idle Claude, Codex and Pi identities through the same controlled OSC path.

A native run needs a foreground-capable AppKit session. Review every new image for private paths, names or content before committing it.
