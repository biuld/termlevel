# Native handle threading audit (libghostty-vt 1.0.1)

`Terminal` and `RenderState` implement **Send**, and deliberately do not
implement **Sync**. Termlevel keeps both inside `Session.emulator: Mutex<_>`.
UI mount lifetime does not own this mutex or the PTY.

The native API expects serialized access rather than a particular OS thread:

- `include/ghostty/vt/render.h`, `ghostty_render_state_begin_update()` explicitly
  describes an IO thread and renderer sharing a locked terminal. The begin
  phase needs terminal access; the end phase reads render-owned memory only.
  The [official rendering guide](https://lib.ghostty.org/vt/guide-rendering.html)
  explains the same model.
- `terminal.h`, `ghostty_terminal_compress()` requires serialization with
  writes, rendering, searches and other access on the same terminal.
- `terminal.h`, `ghostty_terminal_set()` documents synchronous, non-reentrant
  callbacks. Our callbacks copy owned data. Clipboard callbacks invoke a bounded
  Send + Sync host adapter synchronously; it must never reenter the terminal
  or wait on the UI thread. Callback storage is a stable heap allocation.
- `grid_ref.h` disallows using untracked references after terminal mutation.
  This wrapper resolves, reads and discards them under one exclusive borrow.
- `grid_ref_tracked.h` allows tracked references to follow mutations and be
  freed after their terminal. The history floor uses one per screen and owns
  their handles; no native pointer is exposed in its returned ranges.
- `sys.h` declares process-global hooks unsafe to change concurrently. This
  safe wrapper installs its immutable PNG decoder through a process-wide Once
  before constructing any safe terminal. Decoding has bounded input/output and
  allocates returned pixels with the supplied allocator. It never accepts custom
  allocators. Callers
  reaching them through the raw crate must uphold those C contracts themselves.

The Send implementation covers the default-allocator native terminal,
encoders, tracked references, render snapshot and Rust-owned callback data.
Terminal identity uses `Arc`/`Weak`, not `Rc`. Nothing in the safe API retains
an externally borrowed buffer, a thread-bound Rust closure, or a UI handle.
No row iterator or C buffer escapes `RenderState::capture()`.

`!Sync` prevents safe simultaneous access even through shared methods that may
cause native caches to update. `Frame` and `Event` contain owned Rust values
and can travel separately from the terminal. Compile tests assert Send and
reject Sync; a native test moves terminal access and a renderer between threads
under a mutex. Real PTY integration tests exercise reader/UI access using the
same serialization as the app. These tests supplement the ownership argument;
they are not a proof that arbitrary unsynchronized raw C use is safe.
