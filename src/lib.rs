//! Registration only. Session/process ownership and UI bindings live in their modules.

mod clipboard;
mod dialog_keys;
mod git;
mod notifications;
mod runtime;
mod session_view;
mod sessions;
mod terminal;

use nsh_core::prelude::*;

/// Native services available to the application UI.
#[native_module("termlevel/native")]
fn native(registry: &mut Registry) {
    #[cfg(all(debug_assertions, target_os = "macos"))]
    registry.module::<session_view::platform::NativeInputProbe>();
    registry
        .module::<runtime::Runtime>()
        .module::<notifications::Notifications>()
        .module::<git::Git>()
        .module::<sessions::Sessions>()
        .register::<dialog_keys::DialogKeys>()
        .register::<dialog_keys::ResizeKeys>()
        .register::<terminal::TerminalEngine>()
        .register::<runtime::TerminalMetrics>()
        .register::<session_view::SessionTerminal>();
}

/// Register the app-owned AppKit renderer in development and packaged hosts.
#[cfg(target_os = "macos")]
#[native_components(macos)]
fn native_components(components: &nsh_platform_macos::Components) {
    session_view::platform::register_components(components);
}
