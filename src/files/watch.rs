//! Expanded-directory watches. This is separate from Git discovery and its
//! 1_024 directory quota. Only the directories the view asks for are watched,
//! non-recursively. macOS uses kqueue; other platforms report that watching is
//! unavailable and leave refresh to the user.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

#[cfg(target_os = "macos")]
use std::path::Path;
#[cfg(target_os = "macos")]
use std::time::Duration;

#[cfg(target_os = "macos")]
use notify::Watcher;

use super::WatchNote;
#[cfg(target_os = "macos")]
use super::path;

#[cfg(target_os = "macos")]
const WINDOW: Duration = Duration::from_millis(200);
/// The view's expanded set is capped at 64, plus the group root.
const MAX_WATCHED: usize = 65;
const UNAVAILABLE: &str = "Directory watching is unavailable.";
const NOT_WATCHED: &str = "This folder is not watched.";
#[cfg(target_os = "macos")]
const CANNOT_WATCH: &str = "This folder cannot be watched.";

static NEXT: AtomicU32 = AtomicU32::new(1);
static SLOTS: OnceLock<Mutex<HashMap<u32, Arc<Subscription>>>> = OnceLock::new();

struct State {
    request: String,
    watched: HashMap<PathBuf, Vec<String>>,
    due: HashMap<String, Instant>,
    failures: Vec<WatchNote>,
}

struct Subscription {
    state: Arc<Mutex<State>>,
    #[cfg(target_os = "macos")]
    watcher: Mutex<Option<notify::RecommendedWatcher>>,
}

fn slots() -> &'static Mutex<HashMap<u32, Arc<Subscription>>> {
    SLOTS.get_or_init(Default::default)
}

fn lock<'a, T>(mutex: &'a Mutex<T>) -> std::sync::MutexGuard<'a, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(crate) fn open() -> u32 {
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let subscription = Subscription {
        state: Arc::new(Mutex::new(State {
            request: String::new(),
            watched: HashMap::new(),
            due: HashMap::new(),
            failures: Vec::new(),
        })),
        #[cfg(target_os = "macos")]
        watcher: Mutex::new(None),
    };
    lock(slots()).insert(id, Arc::new(subscription));
    id
}

pub(crate) fn close(id: u32) {
    let Some(subscription) = lock(slots()).remove(&id) else {
        return;
    };
    drop_watcher(&subscription);
}

pub(crate) fn poll(id: u32) -> Vec<String> {
    let Some(subscription) = lock(slots()).get(&id).cloned() else {
        return Vec::new();
    };
    let mut state = lock(&subscription.state);
    let now = Instant::now();
    let mut ready: Vec<_> = state
        .due
        .iter()
        .filter(|(_, due)| **due <= now)
        .map(|(directory, _)| directory.clone())
        .collect();
    for directory in &ready {
        state.due.remove(directory);
    }
    ready.sort();
    ready
}

pub(crate) fn install(id: u32, root: &str, directories: &[String]) -> Vec<WatchNote> {
    let Some(subscription) = lock(slots()).get(&id).cloned() else {
        return Vec::new();
    };
    let request = format!("{root}\0{}", directories.join("\0"));
    {
        let state = lock(&subscription.state);
        if state.request == request {
            return state.failures.clone();
        }
    }

    let (kept, mut notes) = select_directories(directories);
    #[cfg(target_os = "macos")]
    {
        notes.extend(arm(&subscription, root, &kept));
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = root;
        drop_watcher(&subscription);
        notes.extend(kept.into_iter().map(|directory| WatchNote {
            directory,
            message: UNAVAILABLE.into(),
        }));
        let mut state = lock(&subscription.state);
        state.watched.clear();
        state.due.clear();
    }
    {
        let mut state = lock(&subscription.state);
        state.request = request;
        state.failures = notes.clone();
    }
    notes
}

fn select_directories(directories: &[String]) -> (Vec<String>, Vec<WatchNote>) {
    let mut kept = Vec::new();
    let mut seen = HashSet::new();
    for directory in directories {
        if seen.insert(directory.clone()) {
            kept.push(directory.clone());
        }
    }
    let mut notes = Vec::new();
    if kept.len() > MAX_WATCHED {
        for directory in kept.drain(MAX_WATCHED..) {
            notes.push(WatchNote {
                directory,
                message: NOT_WATCHED.into(),
            });
        }
    }
    (kept, notes)
}

#[cfg(target_os = "macos")]
fn publish(subscription: &Subscription, watched: HashMap<PathBuf, Vec<String>>) {
    let mut state = lock(&subscription.state);
    state.due.retain(|directory, _| {
        watched
            .values()
            .any(|ids| ids.iter().any(|id| id == directory))
    });
    state.watched = watched;
}

#[cfg(target_os = "macos")]
fn directory_to_watch(root: &str, relative: &str) -> Result<PathBuf, String> {
    let located = path::locate(root, relative)?;
    let meta = std::fs::symlink_metadata(&located.path)
        .map_err(|_| "The folder is unavailable.".to_owned())?;
    if meta.file_type().is_symlink() {
        let target = located
            .path
            .canonicalize()
            .map_err(|_| "The link is unavailable.".to_owned())?;
        if !path::within(&located.root, &target) || !target.is_dir() {
            return Err("The link cannot be opened.".into());
        }
        return Ok(target);
    }
    if !meta.is_dir() {
        return Err("The path is not a folder.".into());
    }
    located
        .path
        .canonicalize()
        .map_err(|_| "The folder is unavailable.".to_owned())
}

#[cfg(target_os = "macos")]
fn arm(subscription: &Subscription, root: &str, directories: &[String]) -> Vec<WatchNote> {
    drop_watcher(subscription);
    let mut wanted: HashMap<PathBuf, Vec<String>> = HashMap::new();
    for directory in directories {
        if let Ok(path) = directory_to_watch(root, directory) {
            wanted.entry(path).or_default().push(directory.clone());
        }
    }
    publish(subscription, wanted.clone());
    if wanted.is_empty() {
        return Vec::new();
    }
    let state = Arc::clone(&subscription.state);
    let mut watcher =
        match notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            record(&state, event)
        }) {
            Ok(watcher) => watcher,
            Err(_) => {
                publish(subscription, HashMap::new());
                return directories
                    .iter()
                    .map(|directory| WatchNote {
                        directory: directory.clone(),
                        message: UNAVAILABLE.into(),
                    })
                    .collect();
            }
        };
    let mut failed = Vec::new();
    for path in wanted.keys() {
        if watcher
            .watch(path, notify::RecursiveMode::NonRecursive)
            .is_err()
        {
            failed.push(path.clone());
        }
    }
    let mut notes = Vec::new();
    for path in failed {
        if let Some(ids) = wanted.remove(&path) {
            notes.extend(ids.into_iter().map(|directory| WatchNote {
                directory,
                message: CANNOT_WATCH.into(),
            }));
        }
    }
    publish(subscription, wanted.clone());
    if wanted.is_empty() {
        drop(watcher);
        return notes;
    }
    *lock(&subscription.watcher) = Some(watcher);
    notes
}

#[cfg(target_os = "macos")]
fn record(state: &Mutex<State>, event: notify::Result<notify::Event>) {
    let mut state = lock(state);
    match event {
        Err(_) => mark_all(&mut state),
        Ok(event) if event.need_rescan() => mark_all(&mut state),
        // Reading a directory can update its metadata. Listing that again would loop.
        Ok(event)
            if matches!(
                event.kind,
                notify::EventKind::Access(_)
                    | notify::EventKind::Modify(notify::event::ModifyKind::Metadata(_))
            ) => {}
        Ok(event) => {
            let mut directories = Vec::new();
            for path in &event.paths {
                directories.extend(owners(&state, path));
            }
            directories.sort();
            directories.dedup();
            let due = Instant::now() + WINDOW;
            for directory in directories {
                state.due.entry(directory).or_insert(due);
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn owners(state: &State, path: &Path) -> Vec<String> {
    let mut current = Some(path);
    while let Some(candidate) = current {
        if let Some(ids) = state.watched.get(candidate) {
            return ids.clone();
        }
        current = candidate.parent();
    }
    Vec::new()
}

#[cfg(target_os = "macos")]
fn mark_all(state: &mut State) {
    let mut directories: Vec<_> = state.watched.values().flatten().cloned().collect();
    directories.sort();
    directories.dedup();
    let due = Instant::now() + WINDOW;
    for directory in directories {
        state.due.entry(directory).or_insert(due);
    }
}

fn drop_watcher(subscription: &Subscription) {
    #[cfg(target_os = "macos")]
    {
        lock(&subscription.watcher).take();
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = subscription;
    }
}
