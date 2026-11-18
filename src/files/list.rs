//! One directory level. Nothing here walks into children.

use std::fs::DirEntry;

use super::path::{self, within};

pub(crate) const MAX_ENTRIES: usize = 5_000;
const COLLECT_CAP: usize = 50_000;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Entry {
    pub name: String,
    pub kind: &'static str,
    pub hidden: bool,
    pub bytes: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Listing {
    pub entries: Vec<Entry>,
    pub truncated: bool,
    pub unreadable: Option<String>,
}

pub(crate) fn list_dir(root: &str, relative: &str) -> Result<Listing, String> {
    let located = path::locate(root, relative)?;
    let meta = std::fs::symlink_metadata(&located.path)
        .map_err(|_| "The folder is unavailable.".to_owned())?;
    let directory = if meta.file_type().is_symlink() {
        let target = located
            .path
            .canonicalize()
            .map_err(|_| "The link is unavailable.".to_owned())?;
        if !within(&located.root, &target) {
            return Ok(unreadable("The link leaves this folder."));
        }
        if !target.is_dir() {
            return Ok(unreadable("The link cannot be opened."));
        }
        target
    } else if meta.is_dir() {
        located.path
    } else {
        return Err("The path is not a folder.".into());
    };
    let read = match std::fs::read_dir(&directory) {
        Ok(read) => read,
        Err(_) => return Ok(unreadable("The folder cannot be read.")),
    };
    let mut entries = Vec::new();
    let mut overflow = false;
    for item in read {
        let item = match item {
            Ok(item) => item,
            Err(_) => continue,
        };
        if entries.len() == COLLECT_CAP {
            overflow = true;
            break;
        }
        if let Some(entry) = entry_of(item) {
            entries.push(entry);
        }
    }
    entries.sort_by(|left, right| {
        rank(left.kind)
            .cmp(&rank(right.kind))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.name.as_bytes().cmp(right.name.as_bytes()))
    });
    let truncated = overflow || entries.len() > MAX_ENTRIES;
    entries.truncate(MAX_ENTRIES);
    Ok(Listing {
        entries,
        truncated,
        unreadable: None,
    })
}

fn unreadable(message: &str) -> Listing {
    Listing {
        entries: Vec::new(),
        truncated: false,
        unreadable: Some(message.to_owned()),
    }
}

fn rank(kind: &str) -> u8 {
    match kind {
        "directory" => 0,
        "symlink" => 1,
        _ => 2,
    }
}

fn entry_of(item: DirEntry) -> Option<Entry> {
    let name = item.file_name().to_string_lossy().into_owned();
    if name.is_empty() || name.contains('\0') || name.contains('/') || name.contains('\\') {
        return None;
    }
    let kind = item.file_type().ok().map(|kind| {
        if kind.is_symlink() {
            "symlink"
        } else if kind.is_dir() {
            "directory"
        } else if kind.is_file() {
            "file"
        } else {
            "other"
        }
    })?;
    let bytes = if kind == "file" {
        item.metadata().ok().map(|meta| meta.len())
    } else {
        None
    };
    Some(Entry {
        hidden: name.starts_with('.'),
        name,
        kind,
        bytes,
    })
}
