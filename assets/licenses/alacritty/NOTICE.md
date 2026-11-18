# Alacritty built-in terminal glyphs

The glyph algorithms in this directory are adapted from
[`builtin_font.rs`](https://github.com/alacritty/alacritty/blob/18372031d1f1bda853b2027b7ef1c75b45942829/alacritty/src/renderer/text/builtin_font.rs)
at commit `18372031d1f1bda853b2027b7ef1c75b45942829`.

Copyright 2020 The Alacritty Project. Licensed under the Apache License,
Version 2.0; see [LICENSE-APACHE](LICENSE-APACHE).

Termlevel splits the source into modules, supplies cell dimensions directly
instead of crossfont metrics, and converts the resulting coverage to a
Core Graphics image mask. For fractional device-pixel cell widths, line weight
uses the shared unsnapped width while each mask uses its snapped cell width.
The remaining glyph algorithms are retained from upstream.
