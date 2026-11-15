//! Geometry and time-based movement shared by terminal history interactions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Scrollbar {
    pub top: f32,
    pub thumb: f32,
    pub travel: f32,
    pub maximum: usize,
}
impl Scrollbar {
    pub fn new(count: usize, rows: usize, origin: usize, height: f32) -> Self {
        let maximum = count.saturating_sub(rows);
        let thumb = (height * rows as f32 / count.max(1) as f32)
            .max((height / rows.max(1) as f32 * 2.0).min(height))
            .min(height);
        let travel = (height - thumb).max(0.0);
        Self {
            top: travel * origin.min(maximum) as f32 / maximum.max(1) as f32,
            thumb,
            travel,
            maximum,
        }
    }
    pub fn grab(&self, y: f32) -> f32 {
        if y >= self.top && y <= self.top + self.thumb {
            ((y - self.top) / self.thumb.max(1.0)).clamp(0.0, 1.0)
        } else {
            0.5
        }
    }
    pub fn origin(&self, y: f32, grab: f32) -> usize {
        if self.travel <= 0.0 {
            return 0;
        }
        (((y - grab * self.thumb) / self.travel).clamp(0.0, 1.0) * self.maximum as f32).round()
            as usize
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct SelectionDrag {
    pub col: u16,
    pub row: usize,
    pub rectangle: bool,
    speed: f32,
    remainder: f32,
}
impl SelectionDrag {
    pub fn new(
        col: u16,
        row: usize,
        rectangle: bool,
        y: f32,
        height: f32,
        line_height: f32,
    ) -> Self {
        let edge = line_height.max(1.0) * 0.5;
        let distance = if y < edge {
            y - edge
        } else if y > height - edge {
            y - (height - edge)
        } else {
            0.0
        };
        let speed = if distance == 0.0 {
            0.0
        } else {
            distance.signum() * (8.0 + distance.abs() / line_height.max(1.0) * 12.0).min(120.0)
        };
        Self {
            col,
            row,
            rectangle,
            speed,
            remainder: 0.0,
        }
    }
    pub fn inherit_fraction(&mut self, previous: Self) {
        if self.speed != 0.0 && self.speed.signum() == previous.speed.signum() {
            self.remainder = previous.remainder;
        }
    }
    /// Time is capped after a stall; there is no catch-up jump on resuming.
    pub fn advance(&mut self, seconds: f32) -> f32 {
        let distance = self.remainder + self.speed * seconds.clamp(0.0, 0.05);
        self.remainder = distance.fract();
        distance.trunc()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scrollbar_round_trips_top_middle_and_bottom_and_clamps_outside() {
        for origin in [0, 480, 960] {
            let bar = Scrollbar::new(1000, 40, origin, 800.0);
            assert_eq!(bar.origin(bar.top + 0.3 * bar.thumb, 0.3), origin);
        }
        let bar = Scrollbar::new(1000, 40, 0, 800.0);
        assert_eq!(bar.origin(-900.0, 0.5), 0);
        assert_eq!(bar.origin(9000.0, 0.5), 960);
        let bar = Scrollbar::new(10, 40, 0, 100.0);
        assert_eq!(bar.origin(90.0, 0.0), 0);
    }
    #[test]
    fn stationary_edge_drag_keeps_scrolling_and_stops_inside_or_at_reversal() {
        let mut drag = SelectionDrag::new(2, 0, false, -40.0, 400.0, 20.0);
        let total: f32 = (0..60).map(|_| drag.advance(1.0 / 60.0)).sum();
        assert!(total < -20.0);
        let mut center = SelectionDrag::new(2, 10, false, 200.0, 400.0, 20.0);
        assert_eq!(center.advance(1.0), 0.0);
        let mut below = SelectionDrag::new(2, 19, false, 500.0, 400.0, 20.0);
        assert!(below.advance(1.0) > 0.0);
        assert!(below.advance(1.0) <= 6.0);
    }
    #[test]
    fn continuous_motion_keeps_fractional_speed_and_reversal_resets_it() {
        let mut drag = SelectionDrag::new(0, 0, false, -1.0, 400.0, 20.0);
        let mut rows = 0.0;
        for i in 0..60 {
            let mut next = SelectionDrag::new(0, 0, false, -1.0 - (i % 2) as f32, 400.0, 20.0);
            next.inherit_fraction(drag);
            rows += next.advance(1.0 / 60.0);
            drag = next;
        }
        assert!(rows < -10.0);
        let mut down = SelectionDrag::new(0, 19, false, 401.0, 400.0, 20.0);
        down.inherit_fraction(drag);
        assert_eq!(down.remainder, 0.0);
    }
}
