//! Explicit extension point for a future native terminal backend. This is a
//! placeholder, not a renderer or a claim of working non-macOS PTY support.
use crate::{session_view::paint::Scene, sessions::Session};
use nsh_core::prelude::*;
use std::sync::Arc;

pub(in crate::session_view) fn surface(
    _scene: impl Fn() -> Scene + 'static,
    label: impl Fn() -> SharedString + 'static,
) -> impl IntoElement {
    view()
        .absolute()
        .inset_0()
        .role(AccessibilityRole::Text)
        .label(label)
        .pointer_events(PointerEvents::None)
        .child(text(
            "Terminal rendering is not available on this platform yet.",
        ))
        .into_element()
}

pub(in crate::session_view) fn install_input(
    _session: Arc<Session>,
    _active: Prop<bool>,
    _cw: Memo<Points>,
    _lh: Prop<Points>,
) {
    // NSH shortcut and field-editor input remains the portable fallback.
}

pub(in crate::session_view) fn synthetic_keys() -> bool {
    true
}

#[cfg(debug_assertions)]
pub(in crate::session_view) fn track(_id: String, _input: ElementRef, _cursor: ElementRef) {}
