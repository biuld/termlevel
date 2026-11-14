//! A fixed terminal grid. Rust owns snapshots and native drawing; NSH owns layout.
use super::{SessionTerminalProps, input::InputState, keys, paint, window::Window};
use crate::{
    sessions::Session,
    terminal::{GridRun, SearchHit, WindowSnap},
};
use nsh_base::{TerminalColor, TerminalEmulator, TerminalSize, input, theme};
use nsh_core::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone)]
struct Grid {
    session: Arc<Session>,
    window: Rc<RefCell<Window>>,
    snap: Signal<Arc<WindowSnap>>,
    epoch: Rc<Cell<u64>>,
    pending_pin: Rc<Cell<bool>>,
    last_error: Rc<RefCell<Option<String>>>,
}
impl Grid {
    fn capture(&self, user: bool) {
        use std::sync::atomic::Ordering;
        let started = Instant::now();
        let emulator = self.session.emulator.lock().unwrap();
        // Freeze all published geometry and history during synchronized output.
        if emulator.render_held() {
            return;
        }
        let epoch = self.session.input_epoch.load(Ordering::Acquire);
        if self.epoch.replace(epoch) != epoch {
            self.pending_pin.set(true);
        }
        let mut window = self.window.borrow_mut();
        if self.pending_pin.replace(false) {
            window.pin();
        }
        let previous = self.snap.get_untracked();
        let next = emulator.begin_window(&mut window, user, Some(&previous));
        drop(window);
        drop(emulator);
        let captured = started.elapsed();
        let next = next.map(crate::terminal::WindowCapture::finish);
        match next {
            Ok(next) => {
                self.last_error.borrow_mut().take();
                if *next != *previous {
                    self.snap.set(next);
                }
            }
            Err(error) => {
                if self.last_error.borrow().as_ref() != Some(&error) {
                    *self.last_error.borrow_mut() = Some(error.clone());
                    self.session.fail(format!("Rendering terminal: {error}"));
                }
            }
        }
        #[cfg(debug_assertions)]
        if started.elapsed().as_millis() >= 8 {
            eprintln!(
                "terminal snapshot+lock wait: {:?}, unlocked merge+publish: {:?} (excludes native replay)",
                captured,
                started.elapsed().saturating_sub(captured)
            );
        }
        let _ = (started, captured);
    }
    fn pin(&self) {
        self.pending_pin.set(true);
        self.capture(false);
    }
    fn reveal(&self, origin: usize) {
        if self.session.emulator.lock().unwrap().render_held() {
            return;
        }
        self.window.borrow_mut().follow_user(origin);
        self.capture(true);
    }
    fn wheel(&self, dy: f32) {
        if self.session.emulator.lock().unwrap().render_held() {
            return;
        }
        let changed = self.window.borrow_mut().wheel(dy);
        if changed {
            self.capture(true);
        }
    }
}

#[derive(Clone)]
pub(super) struct Display {
    session: Arc<Session>,
    props: SessionTerminalProps,
    pub input: ElementRef,
    cursor_ref: ElementRef,
    pub revision: Signal<u64>,
    focused: Signal<bool>,
    composing: Signal<bool>,
    value: Signal<String>,
    state: Rc<RefCell<InputState>>,
    viewport: Signal<Size<Points>>,
    cell_width: Memo<Points>,
    drag: Rc<Cell<Option<(u16, usize)>>>,
    blink: Signal<bool>,
    synthetic_keys: Signal<bool>,
    wheel_delta: Rc<Cell<(f32, f32)>>,
    hover: Signal<Option<String>>,
    surface: Grid,
    pub search: Search,
}

struct Scan {
    revision: u64,
    next: usize,
    count: usize,
    hits: Vec<SearchHit>,
}
#[derive(Clone)]
pub(super) struct Search {
    query: Rc<RefCell<String>>,
    hits: Rc<RefCell<Vec<SearchHit>>>,
    index: Rc<Cell<usize>>,
    rev: Signal<u64>,
    surface: Grid,
    deadline: Rc<Cell<Option<Instant>>>,
    scan: Rc<RefCell<Option<Scan>>>,
    reveal_pending: Rc<Cell<bool>>,
}
impl Search {
    pub(super) fn watch(&self) {
        self.rev.get();
    }
    pub(super) fn apply(&self, query: &str) -> u32 {
        if query.is_empty() || !query.is_ascii() {
            self.surface.session.emulator.lock().unwrap().clear_search();
        }
        *self.query.borrow_mut() = query.to_owned();
        self.deadline.set(if query.is_empty() {
            None
        } else {
            Some(Instant::now())
        });
        self.scan.borrow_mut().take();
        self.hits.borrow_mut().clear();
        self.index.set(0);
        self.reveal_pending.set(!query.is_empty());
        self.rev.update(|n| *n += 1);
        self.step();
        self.hits.borrow().len() as u32
    }
    pub(super) fn status(&self) -> super::SearchStatus {
        let count = self.hits.borrow().len() as u32;
        super::SearchStatus {
            query: self.query.borrow().clone(),
            count,
            index: if count == 0 {
                0
            } else {
                self.index.get() as u32 + 1
            },
            searching: self.deadline.get().is_some(),
        }
    }
    fn publish(&self, hits: Vec<SearchHit>) {
        self.index
            .set(self.index.get().min(hits.len().saturating_sub(1)));
        *self.hits.borrow_mut() = hits;
        self.deadline.set(None);
        if self.reveal_pending.replace(false) && !self.hits.borrow().is_empty() {
            self.reveal(self.index.get());
        }
        self.rev.update(|n| *n += 1);
    }
    pub(super) fn move_by(&self, delta: i32) -> u32 {
        let len = self.hits.borrow().len();
        if len == 0 {
            return 0;
        }
        let next = (self.index.get() as i32 + delta).rem_euclid(len as i32) as usize;
        self.index.set(next);
        self.reveal(next);
        self.rev.update(|n| *n += 1);
        next as u32 + 1
    }
    pub(super) fn clear(&self) {
        self.apply("");
    }
    fn invalidate(&self) {
        if self.query.borrow().is_empty() {
            return;
        }
        let was_searching = self.deadline.get().is_some();
        self.deadline
            .set(Some(Instant::now() + Duration::from_millis(150)));
        self.scan.borrow_mut().take();
        if !self.hits.borrow().is_empty() || !was_searching {
            self.hits.borrow_mut().clear();
            self.rev.update(|n| *n += 1);
        }
    }
    fn copy_rows(&self, first: usize, end: usize) -> Vec<Vec<GridRun>> {
        let emulator = self.surface.session.emulator.lock().unwrap();
        if emulator.render_held() {
            let snap = self.surface.snap.get_untracked();
            return (first..end)
                .map(|row| {
                    row.checked_sub(snap.origin)
                        .and_then(|i| snap.rows.get(i))
                        .cloned()
                        .unwrap_or_default()
                })
                .collect();
        }
        (first..end).map(|row| emulator.grid_runs(row)).collect()
    }
    fn step(&self) {
        use std::sync::atomic::Ordering;
        let Some(due) = self.deadline.get() else {
            return;
        };
        if Instant::now() < due || self.surface.session.emulator.lock().unwrap().render_held() {
            return;
        }
        let revision = self.surface.session.revision.load(Ordering::Acquire);
        let query = self.query.borrow().clone();
        if query.is_ascii() {
            let next = self
                .surface
                .session
                .emulator
                .lock()
                .unwrap()
                .search_ascii_step(&query);
            match next {
                Ok(Some(hits)) => self.publish(hits),
                Ok(None) => {}
                Err(error) => {
                    self.publish(Vec::new());
                    self.surface
                        .session
                        .fail(format!("Searching terminal: {error}"));
                }
            }
            return;
        }
        let mut pending = self.scan.borrow_mut();
        let scan = pending.get_or_insert_with(|| Scan {
            revision,
            next: 0,
            count: self.surface.session.emulator.lock().unwrap().line_count(),
            hits: Vec::new(),
        });
        if scan.revision != revision {
            pending.take();
            self.deadline
                .set(Some(Instant::now() + Duration::from_millis(150)));
            return;
        }
        let end = scan.count.min(scan.next + 64);
        let rows = self.copy_rows(scan.next, end);
        scan.hits
            .extend(match_rows(&self.query.borrow(), scan.next, &rows));
        scan.next = end;
        if end == scan.count && self.surface.session.revision.load(Ordering::Acquire) == revision {
            let hits = std::mem::take(&mut scan.hits);
            pending.take();
            drop(pending);
            self.publish(hits);
        }
    }
    fn reveal(&self, index: usize) {
        if let Some(hit) = self.hits.borrow().get(index).copied() {
            let rows = self.surface.snap.get_untracked().frame.rows as usize;
            self.surface
                .reveal(hit.line.saturating_sub((rows as f32 * 0.3) as usize));
        }
    }
}
fn match_rows(query: &str, first: usize, rows: &[Vec<GridRun>]) -> Vec<SearchHit> {
    let needle: Vec<_> = query.chars().flat_map(char::to_lowercase).collect();
    if needle.is_empty() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for (row, runs) in rows.iter().enumerate() {
        let mut chars = Vec::new();
        let mut owners = Vec::new();
        for run in runs {
            for (offset, ch) in run.run.text.chars().enumerate() {
                let ascii = run.run.text.is_ascii();
                for lower in ch.to_lowercase() {
                    chars.push(lower);
                    owners.push((
                        run.col + if ascii { offset } else { 0 },
                        if ascii { 1 } else { run.cells },
                    ));
                }
            }
        }
        let mut at = 0;
        while at + needle.len() <= chars.len() {
            if chars[at..at + needle.len()] == needle {
                let (col, _) = owners[at];
                let (last, width) = owners[at + needle.len() - 1];
                hits.push(SearchHit {
                    line: first + row,
                    col,
                    cols: last + width - col,
                });
                at += needle.len();
            } else {
                at += 1;
            }
        }
    }
    hits
}

impl Display {
    pub fn new(session: Arc<Session>, props: SessionTerminalProps) -> Result<Self, String> {
        let cell_width = computed(move || {
            Points(crate::runtime::terminal_cell_width(
                &family(props),
                props.font_size.get().0,
            ))
        });
        let mut window = Window::default();
        let capture = session
            .emulator
            .lock()
            .unwrap()
            .begin_window(&mut window, false, None)?;
        let snap = capture.finish();
        let surface = Grid {
            session: session.clone(),
            window: Rc::new(RefCell::new(window)),
            snap: signal(snap),
            epoch: Rc::new(Cell::new(0)),
            pending_pin: Rc::new(Cell::new(false)),
            last_error: Rc::default(),
        };
        let display = Self {
            session: session.clone(),
            props,
            input: element_ref(),
            cursor_ref: element_ref(),
            revision: signal(0),
            focused: signal(false),
            composing: signal(false),
            value: signal(String::new()),
            state: Rc::default(),
            viewport: signal(Size::default()),
            cell_width,
            drag: Rc::default(),
            blink: signal(true),
            synthetic_keys: signal(synthetic_keys()),
            wheel_delta: Rc::default(),
            hover: signal(None),
            surface: surface.clone(),
            search: Search {
                query: Rc::default(),
                hits: Rc::default(),
                index: Rc::default(),
                rev: signal(0),
                surface,
                deadline: Rc::default(),
                scan: Rc::default(),
                reveal_pending: Rc::default(),
            },
        };
        #[cfg(all(debug_assertions, target_os = "macos"))]
        super::native_probe::track(display.session.info().id, display.input, display.cursor_ref);
        let clock = display.clone();
        let epoch = Cell::new(0);
        let started = Instant::now();
        let reset = Cell::new(started);
        let rev = Cell::new(0);
        set_interval(Duration::from_millis(16), move || {
            use std::sync::atomic::Ordering;
            clock.synthetic_keys.set(synthetic_keys());
            if !clock.props.visible.get_untracked() {
                return;
            }
            let next = clock.session.input_epoch.load(Ordering::Acquire);
            if epoch.replace(next) != next {
                clock.pin();
                reset.set(Instant::now());
            }
            let next = clock.revision.get_untracked();
            if rev.replace(next) != next {
                reset.set(Instant::now());
            }
            let phase = reset.get().elapsed().as_millis() % 1000 < 500;
            if clock.blink.get_untracked() != phase {
                clock.blink.set(phase);
            }
            if clock
                .session
                .emulator
                .lock()
                .unwrap()
                .tick_images(started.elapsed().as_millis() as u64)
            {
                clock.session.selection_changed();
            }
            clock.search.step();
        });
        Ok(display)
    }
    pub(super) fn output_changed(&self) {
        if !self.props.visible.get_untracked() {
            return;
        }
        let before = self.surface.snap.get_untracked();
        self.surface.capture(false);
        let after = self.surface.snap.get_untracked();
        if !Arc::ptr_eq(&before.frame, &after.frame) || before.count != after.count {
            self.search.invalidate();
            self.hover.set(None);
        }
    }
    fn at_end(&self) -> bool {
        self.surface.window.borrow().pinned
    }
    fn pin(&self) {
        self.surface.pin();
    }
    fn send(&self, text: &str) {
        if !text.is_empty() {
            self.pin();
            self.session.text_input(text);
        }
    }
    fn grid_point(&self, p: Point<Points>) -> (u16, usize) {
        self.surface.window.borrow().hit(
            p.x.0,
            p.y.0,
            self.cell_width.get_untracked().0,
            self.props.line_height.get_untracked().0,
        )
    }
    fn changed(&self, text: &str) {
        let committed = self.state.borrow_mut().changed(text);
        // Acknowledge the native value first. NSH diffs Rust host props;
        // assigning the already-empty signal would not clear a native edit.
        self.value.set(text.to_owned());
        if let Some(text) = committed {
            self.send(&text);
            self.clear_after_acknowledgement();
        }
    }

    fn clear_after_acknowledgement(&self) {
        let display = self.clone();
        after_answer(move || {
            if !display.state.borrow().composing {
                display.value.set(String::new());
            }
        });
    }

    fn composition(&self, event: &Composition) {
        if event.composing && !self.composing.get_untracked() {
            self.pin();
        }
        let committed = self
            .state
            .borrow_mut()
            .composition(event.composing, &event.text);
        self.composing.set(event.composing);
        if let Some(text) = committed {
            self.send(&text);
        }
        if !event.composing {
            self.clear_after_acknowledgement();
        }
    }

    fn reporting(&self, mods: nsh_core::keymap::Modifiers) -> bool {
        !mods.shift && self.session.emulator.lock().unwrap().mouse_enabled()
    }
    fn report(
        &self,
        p: Point<Points>,
        mods: nsh_core::keymap::Modifiers,
        action: u32,
        button: Option<u32>,
        pressed: bool,
    ) {
        self.report_n(p, mods, action, button, pressed, 1);
    }
    fn report_n(
        &self,
        p: Point<Points>,
        mods: nsh_core::keymap::Modifiers,
        action: u32,
        button: Option<u32>,
        pressed: bool,
        count: usize,
    ) {
        let cw = self.cell_width.get_untracked().0.max(1.0);
        let lh = self.props.line_height.get_untracked().0.max(1.0);
        // Protocol coordinates describe the active screen, independent of scrollback.
        if !self.at_end() {
            self.pin();
        }
        self.session.mouse_repeat(
            termlevel_ghostty_vt::MouseEvent {
                action,
                button,
                modifiers: modifiers(mods),
                x: p.x.0.max(0.0) / cw * cw.round(),
                y: p.y.0.max(0.0) / lh * lh.round(),
                any_button_pressed: pressed,
            },
            count,
            cw.round() as u32,
            lh.round() as u32,
        );
    }
    fn press(&self, e: &PressEvent) {
        use termlevel_ghostty_vt::raw::*;
        let held = self.session.emulator.lock().unwrap().render_held();
        if held && (e.modifiers.cmd || !self.reporting(e.modifiers)) {
            return;
        }
        let at = self.grid_point(e.position);
        if e.modifiers.cmd {
            if let Some(uri) = self.session.emulator.lock().unwrap().hyperlink(at.0, at.1)
                && (uri.starts_with("https://")
                    || uri.starts_with("http://")
                    || uri.starts_with("mailto:"))
            {
                spawn(async move {
                    let _ = open_url(uri).await;
                });
            }
            return;
        }
        if self.reporting(e.modifiers) {
            self.report(
                e.position,
                e.modifiers,
                GHOSTTY_MOUSE_ACTION_PRESS,
                Some(GHOSTTY_MOUSE_BUTTON_LEFT),
                true,
            );
            return;
        }
        let mut emu = self.session.emulator.lock().unwrap();
        if emu.render_held() {
            return;
        }
        emu.clear_selection();
        if e.clicks >= 3 {
            let last = emu.columns() - 1;
            emu.select((0, at.1), (last, at.1), false, false);
            self.drag.set(None)
        } else if e.clicks == 2 {
            emu.select(at, at, false, true);
            self.drag.set(None)
        } else {
            emu.select(at, at, e.modifiers.alt, false);
            self.drag.set(Some(at))
        }
        drop(emu);
        self.session.selection_changed();
    }
    fn motion(&self, e: &MouseEvent) {
        use termlevel_ghostty_vt::raw::*;
        if self.reporting(e.modifiers) {
            self.report(
                e.position,
                e.modifiers,
                GHOSTTY_MOUSE_ACTION_MOTION,
                e.pressed_button.map(button),
                e.pressed_button.is_some(),
            );
            return;
        }
        let at = self.grid_point(e.position);
        let mut emu = self.session.emulator.lock().unwrap();
        let uri = if emu.render_held() {
            None
        } else {
            emu.hyperlink(at.0, at.1)
        };
        drop(emu);
        if self.hover.get_untracked() != uri {
            self.hover.set(uri);
        }
        if e.pressed_button != Some(MouseButton::Left) {
            self.drag.set(None);
            return;
        }
        if self.session.emulator.lock().unwrap().render_held() {
            return;
        }
        if self.drag.get().is_some() {
            let at = self.grid_point(e.position);
            self.session
                .emulator
                .lock()
                .unwrap()
                .extend_selection(at, e.modifiers.alt);
            self.session.selection_changed();
        }
    }
    fn release(&self, e: &MouseEvent) {
        use termlevel_ghostty_vt::raw::*;
        if self.reporting(e.modifiers) {
            self.report(
                e.position,
                e.modifiers,
                GHOSTTY_MOUSE_ACTION_RELEASE,
                e.button.map(button),
                false,
            )
        }
        if self.session.emulator.lock().unwrap().render_held() {
            self.drag.set(None);
            return;
        }
        if self
            .drag
            .get()
            .is_some_and(|start| start == self.grid_point(e.position))
        {
            self.session.emulator.lock().unwrap().clear_selection();
            self.session.selection_changed();
        }
        self.drag.set(None);
    }
    fn copy(&self) {
        if self.session.emulator.lock().unwrap().render_held() {
            return;
        }
        let text = self.session.emulator.lock().unwrap().selection_text();
        if let Some(text) = text {
            set_clipboard(text);
        }
    }
    fn bind_keys(&self) {
        let display = self.clone();
        effect(move || {
            if !display.focused.get() || !display.props.active.get() || display.composing.get() {
                return;
            }
            let copy = display.clone();
            on_key_with(
                "mod+c",
                KeyOptions {
                    when: nsh_core::keymap::When::Always,
                    ..Default::default()
                },
                move |_| copy.copy(),
            );
            let all = display.clone();
            on_key_with(
                "mod+a",
                KeyOptions {
                    when: nsh_core::keymap::When::Always,
                    ..Default::default()
                },
                move |_| {
                    let mut emu = all.session.emulator.lock().unwrap();
                    if emu.render_held() {
                        return;
                    }
                    emu.select_all();
                    drop(emu);
                    all.session.selection_changed();
                },
            );
            let synthetic = display.synthetic_keys.get();
            for (key, spec) in keys::bindings(apple_keys()) {
                let display = display.clone();
                on_key_with(
                    &key,
                    KeyOptions {
                        when: nsh_core::keymap::When::Always,
                        repeat: true,
                        pass_through: !synthetic,
                    },
                    move |_| {
                        if !synthetic_keys() {
                            return;
                        }
                        // Also gate at dispatch, before the reactive key scope has
                        // caught up with a composition/focus event in this batch.
                        if !display.props.active.get_untracked() || display.state.borrow().composing
                        {
                            return;
                        }
                        display.pin();
                        display.session.key_input(&spec.event());
                    },
                );
            }
        });
    }

    pub fn element(&self, cx: Scope<super::SessionTerminalEvent>) -> impl IntoElement + use<> {
        self.bind_keys();
        #[cfg(target_os = "macos")]
        super::keyboard::install(
            self.session.clone(),
            self.props.active,
            self.cell_width,
            self.props.line_height,
        );
        let props = self.props;
        let input = self.input;
        effect(move || {
            if props.active.get() && props.visible.get() {
                input.focus();
            } else {
                input.blur();
            }
        });
        let resumed = self.clone();
        effect(move || {
            if props.visible.get() {
                resumed.output_changed();
            }
        });
        let layout = self.clone();
        let previous = Cell::new(None);
        effect(move || {
            let viewport = layout.viewport.get();
            let (cw, lh) = (layout.cell_width.get().0, props.line_height.get().0);
            if cw <= 0.0 || lh <= 0.0 || viewport.width.0 <= 0.0 || viewport.height.0 <= 0.0 {
                return;
            }
            let size = TerminalSize::new(
                (viewport.width.0 / cw).floor().clamp(1.0, u16::MAX as f32) as u16,
                (viewport.height.0 / lh).floor().clamp(1.0, u16::MAX as f32) as u16,
            );
            let key = (size, cw.round().max(1.0) as u32, lh.round().max(1.0) as u32);
            if previous.replace(Some(key)) != Some(key) {
                layout.session.resize_grid(size, key.1, key.2);
            }
        });
        let (down, movement, up, right, wheel) = (
            self.clone(),
            self.clone(),
            self.clone(),
            self.clone(),
            self.clone(),
        );
        let viewport = self.viewport;
        let cw = self.cell_width;
        let snap = self.surface.snap;
        let cursor_display = self.clone();
        let cursor = computed(move || {
            if cursor_display.composing.get() {
                return None;
            }
            let snap = cursor_display.surface.snap.get();
            let c = snap.frame.cursor;
            if !c.visible {
                return None;
            }
            let (col, row) = c.position?;
            let logical = snap.count.saturating_sub(snap.frame.rows as usize) + row as usize;
            if logical < snap.origin || logical >= snap.origin + snap.frame.rows as usize {
                return None;
            }
            Some((
                col,
                logical - snap.origin,
                c.style,
                c.blinking,
                snap.frame.cursor_color,
            ))
        });
        let cursor_ref = self.cursor_ref;
        let focus = self.focused;
        let blink = self.blink;
        let hover = self.hover;
        let first_layout = Cell::new(true);
        view().overflow_hidden()
            .font_family(move || family(props))
            .text_size(move || Length::from(props.font_size.get()))
            .text_color(move || default_color(props.default_foreground, theme().foreground))
            .on_press(move |_: &PressEvent| { cx.emit(super::SessionTerminalEvent::FocusRequest); input.focus(); })
            .child(view().flex_1().min_h(px(0.0)).overflow_hidden()
                .on_layout(move |b: &Bounds<Points>| {
                    viewport.set(b.size);
                    if first_layout.replace(false) && props.active.get_untracked() { input.focus(); }
                })
                .child(view().test_id(&format!("terminal-grid-{}", self.session.info().id))
                    .absolute().top_0().left_0().overflow_hidden()
                    .w(move || px(snap.get().frame.cols as f32 * cw.get().0))
                    .h(move || px(snap.get().frame.rows as f32 * props.line_height.get().0))
                    .on_press_down(move |e: &PressEvent| {
                        cx.emit(super::SessionTerminalEvent::FocusRequest);
                        input.focus();
                        down.press(e);
                    })
                    .on_mouse_move(move |e: &MouseEvent| movement.motion(e))
                    .on_mouse_up(move |e: &MouseEvent| up.release(e))
                    .on_context_menu(move |e: &ContextMenuEvent| {
                        if right.reporting(e.modifiers) {
                            right.report(e.position, e.modifiers,
                                termlevel_ghostty_vt::raw::GHOSTTY_MOUSE_ACTION_PRESS,
                                Some(termlevel_ghostty_vt::raw::GHOSTTY_MOUSE_BUTTON_RIGHT), true);
                        }
                    })
                    .on_wheel(move |e: &WheelEvent| wheel.wheel(e))
                    .child(self.native_paint())
                    .child(canvas(move || paint::semantics(&snap.get(), cw.get().0, props.line_height.get().0))
                        .absolute().inset_0().pointer_events(PointerEvents::None))
                    .child(when(move || cursor.get().is_some(), move || {
                        view().element_ref(&cursor_ref).absolute()
                            .left(move || px(cursor.get().map_or(0.0, |c| c.0 as f32 * cw.get().0)))
                            .top(move || px(cursor.get().map_or(0.0, |c| c.1 as f32 * props.line_height.get().0
                                + if c.2 == termlevel_ghostty_vt::raw::GHOSTTY_CURSOR_STYLE_UNDERLINE {
                                    (props.line_height.get().0 - 2.0).max(0.0) } else { 0.0 })))
                            .w(move || px(if cursor.get().is_some_and(|c| c.2 == termlevel_ghostty_vt::raw::GHOSTTY_CURSOR_STYLE_BAR) { 2.0 } else { cw.get().0 }))
                            .h(move || px(if cursor.get().is_some_and(|c| c.2 == termlevel_ghostty_vt::raw::GHOSTTY_CURSOR_STYLE_UNDERLINE) { 2.0 } else { props.line_height.get().0 }))
                            .bg(move || cursor.get().and_then(|c| c.4)
                                .map(|c| color(TerminalColor::Rgb(c.r,c.g,c.b)))
                                .unwrap_or_else(|| default_color(props.default_foreground, theme().foreground)))
                            .opacity(move || if cursor.get().is_some_and(|c| c.3) && focus.get() && !blink.get() { 0.0_f32 } else { 0.5_f32 })
                            .pointer_events(PointerEvents::None).app_class(props.cursor_class)
                    }))
                    .child(dynamic(move || {
                        hover.get().map(|uri| view().absolute().inset_0().tooltip(text(uri)).into_element())
                    }))
                    .child(self.sink(cx))))
    }

    fn paint_config(&self) -> paint::PaintConfig {
        let t = theme();
        paint::PaintConfig {
            cw: self.cell_width.get().0,
            lh: self.props.line_height.get().0,
            family: family(self.props).to_string(),
            size: self.props.font_size.get().0,
            foreground: nsh_core::element::color(default_color(
                self.props.default_foreground,
                t.foreground,
            )),
            background: nsh_core::element::color(default_color(
                self.props.default_background,
                t.background,
            )),
            selection: nsh_core::element::color(t.primary),
            search: nsh_core::element::color(t.foreground),
        }
    }

    fn wheel(&self, e: &WheelEvent) {
        let (dx, dy) = e.delta.xy();
        let (dx, dy) = if e.delta.precise() {
            (
                dx / self.cell_width.get_untracked().0.max(1.0),
                dy / self.props.line_height.get_untracked().0.max(1.0),
            )
        } else {
            (dx, dy)
        };
        if !self.reporting(e.modifiers) {
            self.wheel_delta.set((0.0, 0.0));
            self.surface.wheel(dy);
            return;
        }
        let (oldx, oldy) = self.wheel_delta.get();
        let (x, y) = (oldx + dx, oldy + dy);
        self.wheel_delta.set((x - x.trunc(), y - y.trunc()));
        use termlevel_ghostty_vt::raw::*;
        for (amount, positive, negative) in [
            (y, GHOSTTY_MOUSE_BUTTON_FIVE, GHOSTTY_MOUSE_BUTTON_FOUR),
            (x, GHOSTTY_MOUSE_BUTTON_SEVEN, GHOSTTY_MOUSE_BUTTON_SIX),
        ] {
            let count = amount.abs().floor().min(32.0) as usize;
            if count > 0 {
                self.report_n(
                    e.position,
                    e.modifiers,
                    GHOSTTY_MOUSE_ACTION_PRESS,
                    Some(if amount > 0.0 { positive } else { negative }),
                    false,
                    count,
                );
            }
        }
    }

    fn native_paint(&self) -> impl IntoElement + use<> {
        let display = self.clone();
        let snap = self.surface.snap;
        #[cfg(target_os = "macos")]
        {
            let publisher = super::native_surface::Publisher::new();
            super::native_surface::element()
                .prop("frame", move || {
                    display.search.rev.get();
                    publisher.publish(paint::Scene {
                        snap: display.surface.snap.get(),
                        config: display.paint_config(),
                        hits: display.search.hits.borrow().clone(),
                        current: display.search.index.get(),
                    })
                })
                .absolute()
                .inset_0()
                .role(AccessibilityRole::Text)
                .label(move || SharedString::from(snap.get().text.clone()))
                .pointer_events(PointerEvents::None)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = display;
            view()
                .absolute()
                .inset_0()
                .label(move || SharedString::from(snap.get().text.clone()))
        }
    }

    fn sink(&self, cx: Scope<super::SessionTerminalEvent>) -> impl IntoElement + use<> {
        let display = self.clone();
        let anchor = computed(move || {
            let snap = display.surface.snap.get();
            let (col, row) = snap.frame.cursor.position.unwrap_or_default();
            let lh = display.props.line_height.get().0;
            let x = col as f32 * display.cell_width.get().0;
            let logical = snap.count.saturating_sub(snap.frame.rows as usize) + row as usize;
            let y = (logical as f32 - snap.origin as f32) * lh;
            (
                x,
                y.clamp(0.0, (snap.frame.rows.saturating_sub(1)) as f32 * lh),
            )
        });
        let props = self.props;
        let composing = self.composing;
        let value = self.value;
        let snap = self.surface.snap;
        let cw = self.cell_width;
        let (change, composition, paste) = (self.clone(), self.clone(), self.clone());
        let focused = self.focused;
        let focus_session = self.session.clone();
        let blur_session = self.session.clone();
        input()
            .font_family(move || family(props))
            .text_size(move || Length::from(props.font_size.get()))
            .element_ref(&self.input)
            .test_id(&format!("terminal-input-{}", self.session.info().id))
            .absolute()
            .left(move || px(anchor.get().0))
            .top(move || px(anchor.get().1))
            .w(move || px((snap.get().frame.cols as f32 * cw.get().0 - anchor.get().0).max(1.0)))
            .h(move || Length::from(props.line_height.get()))
            .p_0()
            .border(px(0.0))
            .pointer_events(PointerEvents::None)
            .bg(move || {
                if composing.get() {
                    default_color(props.default_background, theme().background)
                } else {
                    AdaptiveColor::TRANSPARENT
                }
            })
            .text_color(move || {
                if composing.get() {
                    default_color(props.default_foreground, theme().foreground)
                } else {
                    AdaptiveColor::TRANSPARENT
                }
            })
            .caret(move || composing.get())
            .auto_correct(false)
            .auto_capitalize(AutoCapitalize::None)
            .paste_behavior(PasteBehavior::Report)
            .value(move || value.get())
            .label(move || props.accessibility_label.get().unwrap_or_default())
            .on_change(move |text: &str| change.changed(text))
            .on_composition_change(move |event: &Composition| composition.composition(event))
            .on_paste(move |event: &PasteEvent| {
                paste.pin();
                paste.session.paste(&event.text);
            })
            .on_focus(move || {
                focused.set(true);
                focus_session.focus_input(true);
                cx.emit(super::SessionTerminalEvent::FocusRequest);
            })
            .on_blur(move || {
                focused.set(false);
                blur_session.focus_input(false);
            })
    }
}

fn family(props: SessionTerminalProps) -> SharedString {
    props
        .font_family
        .get()
        .or_else(|| theme().font_mono.clone())
        .unwrap_or_else(|| "monospace".into())
}
fn default_color(prop: Prop<Option<SharedString>>, fallback: AdaptiveColor) -> AdaptiveColor {
    prop.get()
        .and_then(|c| c.parse::<Hsla>().ok())
        .map(AdaptiveColor::from)
        .unwrap_or(fallback)
}
fn color(c: TerminalColor) -> AdaptiveColor {
    let (r, g, b) = match c {
        TerminalColor::Rgb(r, g, b) => (r, g, b),
        TerminalColor::Indexed(n) => {
            let rgb = crate::terminal::indexed_color(n);
            (rgb.r, rgb.g, rgb.b)
        }
    };
    nsh_core::color::rgb((r as u32) << 16 | (g as u32) << 8 | b as u32).into()
}

fn modifiers(m: nsh_core::keymap::Modifiers) -> u16 {
    use termlevel_ghostty_vt::raw::*;
    (if m.shift { GHOSTTY_MODS_SHIFT } else { 0 }
        | if m.ctrl { GHOSTTY_MODS_CTRL } else { 0 }
        | if m.alt { GHOSTTY_MODS_ALT } else { 0 }
        | if m.cmd { GHOSTTY_MODS_SUPER } else { 0 }) as u16
}
fn button(b: MouseButton) -> u32 {
    use termlevel_ghostty_vt::raw::*;
    match b {
        MouseButton::Left => GHOSTTY_MOUSE_BUTTON_LEFT,
        MouseButton::Right => GHOSTTY_MOUSE_BUTTON_RIGHT,
        MouseButton::Middle => GHOSTTY_MOUSE_BUTTON_MIDDLE,
        MouseButton::Back => GHOSTTY_MOUSE_BUTTON_EIGHT,
        MouseButton::Forward => GHOSTTY_MOUSE_BUTTON_NINE,
    }
}

fn synthetic_keys() -> bool {
    #[cfg(target_os = "macos")]
    {
        super::keyboard::synthetic_keys()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}
