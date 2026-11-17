use super::*;

pub(crate) struct Captured {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
    pub(crate) truncated: bool,
}

pub(crate) fn git_path() -> Result<PathBuf, String> {
    static PATH: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    PATH.get_or_init(|| requests::uncancelled(resolve_git))
        .clone()
}

pub(super) fn resolve_git() -> Result<PathBuf, String> {
    let path = find_git().ok_or_else(|| {
        "Git is not installed or is not on the application's PATH. \
         Install Git 2.20 or newer, then reopen this overview."
            .to_string()
    })?;
    let output = run_capture(&path, None, &["--version"], VERSION_TIMEOUT, 4096)
        .map_err(|error| format!("Git could not be started: {error}"))?;
    if !output.status.success() {
        return Err(
            "Git could not be started. Run `git --version` in a terminal to check it.".into(),
        );
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let version = parse_version(&text)
        .ok_or_else(|| format!("Git version could not be read: {}", text.trim()))?;
    if version < MIN_GIT {
        return Err(format!(
            "Git {}.{}.{} is too old. Termlevel needs Git 2.20 or newer.",
            version.0, version.1, version.2
        ));
    }
    Ok(path)
}

pub(super) fn find_git() -> Option<PathBuf> {
    let name = if cfg!(windows) { "git.exe" } else { "git" };
    let mut seen = HashSet::new();
    let mut dirs = Vec::new();
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }
    #[cfg(unix)]
    dirs.extend(
        ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"]
            .into_iter()
            .map(PathBuf::from),
    );
    for dir in dirs {
        if !seen.insert(dir.clone()) {
            continue;
        }
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub(super) fn parse_version(text: &str) -> Option<(u32, u32, u32)> {
    let token = text
        .strip_prefix("git version ")?
        .split_whitespace()
        .next()?;
    let mut parts = token.split('.');
    Some((
        leading_number(parts.next()?)?,
        leading_number(parts.next().unwrap_or("0"))?,
        leading_number(parts.next().unwrap_or("0"))?,
    ))
}

pub(super) fn leading_number(text: &str) -> Option<u32> {
    let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

pub(crate) fn run_capture(
    git: &Path,
    root: Option<&Path>,
    args: &[&str],
    timeout: Duration,
    max_stdout: usize,
) -> Result<Captured, String> {
    run_capture_with(git, root, args, timeout, max_stdout, true, None)
}

/// Runs an explicit write operation. Unlike reads, it may take optional locks
/// and is allowed to run the repository's own hooks.
pub(crate) fn run_capture_mutating(
    git: &Path,
    root: Option<&Path>,
    args: &[&str],
    timeout: Duration,
    max_stdout: usize,
) -> Result<Captured, String> {
    run_capture_with(git, root, args, timeout, max_stdout, false, None)
}

/// Pinned revision tips go through stdin, avoiding the OS argument-size limit.
pub(super) fn run_capture_input(
    git: &Path,
    root: Option<&Path>,
    args: &[&str],
    timeout: Duration,
    max_stdout: usize,
    input: Vec<u8>,
) -> Result<Captured, String> {
    run_capture_with(git, root, args, timeout, max_stdout, true, Some(input))
}

pub(super) fn run_capture_with(
    git: &Path,
    root: Option<&Path>,
    args: &[&str],
    timeout: Duration,
    max_stdout: usize,
    read_only: bool,
    input: Option<Vec<u8>>,
) -> Result<Captured, String> {
    if read_only {
        requests::check()?;
    }
    let mut command = Command::new(git);
    if let Some(root) = root {
        command.arg("-C").arg(root);
    }
    command.args(args);
    if read_only {
        command.env("GIT_OPTIONAL_LOCKS", "0");
    }
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_PAGER", "cat")
        .env("LC_ALL", "C")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove("GIT_NAMESPACE")
        .env_remove("GIT_PREFIX")
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command
        .spawn()
        .map_err(|error| format!("Git could not be started: {error}"))?;
    let input_writer = input.map(|bytes| {
        use std::io::Write;
        let mut stdin = child.stdin.take().expect("piped stdin");
        thread::spawn(move || stdin.write_all(&bytes))
    });
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Git stdout was unavailable.".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Git stderr was unavailable.".to_string())?;
    let out_reader = thread::spawn(move || read_capped(stdout, max_stdout));
    let err_reader = thread::spawn(move || read_capped(stderr, MAX_STDERR_BYTES));
    let deadline = Instant::now() + timeout;
    let mut status = None;
    loop {
        if status.is_none() {
            match child.try_wait() {
                Ok(next) => status = next,
                Err(error) => {
                    kill_tree(&mut child);
                    let _ = child.wait();
                    if let Some(writer) = input_writer {
                        let _ = writer.join();
                    }
                    let _ = out_reader.join();
                    let _ = err_reader.join();
                    return Err(error.to_string());
                }
            }
        }
        let cancelled = read_only && requests::check().is_err();
        if cancelled || Instant::now() >= deadline {
            kill_tree(&mut child);
            let _ = child.wait();
            if let Some(writer) = input_writer {
                let _ = writer.join();
            }
            let _ = out_reader.join();
            let _ = err_reader.join();
            return Err(if cancelled {
                "Git query cancelled."
            } else {
                "Git timed out."
            }
            .into());
        }
        // Helpers may keep pipes open after Git exits. The deadline also covers draining.
        if status.is_some()
            && out_reader.is_finished()
            && err_reader.is_finished()
            && input_writer
                .as_ref()
                .is_none_or(|writer| writer.is_finished())
        {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    let status = status.unwrap();
    if let Some(writer) = input_writer {
        writer
            .join()
            .map_err(|_| "Git input writer failed.".to_string())?
            .map_err(|error| format!("Git input failed: {error}"))?;
    }
    let (stdout, truncated) = out_reader
        .join()
        .map_err(|_| "Git output reader failed.".to_string())?;
    let (stderr, _) = err_reader
        .join()
        .map_err(|_| "Git error reader failed.".to_string())?;
    Ok(Captured {
        status,
        stdout,
        stderr,
        truncated,
    })
}

pub(super) fn read_capped(mut reader: impl Read, limit: usize) -> (Vec<u8>, bool) {
    let mut output = Vec::new();
    let mut buffer = [0u8; 8192];
    let mut truncated = false;
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                let remaining = limit.saturating_sub(output.len());
                if read > remaining {
                    output.extend_from_slice(&buffer[..remaining]);
                    truncated = true;
                } else {
                    output.extend_from_slice(&buffer[..read]);
                }
            }
            Err(_) => break,
        }
    }
    (output, truncated)
}

pub(super) fn kill_tree(child: &mut Child) {
    #[cfg(unix)]
    {
        let pid = child.id() as i32;
        if pid > 0 {
            // The child leads its own process group, so helpers go with it.
            unsafe {
                libc::kill(-pid, libc::SIGKILL);
            }
        }
    }
    let _ = child.kill();
}
