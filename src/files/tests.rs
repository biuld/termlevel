use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use std::time::{Duration, Instant};

use super::content::{read_file, save_file};
use super::list::list_dir;
use super::path::{locate, terminal_directory};
use super::watch;

static TEMP: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);
impl TempDir {
    fn new() -> Self {
        let n = TEMP.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("termlevel-files-{nanos}-{n}"));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> &str {
        self.0.to_str().unwrap()
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn sorts_directories_before_files_and_marks_hidden() {
    let dir = TempDir::new();
    fs::create_dir(dir.0.join("zeta")).unwrap();
    fs::write(dir.0.join("b.txt"), "b").unwrap();
    fs::write(dir.0.join("a.txt"), "a").unwrap();
    fs::write(dir.0.join(".secret"), "s").unwrap();
    let listing = list_dir(dir.path(), "").unwrap();
    let names: Vec<_> = listing
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    assert_eq!(names, ["zeta", ".secret", "a.txt", "b.txt"]);
    assert!(listing.entries[1].hidden);
    assert!(!listing.entries[2].hidden);
    assert_eq!(listing.entries[0].kind, "directory");
    assert_eq!(listing.entries[2].bytes, Some(1));
}

#[test]
fn rejects_a_path_that_leaves_the_root() {
    let dir = TempDir::new();
    assert!(locate(dir.path(), "../").is_err());
    assert!(list_dir(dir.path(), "..").is_err());
}

#[test]
fn refuses_to_list_a_symlink_that_leaves_the_root() {
    let dir = TempDir::new();
    let outside = TempDir::new();
    fs::write(outside.0.join("outside.txt"), "no").unwrap();
    symlink(&outside.0, dir.0.join("escape")).unwrap();
    let listing = list_dir(dir.path(), "escape").unwrap();
    assert_eq!(
        listing.unreadable.as_deref(),
        Some("The link leaves this folder.")
    );
}

#[test]
fn lists_a_symlink_whose_target_stays_inside() {
    let dir = TempDir::new();
    fs::create_dir(dir.0.join("real")).unwrap();
    fs::write(dir.0.join("real/note.txt"), "hi").unwrap();
    symlink(dir.0.join("real"), dir.0.join("alias")).unwrap();
    let listing = list_dir(dir.path(), "alias").unwrap();
    assert_eq!(listing.entries.len(), 1);
    assert_eq!(listing.entries[0].name, "note.txt");
}

#[test]
fn reports_an_unreadable_directory() {
    let dir = TempDir::new();
    let locked = dir.0.join("locked");
    fs::create_dir(&locked).unwrap();
    let mut perms = fs::metadata(&locked).unwrap().permissions();
    perms.set_mode(0);
    fs::set_permissions(&locked, perms).unwrap();
    let listing = list_dir(dir.path(), "locked").unwrap();
    assert!(listing.unreadable.is_some());
    let mut perms = fs::metadata(&locked).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&locked, perms).unwrap();
}

#[test]
fn truncates_a_directory_past_the_entry_cap() {
    let dir = TempDir::new();
    for index in 0..=super::list::MAX_ENTRIES {
        fs::write(dir.0.join(format!("f{index:05}")), "x").unwrap();
    }
    let listing = list_dir(dir.path(), "").unwrap();
    assert!(listing.truncated);
    assert_eq!(listing.entries.len(), super::list::MAX_ENTRIES);
}

#[test]
fn reads_text_and_rejects_a_stale_save() {
    let dir = TempDir::new();
    fs::write(dir.0.join("note.txt"), "hello").unwrap();
    let read = read_file(dir.path(), "note.txt").unwrap();
    assert_eq!(read.text, "hello");
    assert!(!read.binary);
    let saved = save_file(
        dir.path(),
        "note.txt",
        "edited",
        read.modified_ms,
        read.bytes,
    )
    .unwrap();
    assert_eq!(saved.text, "edited");
    assert!(
        save_file(
            dir.path(),
            "note.txt",
            "again",
            read.modified_ms,
            read.bytes
        )
        .is_err()
    );
}

#[test]
fn treats_a_nul_as_binary() {
    let dir = TempDir::new();
    fs::write(dir.0.join("blob"), [b'a', 0, b'b']).unwrap();
    let read = read_file(dir.path(), "blob").unwrap();
    assert!(read.binary);
    assert!(read.text.is_empty());
}

struct WatchGuard(u32);
impl Drop for WatchGuard {
    fn drop(&mut self) {
        watch::close(self.0);
    }
}

fn wait_for(id: u32, directory: &str) -> bool {
    let until = Instant::now() + Duration::from_secs(5);
    while Instant::now() < until {
        if watch::poll(id).iter().any(|ready| ready == directory) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(40));
    }
    false
}

#[cfg(target_os = "macos")]
#[test]
fn watch_reports_a_new_file_in_an_expanded_directory() {
    let dir = TempDir::new();
    let nested = dir.0.join("src");
    fs::create_dir(&nested).unwrap();
    let guard = WatchGuard(watch::open());
    let notes = watch::install(guard.0, dir.path(), &[String::new(), "src".into()]);
    assert!(notes.is_empty());
    fs::write(nested.join("new.txt"), "x").unwrap();
    assert!(wait_for(guard.0, "src"), "expanded directory refresh");
    assert!(
        watch::poll(guard.0).is_empty(),
        "a ready directory is reported once"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn collapsing_a_directory_stops_its_watch() {
    let dir = TempDir::new();
    fs::create_dir(dir.0.join("src")).unwrap();
    let guard = WatchGuard(watch::open());
    assert!(watch::install(guard.0, dir.path(), &[String::new(), "src".into()]).is_empty());
    assert!(watch::install(guard.0, dir.path(), &[String::new()]).is_empty());
    fs::write(dir.0.join("src/new.txt"), "x").unwrap();
    fs::write(dir.0.join("root.txt"), "y").unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    let mut seen_root = false;
    while Instant::now() < until {
        let ready = watch::poll(guard.0);
        assert!(
            !ready.iter().any(|directory| directory == "src"),
            "collapsed directory stays unwatched"
        );
        seen_root |= ready.iter().any(|directory| directory.is_empty());
        if seen_root && Instant::now() + Duration::from_millis(300) > until {
            break;
        }
        if seen_root {
            std::thread::sleep(Duration::from_millis(300));
            let ready = watch::poll(guard.0);
            assert!(!ready.iter().any(|directory| directory == "src"));
            break;
        }
        std::thread::sleep(Duration::from_millis(40));
    }
    assert!(seen_root, "root refresh");
}

#[cfg(target_os = "macos")]
#[test]
fn a_symlink_directory_is_watched_without_leaving_the_root() {
    let dir = TempDir::new();
    let outside = TempDir::new();
    fs::create_dir(dir.0.join("real")).unwrap();
    symlink(dir.0.join("real"), dir.0.join("alias")).unwrap();
    symlink(&outside.0, dir.0.join("escape")).unwrap();
    let guard = WatchGuard(watch::open());
    let notes = watch::install(
        guard.0,
        dir.path(),
        &[String::new(), "alias".into(), "escape".into()],
    );
    assert!(notes.iter().all(|note| note.directory != "alias"));
    fs::write(dir.0.join("real/note.txt"), "hi").unwrap();
    assert!(wait_for(guard.0, "alias"), "symlink directory refresh");
}

#[cfg(target_os = "macos")]
#[test]
fn directories_past_the_watch_cap_are_explained() {
    let dir = TempDir::new();
    let mut directories = vec![String::new()];
    for index in 0..70 {
        let name = format!("d{index:02}");
        fs::create_dir(dir.0.join(&name)).unwrap();
        directories.push(name);
    }
    let guard = WatchGuard(watch::open());
    let notes = watch::install(guard.0, dir.path(), &directories);
    assert_eq!(notes.len(), directories.len() - 65);
    assert!(
        notes
            .iter()
            .all(|note| note.message == "This folder is not watched.")
    );
    fs::write(dir.0.join("d00/file.txt"), "x").unwrap();
    assert!(
        wait_for(guard.0, "d00"),
        "a directory inside the cap still refreshes"
    );
}

#[cfg(not(target_os = "macos"))]
#[test]
fn watching_reports_that_it_is_unavailable() {
    let dir = TempDir::new();
    let guard = WatchGuard(watch::open());
    let notes = watch::install(guard.0, dir.path(), &[String::new()]);
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].message, "Directory watching is unavailable.");
    assert!(watch::poll(guard.0).is_empty());
}

#[test]
fn terminal_directory_of_a_file_is_its_parent() {
    let dir = TempDir::new();
    fs::create_dir(dir.0.join("src")).unwrap();
    fs::write(dir.0.join("src/main.rs"), "fn main() {}\n").unwrap();
    let cwd = terminal_directory(dir.path(), "src/main.rs").unwrap();
    assert!(cwd.ends_with("/src"));
}
