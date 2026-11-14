# termlevel-ghostty-vt

Safe Rust wrapper for the **official libghostty-vt 1.0.1 C ABI**. No NSH,
PTY or process dependencies. Termlevel uses this wrapper in its production
Rust SessionManager and in the explicit preview provider.

```rust
use termlevel_ghostty_vt::{Terminal, RenderState};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = Terminal::new(80, 24)?;
    terminal.write("hello 中文\r\n".as_bytes())?;
    println!("{}", terminal.text()?);

    let mut renderer = RenderState::new()?;
    let frame = renderer.capture(&mut terminal)?;
    terminal.resize(100, 30, 8, 16)?;
    drop(terminal);
    assert_eq!(frame.cols, 80); // owns its data, no dangling C references
    Ok(())
}
```

## Build

See [the sys crate](../ghostty-vt-sys/README.md) for prebuilt downloads,
checksums, offline builds and supported platforms. Application builds need
neither Zig nor bindgen/libclang. Currently supported: macOS arm64 and x86_64,
built on a Mac with Xcode command line tools. Only arm64 execution has been
validated locally.

## Ownership and threading

- `Terminal` and `RenderState` own native handles and free them on drop.
- `Terminal` and `RenderState` are `Send`, but remain `!Sync`: serialize all
  terminal access with a mutex when moving between reader and UI threads. This
  follows the native API's serialized-access contract; see [THREADING.md](THREADING.md).
- No custom allocators. An immutable, bounded PNG decoder is installed once
  before safe terminal creation. Clipboard hosts implement a synchronous
  `Send + Sync` adapter; never reenter the terminal or wait on a locking UI.
- Synchronous C callbacks copy replies, titles and working directories into
  an event queue. `take_events()` transfers owned events to the caller; drain
  it regularly. Callback failures are sticky and returned as errors by writes
  and resize. Clipboard callbacks reply synchronously through their bounded host adapter;
  other events are handled after the C callback returns.
- `Frame` owns all cell strings, styles, colors and cursor data. The initial API
  copies the full viewport and exposes dirty state, rather than offering a
  zero-copy renderer. Use one render state per terminal; switching terminal
  identity automatically creates a fresh native snapshot. Mode 2026 holds return
  the frame and screen identity captured at the exact hold boundary. The pinned 1.0.1 native resize commits a hold; callers must recheck hold state after resize. Captures invalidate other
  renderer caches because native updates consume dirty state. History image
  snapshots restore both viewport position and its bottom-follow semantics.
- Key encoding uses the current legacy/Kitty modes; the caller supplies layout
  text, modifiers and the actual unshifted codepoint. Mouse encoding uses the
  current tracking/format modes and real cell dimensions. Encoded bytes are
  returned, never sent to a process or fed back into the output parser.

## Current safe API boundary

Implemented: create/write/reset/resize, plain text including active-screen
history, owned reply/bell/title/cwd/reset events, viewport snapshots with full
graphemes, wide spacers, palette/truecolor and style flags, key/mouse/focus
encoding and sanitized plain-text bracketed paste, native one-shot search,
fixed-key colors/cursor/history access and tracked logical history windows,
tracked selection and text export, a terminal-owned scroll anchor that follows reflow/eviction, incremental search feed/tick, OSC8 targets, owned Kitty image placements
and pixels (including bounded PNG decode), animation ticking, and synchronous
text clipboard callbacks.
`set_scrollback()` sets the native page-granular storage bound; `retained_range()`
returns an exact logical window and never resurrects previously discarded rows
when a limit is increased. `text()` formats the native storage; use the returned
range and `screen_row()` when exporting only the logical application history.

Hosts implement gestures, drawing and clipboard policy; persistence snapshots
are not wrapped. Image file/temp/shared-memory loading stays disabled. Hosts decide the hold
timeout policy and call `release_render_hold()` on timeout/EOF; Termlevel uses
150 ms. `screen_row()` is for historical access, while active rendering should
use `RenderState`. All native declarations are exposed through `raw` for explicitly
unsafe integration work. This crate does not implement a renderer, IME or a PTY.

Run both crates' native tests from the repository root:

```sh
cargo test -p termlevel-ghostty-vt-sys -p termlevel-ghostty-vt
```

The crate name is prefixed to avoid claiming ownership of existing public
`ghostty-vt` crates. These packages have not been published to crates.io.
