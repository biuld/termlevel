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

/// Columns the field editor occupies. The measured text gets one extra cell
/// for the caret, and the result stays inside the columns left on this row.
pub(super) fn field_columns(text_px: f32, cell_px: f32, cursor_col: u16, grid_cols: u16) -> u16 {
    let cell = cell_px.max(1.0);
    let used = if text_px <= 0.0 {
        0
    } else {
        (text_px / cell).ceil().min(u16::MAX as f32) as u16
    };
    let want = used.saturating_add(1).max(1);
    let remain = grid_cols.saturating_sub(cursor_col).max(1);
    want.min(remain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_covers_the_composition_and_one_caret_cell() {
        assert_eq!(field_columns(11.0 * 8.0, 8.0, 4, 80), 12);
        assert_eq!(field_columns(16.0, 8.0, 0, 80), 3);
        assert_eq!(field_columns(0.0, 8.0, 10, 80), 1);
        assert_eq!(field_columns(8.1, 8.0, 0, 80), 3);
        assert_eq!(field_columns(11.0 * 8.0, 8.0, 75, 80), 5);
        assert_eq!(field_columns(88.0, 8.0, 79, 80), 1);
    }

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
