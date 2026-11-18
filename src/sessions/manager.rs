use super::*;

enum Entry {
    Session(Arc<Session>),
    Failed(Box<SessionInfo>),
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
    pub(super) fn open(&mut self, cwd: String, command: CommandBuilder) -> SessionInfo {
        self.open_with_colors(cwd, command, (None, None), None)
    }

    pub(super) fn open_with_colors(
        &mut self,
        cwd: String,
        command: CommandBuilder,
        colors: (Option<&str>, Option<&str>),
        identity: Option<&str>,
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
            .and_then(|cwd| Session::start(id.clone(), cwd, command, colors, scrollback, identity))
        {
            Ok(session) => Entry::Session(session),
            Err(error) => Entry::Failed(Box::new(SessionInfo {
                id: id.clone(),
                cwd,
                title: String::new(),
                state: "error".into(),
                exit_code: None,
                error: Some(error),
                input_warning: None,
                pid: None,
                progress: None,
                activity: None,
                current_cwd: None,
                mouse_shape: "text".into(),
                at_prompt: false,
                shell_phase: "unknown".into(),
                status_revision: 0,
                status: TerminalStatusInfo::default(),
            })),
        };
        let info = match &entry {
            Entry::Session(session) => session.info(),
            Entry::Failed(info) => info.as_ref().clone().resolved(),
        };
        if let Some(error) = &info.error {
            crate::notifications::record(
                "app",
                "error",
                Some(&id),
                "Terminal could not start",
                error,
                &info.title,
            );
        }
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

    pub(crate) fn acknowledge(&self, id: &str, observed_revision: u64) {
        if let Ok(session) = self.get(id) {
            session.acknowledge(observed_revision);
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

    pub(super) fn list(&self) -> Vec<SessionInfo> {
        self.entries
            .values()
            .map(|entry| match entry {
                Entry::Session(session) => session.info(),
                Entry::Failed(info) => info.as_ref().clone().resolved(),
            })
            .collect()
    }

    pub(super) fn close(&mut self, id: &str) {
        if let Some(Entry::Session(session)) = self.entries.remove(id) {
            session.close();
            self.closing.push(Arc::downgrade(&session));
        }
        self.settled();
    }

    pub(super) fn shutdown(&mut self) {
        for (_, entry) in std::mem::take(&mut self.entries) {
            if let Entry::Session(session) = entry {
                session.close();
                self.closing.push(Arc::downgrade(&session));
            }
        }
    }

    pub(super) fn settled(&mut self) -> bool {
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

pub(crate) fn home_directory() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

pub(super) fn default_shell() -> PathBuf {
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
