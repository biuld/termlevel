//! Bounded text read and whole-file save. Binary and oversize files stay closed.

use std::fs::File;
use std::io::Read;
use std::time::SystemTime;

use super::path::{self, within};

pub(crate) const MAX_BYTES: u64 = 512 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextFile {
    pub text: String,
    pub binary: bool,
    pub truncated: bool,
    pub lossy: bool,
    pub bytes: u64,
    pub modified_ms: u64,
}

pub(crate) fn read_file(root: &str, relative: &str) -> Result<TextFile, String> {
    let file = regular_file(root, relative)?;
    let meta = std::fs::metadata(&file).map_err(|_| "The file is unavailable.".to_owned())?;
    let bytes = meta.len();
    let modified_ms = modified_ms(&meta);
    if bytes > MAX_BYTES {
        return Ok(TextFile {
            text: String::new(),
            binary: false,
            truncated: true,
            lossy: false,
            bytes,
            modified_ms,
        });
    }
    let opened = File::open(&file).map_err(|_| "The file cannot be read.".to_owned())?;
    let mut buffer = Vec::new();
    opened
        .take(MAX_BYTES + 1)
        .read_to_end(&mut buffer)
        .map_err(|_| "The file cannot be read.".to_owned())?;
    if buffer.len() as u64 > MAX_BYTES || buffer.contains(&0) {
        return Ok(TextFile {
            text: String::new(),
            binary: buffer.contains(&0),
            truncated: buffer.len() as u64 > MAX_BYTES,
            lossy: false,
            bytes,
            modified_ms,
        });
    }
    match String::from_utf8(buffer) {
        Ok(text) => Ok(TextFile {
            text,
            binary: false,
            truncated: false,
            lossy: false,
            bytes,
            modified_ms,
        }),
        Err(error) => Ok(TextFile {
            text: String::from_utf8_lossy(error.as_bytes()).into_owned(),
            binary: false,
            truncated: false,
            lossy: true,
            bytes,
            modified_ms,
        }),
    }
}

pub(crate) fn save_file(
    root: &str,
    relative: &str,
    text: &str,
    modified_ms_seen: u64,
    bytes_seen: u64,
) -> Result<TextFile, String> {
    let file = regular_file(root, relative)?;
    let meta = std::fs::metadata(&file).map_err(|_| "The file is unavailable.".to_owned())?;
    // Modification time can stay inside one filesystem tick, so the length is checked too.
    if modified_ms(&meta) != modified_ms_seen || meta.len() != bytes_seen {
        return Err("The file changed on disk. Reload it before saving.".into());
    }
    if meta.len() > MAX_BYTES {
        return Err("The file is too large to save here.".into());
    }
    std::fs::write(&file, text.as_bytes()).map_err(|_| "The file cannot be saved.".to_owned())?;
    read_file(root, relative)
}

pub(crate) fn regular_file(root: &str, relative: &str) -> Result<std::path::PathBuf, String> {
    let located = path::locate(root, relative)?;
    let meta = std::fs::symlink_metadata(&located.path)
        .map_err(|_| "The file is unavailable.".to_owned())?;
    let path = if meta.file_type().is_symlink() {
        let target = located
            .path
            .canonicalize()
            .map_err(|_| "The link is unavailable.".to_owned())?;
        if !within(&located.root, &target) {
            return Err("The link leaves this folder.".into());
        }
        target
    } else {
        located.path
    };
    let meta =
        std::fs::symlink_metadata(&path).map_err(|_| "The file is unavailable.".to_owned())?;
    if !meta.is_file() {
        return Err("The path is not a file.".into());
    }
    Ok(path)
}

fn modified_ms(meta: &std::fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|time| time.as_millis() as u64)
        .unwrap_or(0)
}
