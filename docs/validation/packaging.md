# Validation: Packaging

| Date | Entry |
|---|---|
| 2026-11-15 | Packaging app icon |

## 2026-11-15 · Packaging app icon

`nsh.json`'s `icon` points to `assets/brand/termlevel-icon-v1.png`; the generation prompt and provenance are kept in a Markdown file in the same directory. The icon is a square PNG with alpha, automatically converted to a macOS icon resource by NSH 0.5.0; the normal command is `./scripts/nsh package --target macos`, with no extra icon argument. README documents how to package.

- `./scripts/nsh check`, `./scripts/nsh build` passed.
- `./scripts/nsh package --target macos --out .nsh/package-icon-check` passed, producing `.nsh/package-icon-check/Termlevel.app`, log `artifacts/package-icon.log`. The offline build used the pinned Ghostty archive and `CARGO_NET_OFFLINE=true`.
- Inspected `Info.plist`: bundle ID is `app.termlevel.desktop`, and `CFBundleIconFile` points to `Contents/Resources/AppIcon.icns`. The decoded icon is 1024×1024 RGBA with alpha range 0–255; the mean pixel difference from the source PNG scaled to the same size is about 0.427 / 255, confirming the selected icon is actually used.
- `codesign --verify --deep --strict .nsh/package-icon-check/Termlevel.app` passed; the bundle uses the default ad hoc signature, which is not equivalent to a distribution signature or notarization.
- The first `iconutil` call inside the sandbox reported Invalid Iconset; rerunning with the system tool permissions required for packaging succeeded. The first release host compile took about three minutes and still had 9 warnings from the earlier diagnostic code.

This change only altered icon assets and packaging configuration, and the full interaction tests were not rerun; validation targeted the real app bundle and icon resource, and the packaged app was not launched to confirm Finder / Dock icon cache behavior. Packaging and build artifacts are in ignored directories and are not committed.

Entries were translated from the consolidated validation log; failures above remain part of the record and must not be erased.
