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

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn locale_is_utf8(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    value == "utf-8" || value == "utf8" || value.ends_with(".utf-8") || value.ends_with(".utf8")
}

/// Locale assignments for a new shell. The effective charset is `LC_ALL`,
/// then `LC_CTYPE`, then `LANG`. An existing UTF-8 locale is kept.
fn utf8_locale_env(
    lc_all: Option<String>,
    lc_ctype: Option<String>,
    lang: Option<String>,
) -> Vec<(&'static str, &'static str)> {
    let effective = lc_all
        .as_deref()
        .or(lc_ctype.as_deref())
        .or(lang.as_deref());
    if effective.is_some_and(locale_is_utf8) {
        return Vec::new();
    }
    let mut assigned = Vec::new();
    if lc_all.is_some() {
        assigned.push(("LC_ALL", "en_US.UTF-8"));
    }
    if lc_ctype.is_some() {
        assigned.push(("LC_CTYPE", "en_US.UTF-8"));
    }
    if !lang.as_deref().is_some_and(locale_is_utf8) {
        assigned.push(("LANG", "en_US.UTF-8"));
    }
    assigned
}

use crate::terminal::{BackendEvent, GhosttyEmulator, Rgb};

mod input_queue;
use input_queue::InputQueue;

/// Latest OSC 9;4 snapshot for one terminal.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct ProgressInfo {
    /// set, error, indeterminate, or pause. Remove is represented by absence.
    pub state: String,
    /// 0 through 100 when the program reported one.
    pub percent: Option<u32>,
}

/// Aggregate OSC 7501 state for one terminal. The full record map stays in Rust.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct ActivityInfo {
    /// idle, working, done, blocked, or error.
    pub state: String,
    pub id: String,
    pub version: u64,
    pub unread: bool,
    pub pending_count: u32,
    /// permission, question, or auth for a blocked record.
    pub kind: Option<String>,
    /// Stable program key, such as cargo or terraform.
    pub app: Option<String>,
    /// Short human label for the record.
    pub title: Option<String>,
    /// One line saying what the program is doing or needs.
    pub message: Option<String>,
    /// 0 through 100 when the selected record reported one.
    pub percent: Option<u32>,
}

/// One resolved presentation, shared by the sidebar, tooltip and accessibility.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct TerminalStatusInfo {
    pub state: String,
    pub label: String,
    pub detail: Option<String>,
    pub app: Option<String>,
    pub pending_count: u32,
    pub progress: Option<ResolvedProgress>,
}

#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct ResolvedProgress {
    /// osc9 or osc7501; the two reports need not describe the same task.
    pub source: String,
    pub state: String,
    pub percent: Option<u32>,
    pub indeterminate: bool,
    pub animate: bool,
    pub label: String,
}

/// Low-frequency session metadata. No terminal output crosses this interface.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct SessionInfo {
    /// Stable application identity, independent of PID or view identity.
    pub id: String,
    /// Launch directory. OSC 7 / ConEmu / iTerm2 updates go to `current_cwd`.
    pub cwd: String,
    /// Latest decoded working directory reported by the shell.
    pub current_cwd: Option<String>,
    /// Most recently reported terminal title.
    pub title: String,
    /// running, exited, error, or closed.
    pub state: String,
    /// Exit code, after the process has been reaped.
    pub exit_code: Option<u32>,
    /// Startup or I/O error, when one occurred.
    pub error: Option<String>,
    /// Recoverable input rejection, independent of process failure.
    pub input_warning: Option<String>,
    /// Diagnostic PID only. Never used as the session identity.
    pub pid: Option<u32>,
    /// Latest OSC 9;4 progress, or None after remove/reset/exit.
    pub progress: Option<ProgressInfo>,
    /// Aggregate OSC 7501 state, or None when the program has no records.
    pub activity: Option<ActivityInfo>,
    /// OSC 22 pointer shape requested by the running program.
    pub mouse_shape: String,
    /// Whether OSC 133 says the cursor is on a shell prompt.
    pub at_prompt: bool,
    /// unknown, prompt, input, output, or commandEnd. Never inferred from silence.
    pub shell_phase: String,
    /// Monotonic report sequence, never reset within a session (avoids ABA).
    pub status_revision: u64,
    pub status: TerminalStatusInfo,
}

/// Outcome of the one ordered mouse input path.
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

const STATUS_RECORD_LIMIT: usize = 256;

#[derive(Clone)]
struct StatusRecord {
    state: ProgramStatusState,
    kind: ProgramStatusKind,
    percent: Option<u8>,
    app: String,
    title: String,
    message: String,
    updated: u64,
    version: u64,
    acknowledged: bool,
}

#[derive(Default)]
struct StatusState {
    records: BTreeMap<String, StatusRecord>,
    sequence: u64,
}

/// Program-supplied text is single-line display text, not markup. Strip
/// control and invisible formatting characters before it leaves Rust.
fn clean_status_text(text: &str, limit: usize) -> String {
    text.chars()
        .filter(|ch| !ch.is_control())
        .filter(|ch| {
            !matches!(
                ch,
                '\u{200b}'..='\u{200f}'
                    | '\u{202a}'..='\u{202e}'
                    | '\u{2060}'..='\u{2064}'
                    | '\u{2066}'..='\u{2069}'
                    | '\u{feff}'
            )
        })
        .take(limit)
        .collect()
}

/// Decode OSC 7 URIs and ConEmu/iTerm2 bare paths into an absolute local path.
fn decode_pwd(raw: &str) -> Option<String> {
    if raw.is_empty() || raw.len() > 4096 {
        return None;
    }
    let path = if raw.starts_with("file:") {
        let url = url::Url::parse(raw).ok()?;
        if url.scheme() != "file" {
            return None;
        }
        let host = url.host_str().unwrap_or("");
        if !host.is_empty() && host != "localhost" {
            return None;
        }
        if url.query().is_some() || url.fragment().is_some() {
            return None;
        }
        url.to_file_path().ok()?.to_str()?.to_owned()
    } else {
        if raw.contains("://") {
            return None;
        }
        raw.to_owned()
    };
    if !path.starts_with('/') || path.chars().any(char::is_control) {
        return None;
    }
    Some(path)
}

fn status_rank(state: ProgramStatusState) -> u8 {
    match state {
        ProgramStatusState::Blocked => 4,
        ProgramStatusState::Error => 3,
        ProgramStatusState::Working => 2,
        ProgramStatusState::Done => 1,
        ProgramStatusState::Idle | ProgramStatusState::Clear => 0,
    }
}

fn status_name(state: ProgramStatusState) -> &'static str {
    match state {
        ProgramStatusState::Idle => "idle",
        ProgramStatusState::Working => "working",
        ProgramStatusState::Done => "done",
        ProgramStatusState::Blocked => "blocked",
        ProgramStatusState::Error => "error",
        ProgramStatusState::Clear => "clear",
    }
}

fn status_kind_name(kind: ProgramStatusKind) -> Option<&'static str> {
    match kind {
        ProgramStatusKind::None => None,
        ProgramStatusKind::Permission => Some("permission"),
        ProgramStatusKind::Question => Some("question"),
        ProgramStatusKind::Auth => Some("auth"),
    }
}

fn progress_name(state: ProgressState) -> &'static str {
    match state {
        ProgressState::Remove => "remove",
        ProgressState::Set => "set",
        ProgressState::Error => "error",
        ProgressState::Indeterminate => "indeterminate",
        ProgressState::Pause => "pause",
    }
}

fn effective_app(records: &BTreeMap<String, StatusRecord>, id: &str) -> String {
    if let Some(record) = records.get(id)
        && !record.app.is_empty()
    {
        return record.app.clone();
    }
    let mut current = id;
    while let Some(index) = current.rfind('/') {
        current = &current[..index];
        if let Some(parent) = records.get(current)
            && !parent.app.is_empty()
        {
            return parent.app.clone();
        }
    }
    records
        .get("")
        .map(|record| record.app.clone())
        .unwrap_or_default()
}

fn result_state(state: ProgramStatusState) -> bool {
    matches!(state, ProgramStatusState::Done | ProgramStatusState::Error)
}

fn record_activity(records: &BTreeMap<String, StatusRecord>, id: &str) -> Option<ActivityInfo> {
    let record = records.get(id)?;
    let app = effective_app(records, id);
    Some(ActivityInfo {
        state: status_name(record.state).into(),
        id: id.into(),
        version: record.version,
        unread: !record.acknowledged,
        pending_count: 0,
        kind: status_kind_name(record.kind).map(str::to_owned),
        app: (!app.is_empty()).then_some(app),
        title: (!record.title.is_empty()).then(|| record.title.clone()),
        message: (!record.message.is_empty()).then(|| record.message.clone()),
        percent: record.percent.map(u32::from),
    })
}

fn aggregate(records: &BTreeMap<String, StatusRecord>) -> Option<ActivityInfo> {
    let (id, _) = records
        .iter()
        .filter(|(_, record)| !(result_state(record.state) && record.acknowledged))
        .max_by(|(_, a), (_, b)| {
            status_rank(a.state)
                .cmp(&status_rank(b.state))
                .then(a.updated.cmp(&b.updated))
        })?;
    let mut activity = record_activity(records, id)?;
    activity.pending_count = records
        .iter()
        .filter(|(key, record)| {
            *key != id
                && (record.state == ProgramStatusState::Blocked
                    || (record.state == ProgramStatusState::Error && !record.acknowledged))
        })
        .count() as u32;
    Some(activity)
}

fn blocked_label(kind: Option<&str>) -> &'static str {
    match kind {
        Some("permission") => "Needs approval",
        Some("question") => "Needs answer",
        Some("auth") => "Sign in required",
        _ => "Needs input",
    }
}

fn resolve_status(info: &SessionInfo) -> TerminalStatusInfo {
    let activity = info.activity.as_ref();
    let mut status = TerminalStatusInfo::default();
    if info.state == "error" || info.error.is_some() {
        status.state = "hostError".into();
        status.label = "Terminal error".into();
        status.detail = info.error.clone();
    } else if info.state == "exited" || info.state == "closed" {
        status.state = "exited".into();
        status.label = info
            .exit_code
            .map(|code| format!("Exited (code {code})"))
            .unwrap_or_else(|| "Closed".into());
    } else {
        status.state = activity
            .map(|a| a.state.clone())
            .unwrap_or_else(|| "idle".into());
        status.label = match status.state.as_str() {
            "blocked" => blocked_label(activity.and_then(|a| a.kind.as_deref())),
            "error" => "Task failed",
            "working" => "Working",
            "done" => "Done",
            _ => "",
        }
        .into();
        if let Some(activity) = activity {
            status.detail = activity.message.clone().or_else(|| activity.title.clone());
            status.app = activity.app.clone();
            status.pending_count = activity.pending_count;
        }
        let progress = info.progress.clone().map(|p| ("osc9", p)).or_else(|| {
            activity
                .filter(|a| a.state == "working")
                .and_then(|a| a.percent)
                .map(|percent| {
                    (
                        "osc7501",
                        ProgressInfo {
                            state: "set".into(),
                            percent: Some(percent),
                        },
                    )
                })
        });
        status.progress = progress.map(|(source, progress)| {
            let indeterminate = progress.state == "indeterminate"
                || (progress.state == "set" && progress.percent.is_none());
            let percent = if indeterminate {
                None
            } else {
                progress.percent
            };
            let value = percent
                .map(|v| format!("{v}%"))
                .unwrap_or_else(|| "unknown".into());
            let label = match progress.state.as_str() {
                "pause" => format!("Terminal progress paused · {value}"),
                "error" => format!("Terminal progress error · {value}"),
                _ if indeterminate => "Terminal progress unknown".into(),
                _ => format!("Terminal progress {value}"),
            };
            ResolvedProgress {
                source: source.into(),
                state: progress.state,
                percent,
                indeterminate,
                animate: indeterminate && !matches!(status.state.as_str(), "blocked" | "error"),
                label,
            }
        });
    }
    status
}

impl SessionInfo {
    fn resolved(mut self) -> Self {
        self.status = resolve_status(&self);
        self
    }
}

fn clear_status_subtree(records: &mut BTreeMap<String, StatusRecord>, id: &str) {
    if id.is_empty() {
        records.clear();
        return;
    }
    records.remove(id);
    let prefix = format!("{id}/");
    records.retain(|key, _| !key.starts_with(&prefix));
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

impl Session {
    fn start(
        id: String,
        cwd: String,
        mut command: CommandBuilder,
        colors: (Option<&str>, Option<&str>),
        scrollback: usize,
        identity: Option<&str>,
    ) -> Result<Arc<Self>, String> {
        command.cwd(&cwd);
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        // Grok copies with both its own pbcopy and OSC 52. pbcopy uses the
        // session locale; without a UTF-8 locale it stores MacRoman and the
        // later paste is mojibake even when OSC 52 carried UTF-8.
        for (key, value) in utf8_locale_env(
            env_nonempty("LC_ALL"),
            env_nonempty("LC_CTYPE"),
            env_nonempty("LANG"),
        ) {
            command.env(key, value);
        }
        // Identity is a compatibility hint, not a change of terminal engine.
        // Only known profiles are accepted, including at the native boundary.
        if identity == Some("ghostty") {
            command.env("TERM_PROGRAM", "ghostty");
            command.env_remove("TERM_PROGRAM_VERSION");
        } else {
            command.env("TERM_PROGRAM", "termlevel");
            command.env("TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));
        }
        let emulator = GhosttyEmulator::try_new(TerminalSize::default(), scrollback)?;
        let pair = native_pty_system()
            .openpty(PtySize::default())
            .map_err(|e| e.to_string())?;
        let mut writer = pair.master.take_writer().map_err(|e| e.to_string())?;
        #[cfg(unix)]
        let mut reader = interruptible_reader(&*pair.master)?;
        #[cfg(not(unix))]
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|e| e.to_string())?;
        drop(pair.slave);
        let pid = child.process_id();
        let session = Arc::new(Self {
            emulator: Mutex::new(emulator),
            revision: AtomicU64::new(1),
            bells: AtomicU64::new(0),
            input_epoch: AtomicU64::new(0),
            input_wait_ns: AtomicU64::new(0),
            input_total_ns: AtomicU64::new(0),
            input_count: AtomicU64::new(0),
            input: InputQueue::default(),
            info: Mutex::new(SessionInfo {
                id,
                cwd,
                title: String::new(),
                state: "running".into(),
                exit_code: None,
                error: None,
                input_warning: None,
                pid,
                progress: None,
                activity: None,
                current_cwd: None,
                mouse_shape: "text".into(),
                at_prompt: false,
                shell_phase: "unknown".into(),
                status_revision: 0,
                status: TerminalStatusInfo::default(),
            }),
            status: Mutex::new(StatusState::default()),
            control: Mutex::new(Control {
                master: Some(pair.master),
                child,
            }),
            closed: AtomicBool::new(false),
            workers: Mutex::new(Vec::new()),
        });

        session.set_colors(colors.0, colors.1);
        let read_session = session.clone();
        let read_thread = thread::spawn(move || {
            let mut buffer = [0; 8192];
            while !read_session.closed.load(Ordering::Acquire) {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => read_session.output(&buffer[..count]),
                    Err(e)
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::Interrupted | std::io::ErrorKind::WouldBlock
                        ) =>
                    {
                        read_session.flush_output(false);
                        thread::sleep(Duration::from_millis(4));
                    }
                    // A PTY master commonly returns EIO when the slave closes.
                    Err(e) if cfg!(unix) && e.raw_os_error() == Some(libc::EIO) => break,
                    Err(e) => {
                        read_session.fail(format!("Reading terminal: {e}"));
                        break;
                    }
                }
            }
            if !read_session.closed.load(Ordering::Acquire) {
                read_session.flush_output(true);
            }
        });

        let write_session = session.clone();
        let write_thread = thread::spawn(move || {
            while !write_session.closed.load(Ordering::Acquire) {
                let Some(batch) = write_session.input.next() else {
                    break;
                };
                let mut remaining = batch.bytes.as_slice();
                while !remaining.is_empty() && !write_session.closed.load(Ordering::Acquire) {
                    match writer.write(remaining) {
                        Ok(0) => {
                            write_session.fail("Terminal input closed".into());
                            return;
                        }
                        Ok(count) => remaining = &remaining[count..],
                        Err(e)
                            if matches!(
                                e.kind(),
                                std::io::ErrorKind::Interrupted | std::io::ErrorKind::WouldBlock
                            ) =>
                        {
                            thread::sleep(Duration::from_millis(2))
                        }
                        Err(e) => {
                            write_session.fail(format!("Writing terminal: {e}"));
                            return;
                        }
                    }
                }
                if remaining.is_empty() && write_session.input.delivered(&batch) {
                    write_session.revision.fetch_add(1, Ordering::Release);
                }
            }
        });

        let wait_session = session.clone();
        let wait_thread = thread::spawn(move || {
            let mut closing_since = None;
            loop {
                // Reaping and close serialize on the same lock. In particular,
                // close never signals a PID/group after it has been reaped and
                // possibly reused by an unrelated process.
                let mut control = wait_session.control.lock().unwrap();
                match control.child.try_wait() {
                    Ok(Some(status)) => {
                        let mut programs = wait_session.status.lock().unwrap();
                        programs
                            .records
                            .retain(|_, record| result_state(record.state));
                        let mut info = wait_session.info.lock().unwrap();
                        info.exit_code = Some(status.exit_code());
                        info.progress = None;
                        info.activity = aggregate(&programs.records);
                        info.mouse_shape = "text".into();
                        info.at_prompt = false;
                        info.shell_phase = "unknown".into();
                        if !wait_session.closed.load(Ordering::Acquire) {
                            info.state = "exited".into();
                            if status.exit_code() != 0 {
                                crate::notifications::record(
                                    "app",
                                    "warning",
                                    Some(&info.id),
                                    "Terminal exited",
                                    &format!("Process exited with code {}.", status.exit_code()),
                                );
                            }
                        }
                        wait_session.input.close();
                        wait_session.revision.fetch_add(1, Ordering::Release);
                        break;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        wait_session.fail(format!("Waiting for shell: {e}"));
                        break;
                    }
                }
                if wait_session.closed.load(Ordering::Acquire) {
                    let since = closing_since.get_or_insert_with(Instant::now);
                    if since.elapsed() >= Duration::from_millis(50) {
                        // Give an interactive shell time to HUP its background
                        // jobs before forcing a shell that ignores hangup.
                        #[cfg(unix)]
                        if let Some(pid) = control.child.process_id() {
                            unsafe {
                                libc::killpg(pid as libc::pid_t, libc::SIGKILL);
                            }
                        }
                        let _ = control.child.kill();
                    }
                }
                drop(control);
                thread::sleep(Duration::from_millis(10));
            }
        });
        session
            .workers
            .lock()
            .unwrap()
            .extend([read_thread, write_thread, wait_thread]);
        Ok(session)
    }

    fn output(&self, bytes: &[u8]) {
        let events = {
            let mut emulator = self.emulator.lock().unwrap();
            emulator.feed(bytes);
            emulator.flush_sync(Instant::now(), false);
            emulator.backend_events()
        };
        self.handle_events(events);
        self.revision.fetch_add(1, Ordering::Release);
    }

    fn flush_output(&self, force: bool) {
        let events = {
            let mut emulator = self.emulator.lock().unwrap();
            if !emulator.flush_sync(Instant::now(), force) {
                return;
            }
            emulator.backend_events()
        };
        self.handle_events(events);
        self.revision.fetch_add(1, Ordering::Release);
    }

    pub(crate) fn set_scrollback(&self, lines: usize) {
        let events = {
            let mut emulator = self.emulator.lock().unwrap();
            emulator.set_scrollback(lines);
            emulator.backend_events()
        };
        self.handle_events(events);
        self.revision.fetch_add(1, Ordering::Release);
    }

    fn handle_events(&self, events: Vec<BackendEvent>) {
        for event in events {
            match event {
                BackendEvent::Title(title) => self.info.lock().unwrap().title = title,
                BackendEvent::Reply(bytes) => self.input(&bytes),
                BackendEvent::Progress(report) => self.apply_progress(report),
                BackendEvent::ProgramStatus(report) => self.apply_program_status(report),
                BackendEvent::SemanticPrompt(report) => self.apply_semantic_prompt(report),
                BackendEvent::WorkingDirectoryChanged(raw) => {
                    self.info.lock().unwrap().current_cwd = decode_pwd(&raw);
                }
                BackendEvent::MouseShape(shape) => {
                    self.info.lock().unwrap().mouse_shape = shape.name().into();
                }
                BackendEvent::Reset => {
                    let mut status = self.status.lock().unwrap();
                    status.records.clear();
                    let mut info = self.info.lock().unwrap();
                    info.title.clear();
                    info.progress = None;
                    info.activity = None;
                    info.current_cwd = None;
                    info.mouse_shape = "text".into();
                    info.at_prompt = false;
                    info.shell_phase = "unknown".into();
                }
                BackendEvent::Bell => {
                    self.bells.fetch_add(1, Ordering::Release);
                    let id = self.info.lock().unwrap().id.clone();
                    crate::notifications::record(
                        "terminal",
                        "info",
                        Some(&id),
                        "Terminal bell",
                        "This terminal rang the bell.",
                    );
                }
                BackendEvent::Notification { title, body } => {
                    let id = self.info.lock().unwrap().id.clone();
                    crate::notifications::record(
                        "terminal",
                        "info",
                        Some(&id),
                        if title.is_empty() {
                            "Terminal notification"
                        } else {
                            &title
                        },
                        &body,
                    );
                }
                BackendEvent::Error(error) => self.fail(format!("Ghostty: {error}")),
            }
        }
    }

    fn apply_progress(&self, report: ProgressReport) {
        let next = (report.state != ProgressState::Remove).then(|| ProgressInfo {
            state: progress_name(report.state).into(),
            percent: report.percent.map(u32::from),
        });
        let mut info = self.info.lock().unwrap();
        if info.state == "running" {
            info.progress = next;
        }
    }

    fn apply_program_status(&self, report: ProgramStatusReport) {
        // IDs are opaque protocol keys, not display text. Sanitizing them could
        // alias two records or turn a nonempty clear into a root clear.
        let id = report.id;
        let mut status = self.status.lock().unwrap();
        let mut info = self.info.lock().unwrap();
        // The reader may drain final output after the reaper. Preserve final
        // results, but never resurrect work/progress on a dead session.
        if info.state != "running"
            && !(info.state == "exited"
                && (result_state(report.state) || report.state == ProgramStatusState::Clear))
        {
            return;
        }
        status.sequence += 1;
        let sequence = status.sequence;
        let mut notify = None;
        if report.state == ProgramStatusState::Clear {
            clear_status_subtree(&mut status.records, &id);
        } else {
            let previous = status.records.get(&id);
            let entered = previous.is_none_or(|old| old.state != report.state);
            let version = previous
                .filter(|_| !entered)
                .map(|old| old.version)
                .unwrap_or(sequence);
            let acknowledged = previous.is_some_and(|old| !entered && old.acknowledged);
            status.records.insert(
                id.clone(),
                StatusRecord {
                    state: report.state,
                    kind: report.kind,
                    percent: report.percent,
                    app: clean_status_text(&report.app, 128),
                    title: clean_status_text(&report.title, 640),
                    message: clean_status_text(&report.message, 4096),
                    updated: sequence,
                    version,
                    acknowledged,
                },
            );
            if entered
                && matches!(
                    report.state,
                    ProgramStatusState::Blocked | ProgramStatusState::Error
                )
            {
                notify = record_activity(&status.records, &id);
            }
            while status.records.len() > STATUS_RECORD_LIMIT {
                let oldest = status
                    .records
                    .iter()
                    .min_by_key(|(_, r)| r.updated)
                    .map(|(id, _)| id.clone());
                let Some(oldest) = oldest else { break };
                status.records.remove(&oldest);
            }
        }
        // Keep status -> info publication under the same lock as acknowledge.
        info.activity = aggregate(&status.records);
        info.status_revision = sequence;
        if let Some(activity) = notify {
            let headline = if activity.state == "error" {
                "Task failed"
            } else {
                blocked_label(activity.kind.as_deref())
            };
            let terminal = clean_status_text(&info.title, 160);
            let terminal = if terminal.is_empty() {
                "Terminal"
            } else {
                &terminal
            };
            let body = [
                activity.app.as_deref(),
                activity.message.as_deref().or(activity.title.as_deref()),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · ");
            crate::notifications::record_program(
                &info.id,
                crate::notifications::ProgramNotice {
                    record_id: activity.id.clone(),
                    version: activity.version,
                    kind: activity.kind.clone(),
                    app: activity.app.clone(),
                    title: activity.title.clone(),
                    message: activity.message.clone(),
                },
                if activity.state == "error" {
                    "error"
                } else {
                    "warning"
                },
                &format!("{headline} · {terminal}"),
                &body,
            );
        }
    }

    fn apply_semantic_prompt(&self, report: SemanticPromptReport) {
        let mut status = self.status.lock().unwrap();
        let mut info = self.info.lock().unwrap();
        if info.state != "running" {
            return;
        }
        if report.kind == SemanticPromptKind::PromptStart {
            status
                .records
                .retain(|_, record| result_state(record.state));
        }
        if report.kind == SemanticPromptKind::PromptStart {
            info.activity = aggregate(&status.records);
            info.progress = None;
        }
        info.at_prompt = match report.kind {
            SemanticPromptKind::PromptStart => true,
            SemanticPromptKind::InputStart => report.cursor_at_prompt,
            SemanticPromptKind::OutputStart | SemanticPromptKind::CommandEnd => false,
        };
        info.shell_phase = match report.kind {
            SemanticPromptKind::PromptStart => "prompt",
            SemanticPromptKind::InputStart => "input",
            SemanticPromptKind::OutputStart => "output",
            SemanticPromptKind::CommandEnd => "commandEnd",
        }
        .into();
    }

    pub(crate) fn acknowledge(&self, observed_revision: u64) {
        let mut status = self.status.lock().unwrap();
        // A caller cannot acknowledge a snapshot it has not received.
        if observed_revision > status.sequence {
            return;
        }
        let mut changed = false;
        for record in status.records.values_mut() {
            if record.version <= observed_revision
                && !record.acknowledged
                && (result_state(record.state) || record.state == ProgramStatusState::Blocked)
            {
                record.acknowledged = true;
                changed = true;
            }
        }
        if changed {
            self.info.lock().unwrap().activity = aggregate(&status.records);
            self.revision.fetch_add(1, Ordering::Release);
        }
    }

    pub(crate) fn info(&self) -> SessionInfo {
        let mut info = self.info.lock().unwrap().clone();
        info.input_warning = self.input.warning();
        info.resolved()
    }

    pub(crate) fn set_colors(&self, foreground: Option<&str>, background: Option<&str>) {
        fn rgb(value: &str) -> Option<Rgb> {
            let value = value.strip_prefix('#')?;
            if !value.is_ascii() || (value.len() != 6 && value.len() != 8) {
                return None;
            }
            Some(Rgb {
                r: u8::from_str_radix(&value[0..2], 16).ok()?,
                g: u8::from_str_radix(&value[2..4], 16).ok()?,
                b: u8::from_str_radix(&value[4..6], 16).ok()?,
            })
        }
        let (changed, events) = {
            let mut emulator = self.emulator.lock().unwrap();
            let changed = emulator.set_colors(foreground.and_then(rgb), background.and_then(rgb));
            (changed, emulator.backend_events())
        };
        self.handle_events(events);
        if changed {
            self.revision.fetch_add(1, Ordering::Release);
        }
    }

    pub(crate) fn input(&self, bytes: &[u8]) {
        if self.closed.load(Ordering::Acquire) || self.info.lock().unwrap().exit_code.is_some() {
            return;
        }
        if self.input.push(bytes).is_err() {
            let info = self.info();
            if let Some(warning) = info.input_warning {
                crate::notifications::record(
                    "app",
                    "warning",
                    Some(&info.id),
                    "Input not sent",
                    &warning,
                );
            }
            self.revision.fetch_add(1, Ordering::Release);
        }
    }

    fn encoded_input(&self, encoded: Result<Vec<u8>, String>) {
        match encoded {
            Ok(bytes) if !bytes.is_empty() => self.input(&bytes),
            Ok(_) => {}
            Err(error) => self.fail(format!("Encoding terminal input: {error}")),
        }
    }
    /// Max lock wait and total encode+send time for input since the last call.
    /// Diagnostic only: the counters are reset by the display summary.
    pub(crate) fn input_probe(&self) -> (u64, u64, u64) {
        (
            self.input_count.swap(0, Ordering::Relaxed),
            self.input_wait_ns.swap(0, Ordering::Relaxed),
            self.input_total_ns.swap(0, Ordering::Relaxed),
        )
    }
    fn observe_input(&self, started: Instant, waited: Duration) {
        self.input_count.fetch_add(1, Ordering::Relaxed);
        self.input_wait_ns
            .fetch_max(waited.as_nanos() as u64, Ordering::Relaxed);
        self.input_total_ns
            .fetch_max(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }
    pub(crate) fn text_input(&self, text: &str) {
        let started = Instant::now();
        self.input_epoch.fetch_add(1, Ordering::Release);
        let lock_at = Instant::now();
        let encoded = self.emulator.lock().unwrap().encode_text(text);
        let waited = lock_at.elapsed();
        self.encoded_input(encoded);
        self.observe_input(started, waited);
    }
    pub(crate) fn key_input(&self, event: &termlevel_ghostty_vt::KeyEvent<'_>) {
        let started = Instant::now();
        if event.action != termlevel_ghostty_vt::raw::GHOSTTY_KEY_ACTION_RELEASE {
            self.input_epoch.fetch_add(1, Ordering::Release);
        }
        let lock_at = Instant::now();
        let encoded = self.emulator.lock().unwrap().encode_key(event);
        let waited = lock_at.elapsed();
        self.encoded_input(encoded);
        self.observe_input(started, waited);
    }

    /// Encodes mouse reports only when the terminal's current mode accepts
    /// them. The mode check and the encoding share one lock, so an event can
    /// never be classified under one mode and encoded under another. Every
    /// mouse entrance goes through this single ordered path.
    ///
    /// Returns `NotReporting` with the hold state observed under the same lock,
    /// which lets a history scroll avoid a second lock.
    pub(crate) fn mouse_reports(
        &self,
        shift: bool,
        events: &[(termlevel_ghostty_vt::MouseEvent, usize)],
        cw: u32,
        lh: u32,
    ) -> MouseReport {
        let started = Instant::now();
        let mut bytes = Vec::new();
        let waited;
        {
            let lock_at = Instant::now();
            let mut emulator = self.emulator.lock().unwrap();
            waited = lock_at.elapsed();
            if shift || !emulator.mouse_enabled() {
                return MouseReport::NotReporting {
                    held: emulator.render_held(),
                };
            }
            for (event, count) in events {
                for _ in 0..*count {
                    match emulator.encode_mouse(*event, cw, lh) {
                        Ok(part) => bytes.extend(part),
                        Err(error) => {
                            drop(emulator);
                            self.fail(format!("Encoding terminal input: {error}"));
                            return MouseReport::Encoded;
                        }
                    }
                }
            }
        }
        if !bytes.is_empty() {
            // All mouse entrances ask the display to follow before its next
            // published window. A rejected send stays visible; it is not a
            // delivery guarantee. A realistic event fits one message, so the
            // sequence is never split; only an absurd burst is chunked.
            self.input_epoch.fetch_add(1, Ordering::Release);
            if bytes.len() <= 1024 * 1024 {
                self.input(&bytes);
            } else {
                for chunk in bytes.chunks(1024 * 1024) {
                    self.input(chunk);
                }
            }
        }
        self.observe_input(started, waited);
        MouseReport::Encoded
    }
    pub(crate) fn paste(&self, text: &str) {
        let started = Instant::now();
        self.input_epoch.fetch_add(1, Ordering::Release);
        let lock_at = Instant::now();
        let encoded = self.emulator.lock().unwrap().encode_paste(text);
        let waited = lock_at.elapsed();
        self.encoded_input(encoded);
        self.observe_input(started, waited);
    }
    pub(crate) fn focus_input(&self, focused: bool) {
        let encoded = self.emulator.lock().unwrap().encode_focus(focused);
        self.encoded_input(encoded);
    }

    #[cfg(test)]
    pub(crate) fn resize(&self, size: TerminalSize) {
        self.resize_grid(size, 0, 0);
    }

    pub(crate) fn resize_grid(&self, size: TerminalSize, cw: u32, lh: u32) {
        if self.closed.load(Ordering::Acquire) {
            return;
        }
        // Hold the parser lock across the kernel resize: SIGWINCH may cause
        // immediate output, which must see the matching grid dimensions.
        let mut emulator = self.emulator.lock().unwrap();
        let result = self.control.lock().unwrap().master.as_ref().map(|master| {
            master.resize(PtySize {
                rows: size.rows,
                cols: size.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
        });
        if let Some(Err(error)) = result {
            self.fail(format!("Resizing terminal: {error}"));
            return;
        }
        emulator.resize_cells(size, cw, lh);
        let events = emulator.backend_events();
        drop(emulator);
        self.handle_events(events);
        self.revision.fetch_add(1, Ordering::Release);
    }

    pub(crate) fn fail(&self, error: String) {
        if self.closed.load(Ordering::Acquire) {
            return;
        }
        let mut status = self.status.lock().unwrap();
        status.records.clear();
        let mut info = self.info.lock().unwrap();
        let id = info.id.clone();
        let changed = info.error.as_ref() != Some(&error);
        info.state = "error".into();
        info.error = Some(error.clone());
        info.progress = None;
        info.activity = None;
        info.mouse_shape = "text".into();
        info.at_prompt = false;
        info.shell_phase = "unknown".into();
        drop(info);
        if changed {
            crate::notifications::record("app", "error", Some(&id), "Terminal error", &error);
        }
        self.revision.fetch_add(1, Ordering::Release);
    }

    pub(crate) fn close(&self) {
        if self.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        let mut control = self.control.lock().unwrap();
        let running = matches!(control.child.try_wait(), Ok(None));
        #[cfg(unix)]
        if running {
            let foreground = control
                .master
                .as_ref()
                .and_then(|master| master.process_group_leader());
            let shell = self.info.lock().unwrap().pid.map(|pid| pid as libc::pid_t);
            // Terminate foreground jobs immediately; let the shell's hangup
            // handler clean up background jobs before the reaper's deadline.
            for group in [foreground, shell].into_iter().flatten() {
                unsafe {
                    if group > 1 && group != libc::getpgrp() {
                        libc::killpg(group, libc::SIGHUP);
                        if Some(group) != shell {
                            libc::killpg(group, libc::SIGKILL);
                        }
                    }
                }
            }
        }
        #[cfg(not(unix))]
        if running {
            let _ = control.child.kill();
        }
        self.input.close();
        control.master.take();
        let mut status = self.status.lock().unwrap();
        status.records.clear();
        {
            let mut info = self.info.lock().unwrap();
            info.state = "closed".into();
            info.progress = None;
            info.activity = None;
            info.current_cwd = None;
            info.mouse_shape = "text".into();
            info.at_prompt = false;
            info.shell_phase = "unknown".into();
        }
        self.revision.fetch_add(1, Ordering::Release);
    }
}

// O_NONBLOCK belongs to the shared PTY file description. Both worker loops
// handle WouldBlock and check cancellation, so close never strands a reader
// if a descendant happens to retain its end of the PTY.
#[cfg(unix)]
fn interruptible_reader(master: &dyn MasterPty) -> Result<std::fs::File, String> {
    use std::os::fd::FromRawFd;
    let fd = master
        .as_raw_fd()
        .ok_or("PTY has no native file descriptor")?;
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        if flags < 0 || libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) < 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let copy = libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 0);
        if copy < 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(std::fs::File::from_raw_fd(copy))
    }
}

enum Entry {
    Session(Arc<Session>),
    Failed(Box<SessionInfo>),
}

pub(crate) struct SessionManager {
    entries: BTreeMap<String, Entry>,
    closing: Vec<Weak<Session>>,
    scrollback: usize,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
            closing: Vec::new(),
            scrollback: crate::terminal::SCROLLBACK_LIMIT,
        }
    }
}

impl SessionManager {
    #[cfg(test)]
    fn open(&mut self, cwd: String, command: CommandBuilder) -> SessionInfo {
        self.open_with_colors(cwd, command, (None, None), None)
    }

    fn open_with_colors(
        &mut self,
        cwd: String,
        command: CommandBuilder,
        colors: (Option<&str>, Option<&str>),
        identity: Option<&str>,
    ) -> SessionInfo {
        static SEQUENCE: AtomicU64 = AtomicU64::new(1);
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let id = format!(
            "session-{time:x}-{:x}",
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let scrollback = self.scrollback;
        let entry = match resolve_cwd(&cwd)
            .and_then(|cwd| Session::start(id.clone(), cwd, command, colors, scrollback, identity))
        {
            Ok(session) => Entry::Session(session),
            Err(error) => Entry::Failed(Box::new(SessionInfo {
                id: id.clone(),
                cwd,
                title: String::new(),
                state: "error".into(),
                exit_code: None,
                error: Some(error),
                input_warning: None,
                pid: None,
                progress: None,
                activity: None,
                current_cwd: None,
                mouse_shape: "text".into(),
                at_prompt: false,
                shell_phase: "unknown".into(),
                status_revision: 0,
                status: TerminalStatusInfo::default(),
            })),
        };
        let info = match &entry {
            Entry::Session(session) => session.info(),
            Entry::Failed(info) => info.as_ref().clone().resolved(),
        };
        if let Some(error) = &info.error {
            crate::notifications::record(
                "app",
                "error",
                Some(&id),
                "Terminal could not start",
                error,
            );
        }
        self.entries.insert(id, entry);
        info
    }

    pub(crate) fn get(&self, id: &str) -> Result<Arc<Session>, String> {
        match self.entries.get(id) {
            Some(Entry::Session(session)) => Ok(session.clone()),
            Some(Entry::Failed(info)) => Err(info.error.clone().unwrap_or_default()),
            None => Err("This terminal session is closed or missing".into()),
        }
    }

    pub(crate) fn acknowledge(&self, id: &str, observed_revision: u64) {
        if let Ok(session) = self.get(id) {
            session.acknowledge(observed_revision);
        }
    }

    pub(crate) fn scrollback(&self) -> usize {
        self.scrollback
    }

    pub(crate) fn set_scrollback(&mut self, lines: usize) {
        let lines = crate::terminal::clamp_scrollback(lines);
        self.scrollback = lines;
        for entry in self.entries.values() {
            if let Entry::Session(session) = entry {
                session.set_scrollback(lines);
            }
        }
    }

    fn list(&self) -> Vec<SessionInfo> {
        self.entries
            .values()
            .map(|entry| match entry {
                Entry::Session(session) => session.info(),
                Entry::Failed(info) => info.as_ref().clone().resolved(),
            })
            .collect()
    }

    fn close(&mut self, id: &str) {
        if let Some(Entry::Session(session)) = self.entries.remove(id) {
            session.close();
            self.closing.push(Arc::downgrade(&session));
        }
        self.settled();
    }

    fn shutdown(&mut self) {
        for (_, entry) in std::mem::take(&mut self.entries) {
            if let Entry::Session(session) = entry {
                session.close();
                self.closing.push(Arc::downgrade(&session));
            }
        }
    }

    fn settled(&mut self) -> bool {
        self.closing.retain(|session| {
            session.upgrade().is_some_and(|session| {
                !session
                    .workers
                    .lock()
                    .unwrap()
                    .iter()
                    .all(thread::JoinHandle::is_finished)
            })
        });
        self.closing.is_empty()
    }
}

impl Drop for SessionManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub(crate) fn manager() -> &'static Mutex<SessionManager> {
    static MANAGER: OnceLock<Mutex<SessionManager>> = OnceLock::new();
    MANAGER.get_or_init(Mutex::default)
}

fn resolve_cwd(cwd: &str) -> Result<String, String> {
    let path = if cwd == "~" {
        home_directory()
    } else if let Some(relative) = cwd.strip_prefix("~/") {
        home_directory().join(relative)
    } else {
        PathBuf::from(cwd)
    };
    let path = path
        .canonicalize()
        .map_err(|e| format!("Cannot open directory {}: {e}", path.display()))?;
    if !path.is_dir() {
        return Err(format!("Not a directory: {}", path.display()));
    }
    Ok(path.to_string_lossy().into_owned())
}

pub(crate) fn home_directory() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

fn default_shell() -> PathBuf {
    if let Some(shell) = std::env::var_os("SHELL") {
        let path = PathBuf::from(shell);
        if path.is_absolute() && path.is_file() {
            return path;
        }
    }
    #[cfg(unix)]
    unsafe {
        let mut passwd = std::mem::zeroed::<libc::passwd>();
        let mut result = std::ptr::null_mut();
        let mut buffer = vec![0; 16384];
        if libc::getpwuid_r(
            libc::getuid(),
            &mut passwd,
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut result,
        ) == 0
            && !result.is_null()
            && !passwd.pw_shell.is_null()
        {
            let shell = std::ffi::CStr::from_ptr(passwd.pw_shell).to_string_lossy();
            if Path::new(shell.as_ref()).is_file() {
                return PathBuf::from(shell.as_ref());
            }
        }
    }
    PathBuf::from(if cfg!(windows) { "cmd.exe" } else { "/bin/sh" })
}

mod api;
pub use api::Sessions;

#[cfg(test)]
mod tests;
