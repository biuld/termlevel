use nsh_core::prelude::*;

/// Build information and the application environment.
#[module]
pub struct Runtime;

#[module]
impl Runtime {
    /// The application version compiled into the native crate.
    pub fn version() -> String {
        env!("CARGO_PKG_VERSION").to_owned()
    }

    /// Enables the explicit deterministic preview entry for UI-only testing.
    pub fn preview_mode() -> bool {
        std::env::var("TERMLEVEL_PREVIEW").as_deref() == Ok("1")
    }

    /// Existing initial directory for a new workspace.
    pub fn cwd() -> String {
        std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("/"))
            .to_string_lossy()
            .into_owned()
    }
}
