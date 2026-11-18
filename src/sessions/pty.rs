use super::*;

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn locale_is_utf8(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    value == "utf-8" || value == "utf8" || value.ends_with(".utf-8") || value.ends_with(".utf8")
}

/// Locale assignments for a new shell. The effective charset is `LC_ALL`,
/// then `LC_CTYPE`, then `LANG`. An existing UTF-8 locale is kept.
pub(super) fn utf8_locale_env(
    lc_all: Option<String>,
    lc_ctype: Option<String>,
    lang: Option<String>,
) -> Vec<(&'static str, &'static str)> {
    let effective = lc_all
        .as_deref()
        .or(lc_ctype.as_deref())
        .or(lang.as_deref());
    if effective.is_some_and(locale_is_utf8) {
        return Vec::new();
    }
    let mut assigned = Vec::new();
    if lc_all.is_some() {
        assigned.push(("LC_ALL", "en_US.UTF-8"));
    }
    if lc_ctype.is_some() {
        assigned.push(("LC_CTYPE", "en_US.UTF-8"));
    }
    if !lang.as_deref().is_some_and(locale_is_utf8) {
        assigned.push(("LANG", "en_US.UTF-8"));
    }
    assigned
}

impl Control {
    pub(super) fn foreground_group(&self) -> Option<i32> {
        #[cfg(unix)]
        return self
            .master
            .as_ref()?
            .process_group_leader()
            .filter(|group| *group > 1);
        #[cfg(not(unix))]
        None
    }
}

impl Session {
    pub(super) fn start(
        id: String,
        cwd: String,
        mut command: CommandBuilder,
        colors: (Option<&str>, Option<&str>),
        scrollback: usize,
        identity: Option<&str>,
    ) -> Result<Arc<Self>, String> {
        command.cwd(&cwd);
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        // Grok copies with both its own pbcopy and OSC 52. pbcopy uses the
        // session locale; without a UTF-8 locale it stores MacRoman and the
        // later paste is mojibake even when OSC 52 carried UTF-8.
        for (key, value) in utf8_locale_env(
            env_nonempty("LC_ALL"),
            env_nonempty("LC_CTYPE"),
            env_nonempty("LANG"),
        ) {
            command.env(key, value);
        }
        // Identity is a compatibility hint, not a change of terminal engine.
        // Only known profiles are accepted, including at the native boundary.
        if identity == Some("ghostty") {
            command.env("TERM_PROGRAM", "ghostty");
            command.env_remove("TERM_PROGRAM_VERSION");
        } else {
            command.env("TERM_PROGRAM", "termlevel");
            command.env("TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));
        }
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
        let session = Arc::new(Self {
            emulator: Mutex::new(emulator),
            revision: AtomicU64::new(1),
            bells: AtomicU64::new(0),
            input_epoch: AtomicU64::new(0),
            input_wait_ns: AtomicU64::new(0),
            input_total_ns: AtomicU64::new(0),
            input_count: AtomicU64::new(0),
            input: InputQueue::default(),
            info: Mutex::new(SessionInfo {
                id,
                cwd,
                title: String::new(),
                state: "running".into(),
                exit_code: None,
                error: None,
                input_warning: None,
                pid,
                progress: None,
                activity: None,
                current_cwd: None,
                mouse_shape: "text".into(),
                at_prompt: false,
                shell_phase: "unknown".into(),
                status_revision: 0,
                status: TerminalStatusInfo::default(),
            }),
            status: Mutex::new(StatusState::default()),
            control: Mutex::new(Control {
                master: Some(pair.master),
                child,
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
                        #[cfg(unix)]
                        wait_for_output(&reader);
                        #[cfg(not(unix))]
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
                let Some(batch) = write_session.input.next() else {
                    break;
                };
                let mut remaining = batch.bytes.as_slice();
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
                if remaining.is_empty() && write_session.input.delivered(&batch) {
                    write_session.revision.fetch_add(1, Ordering::Release);
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
                        let mut programs = wait_session.status.lock().unwrap();
                        programs
                            .records
                            .retain(|_, record| result_state(record.state));
                        programs.agent = None;
                        let mut info = wait_session.info.lock().unwrap();
                        info.status.agent_app = None;
                        info.exit_code = Some(status.exit_code());
                        info.progress = None;
                        info.activity = aggregate(&programs.records);
                        info.mouse_shape = "text".into();
                        info.at_prompt = false;
                        info.shell_phase = "unknown".into();
                        if !wait_session.closed.load(Ordering::Acquire) {
                            info.state = "exited".into();
                            if status.exit_code() != 0 {
                                crate::notifications::record(
                                    "app",
                                    "warning",
                                    Some(&info.id),
                                    "Terminal exited",
                                    &format!("Process exited with code {}.", status.exit_code()),
                                    &info.title,
                                );
                            }
                        }
                        wait_session.input.close();
                        wait_session.revision.fetch_add(1, Ordering::Release);
                        break;
                    }
                    Ok(None) => {
                        wait_session.check_agent_foreground(&control);
                    }
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

    fn check_agent_foreground(&self, control: &Control) {
        let mut status = self.status.lock().unwrap();
        if let Some(group) = status.agent.as_ref().and_then(|agent| agent.foreground)
            && control
                .foreground_group()
                .is_some_and(|current| current != group)
        {
            status.agent = None;
            self.info.lock().unwrap().status.agent_app = None;
            self.revision.fetch_add(1, Ordering::Release);
        }
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
        if self.closed.load(Ordering::Acquire) {
            return;
        }
        let mut status = self.status.lock().unwrap();
        status.records.clear();
        status.agent = None;
        let mut info = self.info.lock().unwrap();
        let id = info.id.clone();
        let source_title = info.title.clone();
        let changed = info.error.as_ref() != Some(&error);
        info.state = "error".into();
        info.error = Some(error.clone());
        info.progress = None;
        info.activity = None;
        info.status.agent_app = None;
        info.mouse_shape = "text".into();
        info.at_prompt = false;
        info.shell_phase = "unknown".into();
        drop(info);
        if changed {
            crate::notifications::record(
                "app",
                "error",
                Some(&id),
                "Terminal error",
                &error,
                &source_title,
            );
        }
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
        self.input.close();
        control.master.take();
        let mut status = self.status.lock().unwrap();
        status.records.clear();
        status.agent = None;
        {
            let mut info = self.info.lock().unwrap();
            info.state = "closed".into();
            info.progress = None;
            info.activity = None;
            info.status.agent_app = None;
            info.current_cwd = None;
            info.mouse_shape = "text".into();
            info.at_prompt = false;
            info.shell_phase = "unknown".into();
        }
        self.revision.fetch_add(1, Ordering::Release);
    }
}

// Wake as soon as the child supplies another chunk instead of adding a
// polling sleep to every gap in a large graphics transfer. The bounded wait
// still lets the reader notice session close and commit a sync deadline.
#[cfg(unix)]
fn wait_for_output(reader: &std::fs::File) {
    use std::os::fd::AsRawFd;
    let mut descriptor = libc::pollfd {
        fd: reader.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: descriptor is valid for this call and reader owns the live fd.
    unsafe {
        libc::poll(&mut descriptor, 1, 4);
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
