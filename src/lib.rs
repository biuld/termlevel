//! Registration only. Session/process ownership and UI bindings live in their modules.

mod dialog_keys;
mod runtime;
mod session_view;
mod sessions;
mod terminal;

use nsh_core::prelude::*;

/// Native services available to the application UI.
#[native_module("termlevel/native")]
fn native(registry: &mut Registry) {
    #[cfg(all(debug_assertions, target_os = "macos"))]
    registry.module::<session_view::native_probe::NativeInputProbe>();
    registry
        .module::<runtime::Runtime>()
        .module::<sessions::Sessions>()
        .register::<dialog_keys::DialogKeys>()
        .register::<dialog_keys::ResizeKeys>()
        .register::<terminal::TerminalEngine>()
        .register::<runtime::TerminalMetrics>()
        .register::<session_view::SessionTerminal>();
}
