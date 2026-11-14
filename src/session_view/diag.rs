//! Debug-only diagnostic sink. A GUI run's stderr is not the development
//! host's terminal, so summaries are written to a file instead. `nsh run`
//! passes the environment through, so a custom path can be selected; the
//! default is `/tmp/termlevel-surface.log`, truncated once per app process.
use std::sync::{Mutex, OnceLock};

#[cfg(debug_assertions)]
fn open_sink() -> Option<Mutex<std::fs::File>> {
    use std::io::Write;
    let path = std::env::var_os("TERMLEVEL_SURFACE_LOG")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp/termlevel-surface.log"));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .ok()?;
    let _ = writeln!(file, "# termlevel surface diagnostics");
    Some(Mutex::new(file))
}

pub(super) fn line(text: &str) {
    #[cfg(debug_assertions)]
    {
        use std::io::Write;
        static SINK: OnceLock<Option<Mutex<std::fs::File>>> = OnceLock::new();
        if let Some(file) = SINK.get_or_init(open_sink)
            && let Ok(mut file) = file.lock()
        {
            let _ = writeln!(file, "{text}");
            let _ = file.flush();
        }
    }
    #[cfg(not(debug_assertions))]
    let _ = text;
}
