//! NSH is a view onto an existing Rust session, never its lifetime owner.

mod diag;
mod file_drop;
mod input;
mod keys;
mod paint;
pub(crate) mod platform;
mod presentation;
mod surface;
mod view;
pub(crate) mod window;

use crate::sessions::{SessionInfo, manager};
use nsh_core::prelude::*;
use std::{
    cell::{Cell, RefCell},
    sync::atomic::Ordering,
    time::Duration,
};

/// Display parameters for a pre-existing real terminal session.
#[derive(Clone, Copy, PartialEq, Props)]
pub struct SessionTerminalProps {
    /// Stable identity returned by Sessions.open. Changing it needs a new view.
    pub session_id: Prop<SharedString>,
    /// Whether to focus this view, independently of its session's lifetime.
    #[prop(default = true)]
    pub active: Prop<bool>,
    /// Whether a pane is actually displayed. Hidden sessions keep their PTY,
    /// but do not sample frames or record drawings for new output.
    #[prop(default = true)]
    pub visible: Prop<bool>,
    /// Modal UI blocks external file input independently of pane focus.
    #[prop(default = false)]
    pub input_blocked: Prop<bool>,
    /// Monospaced text family.
    pub font_family: Prop<Option<SharedString>>,
    /// Text size in points.
    #[prop(default = Points(13.0))]
    pub font_size: Prop<Points>,
    /// Text row height in points.
    #[prop(default = Points(21.0))]
    pub line_height: Prop<Points>,
    /// Accessible name for the native terminal input.
    pub accessibility_label: Prop<Option<SharedString>>,
    /// Theme classes for the native cursor.
    pub cursor_class: Prop<Option<SharedString>>,
    /// Theme default foreground for terminal color query replies.
    pub default_foreground: Prop<Option<SharedString>>,
    /// Theme default background for terminal color query replies.
    pub default_background: Prop<Option<SharedString>>,
}

/// Find-bar results arrive independently of frame updates.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct SearchStatus {
    pub query: String,
    pub count: u32,
    pub index: u32,
    pub searching: bool,
}

/// Low-frequency updates only; PTY output and keystrokes stay in Rust.
#[derive(Debug, Clone, PartialEq, Event)]
pub enum SessionTerminalEvent {
    /// Initial status and changes to title, exit status or errors.
    #[args(info)]
    StatusChange(SessionInfo),
    /// Bounded background search progressed or was invalidated.
    #[args(result)]
    SearchChange(SearchStatus),
    /// The program rang the terminal bell.
    Bell,
    /// A displayed terminal was clicked or its native input acquired focus.
    FocusRequest,
}

/// Native view methods. Closing is an explicit Sessions.close operation.
pub struct SessionTerminalApi {
    input: ElementRef,
    search: view::Search,
}

#[methods]
impl SessionTerminalApi {
    /// Gives the terminal's native input focus.
    pub fn focus(&self) {
        self.input.focus();
    }

    /// Starts a bounded search. Returns immediately available hits; SearchChange
    /// supplies completed counts and the first match is revealed once ready.
    pub fn search(&self, query: String) -> u32 {
        self.search.apply(&query)
    }

    /// Moves to another hit. Negative wraps backward. Returns the 1-based index, or 0.
    pub fn search_move(&self, delta: i32) -> u32 {
        self.search.move_by(delta)
    }

    /// Clears the query and its highlights.
    pub fn clear_search(&self) {
        self.search.clear();
    }
}

/// A native grid and input bound to an independently owned Rust session.
#[component(methods = SessionTerminalApi)]
pub fn SessionTerminal(
    props: SessionTerminalProps,
    cx: Scope<SessionTerminalEvent>,
) -> impl IntoElement {
    let id = props.session_id.get_untracked().to_string();
    let session = match manager().lock().unwrap().get(&id) {
        Ok(session) => session,
        Err(error) => return view().child(text(error)).into_element(),
    };
    let display = match view::Display::new(session.clone(), props) {
        Ok(display) => display,
        Err(error) => return view().child(text(error)).into_element(),
    };
    cx.methods(SessionTerminalApi {
        input: display.input,
        search: display.search.clone(),
    });
    let search = display.search.clone();
    effect(move || {
        search.watch();
        cx.emit(SessionTerminalEvent::SearchChange(search.status()));
    });
    let colors_session = session.clone();
    effect(move || {
        colors_session.set_colors(
            props.default_foreground.get().as_deref(),
            props.default_background.get().as_deref(),
        )
    });
    let revision = Cell::new(0);
    let metadata = RefCell::new(None);
    let bells = Cell::new(session.bells.load(Ordering::Acquire));
    let refresh = display.clone();
    set_interval(Duration::from_millis(16), move || {
        // The only frame scheduler: capture, pointer work, blink, image
        // animation and search all advance here, at most one snapshot each.
        refresh.frame();
        let next = session.revision.load(Ordering::Acquire);
        if next == revision.get() {
            return;
        }
        revision.set(next);
        let next_bells = session.bells.load(Ordering::Acquire);
        if bells.replace(next_bells) != next_bells {
            cx.emit(SessionTerminalEvent::Bell);
        }
        let info = session.info();
        if metadata.borrow().as_ref() != Some(&info) {
            *metadata.borrow_mut() = Some(info.clone());
            cx.emit(SessionTerminalEvent::StatusChange(info));
        }
    });
    display.element(cx).into_element()
}
