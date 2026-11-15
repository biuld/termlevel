//! Per-session display controller: viewport policy, invalidation and one capture
//! per scheduling round. Input never waits on this layer; it only asks the next
//! published window to follow output.
use super::{scroll::SelectionDrag, window::Window};
use crate::{
    sessions::Session,
    terminal::{WindowCapture, WindowSnap},
};
use nsh_core::prelude::*;
use std::{cell::RefCell, rc::Rc, sync::Arc, time::Instant};

/// Invitations from event handlers. Every field is engine-thread only.
#[derive(Default)]
struct Demand {
    capture: bool,
    follow: bool,
    /// The pending origin came from an explicit user action, not output.
    user: bool,
    /// Latest pointer cell; only the newest value matters for hover.
    hover: Option<(u16, usize)>,
    /// Latest host-selection endpoint and rectangle flag.
    drag: Option<(u16, usize, bool)>,
}

impl Demand {
    fn request_capture(&mut self) {
        self.capture = true;
    }

    fn request_follow(&mut self) {
        self.follow = true;
    }

    /// Pinning is a display policy, not an input action: it only changes the
    /// window when the user had scrolled away, and it is applied before the
    /// single capture of the round.
    fn apply_follow(&mut self, window: &mut Window) {
        if !std::mem::take(&mut self.follow) || window.pinned {
            return;
        }
        window.pin();
        self.capture = true;
    }

    /// Takes one capture request without losing a held frame: callers put it
    /// back when synchronized output is still in progress.
    fn take_capture(&mut self) -> Option<bool> {
        if !self.capture {
            return None;
        }
        self.capture = false;
        Some(std::mem::take(&mut self.user))
    }
}

/// Diagnostic counters, printed in debug builds every two seconds like the
/// native renderer's own line. They distinguish sampling, lock contention,
/// engine work and input latency so a smooth-looking FPS counter cannot hide
/// where a stall came from.
#[derive(Clone, Copy, Default, Debug)]
#[allow(dead_code)] // Read only by the debug summary and the release build has none.
pub(super) struct Stats {
    ticks: u64,
    revisions: u64,
    attempted: u64,
    held: u64,
    cached: u64,
    fresh: u64,
    published: u64,
    wheel_report: u64,
    wheel_history: u64,
    held_run_max: u64,
    hold_ns_max: u64,
    wait_ns: u64,
    wait_max_ns: u64,
    begin_ns: u64,
    begin_max_ns: u64,
    capture_ns: u64,
    capture_max_ns: u64,
    tick_ns: u64,
    tick_max_ns: u64,
    gap_max_ns: u64,
}

fn ms(total_ns: u64, count: u64) -> f64 {
    if count == 0 {
        0.0
    } else {
        total_ns as f64 / count as f64 / 1_000_000.0
    }
}
fn max_ms(ns: u64) -> f64 {
    ns as f64 / 1_000_000.0
}

impl Stats {
    fn report(&mut self, seconds: f64, input: (u64, u64, u64), modes: &str, label: &str) {
        let n = self.attempted.max(1);
        let line = format!(
            "[{}] terminal surface {seconds:.0}s: ticks={} rev={} capture={} fresh={} cached={} held={} publish={} | \
             wheel report={} history={} | hold ticks max={} ms max={:.1} | \
             lock wait mean {:.2} max {:.2} ms | begin mean {:.2} max {:.2} ms | \
             capture mean {:.2} max {:.2} ms | tick mean {:.2} max {:.2} ms | gap max {:.2} ms | \
             input n={} wait max {:.2} ms total max {:.2} ms | {}",
            label,
            self.ticks,
            self.revisions,
            self.attempted,
            self.fresh,
            self.cached,
            self.held,
            self.published,
            self.wheel_report,
            self.wheel_history,
            self.held_run_max,
            max_ms(self.hold_ns_max),
            ms(self.wait_ns, n),
            max_ms(self.wait_max_ns),
            ms(self.begin_ns, n),
            max_ms(self.begin_max_ns),
            ms(self.capture_ns, n),
            max_ms(self.capture_max_ns),
            ms(self.tick_ns, self.ticks.max(1)),
            max_ms(self.tick_max_ns),
            max_ms(self.gap_max_ns),
            input.0,
            max_ms(input.1),
            max_ms(input.2),
            modes,
        );
        eprintln!("{line}");
        super::diag::line(&line);
        *self = Stats::default();
    }
}

struct Inner {
    window: Window,
    demand: Demand,
    last_error: Option<String>,
    stats: Stats,
    last_revision: u64,
    last_tick: Option<Instant>,
    report: Option<Instant>,
    held_since: Option<Instant>,
    held_run: u64,
    selection_drag: Option<SelectionDrag>,
}

/// A display handle to one independently owned session. Cloning shares the
/// engine-thread state; it never clones the terminal.
#[derive(Clone)]
pub(super) struct SurfaceController {
    session: Arc<Session>,
    snap: Signal<Arc<WindowSnap>>,
    label: String,
    inner: Rc<RefCell<Inner>>,
}

pub(super) struct Tick {
    /// The published window changed in a way that invalidates search and hover.
    pub changed: bool,
    /// `Some(value)` when a queued pointer position resolved this round.
    pub hover: Option<Option<String>>,
}

impl SurfaceController {
    pub(super) fn new(session: Arc<Session>) -> Result<Self, String> {
        let info = session.info();
        let label = info.id.rsplit('-').next().unwrap_or(&info.id).to_owned();
        let mut window = Window::default();
        let capture = session
            .emulator
            .lock()
            .unwrap()
            .begin_window(&mut window, false, None)?;
        Ok(Self {
            session,
            snap: signal(capture.finish()),
            label,
            inner: Rc::new(RefCell::new(Inner {
                window,
                demand: Demand::default(),
                last_error: None,
                stats: Stats::default(),
                last_revision: 0,
                last_tick: None,
                report: None,
                held_since: None,
                held_run: 0,
                selection_drag: None,
            })),
        })
    }

    pub(super) fn snap(&self) -> Signal<Arc<WindowSnap>> {
        self.snap
    }

    pub(super) fn request_capture(&self) {
        self.inner.borrow_mut().demand.request_capture();
    }

    pub(super) fn request_follow(&self) {
        self.inner.borrow_mut().demand.request_follow();
    }

    /// Debug accounting: was this wheel event delivered to the program or
    /// handled as terminal history? With no wheel count at all, the pointer
    /// never reached the grid.
    pub(super) fn note_wheel(&self, reported: bool) {
        let mut inner = self.inner.borrow_mut();
        if reported {
            inner.stats.wheel_report += 1;
        } else {
            inner.stats.wheel_history += 1;
        }
    }

    /// Latest pointer position in window cells. Resolved once per round.
    pub(super) fn queue_hover(&self, at: Option<(u16, usize)>) {
        self.inner.borrow_mut().demand.hover = at;
    }

    /// Latest drag endpoint in window cells. Applying only the newest endpoint
    /// is equivalent to applying every intermediate one because the selection
    /// start is owned by the terminal and only the end is replaced.
    pub(super) fn queue_drag(&self, at: Option<(u16, usize, bool)>) {
        let mut inner = self.inner.borrow_mut();
        inner.demand.drag = at;
        if at.is_none() {
            inner.selection_drag = None;
        }
    }

    pub(super) fn selection_pointer(&self, x: f32, y: f32, cw: f32, lh: f32, rectangle: bool) {
        let mut inner = self.inner.borrow_mut();
        let (col, row) = inner.window.cell(x, y, cw, lh);
        let mut drag =
            SelectionDrag::new(col, row, rectangle, y, inner.window.rows as f32 * lh, lh);
        if let Some(previous) = inner.selection_drag {
            drag.inherit_fraction(previous);
        }
        inner.selection_drag = Some(drag);
        inner.demand.drag = Some((col, row, rectangle));
    }
    pub(super) fn local_hit(&self, x: f32, y: f32, cw: f32, lh: f32) -> (u16, usize) {
        let inner = self.inner.borrow();
        inner.window.cell(x, y, cw, lh)
    }
    pub(super) fn hit(&self, x: f32, y: f32, cw: f32, lh: f32) -> (u16, usize) {
        self.inner.borrow().window.hit(x, y, cw, lh)
    }

    /// Search reveal and other explicit window changes. Held frames stay frozen.
    pub(super) fn reveal(&self, origin: usize) {
        if self.session.emulator.lock().unwrap().render_held() {
            return;
        }
        let mut inner = self.inner.borrow_mut();
        inner.window.follow_user(origin);
        inner.window.wheel_frac = 0.0;
        inner.demand.request_capture();
        inner.demand.user = true;
    }

    /// History scrolling is a logical window change; sampling is deferred to
    /// the round. `held` comes from the same lock that decided not to report.
    pub(super) fn wheel_history(&self, dy: f32, held: bool) {
        if held {
            return;
        }
        let mut inner = self.inner.borrow_mut();
        if inner.window.wheel(dy) {
            inner.demand.request_capture();
            inner.demand.user = true;
        }
    }

    /// The only place that samples the terminal for display. Pointer updates,
    /// follow requests and window changes are all folded into at most one
    /// capture and publish per call.
    pub(super) fn tick(&self) -> Tick {
        let started = Instant::now();
        let revision = self
            .session
            .revision
            .load(std::sync::atomic::Ordering::Acquire);
        let selecting = self.inner.borrow().selection_drag.is_some();
        // Keep the hold decision and selection mutation under the same lock;
        // a PTY thread must not start a synchronized transaction between them.
        let mut selection_emulator = selecting.then(|| self.session.emulator.lock().unwrap());
        let held = selection_emulator
            .as_ref()
            .is_some_and(|emulator| emulator.render_held());
        let (hover, drag, origin) = {
            let mut inner = self.inner.borrow_mut();
            let inner = &mut *inner;
            inner.stats.ticks += 1;
            if inner.last_revision != revision {
                inner.last_revision = revision;
                inner.stats.revisions += 1;
            }
            if let Some(last) = inner.last_tick {
                let gap = started.duration_since(last).as_nanos() as u64;
                inner.stats.gap_max_ns = inner.stats.gap_max_ns.max(gap);
            }
            let seconds = inner
                .last_tick
                .map_or(0.0, |last| started.duration_since(last).as_secs_f32());
            inner.last_tick = Some(started);
            if !held && let Some(mut drag) = inner.selection_drag {
                let rows = drag.advance(seconds);
                let moved = rows != 0.0 && inner.window.wheel(rows);
                if moved || inner.demand.user {
                    inner.demand.drag = Some((drag.col, drag.row, drag.rectangle));
                    inner.demand.request_capture();
                    inner.demand.user = true;
                }
                inner.selection_drag = Some(drag);
            }
            let window = &mut inner.window;
            let demand = &mut inner.demand;
            demand.apply_follow(window);
            (demand.hover.take(), demand.drag.take(), window.origin)
        };
        let mut hover_uri = None;
        if hover.is_some() || drag.is_some() {
            let mut emulator = selection_emulator
                .take()
                .unwrap_or_else(|| self.session.emulator.lock().unwrap());
            if emulator.render_held() {
                // Frozen frames ignore pointer work, matching the per-event
                // path: a stale tooltip is cleared and the drag is dropped.
                if hover.is_some() {
                    hover_uri = Some(None);
                }
            } else {
                if let Some((col, row)) = hover {
                    hover_uri = Some(emulator.hyperlink(col, origin.saturating_add(row)));
                }
                if let Some((col, row, rectangle)) = drag {
                    emulator.extend_selection((col, origin.saturating_add(row)), rectangle);
                    self.inner.borrow_mut().demand.request_capture();
                }
            }
        }
        drop(selection_emulator);
        let changed = self.capture();
        #[cfg(debug_assertions)]
        {
            let elapsed = started.elapsed();
            let input = self.session.input_probe();
            let (pinned, origin, count) = {
                let inner = self.inner.borrow();
                (inner.window.pinned, inner.window.origin, inner.window.count)
            };
            let modes = match self.session.emulator.try_lock() {
                Ok(emulator) => {
                    let (bits, alt) = emulator.mode_snapshot();
                    format!(
                        "modes={bits:#x} alt={alt} pinned={pinned} origin={origin} count={count}"
                    )
                }
                Err(_) => format!("modes=busy pinned={pinned} origin={origin} count={count}"),
            };
            let mut inner = self.inner.borrow_mut();
            inner.stats.tick_ns += elapsed.as_nanos() as u64;
            inner.stats.tick_max_ns = inner.stats.tick_max_ns.max(elapsed.as_nanos() as u64);
            let due = *inner.report.get_or_insert(started);
            let window = started.duration_since(due).as_secs_f64();
            if window >= 2.0 {
                inner.report = Some(started);
                inner.stats.report(window, input, &modes, &self.label);
            }
        }
        #[cfg(not(debug_assertions))]
        let _ = started;
        Tick {
            changed,
            hover: hover_uri,
        }
    }
    fn capture(&self) -> bool {
        if !self.inner.borrow().demand.capture {
            return false;
        }
        let started = Instant::now();
        let previous = self.snap.get_untracked();
        let (captured, waited, begin, held) = {
            let mut inner = self.inner.borrow_mut();
            let lock_at = Instant::now();
            let emulator = self.session.emulator.lock().unwrap();
            let waited = lock_at.elapsed();
            let held = emulator.render_held();
            if held {
                // An active viewport may advance to the next transaction's
                // owned boundary frame even while the live terminal is held.
                let now = Instant::now();
                inner.held_since.get_or_insert(now);
            }
            let Some(user) = inner.demand.take_capture() else {
                return false;
            };
            inner.stats.attempted += 1;
            let begin_at = Instant::now();
            let captured = emulator.begin_window(&mut inner.window, user, Some(&previous));
            if held {
                // Preserve demand through this transaction's eventual release.
                inner.demand.request_capture();
                inner.demand.user |= user;
                if matches!(&captured, Ok(WindowCapture::Cached(_))) {
                    inner.held_run += 1;
                    inner.stats.held_run_max = inner.stats.held_run_max.max(inner.held_run);
                    inner.stats.held += 1;
                }
            }
            (captured, waited, begin_at.elapsed(), held)
        };
        let captured = captured.map(WindowCapture::finish);
        {
            let total = started.elapsed();
            let mut inner = self.inner.borrow_mut();
            inner.stats.wait_ns += waited.as_nanos() as u64;
            inner.stats.wait_max_ns = inner.stats.wait_max_ns.max(waited.as_nanos() as u64);
            inner.stats.begin_ns += begin.as_nanos() as u64;
            inner.stats.begin_max_ns = inner.stats.begin_max_ns.max(begin.as_nanos() as u64);
            inner.stats.capture_ns += total.as_nanos() as u64;
            inner.stats.capture_max_ns = inner.stats.capture_max_ns.max(total.as_nanos() as u64);
            if !held || matches!(&captured, Ok(next) if !Arc::ptr_eq(next, &previous)) {
                if let Some(since) = inner.held_since.take() {
                    let held = Instant::now().duration_since(since).as_nanos() as u64;
                    inner.stats.hold_ns_max = inner.stats.hold_ns_max.max(held);
                }
                inner.held_run = 0;
            }
        }
        match captured {
            Ok(next) => {
                let changed =
                    !Arc::ptr_eq(&next.frame, &previous.frame) || next.count != previous.count;
                let cached = Arc::ptr_eq(&next, &previous);
                {
                    let mut inner = self.inner.borrow_mut();
                    if cached {
                        inner.stats.cached += 1;
                    } else {
                        inner.stats.fresh += 1;
                    }
                }
                if !cached {
                    self.snap.set(next);
                    self.inner.borrow_mut().stats.published += 1;
                }
                changed
            }
            Err(error) => {
                let mut inner = self.inner.borrow_mut();
                if inner.last_error.as_ref() != Some(&error) {
                    inner.last_error = Some(error.clone());
                    drop(inner);
                    self.session.fail(format!("Rendering terminal: {error}"));
                }
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(count: usize, rows: usize) -> Window {
        let mut window = Window::default();
        window.follow(count, rows, 80, false, None);
        window
    }

    #[test]
    fn follow_only_captures_when_it_leaves_the_bottom() {
        let mut window = window(100, 3);
        let mut demand = Demand::default();
        demand.request_follow();
        demand.apply_follow(&mut window);
        assert_eq!((window.origin, window.pinned), (97, true));
        assert!(!demand.capture);

        window.wheel(-2.0);
        let mut demand = Demand::default();
        demand.request_follow();
        demand.apply_follow(&mut window);
        assert_eq!((window.origin, window.pinned), (97, true));
        assert!(demand.capture);
        assert_eq!(demand.take_capture(), Some(false));
        assert_eq!(demand.take_capture(), None);
    }

    #[test]
    fn subrow_wheel_waits_and_a_round_captures_once() {
        let mut window = window(100, 3);
        let mut demand = Demand::default();
        assert!(!window.wheel(-0.4));
        assert!(!demand.capture);
        assert!(!window.wheel(-0.4));
        assert!(!demand.capture);
        assert!(window.wheel(-0.4));
        demand.request_capture();
        demand.request_capture();
        assert_eq!(demand.take_capture(), Some(false));
        assert_eq!(demand.take_capture(), None);
        assert_eq!((window.origin, window.pinned), (96, false));
    }

    #[test]
    fn pointer_and_drag_keep_only_the_latest_position() {
        let mut demand = Demand {
            hover: Some((3, 4)),
            drag: Some((5, 6, true)),
            ..Default::default()
        };
        assert_eq!(demand.hover.take(), Some((3, 4)));
        assert_eq!(demand.drag.take(), Some((5, 6, true)));
        assert_eq!(demand.hover.take(), None);
    }

    #[test]
    fn user_origin_is_kept_for_the_round() {
        let mut demand = Demand::default();
        demand.request_capture();
        demand.user = true;
        assert_eq!(demand.take_capture(), Some(true));
        assert_eq!(demand.take_capture(), None);
    }
}
