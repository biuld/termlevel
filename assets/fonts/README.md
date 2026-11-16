# Bundled terminal fonts

Default cascade: JetBrains Mono, Sarasa Term SC, ui-monospace.

- JetBrains Mono 2.304: Regular, Bold, Italic, BoldItalic.
  Official unmodified TTF files, with OFL.txt and AUTHORS.txt.
- Sarasa Term SC 1.0.43: Regular and Bold. Chinese italic uses upright
  fallback when an italic face is unavailable, matching NSH's behavior.
  Official unmodified TTF files, with the release tag's OFL license.

`manifest.json` pins upstream archive URLs, archive hashes, file hashes and sizes.
The fonts are licensed under SIL OFL 1.1, independently of the app.

The Rust native module's `Registry::assets(assets!())` registers fonts before
layout and includes every resource here in development and packaged apps.
It registers them within the host process, without system-wide installation.

JetBrains Mono does not contain Chinese or circled digits. Sarasa Term SC
supplies both: its circled digits use a half-em advance and Han glyphs a
full-em advance. The terminal parser still owns column widths; rendering,
text measurement and composition share the explicit CoreText cascade.
This font pair is not a claim of complete Unicode or emoji coverage.

Existing saved font selections are retained. New workspaces use the bundled
pair; it is also available as JetBrains Mono in Settings > Terminal.
