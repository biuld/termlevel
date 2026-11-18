# Terminal Grid and Surface

Termlevel parses terminal output with the official libghostty-vt 1.0.1, maintains the grid and history in Rust, and paints through an application-owned AppKit view. The safe wrapper API and its ownership rules are in the [ghostty-vt README](../../crates/ghostty-vt/README.md). Workspace ownership is in the [workspace architecture](../architecture/workspace.md); the platform integration boundary is in the [terminal architecture](../architecture/terminal.md); toolbar and pane presentation are in [ui.md](ui.md); automated and native evidence is in the [terminal validation record](../validation/terminal.md).

## Why a fixed-cell native grid

libghostty-vt 1.0.1 parses output, maintains the grid and history, and encodes input. It does not include the Ghostty application renderer, scroll gestures or an input method. Speaking the protocol does not by itself produce a complete terminal surface.

The earlier production path placed terminal logical lines in NSH `virtual_list`, with separately mounted paint, image, link and cursor nodes for each visible line. `ListAlignment::Bottom` could cut into the previous line of history in a non-whole-line viewport, and wheel input in program mouse mode triggered a TUI redraw of the active screen while list nodes and bottom-pinning still participated in refresh. Further tuning of estimated row heights, overdraw and masks was not the design path.

The list was a confirmed structural problem, and users still felt jank while scrolling inside Codex after the fixed grid replaced it. In a mouse-reporting TUI of the same size and font, NSH Canvas native replay measured about 24–26 ms; CPU samples concentrated on per-`fill_text` font construction and CoreText matching. Reducing node count did not solve text replay cost, so painting moved into the application's own NSView. That controlled load is not a recording of actual Codex.

The resulting ownership is:

- TSX owns the workspace.
- Rust `Session` owns the PTY, process and Ghostty state.
- The Rust native surface owns the display window and interaction.
- NSH owns workspace layout and mounting.
- Termlevel's custom AppKit NSView owns painting.
- The existing NSH field editor owns native text input.

`Terminal` and `RenderState` are `Send + !Sync`; access is serialized through `Session.emulator`. A stable session ID survives collapse, reorder, group changes and selection. The workspace publishes a low-frequency `visible` property describing actual display state. A hidden surface does not capture frames, advance search or publish paint scenes, and restoring visibility immediately captures the latest window. An inactive but visible split keeps refreshing.

NSH and its pinned 0.6.0 version are not modified. Terminal streams and the cell grid never enter TSX. The preview transport still executes no commands. Pane titles use `tab.liveCwd ?? tab.cwd`; OSC 7 / ConEmu / iTerm2 CurrentDir are connected. Kitty file, temporary-file and shared-memory media remain disabled, and image RGBA is never written to a file. The OSC 52 policy is unchanged. There is no wheel cancel and no footer inside scrolling content.

Industry implementations informed the contracts rather than supplying code: [Alacritty's text renderer](https://github.com/alacritty/alacritty/blob/master/alacritty/src/renderer/text/mod.rs) consumes drawable cells with glyph caches, atlases and batching; [WezTerm Screen](https://github.com/wezterm/wezterm/blob/main/term/src/screen.rs) distinguishes physical row numbers from stable row indices; [Ghostty render state](https://github.com/ghostty-org/ghostty/blob/main/src/terminal/render.zig) and [Alacritty display](https://github.com/alacritty/alacritty/blob/master/alacritty/src/display/mod.rs) release the terminal lock early; [Ghostty's renderer](https://github.com/ghostty-org/ghostty/blob/main/src/renderer/generic.zig) freezes output during synchronized updates and caches shaping; and [Ghostty's AppKit surface](https://github.com/ghostty-org/ghostty/blob/main/macos/Sources/Ghostty/Surface%20View/SurfaceView_AppKit.swift) implements `NSTextInputClient`. The upstream architecture is a constraint reference only; the pinned 1.0.1 headers are authoritative.

The fixed grid uses an application-owned native renderer. The NSH 0.6.0 public Native Components interface lets an application crate register a `NativeComponent` with `#[native_components(macos)]`, return its own NSView, and participate in layout through `native(name)`. No NSH change, upgrade or whole-frame image conversion is required. The API is documented in [NSH Native Components](https://nsh.dev/docs/native-components/); signatures are those of the locally pinned 0.6.0 crate.

The renderer is Termlevel's own CoreText / Core Graphics implementation, not an embedding of Ghostty's Metal renderer and not dependent on a Ghostty internal embedding ABI. libghostty-vt provides state and encoding only. Whether a GPU atlas is needed will be decided by a new measured bottleneck. Only the macOS renderer exists; other platforms require independent implementation and acceptance.

## Three rectangles

1. The `SessionTerminal` root keeps host padding and the terminal background. Clicking the root requests focus.
2. The `flex_1` measurement view, which has no test id, is the only resize input. Rows and columns are `floor(size / cell metrics)`, at least 1. A non-positive viewport never sends 0×0. PTY resize, emulator resize and cell-size updates run under the same emulator lock. A font change must update cell size even when it leaves rows and columns unchanged.
3. `terminal-grid-{id}` is an explicit rectangle top-left aligned inside the measurement view. Its width is `Frame.cols × cell_width` and its height is `Frame.rows × line_height`. This rectangle's layout must never be used as resize input.

Pixels left outside the cell rectangle show only the terminal background: they do not scroll, start a selection or accept middle-click. A drag that begins inside the cells and leaves them is clamped to the first or last cell. `process_mouse` in AppKit checks both cell bounds and `visibleRect`, so hits are not accepted in areas clipped by a parent view during hold or window shrink.

## Integer windows and stable history anchors

`Window` owns only integer origin, `pinned`, `wheel_frac`, painted rows and columns, logical total rows and the main-screen policy stash. It never stores a pseudo-stable `native_top` number.

- Bottom-pinned: `origin = line_count.saturating_sub(Frame.rows)`. Painting starts at y=0. Extra pixels remain outside the cell rectangle, and history retained by ED2 clear cannot cut into the active screen.
- When the user changes the window, origin is clamped to `0..=bottom`. A whole-row displacement that lands at bottom restores `pinned` and clears the fraction. Wheel events smaller than one row only accumulate gesture; they publish no new window. Even while bottom-pinned those sub-row fractions are retained, otherwise a slow trackpad could never leave the bottom. New-output following, an explicit pin and reaching a boundary clear the accumulation. Gesture accumulation is not a sub-row paint offset.
- When not bottom-pinned and following output, the anchor is resolved from the Ghostty tracked reference and then reduced by the current retained `range.start`; if the reference was evicted or is unavailable, it is clamped to the oldest logical row.
- After narrowing, widening or reflow, the anchor follows the original content. The current row number minus `scroll_base` is not sufficient.
- The safe `Terminal` owns one host scroll anchor exposed through `set_scroll_anchor` / `scroll_anchor_row`. The native tracked handle does not leave `Terminal`; setting, reading and releasing it are serialized with the terminal, and no independent raw handle is given to the UI. One production session has one stable surface; preview has its own emulator.
- Entering the alternate screen saves the main screen's `pinned` and `wheel_frac` and sets origin to 0; the main screen's tracked anchor stays in `Terminal`. Alternate-screen output, clear and non-reporting wheel events do not modify the main stash. Leaving restores the policy and then resolves the main anchor. Screen identity comes from `ACTIVE_SCREEN`, never from guessing by row count.

## Snapshot capture and synchronized output

`WindowSnap` is an owned display window containing `Arc<Frame>`, origin, count, screen identity, visible `GridRuns` and images. A normal display update has two phases: take data under the lock, then organize outside it. Under one emulator lock it:

1. takes `State::frame`;
2. updates `Window` with that `Frame.rows` / `cols`, current screen identity and retained range, resolving the history anchor under the same guard;
3. keeps the owned `Arc<Frame>` for the active screen and copies only the needed `screen_row` cells for a history window, along with the palette and defaults required to interpret colors; active-screen runs are not merged under the lock;
4. clones `frame.images` directly for the active window, adding the active-screen row offset; only history windows use the independent `image_renderer`, and the active path does not call the old `images_at` fast path keyed on `State.size.rows`;
5. keeps the cursor and input-anchor information, then releases the lock.

`WindowCapture::finish` merges active and history rows outside the lock with the same pure `grid_runs_from` and organizes accessible text; the native renderer consumes the `WindowSnap`. The safe wrapper's native capture still copies the viewport under the lock and does not claim library-level begin/end separation.

Synchronized output must not sample an unfinished live screen. A bottom-pinned window may publish the owned `Frame` at the current hold start boundary, which contains previously completed output; the same boundary is reused by `Arc` identity. A history window still freezes the whole `WindowSnap`, including history, origin, screen identity, cursor, rows and columns. User history wheel, search reveal, selection, selection copy and link opening do not act on the background live screen while a hold is active. Input is still encoded and sent, and PTY resize still happens immediately. Real-library tests confirm that this project's 1.0.1 ends a hold on resize. Switching screens inside a hold does not itself end the hold and must not consume the main-screen stash early.

If a surface is first bound while already in a hold, it initializes from the frame, `total_rows` and screen identity saved at the hold boundary and displays bottom-pinned; it does not splice history from the background live screen. An existing bottom surface may also advance to a newer completed hold-boundary frame, while a history surface keeps its last published frame. Polling cannot be required to land exactly between two synchronized transactions, otherwise continuous wheel input could go seconds without publishing.

When the hold ends through completion, 150 ms timeout, EOF or native resize, the next sample publishes a new window at once. Paint code uses `Frame` dimensions, never `State.size` guessed frame height. Tests separately verify freezing during a screen switch and resize committing new dimensions.

## Invalidation and painting

Snapshot updates compare `Arc<Frame>` identity, window origin, count and screen identity. `Frame.dirty` cannot be used to skip work: capture cleans the native state, so a cached dirty flag is stale from the moment of capture. The snapshot signal changes only when display data changes.

The native scene reads `WindowSnap`, `cell_width`, `line_height`, font family, font size, default colors, theme and `search.rev`. The font key cannot rely only on measured `cell_width`; a family switch with the same width must also invalidate. The cursor is a separate NSH node, so blinking does not repaint text.

### Native embedding and threading

`TermlevelTerminalSurface` in `src/session_view/platform/macos/view.rs` is the actual NSView subclass. `src/lib.rs` registers it at the app crate root through `#[native_components(macos)]`; the NSH development host and packaged host call it. NSH only creates the container, sets geometry and sends invalidation properties; painting never calls NSH `fill_text`.

The engine thread holds a `Publisher` and publishes the complete owned snapshot, `PaintConfig`, search hits and the current index as `Arc<Scene>`. The main thread takes the latest scene from a mutex-protected slot. The read lock only clones the `Arc`; painting holds neither the slot lock nor `Session.emulator`. Backlogged scenes overwrite rather than queue. The cross-NSH frame property contains only the slot ID and revision scalars, with no cells, text arrays or image pixels; this Rust property does not pass through TSX.

The global registry stores only `Weak<Slot>`. The `Publisher` and native component each hold their own `Arc`, and the last user releases the slot and unregisters it. Mounting and unmounting never create, close or hold a PTY. CoreText fonts, `CTLine` and `CGImage` belong only to the main-thread renderer and are never placed back into `Session` or Ghostty.

The NSView is a transparent, flipped paint subview whose `hitTest` returns nil; the NSH container and the original cell parent view continue to handle mouse events. It does not become first responder, create an `NSTextInputClient` or intercept the existing field editor. Window scaling and Retina backing use AppKit native point coordinates.

### Paint order and caches

One native NSView paints in this order:

1. image layer 0, `BELOW_BACKGROUND`;
2. cell backgrounds and selection colors;
3. image layer 1, `BELOW_TEXT`;
4. search rectangles, text, underlines and strikethroughs;
5. image layer 2, `ABOVE_TEXT`.

Above it remain the independent cursor, one hover tooltip and the unique `terminal-input-{id}`. The default terminal background belongs to the root view; no additional opaque fill is placed over image layer 0. Each native paint first clears its transparent backing and clips to the current `Frame` cell range.

Structural nodes do not grow with history, rows or run count. A Canvas Drawing with no drawing operations exists only to carry `CanvasNode` row semantics so the NSH test model can still locate text by cell coordinates; it paints no background, glyphs or images. The native component container carries the accessibility label for the whole visible window; fine-grained AppKit AX row children are not implemented. Hidden styles do not expose text. Per-row views are not remounted for tests.

Each run uses `x = col × cell_width` and `y = row × line_height`. Core Graphics `save → clip_to_rect → draw → restore` gives every run an independent clip; no mutable path is shared. Color and dim alpha are set in the current scene; hidden runs keep only their background. CoreText text is flipped back to y-up, fallback glyphs paint on the main font baseline, and font-provided ligatures are enabled. Same-style ASCII runs are merged; non-ASCII graphemes keep the cell start and span supplied by Ghostty.

Fallback glyph fitting belongs to `platform/macos/renderer/shaping.rs`. It caches the actual font identity, advance and ink bounds of each `CTLine`. Only when an actual fallback font is used, its advance exceeds the allocated cell width, and ink exceeds the cell, is the glyph scaled proportionally to the ink bounds and translated horizontally on the main font baseline. Italic overhang and accent marks of normal fonts are not scaled merely because ink exceeds the cell, and Ghostty columns, cursor, background and selection are never changed. The rule uses no character whitelist such as ①②③ and never borrows an adjacent cell. Box-drawing characters use a separate reference plane.

The design comparison is [Ghostty `Glyph.Constraint`](https://github.com/ghostty-org/ghostty/blob/main/src/font/Glyph.zig), which offers aspect-preserving fit, and [WezTerm glyphcache](https://github.com/wezterm/wezterm/blob/main/wezterm-gui/src/glyphcache.rs), which scales over-wide scalable fallback glyphs and can borrow the next blank cell. Termlevel uses the fixed allocated cell range and does not implement borrowing; it does not claim the same font policy as those terminals.

### Backgrounds, built-in glyphs, fonts and caches

Backgrounds do not follow text shaping segments. Adjacent runs in a row with the same valid background color, including inverse and host selection, merge into a continuous color band. Shared rectangle boundaries are converted to device space by the current `CGContext`, uniformly rounded, then converted back to points. Geometric antialiasing is disabled only during the background phase and the `CGContext` state is restored afterwards, so fractional cell sizes, Retina scale and view-origin offsets cannot create transparent seams between text segments or adjacent rows. Text keeps its normal antialiasing and cell positioning. Different background colors share the same pixel boundary instead of overlapping rectangles to hide seams.

Ghostty's blank cells do not necessarily store a background through style; erase and fill may use the packed cell's `BG_COLOR_RGB` / `BG_COLOR_PALETTE` content tag. The safe wrapper merges all such sources into `Cell.style.background`, using one rule for active `RenderState`, hold boundaries and historical `screen_row` cells. Palette indices are preserved and not resolved to RGB early. Hosts must not read the raw style alone and lose blank-cell backgrounds.

Single-scalar `U+2500–U+259F` line and block characters, plus the upstream-supported `U+1FB00–U+1FB3B`, `U+1FB82–U+1FB8B` and `U+E0B0–U+E0B3`, use the ported Alacritty built-in glyph algorithm described in [../../src/session_view/platform/macos/renderer/builtin_font/NOTICE.md](../../src/session_view/platform/macos/renderer/builtin_font/NOTICE.md). They generate a coverage mask for the pixel-aligned actual device-pixel cell and use the existing foreground color, dim, inverse, background and selection. The cache is keyed by character and pixel size, clears beyond 1024 entries, and clears on font changes. Upstream falls back for too-narrow Powerline cells; generation or image-allocation failure also falls back to CoreText. Normal text continues through CoreText fonts, baselines and ligatures.

This is a port of Alacritty's glyph algorithm, not Ghostty's application sprite renderer; the fixed libghostty-vt 1.0.1 C ABI does not export the latter. Source attribution and the Apache-2.0 license remain with the module. Cross-platform terminal state and the scene contract are unchanged.

The default font chain is bundled JetBrains Mono 2.304, Sarasa Term SC 1.0.43, then the system monospace font. JetBrains Mono's four styles cover Latin and programming ligatures; Sarasa regular and bold cover Chinese and missing symbols, and Chinese text stays upright when no italic exists. Original fonts, licenses and source hashes are bundled through NSH resource registration and require no system installation. Font selection, the measurement CSS list and the CoreText cascade use the same order; the whole family list is never referenced as one font name.

Ligatures occur only within an existing same-style ASCII run. `->`, `!=`, `=>` and similar sequences still occupy their original character count and keep Ghostty's column definitions; CoreText tests verify actual glyph substitution and total advance under the four main font styles. Ligatures are not currently recombined across color, selection or hyperlink boundaries.

The renderer caches normal, bold, italic and bold-italic faces of the selected font. The `CTLine` cache key is text plus literal style. Changing family or size clears the font and text caches; color, selection and search do not participate in the shaping key. The text cache is limited to 2048 entries and 256 KiB of source text with least-recently-used eviction. That limit is not an absolute memory bound for CoreText objects, whose memory still needs sustained-output stress testing.

Images continue to consume copied RGBA. Native `CGImage` objects are cached by `Arc<Pixels>` identity and only pixels referenced by the current snapshot are retained. Source cropping, offset, scaling and the three-layer order are preserved, and an animation generation that changes pixels rebuilds the image. There is no per-frame full-screen raster, image encoding or `ImageSrc` upload. Kitty file, temporary-file and shared-memory media remain disabled.

Complex emoji, ligatures crossing style or selection boundaries, bold-italic synthesis, curved underlines and underline colors still require separate extension or acceptance. The replacement does not claim the complete Ghostty application text renderer.

## Input and hit testing

No encoder or second event bus is added. Keys, mouse, focus and paste still flow through the existing `encode_*` functions into the Rust PTY. Ghostty events are still handled by `Session::handle_events` and `SystemClipboard`.

- Cell-local coordinates are divided by cell metrics for hits; selection and OSC 8 receive `origin + row_in_window`.
- In mouse-reporting mode without Shift, left, right and wheel input first request bottom-pinning when not already pinned; when already bottom-pinned they only encode and do not change origin. Middle and side buttons enter the same `Session` input-epoch mechanism from AppKit. Every successfully encoded mouse entry applies the bottom request before the next window publish.
- With Shift or outside reporting mode, the main screen adjusts origin by whole rows; the alternate screen never scrolls history. Points are divided by `line_height` and `Lines` are used directly. Fractions truncate toward zero and remain in `wheel_frac`; positive `dy` moves toward newer rows. Horizontal history scrolling is ignored.
- The report-wheel dual-axis accumulator is separate from the history accumulator. Integer displacement is encoded losslessly and chunked by the writer message limit, using the existing `mouse_repeat` encoding.
- The wheel listener is the non-scrolling cell view, using NSH's existing hit rules, with no cancel.
- Selection copy, double-click word, triple-click line, rectangular selection and Cmd+links with restricted protocols remain.

The unique field editor is positioned by `input_cursor`, and the input anchor remains even when the output cursor is hidden. `y` is clamped inside the cells. Width is the current text measured in terminal-font columns plus one cursor cell, capped at the remaining columns in the row; empty text occupies one cell. The composition background uses the currently painted background of the cursor cell. The terminal cursor is hidden during composition; composition commit deltas, `after_answer` clearing and commit attribution to the original `Arc<Session>` are unchanged. Non-ASCII character keys still go to the input method. Replacing native painting does not mean focus and candidate windows have been accepted.

## Search independent of frame refresh

Paint snapshots no longer rescan the search text. A search call immediately returns currently available results, and a new low-frequency `SearchChange` carries `query`, `count`, `index` and `searching` so the find bar can show Searching… and the final count. Asynchronous results must be checked for query ownership. TSX never receives cells or hit lists.

ASCII search uses the stepwise `ghostty_search_feed` / `tick` interface already present in this repository's 1.0.1 `search.h`, preserving native matching across soft wraps and screen tracking; it does not call blocking `ghostty_search_run` on every revision. Each advance performs one feed and at most 32 ticks.

Unicode search keeps Rust lowercase matching, copies at most 64 lines of `GridRun` per pass and matches outside the lock. Output changes invalidate old results, and after 150 ms of quiet a rescan runs in batches; a revision change across batches discards that scan. New output never forces a jump to a search result; only a new query completing for the first time, or a user next/previous action, reveals a result.

The export cost of incremental search still depends on hit count. A bounded feed cannot be described as an absolute time guarantee for a whole search. Unicode queries still match within each physical line and do not extend to cross-line semantics. Search remains in `src/session_view/view.rs` and holds the `SurfaceController` and session; it has no separate module.

## SurfaceController

The `SurfaceController` in `src/session_view/surface.rs` sits between NSH / AppKit and the Rust `Session`, one per session. It normalizes events, decides interaction strategy, registers invalidation and schedules the single necessary snapshot and publish.

It is not a general event bus. It does not rewrite terminal protocols, replace the Ghostty parser, modify NSH or add an encoder. Protocol bytes are still produced by `encode_*` in `crates/ghostty-vt`; PTY, process and Ghostty state remain owned by `SessionManager` / `Session`; painting remains the application NSView in `platform/macos/view.rs`; TSX only mounts components and receives low-frequency events.

The goal is not to make every event asynchronous, but to give each event class correct ownership:

- Input (keys, IME commits, paste, press / release, focus) stays synchronous, ordered and explicitly rejectable.
- Display (window, selection, hover, search) only registers invalidation, with at most one expensive sample per round.
- Time has one scheduler; animation, blink, holds and search each have explicit deadlines.

### Ownership and threading

```text
AppKit main thread                    Engine thread                          Reader thread
  NSEvent local monitor                NSH components / effects / set_interval PTY reads
  ├ key press/repeat/release           ├ gestures, wheel, IME, paste, focus    ├ parse bytes
  ├ middle / side buttons              ├ SurfaceController                     ├ update Ghostty state
  └ gestures (dispatched by NSH        │   ├ Window / dirty / pending pointers  └ revision += 1
      to the engine thread)            │   ├ one capture → Arc<WindowSnap>
                                       │   └ Signal update → scene publish
                                       └ Search (deadline and step budget)
```

- **Engine thread**: NSH component functions, `effect`, `set_interval` callbacks, key bindings and pointer handlers all run here, not on the AppKit main thread. NSH 0.6.0 uses `run_on_main_thread` to hand work to the UI thread. `SurfaceController`, `Window`, `Signal` and `Search` belong only to this thread.
- **AppKit main thread**: the local monitor in `platform/macos/keyboard.rs` directly encodes physical keys and middle / side buttons, then hands them to the writer; `drawRect` in `platform/macos/view.rs` only clones `Arc<Scene>` from the scene slot. Neither waits for the engine thread.
- **Reader thread**: holds `Session.emulator` while parsing bytes, releases it and increments `revision`. It does not touch controller state.
- **Reaper / writer**: handle process and input lifecycle. The reaper also checks the foreground ownership of reported agent identities.

Lock order, enforced as a constraint:

```text
emulator → control → status → info
```

- No thread may hold `emulator` while synchronously waiting for the main thread. The main thread holds it only briefly while encoding current input and cloning the scene `Arc`.
- The [PTY module](../../src/sessions/pty.rs) holds `emulator` across both PTY resize and grid resize.
- The scene slot lock is only used to replace or clone `Arc<Scene>`. Painting holds neither the slot lock nor `emulator`.

### Controller state

```text
SurfaceController
  session: Arc<Session>
  window: Window                  // existing integer-window policy, reused unchanged
  snap: Signal<Arc<WindowSnap>>   // published window
  dirty: DirtySet                 // registration only, no immediate sampling
  follow: bool                    // needs bottom-pinning before next publish
  hover: Option<(u16, usize)>     // pending pointer cell (relative to window)
  drag_end: Option<(u16, usize)>  // pending host drag-selection endpoint
  last_error: Option<String>
  clock: Clock                    // reset time, image deadline, search deadline
  stats: Stats                    // debug / acceptance counters only
```

`Window` is unchanged: it still owns only integer origin, `pinned`, `wheel_frac`, rows and columns, and the main-screen policy stash. The history anchor is still owned by the Ghostty tracked reference; the controller does not invent another stable row number.

`DirtySet` has three boolean semantic bits:

- `capture`: window, output, selection, size, theme, font or search reveal changed.
- `publish`: only paint inputs changed and no resample is needed, such as a search hit-set change or a color configuration change.
- `search`: output changes invalidated the old search results.

Registering the same invalidation repeatedly costs nothing; a round processes the set once.

### Event classification and merging

"Thread" means the thread on which the event enters Rust. "Strategy" is this layer's handling contract.

| Event | Thread | Strategy | Ordering / correctness requirement |
|---|---|---|---|
| Key press / repeat / release, modifiers | main | synchronous encode, no merge or delay | strict order within the source; after encoding request follow only, do not capture |
| IME commit / cancel | engine | synchronous commit of final delta | keys during composition still belong to the input method; commit is attributed to the original `Arc<Session>` |
| Paste | engine | synchronous encode | ≤1 MiB; over-limit or queue-full must be explicitly visible |
| Focus enter / leave | engine | synchronous focus-report encode | does not depend on an occasionally lost first responder |
| Mouse press / release (report mode) | engine | synchronous encode | same-source order with wheel and keys; request follow only |
| Mouse press / release (host selection) | engine | synchronous selection-state change | pending drag endpoint must be applied before release |
| Motion (report mode) | engine | synchronous encode, no merge | the 1002 / 1003 position sequence must not be silently dropped; the encoder decides output from the current mode |
| Motion (hover link) | engine | take latest cell, at most one resolution per round | result affects the tooltip only; remap with the round's origin after scrolling |
| Motion (host drag selection) | engine | take latest endpoint, apply at most once per round | proof below |
| Wheel (report mode) | engine | synchronous lossless encode; fractional remainder kept; chunk by writer limit, no integer discard | no "old event under a new mode": never insert a terminal-state observation between event and encode |
| Wheel (history / Shift) | engine | update integer `Window` immediately, register `capture` only | sub-row input only accumulates gesture; direction reversal is processed in arrival order, not netted |
| Middle / side buttons | main | synchronous encode | same `mouse_repeat` path as ordinary mouse; hit testing checks bounds and `visibleRect` |
| Output revision | reader → engine | merge, observe at most once per round | after observing, register `capture` only |
| Resize | engine | synchronous, PTY and grid under one lock | a font change must update cell size even when rows and columns are unchanged; register `capture` |
| Search | engine | deadline-driven, bounded steps | each step is one feed plus ≤32 ticks; output changes invalidate the old scan |
| Image animation | engine | deadline-driven | a frame change registers `capture` and invalidates selection |
| Theme / font / cell metrics | engine | register `capture` + `publish` | the font key cannot use cell width alone |
| Visibility | engine | hidden stops sampling, animation and search; restore observes immediately once | inactive but visible keeps painting |
| Unmount | engine | cancel timers and pending work | never deliver old events to a released or newly bound session |

**Why drag selection can use the latest endpoint.** `extend_selection` reads the current tracked selection and only replaces the end (`crates/ghostty-vt/src/selection.rs`). The start is fixed on press and intermediate endpoints do not participate in later calls. For one drag, applying every endpoint in arrival order and applying only the last endpoint produce the same final selection. Double-click word and triple-click line are determined once on press, and Motion does not change them; rectangular selection is decided only by that round's modifiers. Report-mode motion is not covered by this proof and therefore stays synchronous.

**Why wheel input is not deferred.** Ghostty decides whether to emit a wheel report from the terminal mode at encode time. If an event were queued for the next round, program output in between could switch mouse mode or screen and the old event would be encoded under the new mode. The input path therefore never queues: wheel input is encoded synchronously on arrival. Work is bounded losslessly in two ways: a single event's repeated reports can be split into several bounded writer messages (for example ≤256 reports each), and sub-row / fractional remainders stay in the accumulator until the next event or row. Neither drops displacement. Real trackpad and wheel integer displacement is single digits to dozens of rows; the chunk limit isolates anomalies without changing semantics. The current implementation sends one writer message per event up to 1 MiB and chunks only exceptionally large sequences.

### Order and barriers

1. **Same-source order.** Within one thread, PTY-directed bytes are produced in event arrival order. Display sampling never jumps the queue or reorders input.
2. **Divergence boundary.** The main thread (AppKit monitor) and engine thread (NSH handlers) are independent event sources; the OS provides no total order. This layer guarantees per-source order only, never pretends to a global order, and does not move events from one source to the other to "unify" them.
3. **Release barrier.** A pending drag endpoint is applied before release, and pending intermediate endpoints are discarded so an old endpoint cannot reselect after a click clears; then double-click / single-click semantics decide whether to clear.
4. **Mode changes affect only future events.** The controller never caches keys, mouse or wheel input for later encoding. A mode, screen or size change observed in output affects only events arriving afterwards.
5. **Search reveal is a display action.** It changes only `Window` and registers `capture`; it encodes no input.
6. **Hold.** A bottom-pinned window may publish the complete owned frame from the current transaction start boundary, and the same `Arc` is not republished; a history window remains frozen. Dirty bits are retained until the hold ends, goes quiet for 150 ms (a single hold lasts at most 1 s), hits EOF or native resize, so the final transaction result is published. Ticks are never required to land exactly between two transactions.

### Single frame scheduling

`SessionTerminal` keeps exactly one `set_interval(16 ms)`, whose callback calls `SurfaceController::tick(now)`. The second clock is not present. NSH intervals do not pile up late runs; when the engine is busy, frames are skipped naturally.

One tick follows a fixed order:

```text
1. observe session.revision        → if changed, dirty.capture and invalidate old search
2. observe session.input_epoch     → if changed, follow = true
3. blink phase                     → update only the independent cursor overlay Signal
4. image animation due?            → at most one tick_images; a frame change sets dirty.capture
5. search due?                     → at most one bounded step; completion / reveal sets dirty
6. apply pending hover / drag_end  → at most one lock per round
7. if dirty.capture and visible    → one capture; during a hold only owned boundary frames, set snap if changed
8. if dirty.publish or snap changed → publish the scene through the existing reactive prop path
```

Time sources:

| Purpose | Source |
|---|---|
| Output / input / frame beat | the single 16 ms interval |
| Cursor blink | 500 ms phase from each controller reset time |
| Image animation | next deadline returned by `tick_images` |
| Search quiet period / step | 150 ms deadline plus ≤64 lines or ≤32 ticks per step |
| Synchronized-output hold | reader-side `flush_sync`: non-empty output quiet for 150 ms, single hold at most 1 s; the UI observes only revision |
| Native repaint | AppKit `setNeedsDisplay` coalescing |

There is no second fallback timer. Animation and the cursor do not trigger terminal resampling; only the `capture` bit does.

### Invalidation, snapshot and publish

- The capture criterion remains frame identity, window origin, count and screen identity. `Frame.dirty` cannot be used because capture cleans it.
- `capture()` is called only inside a tick; initial mount and visibility restore may each make one immediate call. Event handlers only register `dirty` and `follow`.
- `follow` is applied before capture: if `!window.pinned`, call `window.pin()`; if already bottom-pinned, do nothing. Each input batch therefore causes at most one window update instead of one per input.
- Capture returns one of three states: `Held` (keep dirty), `Cached(Arc)` (reuse the last window) or `Fresh(Arc)`. Only a `Fresh` window that differs from the published window updates the Signal.
- Publishing is unchanged: `Scene { snap, config, hits, current }` is published through the reactive prop path, and the cross-NSH property still carries only slot ID and revision.
- Counters are recorded separately for debug / acceptance: `capture_attempted` / `capture_held` / `capture_cached` / `capture_fresh`, `snapshot_published`, `scene_published` (incremented inside `Publisher`), `native_draw` (existing `frames`) and `lock_wait` around capture on demand. They show whether repeated sampling converges; NSH FPS is not the terminal publish rate, and an empty canvas's zero is not zero cost.
- Debug builds emit diagnostics every two seconds. `TERMLEVEL_SURFACE_LOG` chooses the file, default `/tmp/termlevel-surface.log`. `held` counts rounds that reuse an old window inside a hold, and `hold ms max` is the wait until a publishable frame advances; `attempted` / `cached` include cheap boundary checks performed during a hold.

The continuous-synchronization path was corrected after real Codex / Claude logs showed 111 reported wheel events and 115 revision changes but only about one publish in 2 seconds, with the controller waiting up to 2100.8 ms. The earlier policy of skipping all hold rounds missed the completed boundary between two transactions. `begin_window` now reuses the complete `Frame`, screen identity, colors and images already saved by the hold callback for a bottom-pinned window, finishes run merging outside the lock and performs no extra live-cell fetch. History windows keep the frozen policy. The main-screen policy stash is validated against a candidate `Window` before being applied, preventing early consumption. Timeouts were not shortened and unfinished transaction content is never displayed. Deterministic tests cover 160 consecutive transactions, same-boundary reuse and incomplete screen-switch isolation. The native wheel fixture adds `--overlap-sync`, which requires the frame number to advance visibly every 500 ms while input continues. This wheel fix was manually re-verified; other compatibility and sustained multi-pane acceptance boundaries are unchanged.

### Congestion and rejection

The contract is honest about finite memory; it does not promise unlimited delivery.

- **Keys / focus / wheel / press-release**: encode synchronously and write to the writer. A `try_send` failure must be explicit, never silently dropped. Same-source order is preserved.
- **Paste**: one request is ≤1 MiB; exceeding the limit is rejected immediately and visibly, never truncated.
- **Writer queue**: fixed at 128 messages. It is a rejection boundary, not a backpressure protocol; it is not made unbounded, and a full queue is not described as recoverable.
- **Scene**: latest-only; a later publish overwrites the previous one and painting cannot backlog.
- **Sampling**: at most one capture per round; denser output is merged.
- **Not done**: no stretched debounce to make things "look stable", and no debounce on reliable events.

A `fail()` currently marks the session as `error`. The requirement that rejection be visible is retained, and a useful remaining refinement is to separate transient input rejection from session failure into two low-frequency states (for example, show "input busy" while the process stays running). This is not required for the scheduling layer.

### Lifecycle

- **Hidden**: stop capture, search advancement and animation. Revision, bell and status are still observed, so sidebar markers are unaffected.
- **Visibility restored**: observe revision immediately and perform one capture rather than waiting for the next tick.
- **Inactive but visible**: keep refreshing on the same beat; `active` controls input authorization and focus only.
- **Unmount**: the interval is released with its owner, and key listeners use the existing cleanup. The controller does not own PTY lifetime and must not deliver pending pointer or search results to a released or newly bound session.
- **Rebind**: the new component reinitializes `Window` and the snapshot from current emulator state, without relying on an old JS copy and without replaying old events.

## Performance and acceptance

Automated tests separately verify window math, the real Ghostty tracked anchor, resize and screen switching inside a hold, selection invalidation, the image fast path, native incremental search, native cache invalidation, image-layer order and source cropping, row semantics without drawing operations, and wide-character coordinates. A simulated incrementing `scroll_base` is not sufficient evidence of history stability.

Use:

```sh
export GHOSTTY_VT_ARCHIVE=/tmp/termlevel-ghostty-vt-1.0.1.xcframework.zip
export CARGO_NET_OFFLINE=true
./scripts/nsh check
./scripts/nsh test
./scripts/nsh build
```

Record native manual acceptance separately:

```sh
./scripts/nsh run --window-size 1200x780
```

At minimum inspect Codex report-mode scrolling, main-screen history and Shift wheel, whether clear reveals the previous line, resize inside a hold and main/alternate switching, Chinese / Japanese / dead-key composition and candidate windows, cross-line selection, OSC 8, three image layers and animation, and sustained output from multiple panes.

Performance comparisons must hold window size, font size, command and load constant and record the build mode. Record emulator lock wait and snapshot, run merge and publish, native `drawRect`, NSH presentation interval and input response separately. Native `drawRect` statistics include font / `CTLine` cache misses, image conversion and CoreText drawing; NSH empty-canvas metrics are not terminal paint cost. Debug builds summarize native paint count, mean, maximum and cache sizes per second. Wall-clock thresholds are not written as library test assertions.

The controlled fixture is `scripts/test-native-scroll`: 1200×780, Menlo 13 pt, 21 pt line height, a mouse-reporting TUI, 2026 synchronized output, Chinese and English text, and colored text. The old path recorded `artifacts/scroll-before.log` and a CPU sample; the native path records `artifacts/scroll-native-surface.log`. The native path paints in about 2.6–2.9 ms and the NSH frame meter shows mostly 54–59 FPS during the stable phase; the old path recorded about 24–26 ms of Canvas replay and 38–44 FPS. A 25-second extended load that reached the 2048-entry cache limit painted in about 2.7–3.1 ms with the later NSH frame meter around 54–60 FPS, recorded in `artifacts/scroll-native-surface-sustained-final.log`. The native path also has intervals around 50 FPS and cold-start gaps, so it does not claim a locked 60 FPS. The NSH frame meter and the actual terminal snapshot publish count are different metrics, and this fixture does not replace real Codex gesture acceptance.

## Current implementation and limitations

The fixed window, tracked anchor, owned snapshot and stepwise search remain. The production grid's background and text canvases and the NSH image nodes have been replaced by one native component; only the no-drawing semantic metadata adapter remains. There is no old-renderer feature flag and no long-term dual path, and the code does not return to `virtual_list`, `PaintedScreen` or the list scrolling API.

Remaining explicit limitations:

- Native capture in the safe wrapper still copies the viewport under the lock; library-level begin/end separation is not claimed.
- Only the macOS renderer exists. Other platforms need an independent renderer and acceptance.
- Fallback glyph fitting uses the fixed allocated cell and does not borrow adjacent cells. A GPU atlas is not implemented.
- Complex emoji, ligatures crossing style or selection boundaries, bold-italic synthesis, curved underlines and underline colors remain incomplete.
- Fine-grained AppKit accessibility row children are not implemented; the native component exposes a whole-window label.
- The Unicode search path matches per physical line and does not add cross-line semantics; export cost still scales with hit count.
- CoreText object memory is not bounded by the text-cache limit and still needs sustained-output stress testing.
- Real Codex gesture feel, IME and candidate-window acceptance, selection and the sustained multi-pane boundary still require manual acceptance. The continuous-sync wheel correction was manually confirmed, but it does not stand in for those remaining checks.
- Transient input rejection and session `error` are not yet separate states.
