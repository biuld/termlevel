use super::*;

impl Session {
    pub(super) fn output(&self, bytes: &[u8]) {
        let events = {
            let mut emulator = self.emulator.lock().unwrap();
            emulator.feed(bytes);
            emulator.flush_sync(Instant::now(), false);
            emulator.backend_events()
        };
        self.handle_events(events);
        self.revision.fetch_add(1, Ordering::Release);
    }

    pub(super) fn flush_output(&self, force: bool) {
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

    pub(super) fn handle_events(&self, events: Vec<BackendEvent>) {
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
                    status.agent = None;
                    let mut info = self.info.lock().unwrap();
                    info.title.clear();
                    info.progress = None;
                    info.activity = None;
                    info.status.agent_app = None;
                    info.current_cwd = None;
                    info.mouse_shape = "text".into();
                    info.at_prompt = false;
                    info.shell_phase = "unknown".into();
                }
                BackendEvent::Bell => {
                    self.bells.fetch_add(1, Ordering::Release);
                    let info = self.info.lock().unwrap();
                    crate::notifications::record(
                        "terminal",
                        "info",
                        Some(&info.id),
                        "Terminal bell",
                        "This terminal rang the bell.",
                        &info.title,
                    );
                }
                BackendEvent::Notification { title, body } => {
                    let info = self.info.lock().unwrap();
                    let notice_title = if title.is_empty() {
                        "Terminal notification".to_string()
                    } else {
                        title
                    };
                    crate::notifications::record(
                        "terminal",
                        "info",
                        Some(&info.id),
                        &notice_title,
                        &body,
                        &info.title,
                    );
                }
                BackendEvent::Error(error) => self.fail(format!("Ghostty: {error}")),
            }
        }
    }

    pub(super) fn apply_progress(&self, report: ProgressReport) {
        let next = (report.state != ProgressState::Remove).then(|| ProgressInfo {
            state: progress_name(report.state).into(),
            percent: report.percent.map(u32::from),
        });
        let mut info = self.info.lock().unwrap();
        if info.state == "running" {
            info.progress = next;
        }
    }

    pub(super) fn apply_program_status(&self, report: ProgramStatusReport) {
        // IDs are opaque protocol keys, not display text. Sanitizing them could
        // alias two records or turn a nonempty clear into a root clear.
        let id = report.id;
        let control = self.control.lock().unwrap();
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
            if status
                .agent
                .as_ref()
                .is_some_and(|agent| agent.is_in_subtree(&id))
            {
                status.agent = None;
                info.status.agent_app = None;
            }
        } else {
            if info.state == "running" && is_agent_app(&report.app) {
                status.agent = Some(AgentSource {
                    id: id.clone(),
                    foreground: control
                        .foreground_group()
                        .filter(|group| Some(*group as u32) != info.pid),
                });
                info.status.agent_app = Some(report.app.clone());
            }
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
        drop(control);
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
                terminal,
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
}

pub(super) fn decode_pwd(raw: &str) -> Option<String> {
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
