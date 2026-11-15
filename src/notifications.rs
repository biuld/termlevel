//! Process-local notification history, independent of terminal view lifetime.
use nsh_core::prelude::*;
use std::{
    collections::VecDeque,
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

const HISTORY_LIMIT: usize = 100;

#[derive(Clone, Debug, PartialEq, JsObject)]
pub struct Notice {
    pub id: String,
    pub source: String,
    pub level: String,
    pub session_id: Option<String>,
    pub title: String,
    pub body: String,
    pub created_at: u64,
    pub unread: bool,
    pub count: u32,
}

#[derive(Default)]
struct History {
    entries: VecDeque<Notice>,
    sequence: u64,
    revision: u64,
}

fn bounded(text: &str, limit: usize) -> String {
    text.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .take(limit)
        .collect()
}

impl History {
    fn push(
        &mut self,
        source: &str,
        level: &str,
        session_id: Option<&str>,
        title: &str,
        body: &str,
        now: u64,
    ) {
        let title = bounded(title, 160);
        let body = bounded(body, 4096);
        if title.is_empty() && body.is_empty() {
            return;
        }
        self.revision += 1;
        if let Some(last) = self.entries.front_mut()
            && last.source == source
            && last.level == level
            && last.session_id.as_deref() == session_id
            && last.title == title
            && last.body == body
            && now.saturating_sub(last.created_at) < 2000
        {
            last.count = last.count.saturating_add(1);
            last.created_at = now;
            last.unread = true;
            return;
        }
        self.sequence += 1;
        self.entries.push_front(Notice {
            id: format!("notice-{}", self.sequence),
            source: source.into(),
            level: level.into(),
            session_id: session_id.map(str::to_owned),
            title,
            body,
            created_at: now,
            unread: true,
            count: 1,
        });
        self.entries.truncate(HISTORY_LIMIT);
    }
    fn read_all(&mut self) {
        if self.entries.iter().any(|entry| entry.unread) {
            for entry in &mut self.entries {
                entry.unread = false;
            }
            self.revision += 1;
        }
    }
}

fn history() -> &'static Mutex<History> {
    static HISTORY: OnceLock<Mutex<History>> = OnceLock::new();
    HISTORY.get_or_init(Default::default)
}

pub(crate) fn record(source: &str, level: &str, session_id: Option<&str>, title: &str, body: &str) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    history()
        .lock()
        .unwrap()
        .push(source, level, session_id, title, body, now);
}

/// In-app notification history for this host process.
#[module]
pub struct Notifications;

#[module]
impl Notifications {
    /// Changes whenever history or read state changes.
    pub fn revision() -> u64 {
        history().lock().unwrap().revision
    }
    /// Newest first, at most 100 entries.
    pub fn list() -> Vec<Notice> {
        history().lock().unwrap().entries.iter().cloned().collect()
    }
    /// Mark all current entries read.
    pub fn read_all() {
        history().lock().unwrap().read_all();
    }
    /// Remove history without altering session error state.
    pub fn clear() {
        let mut history = history().lock().unwrap();
        if !history.entries.is_empty() {
            history.entries.clear();
            history.revision += 1;
        }
    }
    /// Remove one entry; its source session is unaffected.
    pub fn dismiss(id: String) {
        let mut history = history().lock().unwrap();
        let before = history.entries.len();
        history.entries.retain(|entry| entry.id != id);
        if history.entries.len() != before {
            history.revision += 1;
        }
    }
    /// App UI errors share the same history as Rust session diagnostics.
    pub fn report(title: String, body: String) {
        record("app", "error", None, &title, &body);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_history_keeps_sources_and_read_state_and_never_reuses_ids() {
        let mut history = History::default();
        for i in 0..105 {
            history.push(
                "terminal",
                "info",
                Some("s1"),
                "Build",
                &i.to_string(),
                i * 3000,
            );
        }
        assert_eq!(history.entries.len(), 100);
        assert_eq!(history.entries.front().unwrap().body, "104");
        assert_eq!(history.entries.back().unwrap().body, "5");
        history.read_all();
        assert!(history.entries.iter().all(|entry| !entry.unread));
        history.push("app", "error", Some("s1"), "Build", "104", 312001);
        assert!(history.entries.front().unwrap().unread);
        assert_eq!(history.entries.front().unwrap().source, "app");
        assert_eq!(
            history.entries.front().unwrap().session_id.as_deref(),
            Some("s1")
        );
        history.entries.clear();
        history.push("app", "warning", None, "Later", "", 400000);
        assert_eq!(history.entries.front().unwrap().id, "notice-107");
    }
    #[test]
    fn repeated_notices_coalesce_but_new_arrivals_become_unread() {
        let mut history = History::default();
        history.push("terminal", "info", Some("s1"), "Done", "中文", 1);
        history.read_all();
        history.push("terminal", "info", Some("s1"), "Done", "中文", 2);
        assert_eq!(history.entries.len(), 1);
        assert_eq!(history.entries[0].count, 2);
        assert!(history.entries[0].unread);
        history.push("terminal", "info", Some("s2"), "Done", "中文", 3);
        assert_eq!(history.entries.len(), 2);
        history.push("terminal", "info", Some("s2"), "Done", "中文", 3000);
        assert_eq!(history.entries.len(), 3);
    }
}
