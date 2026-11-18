# Validation: Files

| Date | Entry |
|---|---|
| 2026-11-18 | Git marks on rows already listed |
| 2026-11-19 | File previews and default application fallback |
| 2026-11-19 | File toolbar save action |

## 2026-11-18 · Git marks on rows already listed

Uncommitted work based on `235de52`. A file or directory row is drawn before Git status returns. The tree label gains a private suffix when that path is marked, so the row is drawn again. The visible name stays the file name. This covers the first level under the group root and a later change of the same mark.

- `./scripts/nsh test tests/files/GitMark.test.tsx`: **1/1**. The root directory `src` and the root file `readme.md` have no mark until status is released, then both show one.
- `./scripts/nsh test tests/files/GitStatus.test.tsx`: **5/5**. `./scripts/nsh test tests/files/Tree.test.tsx`: **4/4**.
- `./scripts/nsh check`, native AppKit, and a manual look at a real repository were not run.

## 2026-11-19 · File previews and default application fallback

Uncommitted changes based on `0f4d8bd`. The app uses NSH native media views and TextView for previews, and the existing system open action for unsupported files. The native fixture is prepared with `tests/files/setup-native-preview.sh`, which requires ffmpeg.

### Commands

- `./scripts/nsh check` and `./scripts/nsh build` passed.
- `./scripts/nsh test tests/files/Preview.test.tsx` passed 3/3. Cases cover Markdown and HTML rendering, SVG source editing, media selection, unsupported-file action and unsaved edits.
- `./scripts/nsh test tests/files/Actions.test.tsx` passed 3/3.
- `cargo test -p termlevel-native files::tests:: --lib` passed 14/14, including in-root preview URL and escaping-symlink checks. Log is `artifacts/files-preview-rust.log`.

### Native automation

- Prepared fixtures with `tests/files/setup-native-preview.sh`. Copied `tests/files/Preview.acceptance.tsx` to `tests/files/.native-preview.test.tsx`, ran `./scripts/nsh test tests/files/.native-preview.test.tsx --target macos`, and removed the temporary runner. Passed 1/1. Log is `artifacts/files-preview-native.log`.
- Reviewed `artifacts/files-image-preview-native.png`, `artifacts/files-video-preview-native.png`, `artifacts/files-audio-preview-native.png` and `artifacts/files-markdown-preview-native.png`. The native test checks media load events, SVG source switching, a failed video and the unsupported-file action.

### Unverified

Playback interaction and audible output, every codec and image format, screen-reader use, and sustained manual GUI use remain unverified. NSH's image cache may retain an earlier version when an image is modified in place. PDF and Office documents use the default-application fallback.

## 2026-11-19 · File toolbar save action

Uncommitted changes based on `0f4d8bd`, including the file preview changes above. The file toolbar shows a save icon only for unsaved edits, and its height accommodates the preview toggle.

### Commands

- `./scripts/nsh test tests/files/Preview.test.tsx` passed 4/4, including the clean, dirty, preview and saved toolbar states.
- `./scripts/nsh check` and `./scripts/nsh build` passed.

### Native automation and unverified items

- The Files preview native acceptance passed 1/1 using the setup and temporary runner described above. Reviewed `artifacts/files-markdown-preview-native.png`. The log is `artifacts/files-toolbar-native.log`.
- Manual GUI and accessibility acceptance remain unverified.
