//! Group file view. Listings, reads and saves stay under the group directory.
//! Expanded directories are watched separately from Git discovery.

mod content;
mod list;
mod path;
mod watch;

use std::sync::{Condvar, Mutex};

use nsh_core::prelude::*;

use content::TextFile;
use list::Listing;

const SLOTS: usize = 2;
static ADMISSION: Mutex<usize> = Mutex::new(0);
static WAKE: Condvar = Condvar::new();

struct Permit;
fn admit() -> Permit {
    let mut active = ADMISSION.lock().unwrap();
    while *active >= SLOTS {
        active = WAKE.wait(active).unwrap();
    }
    *active += 1;
    Permit
}
impl Drop for Permit {
    fn drop(&mut self) {
        *ADMISSION.lock().unwrap() -= 1;
        WAKE.notify_one();
    }
}

#[derive(Clone, Debug, PartialEq, JsObject)]
pub struct FileEntry {
    pub name: String,
    pub kind: String,
    pub hidden: bool,
    pub bytes: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, JsObject)]
pub struct DirectoryListing {
    pub entries: Vec<FileEntry>,
    pub truncated: bool,
    pub unreadable: Option<String>,
}

#[derive(Clone, Debug, PartialEq, JsObject)]
pub struct WatchNote {
    pub directory: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, JsObject)]
pub struct FileText {
    pub text: String,
    pub binary: bool,
    pub truncated: bool,
    pub lossy: bool,
    pub bytes: u64,
    pub modified_ms: u64,
}

fn listing_of(listing: Listing) -> DirectoryListing {
    DirectoryListing {
        entries: listing
            .entries
            .into_iter()
            .map(|entry| FileEntry {
                name: entry.name,
                kind: entry.kind.to_owned(),
                hidden: entry.hidden,
                bytes: entry.bytes,
            })
            .collect(),
        truncated: listing.truncated,
        unreadable: listing.unreadable,
    }
}

fn text_of(text: TextFile) -> FileText {
    FileText {
        text: text.text,
        binary: text.binary,
        truncated: text.truncated,
        lossy: text.lossy,
        bytes: text.bytes,
        modified_ms: text.modified_ms,
    }
}

async fn off_thread<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    match blocking(move || {
        let _permit = admit();
        work()
    })
    .await
    {
        Ok(result) => result,
        Err(_) => Err("The file task failed.".into()),
    }
}

/// File view operations. Paths are relative to the group directory.
#[module]
pub struct Files;

#[module]
impl Files {
    /// One directory level under `root`. `directory` is relative and may be empty.
    pub async fn list(root: String, directory: String) -> Result<DirectoryListing, String> {
        off_thread(move || list::list_dir(&root, &directory).map(listing_of)).await
    }

    /// Read a text file, or report that it is binary or larger than 512 KiB.
    pub async fn read(root: String, path: String) -> Result<FileText, String> {
        off_thread(move || content::read_file(&root, &path).map(text_of)).await
    }

    /// Replace a text file when its modification time is still `modified_ms`.
    pub async fn save(
        root: String,
        path: String,
        text: String,
        modified_ms: u64,
        bytes: u64,
    ) -> Result<FileText, String> {
        off_thread(move || content::save_file(&root, &path, &text, modified_ms, bytes).map(text_of))
            .await
    }

    /// Absolute path of a listed entry.
    pub async fn absolute(root: String, path: String) -> Result<String, String> {
        off_thread(move || path::display_path(&root, &path)).await
    }

    /// File URL for a regular file resolved within the group directory.
    pub async fn preview_url(root: String, path: String) -> Result<String, String> {
        off_thread(move || {
            let file = content::regular_file(&root, &path)?;
            url::Url::from_file_path(file)
                .map(|url| url.to_string())
                .map_err(|_| "The file path cannot be opened.".to_owned())
        })
        .await
    }

    /// Directory a new terminal uses for this entry.
    pub async fn terminal_directory(root: String, path: String) -> Result<String, String> {
        off_thread(move || path::terminal_directory(&root, &path)).await
    }

    /// Show the entry in the system file manager.
    pub async fn reveal(root: String, path: String) -> Result<(), String> {
        let absolute = off_thread(move || path::display_path(&root, &path)).await?;
        nsh_core::element::reveal_path(absolute).await
    }

    /// Open the entry with its default application.
    pub async fn open_with_default_app(root: String, path: String) -> Result<(), String> {
        let absolute = off_thread(move || path::display_path(&root, &path)).await?;
        nsh_core::element::open_path(absolute).await
    }

    /// Move one entry to the trash. The caller confirms first.
    pub async fn trash(root: String, path: String) -> Result<(), String> {
        let absolute = off_thread(move || path::display_path(&root, &path)).await?;
        nsh_core::element::move_to_trash(absolute).await.map(|_| ())
    }

    /// Open one directory-watch subscription. It watches nothing until `watch_set`.
    pub fn watch_open() -> u32 {
        watch::open()
    }

    /// Watch these relative directories. An empty string is the group root.
    /// Path checks run off the engine thread. Failures are per directory.
    pub async fn watch_set(
        id: u32,
        root: String,
        directories: Vec<String>,
    ) -> Result<Vec<WatchNote>, String> {
        off_thread(move || Ok(watch::install(id, &root, &directories))).await
    }

    /// Relative directories whose coalesced events are ready to list again.
    pub fn watch_poll(id: u32) -> Vec<String> {
        watch::poll(id)
    }

    /// Drop the subscription and its watches.
    pub fn watch_close(id: u32) {
        watch::close(id)
    }
}

#[cfg(test)]
mod tests;
