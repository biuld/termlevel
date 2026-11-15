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
fn helper(read: bool, text: &str) -> Result<String> {
    if text.len() > LIMIT {
        return Err(rejected());
    }
    let mut child = Command::new(if read {
        "/usr/bin/pbpaste"
    } else {
        "/usr/bin/pbcopy"
    })
    // These helpers interpret bytes using their locale, while our clipboard
    // contract is always UTF-8. Finder launches and C/legacy parent locales
    // must not make pbcopy decode UTF-8 as a legacy encoding (or pbpaste emit it).
    .env("LC_ALL", "en_US.UTF-8")
    .env("LC_CTYPE", "en_US.UTF-8")
    .env("LANG", "en_US.UTF-8")
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
    let deadline = Instant::now() + Duration::from_millis(500);
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
