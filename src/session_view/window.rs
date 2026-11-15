//! Integer viewport policy. Native content anchors are owned by Ghostty.
#[derive(Debug, Clone)]
pub(crate) struct Window {
    pub origin: usize,
    pub pinned: bool,
    pub wheel_frac: f32,
    pub rows: usize,
    pub cols: u16,
    pub count: usize,
    alternate: bool,
    primary: Option<(bool, f32)>,
}
impl Default for Window {
    fn default() -> Self {
        Self {
            origin: 0,
            pinned: true,
            wheel_frac: 0.0,
            rows: 1,
            cols: 1,
            count: 1,
            alternate: false,
            primary: None,
        }
    }
}
pub(crate) fn pinned_origin(count: usize, rows: usize) -> usize {
    count.saturating_sub(rows)
}
impl Window {
    pub fn follow(
        &mut self,
        count: usize,
        rows: usize,
        cols: u16,
        alternate: bool,
        anchor: Option<usize>,
    ) {
        if alternate && !self.alternate {
            self.primary = Some((self.pinned, self.wheel_frac));
            self.pinned = true;
            self.wheel_frac = 0.0;
        } else if !alternate
            && self.alternate
            && let Some((pinned, frac)) = self.primary.take()
        {
            self.pinned = pinned;
            self.wheel_frac = frac;
        }
        self.alternate = alternate;
        self.rows = rows.max(1);
        self.cols = cols.max(1);
        self.count = count;
        let bottom = pinned_origin(count, rows);
        self.origin = if alternate {
            0
        } else if self.pinned {
            bottom
        } else {
            anchor.unwrap_or(0).min(bottom)
        };
        if self.origin == bottom {
            self.pinned = true;
            self.wheel_frac = 0.0;
        }
    }
    pub fn pin(&mut self) {
        self.pinned = true;
        self.origin = if self.alternate {
            0
        } else {
            pinned_origin(self.count, self.rows)
        };
        self.wheel_frac = 0.0;
    }
    pub fn follow_user(&mut self, origin: usize) {
        if self.alternate {
            return;
        }
        let bottom = pinned_origin(self.count, self.rows);
        self.origin = origin.min(bottom);
        self.pinned = self.origin == bottom;
        if self.pinned {
            self.wheel_frac = 0.0;
        }
    }
    pub fn wheel(&mut self, dy: f32) -> bool {
        if self.alternate {
            return false;
        }
        let y = self.wheel_frac + dy;
        self.wheel_frac = y - y.trunc();
        if y.trunc() == 0.0 {
            return false;
        }
        let old = self.origin;
        let next = (self.origin as i64 + y.trunc() as i64)
            .clamp(0, pinned_origin(self.count, self.rows) as i64) as usize;
        self.follow_user(next);
        if next == 0 {
            self.wheel_frac = 0.0;
        }
        self.origin != old
    }
    pub fn hit(&self, x: f32, y: f32, cw: f32, lh: f32) -> (u16, usize) {
        let (col, row) = self.cell(x, y, cw, lh);
        (col, self.origin.saturating_add(row))
    }
    pub fn cell(&self, x: f32, y: f32, cw: f32, lh: f32) -> (u16, usize) {
        let col = (x / cw.max(1.0)).floor().max(0.0) as u16;
        let row = (y / lh.max(1.0)).floor().max(0.0) as usize;
        (
            col.min(self.cols.saturating_sub(1)),
            row.min(self.rows.saturating_sub(1)),
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn whole_frame_excludes_history_and_repin_follows_output() {
        let mut w = Window::default();
        w.follow(100, 3, 80, false, None);
        assert_eq!(w.origin, 97);
        w.wheel(-1.25);
        assert_eq!((w.origin, w.pinned), (96, false));
        w.wheel(1.25);
        assert_eq!((w.origin, w.pinned, w.wheel_frac), (97, true, 0.0));
        w.follow(101, 3, 80, false, None);
        assert_eq!(w.origin, 98);
        assert_eq!(w.hit(9999.0, 9999.0, 8.0, 21.0), (79, 100));
        assert_eq!(w.cell(8.0, 21.0, 8.0, 21.0), (1, 1));
        assert_eq!(w.hit(8.0, 21.0, 8.0, 21.0), (1, 99));
    }
    #[test]
    fn subrow_trackpad_events_accumulate_before_leaving_bottom() {
        let mut w = Window::default();
        w.follow(100, 3, 80, false, None);
        for _ in 0..3 {
            assert!(!w.wheel(-0.25));
        }
        assert_eq!(w.origin, 97);
        assert!(w.wheel(-0.25));
        assert_eq!((w.origin, w.pinned), (96, false));
        w.pin();
        assert_eq!(w.wheel_frac, 0.0);
    }
    #[test]
    fn alternate_preserves_primary_policy_and_tracked_anchor() {
        let mut w = Window::default();
        w.follow(100, 3, 80, false, None);
        w.follow_user(30);
        w.wheel_frac = 0.5;
        w.follow(3, 3, 80, true, None);
        w.wheel(-20.0);
        w.pin();
        w.follow(3, 3, 80, true, None);
        w.follow(110, 3, 80, false, Some(35));
        assert_eq!((w.origin, w.pinned, w.wheel_frac), (35, false, 0.5));
        w.follow(110, 3, 80, false, None);
        assert_eq!(w.origin, 0);
    }
}
