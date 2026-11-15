//! Clipboard work runs outside AppKit and never waits for the UI thread while
//! the PTY parser owns its mutex. Each helper has its own AppKit main thread.
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use termlevel_ghostty_vt::{Clipboard, Error, Result, raw};
static ALLOW_READ: AtomicBool = AtomicBool::new(false);
const LIMIT: usize = 1024 * 1024;
pub(crate) fn allow_read(value: bool) {
    ALLOW_READ.store(value, Ordering::Release);
}
pub(crate) struct SystemClipboard;
fn rejected() -> Error {
    Error(raw::GHOSTTY_REJECTED)
}

/// `__CF_USER_TEXT_ENCODING` is `uid:encoding:encoding`. `0x8000100` is UTF-8.
fn cf_utf8_encoding() -> String {
    let uid = unsafe { libc::getuid() };
    format!("0x{uid:X}:0x8000100:0x8000100")
}
fn helper(read: bool, text: &str) -> Result<String> {
    let command = Command::new(if read {
        "/usr/bin/pbpaste"
    } else {
        "/usr/bin/pbcopy"
    });
    run_helper(command, read, text)
}

fn run_helper(mut command: Command, read: bool, text: &str) -> Result<String> {
    if text.len() > LIMIT {
        return Err(rejected());
    }
    // OSC 52 provides UTF-8 even when a GUI launch inherits a legacy locale.
    let mut child = command
        .env("LC_ALL", "en_US.UTF-8")
        .env("LC_CTYPE", "en_US.UTF-8")
        .env("LANG", "en_US.UTF-8")
        .env("__CF_USER_TEXT_ENCODING", cf_utf8_encoding())
        .stdin(if read { Stdio::null() } else { Stdio::piped() })
        .stdout(if read { Stdio::piped() } else { Stdio::null() })
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| rejected())?;
    let pipe = if read {
        let output = child.stdout.take().unwrap();
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            output.take((LIMIT + 1) as u64).read_to_end(&mut bytes)?;
            Ok(bytes)
        })
    } else {
        let mut input = child.stdin.take().unwrap();
        let bytes = text.as_bytes().to_vec();
        std::thread::spawn(move || {
            input.write_all(&bytes)?;
            Ok(Vec::new())
        })
    };
    let deadline = Instant::now() + Duration::from_secs(2);
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let bytes = pipe
        .join()
        .map_err(|_| rejected())?
        .map_err(|_: std::io::Error| rejected())?;
    if !status.is_some_and(|s| s.success()) || bytes.len() > LIMIT {
        return Err(rejected());
    }
    String::from_utf8(bytes).map_err(|_| rejected())
}
#[cfg(test)]
mod tests {
    use std::{path::PathBuf, process::Command};

    struct TempClipboard(PathBuf);

    impl Drop for TempClipboard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn helper_preserves_utf8_and_configures_encoding() {
        let path = TempClipboard(
            std::env::temp_dir().join(format!("termlevel-clipboard-test-{}", std::process::id())),
        );
        let sample = "Claude 中文复制：你好，世界！ café e\u{301} 😀\n第二行";
        let mut write = Command::new("/bin/sh");
        write.args([
            "-c",
            "test \"$LC_ALL\" = en_US.UTF-8 && test \"$LC_CTYPE\" = en_US.UTF-8 && test \"$LANG\" = en_US.UTF-8 && test \"$__CF_USER_TEXT_ENCODING\" = \"$2\" && cat > \"$1\"",
            "clipboard-test",
        ]).arg(&path.0).arg(super::cf_utf8_encoding());
        super::run_helper(write, false, sample).expect("write UTF-8 bytes");
        assert_eq!(
            std::fs::read(&path.0).expect("read sample"),
            sample.as_bytes()
        );

        let mut read = Command::new("/bin/cat");
        read.arg(&path.0);
        assert_eq!(super::run_helper(read, true, "").unwrap(), sample);
    }

    #[test]
    fn helper_rejects_oversized_and_invalid_utf8() {
        assert!(
            super::run_helper(
                Command::new("/bin/cat"),
                false,
                &"x".repeat(super::LIMIT + 1)
            )
            .is_err()
        );
        let mut invalid = Command::new("/bin/sh");
        invalid.args(["-c", "printf '\\377'"]);
        assert!(super::run_helper(invalid, true, "").is_err());
    }
}

impl Clipboard for SystemClipboard {
    fn read(&self) -> Result<String> {
        if !ALLOW_READ.load(Ordering::Acquire) {
            return Err(rejected());
        }
        helper(true, "")
    }
    fn write(&self, text: &str) -> Result<()> {
        helper(false, text).map(|_| ())
    }
}
