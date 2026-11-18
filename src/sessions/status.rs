use super::*;

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
    /// Last recognized app explicitly reported for the current agent run.
    pub agent_app: Option<String>,
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
pub(super) const STATUS_RECORD_LIMIT: usize = 256;

#[derive(Clone)]
pub(super) struct StatusRecord {
    pub(super) state: ProgramStatusState,
    pub(super) kind: ProgramStatusKind,
    pub(super) percent: Option<u8>,
    pub(super) app: String,
    pub(super) title: String,
    pub(super) message: String,
    pub(super) updated: u64,
    pub(super) version: u64,
    pub(super) acknowledged: bool,
}

#[derive(Default)]
pub(super) struct StatusState {
    pub(super) records: BTreeMap<String, StatusRecord>,
    pub(super) sequence: u64,
    pub(super) agent: Option<AgentSource>,
}

pub(super) struct AgentSource {
    pub(super) id: String,
    pub(super) foreground: Option<i32>,
}

impl AgentSource {
    pub(super) fn is_in_subtree(&self, id: &str) -> bool {
        id.is_empty()
            || self.id == id
            || self
                .id
                .strip_prefix(id)
                .is_some_and(|rest| rest.starts_with('/'))
    }
}

pub(super) fn is_agent_app(app: &str) -> bool {
    matches!(
        app,
        "codex" | "claude-code" | "grok" | "grok-build" | "pi" | "opencode"
    )
}

/// Program-supplied text is single-line display text, not markup. Strip
/// control and invisible formatting characters before it leaves Rust.
pub(super) fn clean_status_text(text: &str, limit: usize) -> String {
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

pub(super) fn progress_name(state: ProgressState) -> &'static str {
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

pub(super) fn result_state(state: ProgramStatusState) -> bool {
    matches!(state, ProgramStatusState::Done | ProgramStatusState::Error)
}

pub(super) fn record_activity(
    records: &BTreeMap<String, StatusRecord>,
    id: &str,
) -> Option<ActivityInfo> {
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

pub(super) fn aggregate(records: &BTreeMap<String, StatusRecord>) -> Option<ActivityInfo> {
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

/// A progress line exists only for a reported 0–100. Omitted, -1, and
/// indeterminate reports carry no measurement, so the working indicator stands alone.
fn measured_progress(progress: &ProgressInfo) -> Option<ProgressInfo> {
    match progress.state.as_str() {
        "set" | "pause" | "error" => {
            progress
                .percent
                .filter(|percent| *percent <= 100)
                .map(|percent| ProgressInfo {
                    state: progress.state.clone(),
                    percent: Some(percent),
                })
        }
        _ => None,
    }
}

pub(super) fn blocked_label(kind: Option<&str>) -> &'static str {
    match kind {
        Some("permission") => "Needs approval",
        Some("question") => "Needs answer",
        Some("auth") => "Sign in required",
        _ => "Needs input",
    }
}

fn resolve_status(info: &SessionInfo) -> TerminalStatusInfo {
    let activity = info.activity.as_ref();
    let mut status = TerminalStatusInfo {
        agent_app: info.status.agent_app.clone(),
        ..Default::default()
    };
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
        let measured = info.progress.as_ref().and_then(measured_progress);
        let progress = measured.map(|progress| ("osc9", progress)).or_else(|| {
            activity
                .filter(|activity| activity.state == "working")
                .and_then(|activity| activity.percent)
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
        status.progress = progress.and_then(|(source, progress)| {
            let percent = progress.percent?;
            let label = match progress.state.as_str() {
                "pause" => format!("Terminal progress paused · {percent}%"),
                "error" => format!("Terminal progress error · {percent}%"),
                _ => format!("Terminal progress {percent}%"),
            };
            Some(ResolvedProgress {
                source: source.into(),
                state: progress.state,
                percent: Some(percent),
                indeterminate: false,
                animate: false,
                label,
            })
        });
    }
    status
}

impl SessionInfo {
    pub(super) fn resolved(mut self) -> Self {
        self.status = resolve_status(&self);
        self
    }
}

pub(super) fn clear_status_subtree(records: &mut BTreeMap<String, StatusRecord>, id: &str) {
    if id.is_empty() {
        records.clear();
        return;
    }
    records.remove(id);
    let prefix = format!("{id}/");
    records.retain(|key, _| !key.starts_with(&prefix));
}
