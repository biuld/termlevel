//! Discover working trees in the background without following symlinks.
use nsh_core::prelude::*;
use std::{
    collections::HashMap,
    path::Path,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
};

#[derive(Clone, Default, JsObject)]
pub struct Repositories {
    pub paths: Vec<String>,
    pub error: Option<String>,
    pub done: bool,
}
struct Scan {
    result: Mutex<Repositories>,
    cancelled: AtomicBool,
}
static SCANS: OnceLock<Mutex<HashMap<u32, Arc<Scan>>>> = OnceLock::new();
static NEXT: AtomicU32 = AtomicU32::new(1);
fn scans() -> &'static Mutex<HashMap<u32, Arc<Scan>>> {
    SCANS.get_or_init(Default::default)
}

/// Finds repositories in a workspace directory.
#[module]
pub struct GitDiscovery;
#[module]
impl GitDiscovery {
    /// Starts a background scan and returns its identifier.
    pub fn start(root: String) -> u32 {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let scan = Arc::new(Scan {
            result: Mutex::new(Repositories::default()),
            cancelled: AtomicBool::new(false),
        });
        scans().lock().unwrap().insert(id, scan.clone());
        std::thread::spawn(move || discover_sync(&root, &scan));
        id
    }
    /// Reads all results discovered so far; completed scans are released.
    pub fn poll(id: u32) -> Repositories {
        let mut scans = scans().lock().unwrap();
        let Some(scan) = scans.get(&id) else {
            return Repositories {
                done: true,
                error: Some("Repository scan is unavailable.".into()),
                ..Default::default()
            };
        };
        let result = scan.result.lock().unwrap().clone();
        if result.done {
            scans.remove(&id);
        }
        result
    }
    /// Stops and releases a scan that is no longer displayed.
    pub fn cancel(id: u32) {
        if let Some(scan) = scans().lock().unwrap().remove(&id) {
            scan.cancelled.store(true, Ordering::Relaxed);
        }
    }
}

fn discover_sync(root: &str, scan: &Scan) {
    let root = match crate::git::resolve_root(root) {
        Ok(root) => root,
        Err(error) => {
            let mut result = scan.result.lock().unwrap();
            result.error = Some(error);
            result.done = true;
            return;
        }
    };
    if let Some(parent) = root
        .ancestors()
        .skip(1)
        .find(|path| path.join(".git").exists())
    {
        scan.result.lock().unwrap().paths.push(display(parent));
    }
    let mut pending = vec![root];
    while let Some(dir) = pending.pop() {
        if scan.cancelled.load(Ordering::Relaxed) {
            return;
        }
        if dir.join(".git").exists() {
            scan.result.lock().unwrap().paths.push(display(&dir));
        }
        match std::fs::read_dir(&dir) {
            Ok(entries) => {
                let mut children = Vec::new();
                for entry in entries {
                    match entry {
                        Ok(entry) if entry.file_name() != ".git" => {
                            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                                children.push(entry.path());
                            }
                        }
                        Ok(_) => {}
                        Err(e) => {
                            scan.result.lock().unwrap().error.get_or_insert_with(|| {
                                format!("Could not scan {}: {e}", dir.display())
                            });
                        }
                    }
                }
                children.sort_by(|a, b| b.cmp(a));
                pending.extend(children);
            }
            Err(e) => {
                scan.result
                    .lock()
                    .unwrap()
                    .error
                    .get_or_insert_with(|| format!("Could not scan {}: {e}", dir.display()));
            }
        }
    }
    scan.result.lock().unwrap().done = true;
}
fn display(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovers_nested_and_sibling_worktrees_in_stable_order() {
        let temp = std::env::temp_dir().join(format!("termlevel-discovery-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        for path in ["z/.git", "a/.git", "a/nested/.git"] {
            std::fs::create_dir_all(temp.join(path)).unwrap();
        }
        let temp = temp.canonicalize().unwrap();
        let scan = Scan {
            result: Mutex::new(Repositories::default()),
            cancelled: AtomicBool::new(false),
        };
        discover_sync(temp.to_str().unwrap(), &scan);
        let result = scan.result.lock().unwrap();
        assert_eq!(
            result.paths,
            ["a", "a/nested", "z"].map(|p| display(&temp.join(p)))
        );
        assert!(result.done && result.error.is_none());
        std::fs::remove_dir_all(temp).unwrap();
    }
}
