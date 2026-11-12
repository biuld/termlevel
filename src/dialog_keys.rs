use nsh_core::prelude::*;

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
