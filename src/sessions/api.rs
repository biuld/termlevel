use super::*;

/// Explicit process operations. Unmounting a view calls none of these.
#[module]
pub struct Sessions;

#[module]
impl Sessions {
    /// Opens a real PTY using the user's login shell in an existing directory.
    pub fn open(
        cwd: String,
        foreground: Option<String>,
        background: Option<String>,
        shell: Option<String>,
        identity: Option<String>,
    ) -> SessionInfo {
        let mut command =
            CommandBuilder::new(shell.map(PathBuf::from).unwrap_or_else(default_shell));
        #[cfg(unix)]
        {
            command.arg("-l");
            command.arg("-i");
        }
        manager().lock().unwrap().open_with_colors(
            cwd,
            command,
            (foreground.as_deref(), background.as_deref()),
            identity.as_deref(),
        )
    }

    /// Existing sessions for rebinding after UI hot reload, including failures.
    pub fn list() -> Vec<SessionInfo> {
        manager().lock().unwrap().list()
    }

    /// Terminates a session and removes its identity. Safe to call twice.
    pub fn close(id: String) {
        manager().lock().unwrap().close(&id);
    }

    /// Marks done, blocked and error program status as seen by the user.
    pub fn acknowledge(id: String, observed_revision: u64) {
        manager()
            .lock()
            .unwrap()
            .acknowledge(&id, observed_revision);
    }

    /// Terminates all sessions on explicit window/application close.
    pub fn shutdown() {
        manager().lock().unwrap().shutdown();
    }

    /// Whether explicitly closed sessions have reaped their child and stopped I/O.
    pub fn settled() -> bool {
        manager().lock().unwrap().settled()
    }

    /// Allows terminal programs to read the system clipboard via OSC52.
    pub fn set_clipboard_read(allow: bool) {
        crate::clipboard::allow_read(allow);
    }

    /// Lines kept above the screen. Applies to open sessions and the next one opened.
    pub fn set_scrollback(lines: u32) {
        manager().lock().unwrap().set_scrollback(lines as usize);
    }
}
