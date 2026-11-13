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

/// Pane resize-mode keys, mounted after terminal bindings to take native priority.
#[derive(Debug, Clone, PartialEq, Event)]
pub enum ResizeKeysEvent {
    /// Move the nearest split in this direction.
    #[args(direction)]
    Direction(String),
    /// Finish resizing and return keys to the terminal.
    Finish,
}

/// Temporary native keyboard scope for pane resizing.
#[component]
pub fn ResizeKeys(cx: Scope<ResizeKeysEvent>) -> impl IntoElement {
    for direction in ["left", "right", "up", "down"] {
        on_key_with(
            direction,
            KeyOptions {
                when: nsh_core::keymap::When::Always,
                repeat: true,
                ..Default::default()
            },
            move |_| cx.emit(ResizeKeysEvent::Direction(direction.to_owned())),
        );
    }
    for key in ["enter", "escape"] {
        on_key_with(
            key,
            KeyOptions {
                when: nsh_core::keymap::When::Always,
                ..Default::default()
            },
            move |_| cx.emit(ResizeKeysEvent::Finish),
        );
    }
    view().absolute().w(px(0.0)).h(px(0.0))
}
