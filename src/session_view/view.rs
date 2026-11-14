//! Native grid rendering and a stable AppKit-backed input at the grid cursor.
use super::{SessionTerminalProps, input::InputState, keys};
use crate::sessions::Session;
use crate::terminal::SearchHit;
use nsh_base::{TerminalColor, TerminalEmulator, TerminalSize, TerminalStyle, input, theme};
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
            search: Search {
                query: Rc::default(),
                hits: Rc::default(),
                index: Rc::default(),
                rev: signal(0),
                scroll,
                session,
            },
        };
        #[cfg(all(debug_assertions, target_os = "macos"))]
        super::native_probe::track(display.session.info().id, display.input, display.cursor_ref);
        display
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

    fn bind_keys(&self) {
        let display = self.clone();
        effect(move || {
            if !display.focused.get() || !display.props.active.get() || display.composing.get() {
                return;
            }
            for (key, spec) in keys::bindings(apple_keys()) {
                let display = display.clone();
                on_key_with(
                    &key,
                    KeyOptions {
                        when: nsh_core::keymap::When::Always,
                        repeat: true,
                        ..Default::default()
                    },
                    move |_| {
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
        let props = self.props;
        let input = self.input;
        effect(move || {
            if props.active.get() {
                input.focus();
            } else {
                input.blur();
            }
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
            }
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
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_hidden()
                    .child(rows.selectable(true).flex_1().min_h(px(0.0)).on_layout(
                        move |b: &Bounds<Points>| {
                            viewport.set(b.size);
                            if first_layout.replace(false) && props.active.get_untracked() {
                                input.focus();
                            }
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
            (at.index() == emu.line_count() - emu.screen_rows() + c.row as usize)
                .then_some((c.col, emu.cursor_style()))
        });
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
            .child(dynamic(move || {
                view().absolute().inset_0().children(
                    spans
                        .get()
                        .into_iter()
                        .enumerate()
                        .map(move |(index, span)| {
                            let s = span.run.style;
                            let (fg, bg) = colors(s, props);
                            let mut glyph = text(span.run.text)
                                .font_family(move || family(props))
                                .text_size(move || Length::from(props.font_size.get()))
                                .select_text()
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
                            if s.underline {
                                glyph = glyph.underline();
                            }
                            if s.strikethrough {
                                glyph = glyph.line_through();
                            }
                            if s.hidden || s.dim {
                                glyph = glyph.opacity(if s.hidden { 0.0 } else { 0.6 });
                            }
                            let mut cell = view()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .left(Length::from(cw.get() * span.col as f32))
                                .w(Length::from(cw.get() * span.cells as f32))
                                .justify_center();
                            if let Some(bg) = bg {
                                cell = cell.bg(bg);
                            }
                            cell.child(glyph).into_element()
                        })
                        .collect::<Vec<_>>(),
                )
            }))
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
                        .bg(move || default_color(props.default_foreground, theme().foreground))
                        .opacity(0.5)
                        .app_class(props.cursor_class)
                },
            ))
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
