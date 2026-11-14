//! Terminal platform boundary. The selected backend owns native objects and
//! event adapters; the controller, snapshots and scene channel stay portable.
//! A backend supplies surface mounting, physical input and host registration.
//! Keep these concrete, compile-time hooks until more than one backend exists.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as backend;

#[cfg(not(target_os = "macos"))]
mod unsupported;
#[cfg(not(target_os = "macos"))]
use unsupported as backend;

pub(super) use backend::{install_input, surface, synthetic_keys};

#[cfg(debug_assertions)]
pub(super) use backend::track;

// Host registration necessarily exposes the selected platform's NSH type.
// These are the only platform-specific entry points outside this module tree.
#[cfg(all(debug_assertions, target_os = "macos"))]
pub(crate) use macos::probe::NativeInputProbe;
#[cfg(target_os = "macos")]
pub(crate) use macos::register_components;
