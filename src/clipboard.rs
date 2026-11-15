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
    if text.len() > LIMIT {
        return Err(rejected());
    }
    let mut child = Command::new(if read {
        "/usr/bin/pbpaste"
    } else {
        "/usr/bin/pbcopy"
    })
    // These helpers interpret stdin and stdout with the process text encoding.
    // A GUI launch often has no UTF-8 locale, and CoreFoundation then treats
    // the bytes as MacRoman (`中` becomes `‰∏≠`). OSC 52 always gives us UTF-8.
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
#[cfg(all(test, target_os = "macos"))]
mod tests {
    #[test]
    fn osc52_bytes_reach_the_pasteboard_as_unicode() {
        let sample = "Grok 中文复制：你好，世界！";
        let previous = std::process::Command::new("/usr/bin/pbpaste")
            .output()
            .expect("save clipboard")
            .stdout;
        let saved = String::from_utf8_lossy(&previous).into_owned();
        let restore = || {
            let _ = std::process::Command::new("/usr/bin/pbcopy")
                .env("LC_ALL", "en_US.UTF-8")
                .stdin(std::process::Stdio::piped())
                .spawn()
                .and_then(|mut child| {
                    use std::io::Write;
                    child.stdin.as_mut().unwrap().write_all(saved.as_bytes())?;
                    child.wait()
                });
        };
        let written = super::helper(false, sample);
        if written.is_err() {
            restore();
            panic!("pbcopy rejected the sample");
        }
        let script = r#"use framework "AppKit"
set pb to current application's NSPasteboard's generalPasteboard()
set s to pb's stringForType:"public.utf8-plain-text"
if s is missing value then return "MISSING"
return s as text"#;
        let out = std::process::Command::new("/usr/bin/osascript")
            .arg("-e")
            .arg(script)
            .output()
            .expect("read pasteboard");
        let got = String::from_utf8_lossy(&out.stdout);
        let got = got.trim_end_matches(['\n', '\r']);
        restore();
        assert!(out.status.success(), "osascript failed");
        assert_eq!(got, sample);
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
