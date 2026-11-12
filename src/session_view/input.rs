//! Native text editing is transient; only committed text enters the PTY.

#[derive(Default)]
pub(super) struct InputState {
    pub composing: bool,
    pending: Option<String>,
    emptied: Option<String>,
    marked: String,
    composition_base: Option<String>,
}

impl InputState {
    pub fn changed(&mut self, text: &str) -> Option<String> {
        if self.composing {
            if self.composition_base.is_none() {
                self.composition_base =
                    Some(text.strip_suffix(&self.marked).unwrap_or("").to_owned());
            }
            self.pending = Some(text.to_owned());
            return None;
        }
        Some(self.commit(text))
    }

    pub fn composition(&mut self, composing: bool, marked: &str) -> Option<String> {
        if composing && !self.composing {
            self.composition_base = None;
        }
        self.composing = composing;
        self.marked = marked.to_owned();
        if composing {
            return None;
        }
        let base = self.composition_base.take().unwrap_or_default();
        self.pending.take().map(|text| {
            let typed = text.strip_prefix(&base).unwrap_or(&text).to_owned();
            self.emptied = (!text.is_empty()).then_some(text);
            typed
        })
    }

    fn commit(&mut self, text: &str) -> String {
        // AppKit can report another edit before the controlled empty value
        // reaches it. Remove that acknowledged prefix exactly once. Equal
        // consecutive values are independent keystrokes, not duplicates.
        let typed = match self.emptied.take() {
            Some(old) if text != old && text.starts_with(&old) => text[old.len()..].to_owned(),
            _ => text.to_owned(),
        };
        self.emptied = (!text.is_empty()).then(|| text.to_owned());
        typed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composition_only_sends_final_text() {
        let mut s = InputState::default();
        s.composition(true, "zhong");
        assert_eq!(s.changed("zhong"), None);
        assert_eq!(s.changed("中文"), None);
        assert_eq!(s.composition(false, "").as_deref(), Some("中文"));
        assert_eq!(s.composition(false, ""), None);
    }

    #[test]
    fn cancelled_marked_text_is_not_sent() {
        let mut s = InputState::default();
        s.composition(true, "ni");
        s.changed("ni");
        s.changed("");
        assert_eq!(s.composition(false, "").as_deref(), Some(""));
        assert_eq!(s.changed("a").as_deref(), Some("a"));
    }

    #[test]
    fn composition_can_cancel_before_native_clear_and_repeat_the_same_word() {
        let mut s = InputState::default();
        s.changed("a");
        s.composition(true, "ni");
        s.changed("ani");
        s.changed("a");
        assert_eq!(s.composition(false, "").as_deref(), Some(""));
        s.composition(true, "中文");
        s.changed("中文");
        assert_eq!(s.composition(false, "").as_deref(), Some("中文"));
        s.composition(true, "中文");
        s.changed("中文");
        assert_eq!(s.composition(false, "").as_deref(), Some("中文"));
    }

    #[test]
    fn delayed_clear_and_repeated_keys_do_not_drop_or_duplicate_text() {
        let mut s = InputState::default();
        assert_eq!(s.changed("a").as_deref(), Some("a"));
        assert_eq!(s.changed("ab").as_deref(), Some("b"));
        assert_eq!(s.changed("").as_deref(), Some(""));
        assert_eq!(s.changed("b").as_deref(), Some("b"));
        assert_eq!(s.changed("b").as_deref(), Some("b"));
    }
}
