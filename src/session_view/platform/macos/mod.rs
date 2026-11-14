//! AppKit backend. All terminal NSView, CoreText and local-event-monitor code
//! lives here; none of these objects may cross the scene channel or own a PTY.

mod keyboard;
#[cfg(debug_assertions)]
pub(super) mod probe;
mod renderer;
mod view;

use crate::{
    session_view::{paint::Scene, presentation},
    sessions::Session,
};
use nsh_core::prelude::*;
use std::sync::Arc;

pub(crate) fn register_components(components: &nsh_platform_macos::Components) {
    view::register(components);
}

pub(in crate::session_view) fn surface(
    scene: impl Fn() -> Scene + 'static,
    label: impl Fn() -> SharedString + 'static,
) -> impl IntoElement {
    presentation::element(scene, label)
}

pub(in crate::session_view) fn install_input(
    session: Arc<Session>,
    active: Prop<bool>,
    cw: Memo<Points>,
    lh: Prop<Points>,
) {
    keyboard::install(session, active, cw, lh);
}

pub(in crate::session_view) fn synthetic_keys() -> bool {
    keyboard::synthetic_keys()
}

#[cfg(debug_assertions)]
pub(in crate::session_view) fn track(id: String, input: ElementRef, cursor: ElementRef) {
    probe::track(id, input, cursor);
}
