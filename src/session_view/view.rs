//! Native grid rendering and a stable AppKit-backed input at the grid cursor.
use super::{SessionTerminalProps, input::InputState, keys};
use crate::sessions::Session;
use crate::terminal::SearchHit;
use nsh_base::{TerminalColor, TerminalEmulator, TerminalSize, TerminalStyle, input, theme};
use nsh_core::element::ImageSrc;
use nsh_core::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

#[derive(Clone)]
pub(super) struct Display {
    session: Arc<Session>,
    props: SessionTerminalProps,
    pub input: ElementRef,
    scroll: ElementRef,
    cursor_ref: ElementRef,
    pub revision: Signal<u64>,
    focused: Signal<bool>,
    composing: Signal<bool>,
    value: Signal<String>,
    state: Rc<RefCell<InputState>>,
    viewport: Signal<Size<Points>>,
    offset: Signal<Points>,
    cell_width: Memo<Points>,
    drag: Rc<Cell<Option<(u16, usize)>>>,
    blink: Signal<bool>,
    synthetic_keys: Signal<bool>,
    images: Signal<Vec<termlevel_ghostty_vt::ImagePlacement>>,
    last_count: Rc<Cell<usize>>,
    wheel_delta: Rc<Cell<(f32, f32)>>,
    pub search: Search,
}

/// The active find for one terminal view.
#[derive(Clone)]
pub(super) struct Search {
    query: Rc<RefCell<String>>,
    hits: Rc<RefCell<Vec<SearchHit>>>,
    index: Rc<Cell<usize>>,
    rev: Signal<u64>,
    scroll: ElementRef,
    session: Arc<Session>,
}

impl Search {
    pub(super) fn apply(&self, query: &str) -> u32 {
        *self.query.borrow_mut() = query.to_owned();
        self.rescan();
        self.index.set(0);
        let count = self.hits.borrow().len() as u32;
        if count > 0 {
            self.reveal(0);
        }
        self.rev.update(|n| *n += 1);
        count
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
        self.query.borrow_mut().clear();
        self.hits.borrow_mut().clear();
        self.index.set(0);
        self.rev.update(|n| *n += 1);
    }

    /// Keeps highlights current after new output without jumping the viewport.
    pub(super) fn refresh(&self) {
        if self.query.borrow().is_empty() {
            return;
        }
        self.rescan();
        let len = self.hits.borrow().len();
        if self.index.get() >= len {
            self.index.set(0);
        }
        self.rev.update(|n| *n += 1);
    }

    fn rescan(&self) {
        let query = self.query.borrow().clone();
        let hits = self.session.emulator.lock().unwrap().search(&query);
        *self.hits.borrow_mut() = hits;
    }

    fn reveal(&self, index: usize) {
        let Some(hit) = self.hits.borrow().get(index).copied() else {
            return;
        };
        // Percentage is a 0..=1 fraction. 0.3 keeps the hit near the top of the viewport.
        self.scroll
            .scroll_to_index(hit.line as u32, Percentage(0.3), Points(0.0), false);
    }
}

impl Display {
    pub fn new(session: Arc<Session>, props: SessionTerminalProps) -> Self {
        let cell_width = computed(move || {
            let family = family(props);
            Points(crate::runtime::terminal_cell_width(
                &family,
                props.font_size.get().0,
            ))
        });
        let scroll = element_ref();
        let display = Self {
            session: session.clone(),
            props,
            input: element_ref(),
            scroll,
            cursor_ref: element_ref(),
            revision: signal(0),
            focused: signal(false),
            composing: signal(false),
            value: signal(String::new()),
            state: Rc::default(),
            viewport: signal(Size::default()),
            offset: signal(Points(0.0)),
            cell_width,
            drag: Rc::default(),
            blink: signal(true),
            synthetic_keys: signal(synthetic_keys()),
            images: signal(Vec::new()),
            wheel_delta: Rc::default(),
            last_count: Rc::new(Cell::new(session.emulator.lock().unwrap().line_count())),
            search: Search {
                query: Rc::default(),
                hits: Rc::default(),
                index: Rc::default(),
                rev: signal(0),
                scroll,
                session: session.clone(),
            },
        };
        #[cfg(all(debug_assertions, target_os = "macos"))]
        super::native_probe::track(display.session.info().id, display.input, display.cursor_ref);
        let clock = display.clone();
        let epoch = Cell::new(0);
        let started = std::time::Instant::now();
        let reset = Cell::new(started);
        let rev = Cell::new(0);
        set_interval(std::time::Duration::from_millis(16), move || {
            use std::sync::atomic::Ordering;
            clock.synthetic_keys.set(synthetic_keys());
            let next = clock.session.input_epoch.load(Ordering::Acquire);
            if epoch.replace(next) != next {
                clock.scroll.scroll_to_end(false);
                reset.set(std::time::Instant::now());
            }
            let next = clock.revision.get_untracked();
            if rev.replace(next) != next {
                reset.set(std::time::Instant::now());
            }
            clock
                .blink
                .set(reset.get().elapsed().as_millis() % 1000 < 500);
            if clock
                .session
                .emulator
                .lock()
                .unwrap()
                .tick_images(started.elapsed().as_millis() as u64)
            {
                clock.session.selection_changed();
            }
        });
        display
    }

    pub(super) fn output_changed(&self) {
        let count = self.session.emulator.lock().unwrap().line_count();
        let old = self.last_count.replace(count);
        let lh = self.props.line_height.get_untracked().0;
        if self.offset.get_untracked().0 + self.viewport.get_untracked().height.0
            >= old as f32 * lh - 1.0
        {
            let scroll = self.scroll;
            after_answer(move || scroll.scroll_to_end(false));
        }
    }
    fn send(&self, text: &str) {
        if !text.is_empty() {
            self.scroll.scroll_to_end(false);
            self.session.text_input(text);
        }
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
            self.scroll.scroll_to_end(false);
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

    fn grid_point(&self, p: Point<Points>) -> (u16, usize) {
        let emu = self.session.emulator.lock().unwrap();
        let lh = self.props.line_height.get_untracked().0.max(1.0);
        let top = (self.viewport.get_untracked().height.0 - emu.line_count() as f32 * lh).max(0.0);
        let row = ((p.y.0 + self.offset.get_untracked().0 - top) / lh)
            .floor()
            .max(0.0) as usize;
        let col = (p.x.0 / self.cell_width.get_untracked().0.max(1.0))
            .floor()
            .max(0.0) as u16;
        (
            col.min(emu.columns() - 1),
            row.min(emu.line_count().saturating_sub(1)),
        )
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
        let cw = self.cell_width.get_untracked().0.max(1.0);
        let lh = self.props.line_height.get_untracked().0.max(1.0);
        // Protocol coordinates describe the active screen, independent of scrollback.
        self.scroll.scroll_to_end(false);
        self.session.mouse_input(
            termlevel_ghostty_vt::MouseEvent {
                action,
                button,
                modifiers: modifiers(mods),
                x: p.x.0.max(0.0) / cw * cw.round(),
                y: p.y.0.max(0.0) / lh * lh.round(),
                any_button_pressed: pressed,
            },
            cw.round() as u32,
            lh.round() as u32,
        );
    }
    fn press(&self, e: &PressEvent) {
        use termlevel_ghostty_vt::raw::*;
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
        if e.pressed_button != Some(MouseButton::Left) {
            self.drag.set(None);
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
        if let Some(text) = self.session.emulator.lock().unwrap().selection_text() {
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
                    all.session.emulator.lock().unwrap().select_all();
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
                        ..Default::default()
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
                        display.scroll.scroll_to_end(false);
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
            self.focused,
            self.cell_width,
            self.props.line_height,
        );
        let props = self.props;
        let input = self.input;
        effect(move || {
            if props.active.get() {
                input.focus();
            } else {
                input.blur();
            }
        });
        let image_display = self.clone();
        effect(move || {
            image_display.revision.get();
            let first = (image_display.offset.get().0
                / image_display.props.line_height.get().0.max(1.0))
            .floor()
            .max(0.0) as usize;
            image_display.images.set(
                image_display
                    .session
                    .emulator
                    .lock()
                    .unwrap()
                    .images_at(first),
            );
        });
        let scroll = self.scroll;
        let offset = self.offset;
        watch_scroll(move || scroll.node(), move |e| offset.set(e.offset.y));
        let count_display = self.clone();
        let row_display = self.clone();
        let rows = virtual_list(
            move || {
                count_display.revision.get();
                count_display.session.emulator.lock().unwrap().line_count()
            },
            move |at| row_display.row(at),
        )
        .element_ref(&self.scroll)
        .alignment(ListAlignment::Bottom)
        .estimated_item_size(props.line_height.get_untracked());
        let layout = self.clone();
        let previous_size = Cell::new(None);
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
            if previous_size.replace(Some(size)) != Some(size) {
                layout.session.resize(size);
                layout
                    .session
                    .emulator
                    .lock()
                    .unwrap()
                    .cell_size(cw.round() as u32, lh.round() as u32);
                layout.session.selection_changed();
            }
        });
        let down = self.clone();
        let movement = self.clone();
        let up = self.clone();
        let wheel = self.clone();
        let right = self.clone();
        let enabled = self.clone();
        let mouse = computed(move || {
            enabled.revision.get();
            enabled.session.emulator.lock().unwrap().mouse_enabled()
        });
        let viewport = self.viewport;
        let first_layout = Cell::new(true);
        view()
            .overflow_hidden()
            .font_family(move || family(props))
            .text_size(move || Length::from(props.font_size.get()))
            .text_color(move || default_color(props.default_foreground, theme().foreground))
            .on_press(move |_: &PressEvent| {
                cx.emit(super::SessionTerminalEvent::FocusRequest);
                input.focus();
            })
            // Host classes may add padding. Give the list and its input
            // overlay the same unpadded content box and coordinate origin.
            .child(
                view()
                    .test_id(&format!("terminal-grid-{}", self.session.info().id))
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_hidden()
                    .on_press_down(move |e: &PressEvent| down.press(e))
                    .on_mouse_move(move |e: &MouseEvent| movement.motion(e))
                    .on_mouse_up(move |e: &MouseEvent| up.release(e))
                    .on_context_menu(move |e: &ContextMenuEvent| {
                        if right.reporting(e.modifiers) {
                            right.report(
                                e.position,
                                e.modifiers,
                                termlevel_ghostty_vt::raw::GHOSTTY_MOUSE_ACTION_PRESS,
                                Some(termlevel_ghostty_vt::raw::GHOSTTY_MOUSE_BUTTON_RIGHT),
                                true,
                            )
                        }
                    })
                    .child(rows.selectable(false).flex_1().min_h(px(0.0)).on_layout(
                        move |b: &Bounds<Points>| {
                            viewport.set(b.size);
                            if first_layout.replace(false) && props.active.get_untracked() {
                                input.focus();
                            }
                        },
                    ))
                    .child(when(
                        move || mouse.get(),
                        move || {
                            let wheel = wheel.clone();
                            view().absolute().inset_0().on_wheel(move |e: &WheelEvent| {
                                if e.modifiers.shift {
                                    wheel.scroll.scroll_to_offset(
                                        Points(
                                            (wheel.offset.get_untracked().0
                                                + e.delta
                                                    .points(wheel.props.line_height.get_untracked())
                                                    .y
                                                    .0)
                                                .max(0.0),
                                        ),
                                        false,
                                    );
                                    return;
                                }
                                let (dx, dy) = e.delta.xy();
                                let (dx, dy) = if e.delta.precise() {
                                    (
                                        dx / wheel.cell_width.get_untracked().0.max(1.0),
                                        dy / wheel.props.line_height.get_untracked().0.max(1.0),
                                    )
                                } else {
                                    (dx, dy)
                                };
                                let (oldx, oldy) = wheel.wheel_delta.get();
                                let x = oldx + dx;
                                let y = oldy + dy;
                                wheel.wheel_delta.set((x - x.trunc(), y - y.trunc()));
                                use termlevel_ghostty_vt::raw::*;
                                for (amount, positive, negative) in [
                                    (y, GHOSTTY_MOUSE_BUTTON_FIVE, GHOSTTY_MOUSE_BUTTON_FOUR),
                                    (x, GHOSTTY_MOUSE_BUTTON_SEVEN, GHOSTTY_MOUSE_BUTTON_SIX),
                                ] {
                                    for _ in 0..(amount.abs().floor().min(32.0) as usize) {
                                        wheel.report(
                                            e.position,
                                            e.modifiers,
                                            GHOSTTY_MOUSE_ACTION_PRESS,
                                            Some(if amount > 0.0 { positive } else { negative }),
                                            false,
                                        )
                                    }
                                }
                            })
                        },
                    ))
                    .child(self.sink(cx)),
            )
    }

    fn row(&self, at: Position) -> impl IntoElement + use<> {
        let display = self.clone();
        let spans = computed(move || {
            display.revision.get();
            display
                .session
                .emulator
                .lock()
                .unwrap()
                .grid_runs(at.index())
        });
        let display = self.clone();
        let cursor = computed(move || {
            display.revision.get();
            if display.composing.get() {
                return None;
            }
            let emu = display.session.emulator.lock().unwrap();
            let c = emu.cursor()?;
            (at.index() == emu.line_count() - emu.screen_rows() + c.row as usize).then(|| {
                let (appearance, color) = emu.cursor_appearance().unwrap();
                (c.col, appearance.style, appearance.blinking, color)
            })
        });
        let blink = self.blink;
        let focus = self.focused;
        let props = self.props;
        let cursor_ref = self.cursor_ref;
        let marks = {
            let display = self.clone();
            computed(move || {
                display.search.rev.get();
                let hits = display.search.hits.borrow();
                let current = display.search.index.get();
                hits.iter()
                    .enumerate()
                    .filter(|(_, hit)| hit.line == at.index())
                    .map(|(index, hit)| (hit.col, hit.cols, index == current))
                    .collect::<Vec<_>>()
            })
        };
        let cw = self.cell_width;
        view()
            .h(move || Length::from(props.line_height.get()))
            .child(dynamic(move || {
                let width = cw.get();
                view().absolute().inset_0().children(
                    marks
                        .get()
                        .into_iter()
                        .map(|(col, cols, current)| {
                            view()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .left(Length::from(width * col as f32))
                                .w(Length::from(width * cols.max(1) as f32))
                                .bg(theme().foreground)
                                .opacity(if current { 0.28 } else { 0.12 })
                                .pointer_events(PointerEvents::None)
                                .into_element()
                        })
                        .collect::<Vec<_>>(),
                )
            }))
            .child(self.image_strip(at.index(), 0))
            .child(dynamic(move || {
                view().absolute().inset_0().children(
                    spans
                        .get()
                        .into_iter()
                        .filter_map(move |span| {
                            let (_, bg) = colors(span.run.style, props);
                            let bg = if span.selected {
                                Some(theme().primary)
                            } else {
                                bg
                            };
                            bg.map(|bg| {
                                view()
                                    .absolute()
                                    .top_0()
                                    .bottom_0()
                                    .left(Length::from(cw.get() * span.col as f32))
                                    .w(Length::from(cw.get() * span.cells as f32))
                                    .bg(bg)
                                    .pointer_events(PointerEvents::None)
                                    .into_element()
                            })
                        })
                        .collect::<Vec<_>>(),
                )
            }))
            .child(self.image_strip(at.index(), 1))
            .child(dynamic(move || {
                view().absolute().inset_0().children(
                    spans
                        .get()
                        .into_iter()
                        .enumerate()
                        .map(move |(index, span)| {
                            let s = span.run.style;
                            let (fg, _) = colors(s, props);
                            let mut glyph = text(span.run.text)
                                .font_family(move || family(props))
                                .text_size(move || Length::from(props.font_size.get()))
                                .continues_text(index > 0)
                                .number_of_lines(1u32)
                                .text_overflow(nsh_core::EllipsizeMode::Clip);
                            if let Some(fg) = fg {
                                glyph = glyph.text_color(fg);
                            }
                            if s.bold {
                                glyph = glyph.font_weight(FontWeight::BOLD);
                            }
                            if s.italic {
                                glyph = glyph.italic();
                            }
                            if s.underline || span.hyperlink.is_some() {
                                glyph = glyph.underline();
                            }
                            if s.strikethrough {
                                glyph = glyph.line_through();
                            }
                            if s.hidden || s.dim {
                                glyph = glyph.opacity(if s.hidden { 0.0 } else { 0.6 });
                            }
                            let cell = view()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .left(Length::from(cw.get() * span.col as f32))
                                .w(Length::from(cw.get() * span.cells as f32))
                                .justify_center();
                            let cell = if let Some(uri) = span.hyperlink {
                                cell.tooltip(text(uri))
                            } else {
                                cell
                            };
                            cell.child(glyph).into_element()
                        })
                        .collect::<Vec<_>>(),
                )
            }))
            .child(self.image_strip(at.index(), 2))
            .child(when(
                move || cursor.get().is_some(),
                move || {
                    view()
                        .element_ref(&cursor_ref)
                        .absolute()
                        .top(move || {
                            px(
                                if cursor.get().is_some_and(|c| {
                                    c.1 == termlevel_ghostty_vt::raw::GHOSTTY_CURSOR_STYLE_UNDERLINE
                                }) {
                                    (props.line_height.get().0 - 2.0).max(0.0)
                                } else {
                                    0.0
                                },
                            )
                        })
                        .bottom_0()
                        .left(move || {
                            Length::from(cw.get() * cursor.get().map_or(0, |c| c.0) as f32)
                        })
                        .w(move || {
                            px(
                                if cursor.get().is_some_and(|c| {
                                    c.1 == termlevel_ghostty_vt::raw::GHOSTTY_CURSOR_STYLE_BAR
                                }) {
                                    2.0
                                } else {
                                    cw.get().0
                                },
                            )
                        })
                        .pointer_events(PointerEvents::None)
                        .bg(move || {
                            cursor
                                .get()
                                .and_then(|c| c.3)
                                .map(|c| color(TerminalColor::Rgb(c.r, c.g, c.b)))
                                .unwrap_or_else(|| {
                                    default_color(props.default_foreground, theme().foreground)
                                })
                        })
                        .opacity(move || {
                            if cursor.get().is_some_and(|c| c.2) && focus.get() && !blink.get() {
                                0.0_f32
                            } else {
                                0.5_f32
                            }
                        })
                        .app_class(props.cursor_class)
                },
            ))
    }

    fn image_strip(&self, row: usize, layer: u32) -> impl IntoElement + use<> {
        let images = self.images;
        let cw = self.cell_width;
        let lh = self.props.line_height;
        dynamic(move || {
            let height = lh.get().0;
            let width = cw.get().0;
            view()
                .absolute()
                .inset_0()
                .overflow_hidden()
                .pointer_events(PointerEvents::None)
                .children(
                    images
                        .get()
                        .iter()
                        .filter(|p| p.layer == layer)
                        .filter_map(|p| {
                            let dx = width / width.round().max(1.0);
                            let dy = height / height.round().max(1.0);
                            let x = p.column as f32 * width + p.offset_x as f32 * dx;
                            let y = (p.row as f32 - row as f32) * height + p.offset_y as f32 * dy;
                            if y >= height
                                || y + p.height as f32 * dy <= 0.0
                                || p.source_width == 0
                                || p.source_height == 0
                            {
                                return None;
                            }
                            let sx = p.width as f32 * dx / p.source_width as f32;
                            let sy = p.height as f32 * dy / p.source_height as f32;
                            Some(
                                view()
                                    .absolute()
                                    .left(px(x))
                                    .top(px(y))
                                    .w(px(p.width as f32 * dx))
                                    .h(px(p.height as f32 * dy))
                                    .overflow_hidden()
                                    .child(
                                        image(ImageSrc::rgba(
                                            p.pixels.width,
                                            p.pixels.height,
                                            p.pixels.rgba.clone(),
                                        ))
                                        .absolute()
                                        .left(px(-(p.source_x as f32) * sx))
                                        .top(px(-(p.source_y as f32) * sy))
                                        .w(px(p.pixels.width as f32 * sx))
                                        .h(px(p.pixels.height as f32 * sy))
                                        .object_fit(ResizeMode::Stretch),
                                    )
                                    .into_element(),
                            )
                        })
                        .collect::<Vec<_>>(),
                )
        })
    }
    fn sink(&self, cx: Scope<super::SessionTerminalEvent>) -> impl IntoElement + use<> {
        let display = self.clone();
        let anchor = computed(move || {
            display.revision.get();
            let emu = display.session.emulator.lock().unwrap();
            let cursor = emu.input_cursor();
            let lh = display.props.line_height.get().0;
            let viewport = display.viewport.get();
            let x = cursor.col as f32 * display.cell_width.get().0;
            let content_top = (viewport.height.0 - emu.line_count() as f32 * lh).max(0.0);
            let y = content_top
                + (emu.line_count() - emu.screen_rows() + cursor.row as usize) as f32 * lh
                - display.offset.get().0;
            // Stay in the viewport when history is being inspected. Typing
            // follows the end before the next PTY output is rendered.
            (
                x.min(viewport.width.0),
                y.clamp(0.0, (viewport.height.0 - lh).max(0.0)),
            )
        });
        let props = self.props;
        let composing = self.composing;
        let value = self.value;
        let viewport = self.viewport;
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
            .w(move || px((viewport.get().width.0 - anchor.get().0).max(1.0)))
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
                paste.scroll.scroll_to_end(false);
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
fn colors(
    s: TerminalStyle,
    props: SessionTerminalProps,
) -> (Option<AdaptiveColor>, Option<AdaptiveColor>) {
    let (fg, bg) = (s.fg.map(color), s.bg.map(color));
    if s.inverse {
        (
            Some(bg.unwrap_or_else(|| default_color(props.default_background, theme().background))),
            Some(fg.unwrap_or_else(|| default_color(props.default_foreground, theme().foreground))),
        )
    } else {
        (fg, bg)
    }
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
