//! Rust owns processes and terminal state, independent of any view/JS scope.

use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock, Weak,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use nsh_base::{TerminalEmulator, TerminalSize};
use nsh_core::prelude::*;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use termlevel_ghostty_vt::{
    ProgramStatusKind, ProgramStatusReport, ProgramStatusState, ProgressReport, ProgressState,
    SemanticPromptKind, SemanticPromptReport,
};

use crate::terminal::{BackendEvent, GhosttyEmulator, Rgb};

mod input_queue;
use input_queue::InputQueue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MouseReport {
    /// The terminal is in a mouse mode and the event was encoded.
    Encoded,
    /// Not a program report (Shift or no mouse mode); the caller owns it.
    NotReporting { held: bool },
}

struct Control {
    master: Option<Box<dyn MasterPty + Send>>,
    child: Box<dyn Child + Send + Sync>,
}

pub(crate) struct Session {
    pub(crate) emulator: Mutex<GhosttyEmulator>,
    pub(crate) revision: AtomicU64,
    pub(crate) bells: AtomicU64,
    pub(crate) input_epoch: AtomicU64,
    input_wait_ns: AtomicU64,
    input_total_ns: AtomicU64,
    input_count: AtomicU64,
    input: InputQueue,
    info: Mutex<SessionInfo>,
    status: Mutex<StatusState>,
    control: Mutex<Control>,
    closed: AtomicBool,
    workers: Mutex<Vec<thread::JoinHandle<()>>>,
}

mod events;
mod input;
mod manager;
mod pty;
mod status;

use manager::default_shell;
pub(crate) use manager::{home_directory, manager};
pub use status::SessionInfo;
use status::{
    AgentSource, STATUS_RECORD_LIMIT, StatusRecord, StatusState, aggregate, blocked_label,
    clean_status_text, clear_status_subtree, is_agent_app, progress_name, record_activity,
    result_state,
};
use status::{ProgressInfo, TerminalStatusInfo};
#[cfg(test)]
use {events::decode_pwd, manager::SessionManager, pty::utf8_locale_env};

mod api;
pub use api::Sessions;

#[cfg(test)]
mod tests;
