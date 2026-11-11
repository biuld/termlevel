//! Native exports used by the TSX shell. PTY ownership will live here in M2.

use nsh_core::prelude::*;

/// Native services available to the application UI.
#[native_module("termlevel/native")]
fn native(registry: &mut Registry) {
    registry.module::<Runtime>().register::<DialogKeys>();
}

/// Dismiss events from the dialog's keyboard scope.
#[derive(Debug, Clone, PartialEq, Event)]
pub enum DialogKeysEvent {
    /// The user pressed Escape.
    Dismiss,
}

/// Gives an open dialog priority over the default Terminal's native keys.
#[component]
pub fn DialogKeys(cx: Scope<DialogKeysEvent>) -> impl IntoElement {
    on_key_with(
        "escape",
        KeyOptions {
            when: nsh_core::keymap::When::Always,
            ..Default::default()
        },
        move |_| cx.emit(DialogKeysEvent::Dismiss),
    );
    view().absolute().w(px(0.0)).h(px(0.0))
}

/// Build information for the About surface.
#[module]
pub struct Runtime;

#[module]
impl Runtime {
    /// The application version compiled into the native crate.
    pub fn version() -> String {
        env!("CARGO_PKG_VERSION").to_owned()
    }
}
