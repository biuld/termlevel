//! Session-independent terminal emulation using the official libghostty-vt 1.0.1.
//!
//! This crate owns no PTY, process or UI. Native handles stay on the thread that
//! accesses them exclusively (`Send`, not `Sync`); copied frames contain no borrowed
//! C memory. The full C API remains available through [`raw`].
//!
//! ```
//! use termlevel_ghostty_vt::Terminal;
//! let mut terminal = Terminal::new(80, 24)?;
//! terminal.write(b"hello\r\n")?;
//! assert!(terminal.text()?.contains("hello"));
//! # Ok::<(), termlevel_ghostty_vt::Error>(())
//! ```

use raw as sys;
use std::{
    cell::UnsafeCell,
    ffi::{CStr, c_void},
    ptr,
};
pub use termlevel_ghostty_vt_sys as raw;

mod clipboard;
mod graphics;
mod input;
mod render;
mod selection;
mod state;
pub use clipboard::Clipboard;
pub use graphics::{ImagePlacement, Pixels};
pub use input::{KeyEvent, MouseEvent};
pub use render::{Cell, Color, Cursor, Frame, RenderState, Row, Style, StyleColor};
pub use state::{Colors, Match};

/// A native error code. Preserved as an integer to allow future ABI additions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error(pub sys::GhosttyResult);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "libghostty-vt error {}", self.0)
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

fn check(code: sys::GhosttyResult) -> Result<()> {
    if code == sys::GHOSTTY_SUCCESS {
        Ok(())
    } else {
        Err(Error(code))
    }
}

// Internal owners never escape. Every fallible constructor installs a Drop
// guard before configuring the handle, so errors cannot leak partial objects.
macro_rules! handle {
    ($name:ident, $raw:ty, $new:path, $free:path) => {
        pub(super) struct $name($raw);
        impl $name {
            pub(super) fn new() -> Result<Self> {
                let mut raw = ptr::null_mut();
                // SAFETY: default allocator; valid out pointer for this handle.
                check(unsafe { $new(ptr::null(), &mut raw) })?;
                if raw.is_null() {
                    return Err(Error(sys::GHOSTTY_INVALID_VALUE));
                }
                Ok(Self(raw))
            }
        }
        impl Drop for $name {
            fn drop(&mut self) {
                // SAFETY: exclusively owns a live handle of the corresponding kind.
                unsafe { $free(self.0) }
            }
        }
    };
}
use handle;

/// Version reported by the linked library, rather than the binding headers.
pub fn library_version() -> &'static str {
    // SAFETY: upstream returns a process-lifetime NUL-terminated static string.
    unsafe { CStr::from_ptr(sys::ghostty_version()) }
        .to_str()
        .unwrap_or("invalid UTF-8 version")
}

/// OSC 9;4 progress state. Reports are snapshots: the latest one wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressState {
    Remove,
    Set,
    Error,
    Indeterminate,
    Pause,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressReport {
    pub state: ProgressState,
    /// 0 through 100, or `None` when the program omitted the percentage.
    pub percent: Option<u8>,
}

/// OSC 7501 program status state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramStatusState {
    Idle,
    Working,
    Done,
    Blocked,
    Error,
    Clear,
}

/// What a blocked program needs from the user (OSC 7501).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramStatusKind {
    None,
    Permission,
    Question,
    Auth,
}

/// One OSC 7501 report. The application keeps records keyed by `id`; the
/// terminal itself does not store them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramStatusReport {
    pub state: ProgramStatusState,
    pub kind: ProgramStatusKind,
    /// 0 through 100, or `None` when omitted.
    pub percent: Option<u8>,
    pub id: String,
    pub app: String,
    pub title: String,
    pub message: String,
}

/// The OSC 22 pointer shape requested by the running program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseShape {
    Default,
    ContextMenu,
    Help,
    Pointer,
    Progress,
    Wait,
    Cell,
    Crosshair,
    Text,
    VerticalText,
    Alias,
    Copy,
    Move,
    NoDrop,
    NotAllowed,
    Grab,
    Grabbing,
    AllScroll,
    ColResize,
    RowResize,
    NResize,
    EResize,
    SResize,
    WResize,
    NEResize,
    NWResize,
    SEResize,
    SWResize,
    EWResize,
    NSResize,
    NESWResize,
    NWSEResize,
    ZoomIn,
    ZoomOut,
}

impl MouseShape {
    fn from_raw(raw: sys::GhosttyMouseShape) -> Self {
        match raw {
            sys::GHOSTTY_MOUSE_SHAPE_DEFAULT => Self::Default,
            sys::GHOSTTY_MOUSE_SHAPE_CONTEXT_MENU => Self::ContextMenu,
            sys::GHOSTTY_MOUSE_SHAPE_HELP => Self::Help,
            sys::GHOSTTY_MOUSE_SHAPE_POINTER => Self::Pointer,
            sys::GHOSTTY_MOUSE_SHAPE_PROGRESS => Self::Progress,
            sys::GHOSTTY_MOUSE_SHAPE_WAIT => Self::Wait,
            sys::GHOSTTY_MOUSE_SHAPE_CELL => Self::Cell,
            sys::GHOSTTY_MOUSE_SHAPE_CROSSHAIR => Self::Crosshair,
            sys::GHOSTTY_MOUSE_SHAPE_TEXT => Self::Text,
            sys::GHOSTTY_MOUSE_SHAPE_VERTICAL_TEXT => Self::VerticalText,
            sys::GHOSTTY_MOUSE_SHAPE_ALIAS => Self::Alias,
            sys::GHOSTTY_MOUSE_SHAPE_COPY => Self::Copy,
            sys::GHOSTTY_MOUSE_SHAPE_MOVE => Self::Move,
            sys::GHOSTTY_MOUSE_SHAPE_NO_DROP => Self::NoDrop,
            sys::GHOSTTY_MOUSE_SHAPE_NOT_ALLOWED => Self::NotAllowed,
            sys::GHOSTTY_MOUSE_SHAPE_GRAB => Self::Grab,
            sys::GHOSTTY_MOUSE_SHAPE_GRABBING => Self::Grabbing,
            sys::GHOSTTY_MOUSE_SHAPE_ALL_SCROLL => Self::AllScroll,
            sys::GHOSTTY_MOUSE_SHAPE_COL_RESIZE => Self::ColResize,
            sys::GHOSTTY_MOUSE_SHAPE_ROW_RESIZE => Self::RowResize,
            sys::GHOSTTY_MOUSE_SHAPE_N_RESIZE => Self::NResize,
            sys::GHOSTTY_MOUSE_SHAPE_E_RESIZE => Self::EResize,
            sys::GHOSTTY_MOUSE_SHAPE_S_RESIZE => Self::SResize,
            sys::GHOSTTY_MOUSE_SHAPE_W_RESIZE => Self::WResize,
            sys::GHOSTTY_MOUSE_SHAPE_NE_RESIZE => Self::NEResize,
            sys::GHOSTTY_MOUSE_SHAPE_NW_RESIZE => Self::NWResize,
            sys::GHOSTTY_MOUSE_SHAPE_SE_RESIZE => Self::SEResize,
            sys::GHOSTTY_MOUSE_SHAPE_SW_RESIZE => Self::SWResize,
            sys::GHOSTTY_MOUSE_SHAPE_EW_RESIZE => Self::EWResize,
            sys::GHOSTTY_MOUSE_SHAPE_NS_RESIZE => Self::NSResize,
            sys::GHOSTTY_MOUSE_SHAPE_NESW_RESIZE => Self::NESWResize,
            sys::GHOSTTY_MOUSE_SHAPE_NWSE_RESIZE => Self::NWSEResize,
            sys::GHOSTTY_MOUSE_SHAPE_ZOOM_IN => Self::ZoomIn,
            sys::GHOSTTY_MOUSE_SHAPE_ZOOM_OUT => Self::ZoomOut,
            _ => Self::Text,
        }
    }

    /// CSS-compatible name; the platform maps unsupported names to its default.
    pub fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::ContextMenu => "context-menu",
            Self::Help => "help",
            Self::Pointer => "pointer",
            Self::Progress => "progress",
            Self::Wait => "wait",
            Self::Cell => "cell",
            Self::Crosshair => "crosshair",
            Self::Text => "text",
            Self::VerticalText => "vertical-text",
            Self::Alias => "alias",
            Self::Copy => "copy",
            Self::Move => "move",
            Self::NoDrop => "no-drop",
            Self::NotAllowed => "not-allowed",
            Self::Grab => "grab",
            Self::Grabbing => "grabbing",
            Self::AllScroll => "all-scroll",
            Self::ColResize => "col-resize",
            Self::RowResize => "row-resize",
            Self::NResize => "n-resize",
            Self::EResize => "e-resize",
            Self::SResize => "s-resize",
            Self::WResize => "w-resize",
            Self::NEResize => "ne-resize",
            Self::NWResize => "nw-resize",
            Self::SEResize => "se-resize",
            Self::SWResize => "sw-resize",
            Self::EWResize => "ew-resize",
            Self::NSResize => "ns-resize",
            Self::NESWResize => "nesw-resize",
            Self::NWSEResize => "nwse-resize",
            Self::ZoomIn => "zoom-in",
            Self::ZoomOut => "zoom-out",
        }
    }
}

/// OSC 133 semantic prompt step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticPromptKind {
    PromptStart,
    InputStart,
    OutputStart,
    CommandEnd,
}

/// Which prompt a `PromptStart` event starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticPromptPromptKind {
    Primary,
    Right,
    Continuation,
    Secondary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticPromptReport {
    pub kind: SemanticPromptKind,
    pub prompt_kind: SemanticPromptPromptKind,
    pub has_exit_code: bool,
    pub exit_code: i32,
    pub command: String,
    pub error: String,
    pub cursor_at_prompt: bool,
}

/// Owned effects. The caller decides how to deliver replies to its PTY.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    WritePty(Vec<u8>),
    Bell,
    Notification { title: String, body: String },
    Progress(ProgressReport),
    ProgramStatus(ProgramStatusReport),
    SemanticPrompt(SemanticPromptReport),
    TitleChanged(String),
    WorkingDirectoryChanged(String),
    Reset,
}

fn percent(value: i8) -> Option<u8> {
    u8::try_from(value).ok().filter(|value| *value <= 100)
}

fn progress_state(raw: sys::GhosttyTerminalProgressState) -> Option<ProgressState> {
    Some(match raw {
        sys::GHOSTTY_TERMINAL_PROGRESS_STATE_REMOVE => ProgressState::Remove,
        sys::GHOSTTY_TERMINAL_PROGRESS_STATE_SET => ProgressState::Set,
        sys::GHOSTTY_TERMINAL_PROGRESS_STATE_ERROR => ProgressState::Error,
        sys::GHOSTTY_TERMINAL_PROGRESS_STATE_INDETERMINATE => ProgressState::Indeterminate,
        sys::GHOSTTY_TERMINAL_PROGRESS_STATE_PAUSE => ProgressState::Pause,
        _ => return None,
    })
}

fn program_status_state(raw: sys::GhosttyProgramStatusState) -> Option<ProgramStatusState> {
    Some(match raw {
        sys::GHOSTTY_PROGRAM_STATUS_STATE_IDLE => ProgramStatusState::Idle,
        sys::GHOSTTY_PROGRAM_STATUS_STATE_WORKING => ProgramStatusState::Working,
        sys::GHOSTTY_PROGRAM_STATUS_STATE_DONE => ProgramStatusState::Done,
        sys::GHOSTTY_PROGRAM_STATUS_STATE_BLOCKED => ProgramStatusState::Blocked,
        sys::GHOSTTY_PROGRAM_STATUS_STATE_ERROR => ProgramStatusState::Error,
        sys::GHOSTTY_PROGRAM_STATUS_STATE_CLEAR => ProgramStatusState::Clear,
        _ => return None,
    })
}

fn program_status_kind(raw: sys::GhosttyProgramStatusKind) -> ProgramStatusKind {
    match raw {
        sys::GHOSTTY_PROGRAM_STATUS_KIND_PERMISSION => ProgramStatusKind::Permission,
        sys::GHOSTTY_PROGRAM_STATUS_KIND_QUESTION => ProgramStatusKind::Question,
        sys::GHOSTTY_PROGRAM_STATUS_KIND_AUTH => ProgramStatusKind::Auth,
        _ => ProgramStatusKind::None,
    }
}

fn semantic_prompt_kind(raw: sys::GhosttySemanticPromptKind) -> Option<SemanticPromptKind> {
    Some(match raw {
        sys::GHOSTTY_SEMANTIC_PROMPT_PROMPT_START => SemanticPromptKind::PromptStart,
        sys::GHOSTTY_SEMANTIC_PROMPT_INPUT_START => SemanticPromptKind::InputStart,
        sys::GHOSTTY_SEMANTIC_PROMPT_OUTPUT_START => SemanticPromptKind::OutputStart,
        sys::GHOSTTY_SEMANTIC_PROMPT_COMMAND_END => SemanticPromptKind::CommandEnd,
        _ => return None,
    })
}

fn semantic_prompt_prompt_kind(
    raw: sys::GhosttySemanticPromptPromptKind,
) -> SemanticPromptPromptKind {
    match raw {
        sys::GHOSTTY_SEMANTIC_PROMPT_PROMPT_RIGHT => SemanticPromptPromptKind::Right,
        sys::GHOSTTY_SEMANTIC_PROMPT_PROMPT_CONTINUATION => SemanticPromptPromptKind::Continuation,
        sys::GHOSTTY_SEMANTIC_PROMPT_PROMPT_SECONDARY => SemanticPromptPromptKind::Secondary,
        _ => SemanticPromptPromptKind::Primary,
    }
}

#[derive(Default)]
struct Events {
    queue: Vec<Event>,
    failed: bool,
    held: Option<RenderHold>,
    hold_generation: u64,
    clipboard: Option<std::sync::Arc<dyn Clipboard>>,
}

/// The active viewport at the exact point where mode 2026 was enabled.
#[derive(Debug, Clone)]
pub struct RenderHold {
    pub frame: std::sync::Arc<Frame>,
    pub total_rows: usize,
    pub alternate_screen: bool,
    pub since: std::time::Instant,
}

struct TerminalHandle(sys::GhosttyTerminal);
impl Drop for TerminalHandle {
    fn drop(&mut self) {
        // SAFETY: owns this terminal; callback storage is still alive here.
        unsafe { sys::ghostty_terminal_free(self.0) }
    }
}

/// An emulation state, independent of process and view lifetime.
///
/// Handles can move between threads, but every operation on a terminal must be
/// serialized, e.g. by a `Mutex`. No safe raw-handle access is exposed.
///
/// ```
/// use termlevel_ghostty_vt::Terminal;
/// fn requires_send<T: Send>() {}
/// requires_send::<Terminal>();
/// ```
/// ```compile_fail
/// use termlevel_ghostty_vt::Terminal;
/// fn requires_sync<T: Sync>() {}
/// requires_sync::<Terminal>();
/// ```
pub struct Terminal {
    // Field order matters: free the terminal before its callback storage.
    handle: TerminalHandle,
    events: Box<UnsafeCell<Events>>,
    key_encoder: input::KeyEncoder,
    mouse_encoder: input::MouseEncoder,
    identity: std::sync::Arc<()>,
    retention: state::Retention,
}

// SAFETY: libghostty-vt terminals have no thread affinity. All access, including
// rendering/search/compression, must be serialized (terminal.h and the official
// rendering guide). The wrapper owns the default allocator, native encoders and
// heap-stable callback storage; no user callbacks/allocators or borrowed handles
// escape. All mutations require &mut self, and !Sync prevents concurrent reads
// of native caches. Process-global hooks can only be changed through unsafe raw
// APIs. Arc identity is thread-safe; callbacks execute synchronously on the caller.
unsafe impl Send for Terminal {}

impl Terminal {
    pub fn new(cols: u16, rows: u16) -> Result<Self> {
        graphics::initialize();
        if cols == 0 || rows == 0 {
            return Err(Error(sys::GHOSTTY_INVALID_VALUE));
        }
        let key_encoder = input::KeyEncoder::new()?;
        let mouse_encoder = input::MouseEncoder::new()?;
        let mut raw = ptr::null_mut();
        // SAFETY: nonzero dimensions and correctly typed output storage.
        check(unsafe { sys::ghostty_terminal_new(ptr::null(), &mut raw, cols, rows) })?;
        if raw.is_null() {
            return Err(Error(sys::GHOSTTY_INVALID_VALUE));
        }
        let this = Self {
            handle: TerminalHandle(raw),
            events: Box::new(UnsafeCell::new(Events::default())),
            key_encoder,
            mouse_encoder,
            identity: std::sync::Arc::new(()),
            retention: state::Retention::default(),
        };
        // SAFETY: each key has its exact documented pointer/function type.
        // The Box allocation outlives the terminal and doesn't move with Self.
        unsafe {
            check(sys::ghostty_terminal_set(
                raw,
                sys::GHOSTTY_TERMINAL_OPT_USERDATA,
                this.events.get().cast(),
            ))?;
            for (key, callback) in [
                (
                    sys::GHOSTTY_TERMINAL_OPT_WRITE_PTY,
                    on_write as *const () as *const c_void,
                ),
                (
                    sys::GHOSTTY_TERMINAL_OPT_DESKTOP_NOTIFICATION,
                    on_notification as *const () as *const c_void,
                ),
                (
                    sys::GHOSTTY_TERMINAL_OPT_BELL,
                    on_bell as *const () as *const c_void,
                ),
                (
                    sys::GHOSTTY_TERMINAL_OPT_PROGRESS_REPORT,
                    on_progress as *const () as *const c_void,
                ),
                (
                    sys::GHOSTTY_TERMINAL_OPT_PROGRAM_STATUS,
                    on_program_status as *const () as *const c_void,
                ),
                (
                    sys::GHOSTTY_TERMINAL_OPT_SEMANTIC_PROMPT,
                    on_semantic_prompt as *const () as *const c_void,
                ),
                (
                    sys::GHOSTTY_TERMINAL_OPT_TITLE_CHANGED,
                    on_title as *const () as *const c_void,
                ),
                (
                    sys::GHOSTTY_TERMINAL_OPT_PWD_CHANGED,
                    on_pwd as *const () as *const c_void,
                ),
                (
                    sys::GHOSTTY_TERMINAL_OPT_RESET,
                    on_reset as *const () as *const c_void,
                ),
                (
                    sys::GHOSTTY_TERMINAL_OPT_SIZE,
                    on_size as *const () as *const c_void,
                ),
                (
                    sys::GHOSTTY_TERMINAL_OPT_RENDER_HOLD,
                    on_hold as *const () as *const c_void,
                ),
            ] {
                check(sys::ghostty_terminal_set(raw, key, callback))?;
            }
        }
        Ok(this)
    }

    /// Feed raw output bytes. Parser state survives split UTF-8 and escapes.
    /// Drain [`Self::take_events`] regularly to deliver protocol replies.
    pub fn write(&mut self, bytes: &[u8]) -> Result<()> {
        // SAFETY: exclusive access to the terminal; callbacks only enqueue data.
        unsafe { sys::ghostty_terminal_vt_write(self.handle.0, bytes.as_ptr(), bytes.len()) }
        self.retained_range()?;
        self.callback_result()
    }

    /// Update the grid and physical cell dimensions. Zero pixel dimensions mean
    /// unknown. The caller must also resize its PTY under its own synchronization.
    pub fn resize(
        &mut self,
        cols: u16,
        rows: u16,
        cell_width_px: u32,
        cell_height_px: u32,
    ) -> Result<()> {
        if cols == 0 || rows == 0 {
            return Err(Error(sys::GHOSTTY_INVALID_VALUE));
        }
        // SAFETY: valid terminal and nonzero grid dimensions.
        check(unsafe {
            sys::ghostty_terminal_resize(self.handle.0, cols, rows, cell_width_px, cell_height_px)
        })?;
        self.callback_result()
    }

    /// Reset locally. [`Event::Reset`] reports a reset requested by VT output;
    /// this explicit host operation does not synthesize an additional event.
    pub fn reset(&mut self) -> Result<()> {
        // SAFETY: exclusive access to a live terminal.
        unsafe { sys::ghostty_terminal_reset(self.handle.0) }
        self.callback_result()
    }

    pub fn size(&self) -> Result<(u16, u16)> {
        let (mut cols, mut rows) = (0u16, 0u16);
        // SAFETY: each data key writes a u16 to valid matching storage.
        unsafe {
            check(sys::ghostty_terminal_get(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_DATA_COLS,
                (&mut cols as *mut u16).cast(),
            ))?;
            check(sys::ghostty_terminal_get(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_DATA_ROWS,
                (&mut rows as *mut u16).cast(),
            ))?;
        }
        Ok((cols, rows))
    }

    /// The pointer shape requested by OSC 22. Initial value is text.
    pub fn mouse_shape(&self) -> Result<MouseShape> {
        let mut raw: sys::GhosttyMouseShape = 0;
        // SAFETY: the data key writes the matching C enum to valid storage.
        check(unsafe {
            sys::ghostty_terminal_get(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_DATA_MOUSE_SHAPE,
                (&mut raw as *mut sys::GhosttyMouseShape).cast(),
            )
        })?;
        Ok(MouseShape::from_raw(raw))
    }

    /// Whether the cursor is on a semantic shell prompt (OSC 133).
    pub fn cursor_at_prompt(&self) -> Result<bool> {
        // SAFETY: the data key writes a bool to valid storage.
        unsafe { terminal_bool(self.handle.0, sys::GHOSTTY_TERMINAL_DATA_CURSOR_AT_PROMPT) }
    }

    /// Copy the active screen and its scrollback as trimmed plain UTF-8 text.
    /// No pointer into the terminal escapes this call.
    pub fn text(&self) -> Result<String> {
        struct Formatter(sys::GhosttyFormatter);
        impl Drop for Formatter {
            fn drop(&mut self) {
                // SAFETY: owns the formatter; the borrowed terminal is still alive.
                unsafe { sys::ghostty_formatter_free(self.0) }
            }
        }
        let mut formatter = ptr::null_mut();
        // SAFETY: default allocator; live immutable terminal for the whole call.
        check(unsafe {
            sys::ghostty_formatter_terminal_new(ptr::null(), &mut formatter, self.handle.0)
        })?;
        let formatter = Formatter(formatter);
        // SAFETY: the formatter does not outlive this terminal and cannot mutate it.
        let bytes = buffer(|data, len, written| unsafe {
            sys::ghostty_formatter_format_buf(formatter.0, data, len, written)
        })?;
        String::from_utf8(bytes).map_err(|_| Error(sys::GHOSTTY_INVALID_VALUE))
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events.get_mut().queue)
    }

    pub fn render_hold(&self) -> Option<&RenderHold> {
        // SAFETY: synchronous callbacks cannot overlap this !Sync shared borrow.
        unsafe { &*self.events.get() }.held.as_ref()
    }

    /// Changes when a hold callback captures native dirty state. A separate
    /// incremental renderer must rebuild after this counter changes.
    pub fn render_hold_generation(&self) -> u64 {
        // SAFETY: callbacks are synchronous; this !Sync borrow cannot overlap one.
        unsafe { &*self.events.get() }.hold_generation
    }

    /// Commit a timed-out hold or the last output before EOF. Setting the mode
    /// directly does not invoke the native callback, so clear our snapshot too.
    pub fn release_render_hold(&mut self) -> Result<()> {
        let config = sys::GhosttyTerminalModeConfig {
            mode: sys::GHOSTTY_MODE_SYNCHRONIZED_OUTPUT,
            value: false,
        };
        // SAFETY: exact MODE input type, exclusively owned terminal.
        check(unsafe {
            sys::ghostty_terminal_set(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_OPT_MODE,
                (&config as *const sys::GhosttyTerminalModeConfig).cast(),
            )
        })?;
        self.events.get_mut().held = None;
        Ok(())
    }

    fn callback_result(&self) -> Result<()> {
        // SAFETY: callbacks are synchronous and have finished; no concurrent access.
        if unsafe { &*self.events.get() }.failed {
            Err(Error(sys::GHOSTTY_IO_ERROR))
        } else {
            Ok(())
        }
    }
}

// Private fixed-key getter. Never expose an arbitrary (key, T) pair as safe API.
unsafe fn terminal_string(
    terminal: sys::GhosttyTerminal,
    key: sys::GhosttyTerminalData,
) -> Result<String> {
    let mut value = sys::GhosttyString::default();
    // SAFETY: callers only pass TITLE or PWD, whose output is GhosttyString.
    check(unsafe {
        sys::ghostty_terminal_get(
            terminal,
            key,
            (&mut value as *mut sys::GhosttyString).cast(),
        )
    })?;
    if value.len == 0 {
        return Ok(String::new());
    }
    // SAFETY: upstream guarantees valid bytes until the next terminal mutation.
    let bytes = unsafe { std::slice::from_raw_parts(value.ptr, value.len) };
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

unsafe fn terminal_bool(
    terminal: sys::GhosttyTerminal,
    key: sys::GhosttyTerminalData,
) -> Result<bool> {
    let mut value = false;
    // SAFETY: callers only pass bool data keys; the out pointer matches.
    check(unsafe { sys::ghostty_terminal_get(terminal, key, (&mut value as *mut bool).cast()) })?;
    Ok(value)
}

// Copy callback-lifetime text before returning; the native pointer never
// crosses the ABI. Empty and null strings both read as an empty string.
unsafe fn copy_string(value: sys::GhosttyString, limit: usize) -> String {
    if value.len == 0 || value.ptr.is_null() {
        return String::new();
    }
    // SAFETY: the library borrows valid bytes for the callback's duration.
    let bytes = unsafe { std::slice::from_raw_parts(value.ptr.cast::<u8>(), value.len.min(limit)) };
    String::from_utf8_lossy(bytes).into_owned()
}

// No user closures run across the ABI. Catch even unexpected Rust panics before
// they reach C; report failure on the initiating Rust operation.
unsafe fn enqueue(userdata: *mut c_void, event: impl FnOnce() -> Result<Event>) {
    // SAFETY: all installed callbacks receive the stable Events allocation.
    let events = unsafe { &mut *userdata.cast::<Events>() };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let value = event()?;
        // Progress is a snapshot, not an incremental event. Replace a pending
        // value instead of growing the queue when a program reports rapidly.
        if let Event::Progress(report) = value {
            if let Some(Event::Progress(previous)) = events.queue.last_mut() {
                *previous = report;
                return Ok::<(), Error>(());
            }
            events
                .queue
                .try_reserve(1)
                .map_err(|_| Error(sys::GHOSTTY_OUT_OF_MEMORY))?;
            events.queue.push(Event::Progress(report));
        } else {
            events
                .queue
                .try_reserve(1)
                .map_err(|_| Error(sys::GHOSTTY_OUT_OF_MEMORY))?;
            events.queue.push(value);
        }
        Ok::<(), Error>(())
    }));
    if !matches!(result, Ok(Ok(()))) {
        events.failed = true;
    }
}

unsafe extern "C" fn on_write(
    _: sys::GhosttyTerminal,
    userdata: *mut c_void,
    data: *const u8,
    len: usize,
) {
    // SAFETY: C lends this buffer for the callback only; copy it before returning.
    unsafe {
        enqueue(userdata, || {
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(len)
                .map_err(|_| Error(sys::GHOSTTY_OUT_OF_MEMORY))?;
            if len != 0 {
                bytes.extend_from_slice(std::slice::from_raw_parts(data, len));
            }
            Ok(Event::WritePty(bytes))
        })
    }
}
unsafe extern "C" fn on_notification(
    _: sys::GhosttyTerminal,
    userdata: *mut c_void,
    notification: *const sys::GhosttyTerminalDesktopNotification,
) {
    // SAFETY: native callback strings are valid only until return. Copy bounded
    // UTF-8 owned text synchronously; no native pointer crosses the callback.
    unsafe {
        enqueue(userdata, || {
            let notification = notification
                .as_ref()
                .ok_or(Error(sys::GHOSTTY_INVALID_VALUE))?;
            Ok(Event::Notification {
                title: copy_string(notification.title, 640),
                body: copy_string(notification.body, 16384),
            })
        });
    }
}

unsafe extern "C" fn on_progress(
    _: sys::GhosttyTerminal,
    userdata: *mut c_void,
    report: *const sys::GhosttyTerminalProgressReport,
) {
    // SAFETY: the report is callback-lifetime only; copy the scalar fields and
    // ignore an unknown future state instead of poisoning the terminal.
    let Some(report) = (unsafe { report.as_ref() }) else {
        return;
    };
    let Some(state) = progress_state(report.state) else {
        return;
    };
    let report = ProgressReport {
        state,
        percent: percent(report.progress),
    };
    // SAFETY: registered with the stable Events allocation.
    unsafe { enqueue(userdata, || Ok(Event::Progress(report))) }
}

unsafe extern "C" fn on_program_status(
    _: sys::GhosttyTerminal,
    userdata: *mut c_void,
    report: *const sys::GhosttyTerminalProgramStatus,
) {
    // SAFETY: strings are copied synchronously inside the callback; unknown
    // future enum values are skipped rather than treated as a report.
    let Some(report) = (unsafe { report.as_ref() }) else {
        return;
    };
    let Some(state) = program_status_state(report.state) else {
        return;
    };
    let value = ProgramStatusReport {
        state,
        kind: program_status_kind(report.kind),
        percent: percent(report.progress),
        id: unsafe { copy_string(report.id, 512) },
        app: unsafe { copy_string(report.app, 128) },
        title: unsafe { copy_string(report.title, 640) },
        message: unsafe { copy_string(report.message, 4096) },
    };
    // SAFETY: registered with the stable Events allocation.
    unsafe { enqueue(userdata, || Ok(Event::ProgramStatus(value))) }
}

unsafe extern "C" fn on_semantic_prompt(
    terminal: sys::GhosttyTerminal,
    userdata: *mut c_void,
    event: *const sys::GhosttyTerminalSemanticPrompt,
) {
    // SAFETY: the event and its strings are callback-lifetime only.
    let Some(event) = (unsafe { event.as_ref() }) else {
        return;
    };
    let Some(kind) = semantic_prompt_kind(event.kind) else {
        return;
    };
    let cursor_at_prompt = unsafe {
        terminal_bool(terminal, sys::GHOSTTY_TERMINAL_DATA_CURSOR_AT_PROMPT).unwrap_or(false)
    };
    let value = SemanticPromptReport {
        kind,
        prompt_kind: semantic_prompt_prompt_kind(event.prompt_kind),
        has_exit_code: event.has_exit_code,
        exit_code: event.exit_code,
        command: unsafe { copy_string(event.command, 4096) },
        error: unsafe { copy_string(event.error, 1024) },
        cursor_at_prompt,
    };
    // SAFETY: registered with the stable Events allocation.
    unsafe { enqueue(userdata, || Ok(Event::SemanticPrompt(value))) }
}

unsafe extern "C" fn on_bell(_: sys::GhosttyTerminal, userdata: *mut c_void) {
    // SAFETY: registered with the stable Events allocation.
    unsafe { enqueue(userdata, || Ok(Event::Bell)) }
}
unsafe extern "C" fn on_title(terminal: sys::GhosttyTerminal, userdata: *mut c_void) {
    // SAFETY: title reads are allowed inside this callback; copy before returning.
    unsafe {
        enqueue(userdata, || {
            terminal_string(terminal, sys::GHOSTTY_TERMINAL_DATA_TITLE).map(Event::TitleChanged)
        })
    }
}
unsafe extern "C" fn on_pwd(terminal: sys::GhosttyTerminal, userdata: *mut c_void) {
    // SAFETY: PWD reads are allowed inside this callback; copy before returning.
    unsafe {
        enqueue(userdata, || {
            terminal_string(terminal, sys::GHOSTTY_TERMINAL_DATA_PWD)
                .map(Event::WorkingDirectoryChanged)
        })
    }
}
unsafe extern "C" fn on_reset(_: sys::GhosttyTerminal, userdata: *mut c_void) {
    // SAFETY: registered with the stable Events allocation.
    unsafe { enqueue(userdata, || Ok(Event::Reset)) }
}

unsafe extern "C" fn on_hold(terminal: sys::GhosttyTerminal, userdata: *mut c_void, held: bool) {
    // SAFETY: this callback is synchronous. It only reads native state and owns
    // the resulting snapshot; it never re-enters the parser or retains C refs.
    let events = unsafe { &mut *userdata.cast::<Events>() };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<()> {
        events.held = if held {
            events.hold_generation = events.hold_generation.wrapping_add(1);
            let mut total_rows = 0usize;
            check(unsafe {
                sys::ghostty_terminal_get(
                    terminal,
                    sys::GHOSTTY_TERMINAL_DATA_TOTAL_ROWS,
                    (&mut total_rows as *mut usize).cast(),
                )
            })?;
            let mut screen = sys::GHOSTTY_TERMINAL_SCREEN_PRIMARY;
            // SAFETY: freeze screen identity at the same boundary as the frame.
            check(unsafe {
                sys::ghostty_terminal_get(
                    terminal,
                    sys::GHOSTTY_TERMINAL_DATA_ACTIVE_SCREEN,
                    (&mut screen as *mut sys::GhosttyTerminalScreen).cast(),
                )
            })?;
            Some(RenderHold {
                frame: std::sync::Arc::new(unsafe { render::capture_raw(terminal) }?),
                total_rows,
                alternate_screen: screen == sys::GHOSTTY_TERMINAL_SCREEN_ALTERNATE,
                since: std::time::Instant::now(),
            })
        } else {
            None
        };
        Ok(())
    }));
    if !matches!(result, Ok(Ok(()))) {
        events.failed = true;
    }
}

unsafe extern "C" fn on_size(
    terminal: sys::GhosttyTerminal,
    _: *mut c_void,
    size: *mut sys::GhosttySizeReportSize,
) -> bool {
    // SAFETY: callback supplies writable output; each key has the matching type.
    unsafe {
        if size.is_null() {
            return false;
        }
        let mut cols = 0u16;
        let mut rows = 0u16;
        if sys::ghostty_terminal_get(
            terminal,
            sys::GHOSTTY_TERMINAL_DATA_COLS,
            (&mut cols as *mut u16).cast(),
        ) != sys::GHOSTTY_SUCCESS
            || sys::ghostty_terminal_get(
                terminal,
                sys::GHOSTTY_TERMINAL_DATA_ROWS,
                (&mut rows as *mut u16).cast(),
            ) != sys::GHOSTTY_SUCCESS
        {
            return false;
        }
        // NSH currently measures points, not device pixels: report unknown.
        *size = sys::GhosttySizeReportSize {
            columns: cols,
            rows,
            cell_width: 0,
            cell_height: 0,
        };
    }
    true
}

/// Internal two-pass buffer protocol. Only use for APIs whose size query and
/// retry are documented to be side-effect free (not the stateful mouse encoder).
fn buffer(
    mut call: impl FnMut(*mut u8, usize, *mut usize) -> sys::GhosttyResult,
) -> Result<Vec<u8>> {
    let mut needed = 0;
    let code = call(ptr::null_mut(), 0, &mut needed);
    if code != sys::GHOSTTY_OUT_OF_SPACE {
        check(code)?;
    }
    let mut bytes = vec![0; needed];
    let mut written = 0;
    check(call(bytes.as_mut_ptr(), bytes.len(), &mut written))?;
    if written > bytes.len() {
        return Err(Error(sys::GHOSTTY_INVALID_VALUE));
    }
    bytes.truncate(written);
    Ok(bytes)
}
