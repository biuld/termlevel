//! Rust owns processes and terminal state, independent of any view/JS scope.

use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock, Weak,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, SyncSender},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use nsh_base::{TerminalEmulator, TerminalSize};
use nsh_core::prelude::*;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};

use crate::terminal::{BackendEvent, GhosttyEmulator, Rgb};

/// Low-frequency session metadata. No terminal output crosses this interface.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct SessionInfo {
    /// Stable application identity, independent of PID or view identity.
    pub id: String,
    /// Initial working directory; shell integration may later track changes.
    pub cwd: String,
    /// Most recently reported terminal title.
    pub title: String,
    /// running, exited, error, or closed.
    pub state: String,
    /// Exit code, after the process has been reaped.
    pub exit_code: Option<u32>,
    /// Startup or I/O error, when one occurred.
    pub error: Option<String>,
    /// Diagnostic PID only. Never used as the session identity.
    pub pid: Option<u32>,
}

/// Outcome of the one ordered mouse input path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MouseReport {
    /// The terminal is in a mouse mode and the event was encoded.
    Encoded,
    /// Not a program report (Shift or no mouse mode); the caller owns it.
    NotReporting { held: bool },
}

struct Control {
    master: Option<Box<dyn MasterPty + Send>>,
    child: Box<dyn Child + Send + Sync>,
    input: Option<SyncSender<Vec<u8>>>,
}

pub(crate) struct Session {
    pub(crate) emulator: Mutex<GhosttyEmulator>,
    pub(crate) revision: AtomicU64,
    pub(crate) bells: AtomicU64,
    pub(crate) input_epoch: AtomicU64,
    input_wait_ns: AtomicU64,
    input_total_ns: AtomicU64,
    input_count: AtomicU64,
    info: Mutex<SessionInfo>,
    control: Mutex<Control>,
    closed: AtomicBool,
    workers: Mutex<Vec<thread::JoinHandle<()>>>,
}

impl Session {
    fn start(
        id: String,
        cwd: String,
        mut command: CommandBuilder,
        colors: (Option<&str>, Option<&str>),
        scrollback: usize,
    ) -> Result<Arc<Self>, String> {
        command.cwd(&cwd);
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        command.env("TERM_PROGRAM", "termlevel");
        command.env("TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));
        let emulator = GhosttyEmulator::try_new(TerminalSize::default(), scrollback)?;
        let pair = native_pty_system()
            .openpty(PtySize::default())
            .map_err(|e| e.to_string())?;
        let mut writer = pair.master.take_writer().map_err(|e| e.to_string())?;
        #[cfg(unix)]
        let mut reader = interruptible_reader(&*pair.master)?;
        #[cfg(not(unix))]
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|e| e.to_string())?;
        drop(pair.slave);
        let pid = child.process_id();
        let (input, receive) = mpsc::sync_channel::<Vec<u8>>(128);
        let session = Arc::new(Self {
            emulator: Mutex::new(emulator),
            revision: AtomicU64::new(1),
            bells: AtomicU64::new(0),
            input_epoch: AtomicU64::new(0),
            input_wait_ns: AtomicU64::new(0),
            input_total_ns: AtomicU64::new(0),
            input_count: AtomicU64::new(0),
            info: Mutex::new(SessionInfo {
                id,
                cwd,
                title: String::new(),
                state: "running".into(),
                exit_code: None,
                error: None,
                pid,
            }),
            control: Mutex::new(Control {
                master: Some(pair.master),
                child,
                input: Some(input),
            }),
            closed: AtomicBool::new(false),
            workers: Mutex::new(Vec::new()),
        });

        session.set_colors(colors.0, colors.1);
        let read_session = session.clone();
        let read_thread = thread::spawn(move || {
            let mut buffer = [0; 8192];
            while !read_session.closed.load(Ordering::Acquire) {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => read_session.output(&buffer[..count]),
                    Err(e)
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::Interrupted | std::io::ErrorKind::WouldBlock
                        ) =>
                    {
                        read_session.flush_output(false);
                        thread::sleep(Duration::from_millis(4));
                    }
                    // A PTY master commonly returns EIO when the slave closes.
                    Err(e) if cfg!(unix) && e.raw_os_error() == Some(libc::EIO) => break,
                    Err(e) => {
                        read_session.fail(format!("Reading terminal: {e}"));
                        break;
                    }
                }
            }
            if !read_session.closed.load(Ordering::Acquire) {
                read_session.flush_output(true);
            }
        });

        let write_session = session.clone();
        let write_thread = thread::spawn(move || {
            while !write_session.closed.load(Ordering::Acquire) {
                let data = match receive.recv_timeout(Duration::from_millis(25)) {
                    Ok(data) => data,
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                };
                let mut remaining = data.as_slice();
                while !remaining.is_empty() && !write_session.closed.load(Ordering::Acquire) {
                    match writer.write(remaining) {
                        Ok(0) => {
                            write_session.fail("Terminal input closed".into());
                            return;
                        }
                        Ok(count) => remaining = &remaining[count..],
                        Err(e)
                            if matches!(
                                e.kind(),
                                std::io::ErrorKind::Interrupted | std::io::ErrorKind::WouldBlock
                            ) =>
                        {
                            thread::sleep(Duration::from_millis(2))
                        }
                        Err(e) => {
                            write_session.fail(format!("Writing terminal: {e}"));
                            return;
                        }
                    }
                }
            }
        });

        let wait_session = session.clone();
        let wait_thread = thread::spawn(move || {
            let mut closing_since = None;
            loop {
                // Reaping and close serialize on the same lock. In particular,
                // close never signals a PID/group after it has been reaped and
                // possibly reused by an unrelated process.
                let mut control = wait_session.control.lock().unwrap();
                match control.child.try_wait() {
                    Ok(Some(status)) => {
                        let mut info = wait_session.info.lock().unwrap();
                        info.exit_code = Some(status.exit_code());
                        if !wait_session.closed.load(Ordering::Acquire) {
                            info.state = "exited".into();
                        }
                        control.input.take();
                        wait_session.revision.fetch_add(1, Ordering::Release);
                        break;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        wait_session.fail(format!("Waiting for shell: {e}"));
                        break;
                    }
                }
                if wait_session.closed.load(Ordering::Acquire) {
                    let since = closing_since.get_or_insert_with(Instant::now);
                    if since.elapsed() >= Duration::from_millis(50) {
                        // Give an interactive shell time to HUP its background
                        // jobs before forcing a shell that ignores hangup.
                        #[cfg(unix)]
                        if let Some(pid) = control.child.process_id() {
                            unsafe {
                                libc::killpg(pid as libc::pid_t, libc::SIGKILL);
                            }
                        }
                        let _ = control.child.kill();
                    }
                }
                drop(control);
                thread::sleep(Duration::from_millis(10));
            }
        });
        session
            .workers
            .lock()
            .unwrap()
            .extend([read_thread, write_thread, wait_thread]);
        Ok(session)
    }

    fn output(&self, bytes: &[u8]) {
        let events = {
            let mut emulator = self.emulator.lock().unwrap();
            emulator.feed(bytes);
            emulator.flush_sync(Instant::now(), false);
            emulator.backend_events()
        };
        self.handle_events(events);
        self.revision.fetch_add(1, Ordering::Release);
    }

    fn flush_output(&self, force: bool) {
        let events = {
            let mut emulator = self.emulator.lock().unwrap();
            if !emulator.flush_sync(Instant::now(), force) {
                return;
            }
            emulator.backend_events()
        };
        self.handle_events(events);
        self.revision.fetch_add(1, Ordering::Release);
    }

    pub(crate) fn set_scrollback(&self, lines: usize) {
        let events = {
            let mut emulator = self.emulator.lock().unwrap();
            emulator.set_scrollback(lines);
            emulator.backend_events()
        };
        self.handle_events(events);
        self.revision.fetch_add(1, Ordering::Release);
    }

    fn handle_events(&self, events: Vec<BackendEvent>) {
        for event in events {
            match event {
                BackendEvent::Title(title) => self.info.lock().unwrap().title = title,
                BackendEvent::Reply(bytes) => self.input(&bytes),
                BackendEvent::Bell => {
                    self.bells.fetch_add(1, Ordering::Release);
                }
                BackendEvent::Error(error) => self.fail(format!("Ghostty: {error}")),
            }
        }
    }

    pub(crate) fn info(&self) -> SessionInfo {
        self.info.lock().unwrap().clone()
    }

    pub(crate) fn set_colors(&self, foreground: Option<&str>, background: Option<&str>) {
        fn rgb(value: &str) -> Option<Rgb> {
            let value = value.strip_prefix('#')?;
            if !value.is_ascii() || (value.len() != 6 && value.len() != 8) {
                return None;
            }
            Some(Rgb {
                r: u8::from_str_radix(&value[0..2], 16).ok()?,
                g: u8::from_str_radix(&value[2..4], 16).ok()?,
                b: u8::from_str_radix(&value[4..6], 16).ok()?,
            })
        }
        let (changed, events) = {
            let mut emulator = self.emulator.lock().unwrap();
            let changed = emulator.set_colors(foreground.and_then(rgb), background.and_then(rgb));
            (changed, emulator.backend_events())
        };
        self.handle_events(events);
        if changed {
            self.revision.fetch_add(1, Ordering::Release);
        }
    }

    pub(crate) fn input(&self, bytes: &[u8]) {
        if self.closed.load(Ordering::Acquire) || self.info.lock().unwrap().exit_code.is_some() {
            return;
        }
        // Bound pending input. A rejected paste is visible instead of silently
        // truncating data or blocking the engine behind a stalled process.
        if bytes.len() > 1024 * 1024 {
            self.fail("Paste exceeds the 1 MiB input limit".into());
            return;
        }
        let result = self
            .control
            .lock()
            .unwrap()
            .input
            .as_ref()
            .map(|input| input.try_send(bytes.to_vec()));
        if let Some(Err(error)) = result {
            self.fail(format!("Terminal input unavailable: {error}"));
        }
    }

    fn encoded_input(&self, encoded: Result<Vec<u8>, String>) {
        match encoded {
            Ok(bytes) if !bytes.is_empty() => self.input(&bytes),
            Ok(_) => {}
            Err(error) => self.fail(format!("Encoding terminal input: {error}")),
        }
    }
    /// Max lock wait and total encode+send time for input since the last call.
    /// Diagnostic only: the counters are reset by the display summary.
    pub(crate) fn input_probe(&self) -> (u64, u64, u64) {
        (
            self.input_count.swap(0, Ordering::Relaxed),
            self.input_wait_ns.swap(0, Ordering::Relaxed),
            self.input_total_ns.swap(0, Ordering::Relaxed),
        )
    }
    fn observe_input(&self, started: Instant, waited: Duration) {
        self.input_count.fetch_add(1, Ordering::Relaxed);
        self.input_wait_ns
            .fetch_max(waited.as_nanos() as u64, Ordering::Relaxed);
        self.input_total_ns
            .fetch_max(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }
    pub(crate) fn text_input(&self, text: &str) {
        let started = Instant::now();
        self.input_epoch.fetch_add(1, Ordering::Release);
        let lock_at = Instant::now();
        let encoded = self.emulator.lock().unwrap().encode_text(text);
        let waited = lock_at.elapsed();
        self.encoded_input(encoded);
        self.observe_input(started, waited);
    }
    pub(crate) fn key_input(&self, event: &termlevel_ghostty_vt::KeyEvent<'_>) {
        let started = Instant::now();
        if event.action != termlevel_ghostty_vt::raw::GHOSTTY_KEY_ACTION_RELEASE {
            self.input_epoch.fetch_add(1, Ordering::Release);
        }
        let lock_at = Instant::now();
        let encoded = self.emulator.lock().unwrap().encode_key(event);
        let waited = lock_at.elapsed();
        self.encoded_input(encoded);
        self.observe_input(started, waited);
    }

    /// Encodes mouse reports only when the terminal's current mode accepts
    /// them. The mode check and the encoding share one lock, so an event can
    /// never be classified under one mode and encoded under another. Every
    /// mouse entrance goes through this single ordered path.
    ///
    /// Returns `NotReporting` with the hold state observed under the same lock,
    /// which lets a history scroll avoid a second lock.
    pub(crate) fn mouse_reports(
        &self,
        shift: bool,
        events: &[(termlevel_ghostty_vt::MouseEvent, usize)],
        cw: u32,
        lh: u32,
    ) -> MouseReport {
        let started = Instant::now();
        let mut bytes = Vec::new();
        let waited;
        {
            let lock_at = Instant::now();
            let mut emulator = self.emulator.lock().unwrap();
            waited = lock_at.elapsed();
            if shift || !emulator.mouse_enabled() {
                return MouseReport::NotReporting {
                    held: emulator.render_held(),
                };
            }
            for (event, count) in events {
                for _ in 0..*count {
                    match emulator.encode_mouse(*event, cw, lh) {
                        Ok(part) => bytes.extend(part),
                        Err(error) => {
                            drop(emulator);
                            self.fail(format!("Encoding terminal input: {error}"));
                            return MouseReport::Encoded;
                        }
                    }
                }
            }
        }
        if !bytes.is_empty() {
            // All mouse entrances ask the display to follow before its next
            // published window. A rejected send stays visible; it is not a
            // delivery guarantee. A realistic event fits one message, so the
            // sequence is never split; only an absurd burst is chunked.
            self.input_epoch.fetch_add(1, Ordering::Release);
            if bytes.len() <= 1024 * 1024 {
                self.input(&bytes);
            } else {
                for chunk in bytes.chunks(1024 * 1024) {
                    self.input(chunk);
                }
            }
        }
        self.observe_input(started, waited);
        MouseReport::Encoded
    }
    pub(crate) fn paste(&self, text: &str) {
        let started = Instant::now();
        self.input_epoch.fetch_add(1, Ordering::Release);
        let lock_at = Instant::now();
        let encoded = self.emulator.lock().unwrap().encode_paste(text);
        let waited = lock_at.elapsed();
        self.encoded_input(encoded);
        self.observe_input(started, waited);
    }
    pub(crate) fn focus_input(&self, focused: bool) {
        let encoded = self.emulator.lock().unwrap().encode_focus(focused);
        self.encoded_input(encoded);
    }

    #[cfg(test)]
    pub(crate) fn resize(&self, size: TerminalSize) {
        self.resize_grid(size, 0, 0);
    }

    pub(crate) fn resize_grid(&self, size: TerminalSize, cw: u32, lh: u32) {
        if self.closed.load(Ordering::Acquire) {
            return;
        }
        // Hold the parser lock across the kernel resize: SIGWINCH may cause
        // immediate output, which must see the matching grid dimensions.
        let mut emulator = self.emulator.lock().unwrap();
        let result = self.control.lock().unwrap().master.as_ref().map(|master| {
            master.resize(PtySize {
                rows: size.rows,
                cols: size.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
        });
        if let Some(Err(error)) = result {
            self.fail(format!("Resizing terminal: {error}"));
            return;
        }
        emulator.resize_cells(size, cw, lh);
        let events = emulator.backend_events();
        drop(emulator);
        self.handle_events(events);
        self.revision.fetch_add(1, Ordering::Release);
    }

    pub(crate) fn fail(&self, error: String) {
        let mut info = self.info.lock().unwrap();
        if self.closed.load(Ordering::Acquire) {
            return;
        }
        info.state = "error".into();
        info.error = Some(error);
        self.revision.fetch_add(1, Ordering::Release);
    }

    pub(crate) fn close(&self) {
        if self.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        let mut control = self.control.lock().unwrap();
        let running = matches!(control.child.try_wait(), Ok(None));
        #[cfg(unix)]
        if running {
            let foreground = control
                .master
                .as_ref()
                .and_then(|master| master.process_group_leader());
            let shell = self.info.lock().unwrap().pid.map(|pid| pid as libc::pid_t);
            // Terminate foreground jobs immediately; let the shell's hangup
            // handler clean up background jobs before the reaper's deadline.
            for group in [foreground, shell].into_iter().flatten() {
                unsafe {
                    if group > 1 && group != libc::getpgrp() {
                        libc::killpg(group, libc::SIGHUP);
                        if Some(group) != shell {
                            libc::killpg(group, libc::SIGKILL);
                        }
                    }
                }
            }
        }
        #[cfg(not(unix))]
        if running {
            let _ = control.child.kill();
        }
        control.input.take();
        control.master.take();
        self.info.lock().unwrap().state = "closed".into();
        self.revision.fetch_add(1, Ordering::Release);
    }
}

// O_NONBLOCK belongs to the shared PTY file description. Both worker loops
// handle WouldBlock and check cancellation, so close never strands a reader
// if a descendant happens to retain its end of the PTY.
#[cfg(unix)]
fn interruptible_reader(master: &dyn MasterPty) -> Result<std::fs::File, String> {
    use std::os::fd::FromRawFd;
    let fd = master
        .as_raw_fd()
        .ok_or("PTY has no native file descriptor")?;
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        if flags < 0 || libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) < 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let copy = libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 0);
        if copy < 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(std::fs::File::from_raw_fd(copy))
    }
}

enum Entry {
    Session(Arc<Session>),
    Failed(SessionInfo),
}

pub(crate) struct SessionManager {
    entries: BTreeMap<String, Entry>,
    closing: Vec<Weak<Session>>,
    scrollback: usize,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
            closing: Vec::new(),
            scrollback: crate::terminal::SCROLLBACK_LIMIT,
        }
    }
}

impl SessionManager {
    #[cfg(test)]
    fn open(&mut self, cwd: String, command: CommandBuilder) -> SessionInfo {
        self.open_with_colors(cwd, command, (None, None))
    }

    fn open_with_colors(
        &mut self,
        cwd: String,
        command: CommandBuilder,
        colors: (Option<&str>, Option<&str>),
    ) -> SessionInfo {
        static SEQUENCE: AtomicU64 = AtomicU64::new(1);
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let id = format!(
            "session-{time:x}-{:x}",
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let scrollback = self.scrollback;
        let entry = match resolve_cwd(&cwd)
            .and_then(|cwd| Session::start(id.clone(), cwd, command, colors, scrollback))
        {
            Ok(session) => Entry::Session(session),
            Err(error) => Entry::Failed(SessionInfo {
                id: id.clone(),
                cwd,
                title: String::new(),
                state: "error".into(),
                exit_code: None,
                error: Some(error),
                pid: None,
            }),
        };
        let info = match &entry {
            Entry::Session(session) => session.info(),
            Entry::Failed(info) => info.clone(),
        };
        self.entries.insert(id, entry);
        info
    }

    pub(crate) fn get(&self, id: &str) -> Result<Arc<Session>, String> {
        match self.entries.get(id) {
            Some(Entry::Session(session)) => Ok(session.clone()),
            Some(Entry::Failed(info)) => Err(info.error.clone().unwrap_or_default()),
            None => Err("This terminal session is closed or missing".into()),
        }
    }

    pub(crate) fn scrollback(&self) -> usize {
        self.scrollback
    }

    pub(crate) fn set_scrollback(&mut self, lines: usize) {
        let lines = crate::terminal::clamp_scrollback(lines);
        self.scrollback = lines;
        for entry in self.entries.values() {
            if let Entry::Session(session) = entry {
                session.set_scrollback(lines);
            }
        }
    }

    fn list(&self) -> Vec<SessionInfo> {
        self.entries
            .values()
            .map(|entry| match entry {
                Entry::Session(session) => session.info(),
                Entry::Failed(info) => info.clone(),
            })
            .collect()
    }

    fn close(&mut self, id: &str) {
        if let Some(Entry::Session(session)) = self.entries.remove(id) {
            session.close();
            self.closing.push(Arc::downgrade(&session));
        }
        self.settled();
    }

    fn shutdown(&mut self) {
        for (_, entry) in std::mem::take(&mut self.entries) {
            if let Entry::Session(session) = entry {
                session.close();
                self.closing.push(Arc::downgrade(&session));
            }
        }
    }

    fn settled(&mut self) -> bool {
        self.closing.retain(|session| {
            session.upgrade().is_some_and(|session| {
                !session
                    .workers
                    .lock()
                    .unwrap()
                    .iter()
                    .all(thread::JoinHandle::is_finished)
            })
        });
        self.closing.is_empty()
    }
}

impl Drop for SessionManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub(crate) fn manager() -> &'static Mutex<SessionManager> {
    static MANAGER: OnceLock<Mutex<SessionManager>> = OnceLock::new();
    MANAGER.get_or_init(Mutex::default)
}

fn resolve_cwd(cwd: &str) -> Result<String, String> {
    let path = if cwd == "~" {
        home_directory()
    } else if let Some(relative) = cwd.strip_prefix("~/") {
        home_directory().join(relative)
    } else {
        PathBuf::from(cwd)
    };
    let path = path
        .canonicalize()
        .map_err(|e| format!("Cannot open directory {}: {e}", path.display()))?;
    if !path.is_dir() {
        return Err(format!("Not a directory: {}", path.display()));
    }
    Ok(path.to_string_lossy().into_owned())
}

fn home_directory() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

fn default_shell() -> PathBuf {
    if let Some(shell) = std::env::var_os("SHELL") {
        let path = PathBuf::from(shell);
        if path.is_absolute() && path.is_file() {
            return path;
        }
    }
    #[cfg(unix)]
    unsafe {
        let mut passwd = std::mem::zeroed::<libc::passwd>();
        let mut result = std::ptr::null_mut();
        let mut buffer = vec![0; 16384];
        if libc::getpwuid_r(
            libc::getuid(),
            &mut passwd,
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut result,
        ) == 0
            && !result.is_null()
            && !passwd.pw_shell.is_null()
        {
            let shell = std::ffi::CStr::from_ptr(passwd.pw_shell).to_string_lossy();
            if Path::new(shell.as_ref()).is_file() {
                return PathBuf::from(shell.as_ref());
            }
        }
    }
    PathBuf::from(if cfg!(windows) { "cmd.exe" } else { "/bin/sh" })
}

mod api;
pub use api::Sessions;

#[cfg(test)]
mod tests;
