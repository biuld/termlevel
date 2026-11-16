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
    let git = match crate::git::git_path() {
        Ok(git) => git,
        Err(error) => {
            let mut result = scan.result.lock().unwrap();
            result.error = Some(error);
            result.done = true;
            return;
        }
    };
    inspect_repository(&git, &root, scan);
    // Enumerate only direct children. The root result is published before this read.
    match std::fs::read_dir(&root) {
        Ok(entries) => {
            let mut children = Vec::new();
            for entry in entries {
                match entry {
                    Ok(entry) if entry.file_name() != ".git" => match entry.file_type() {
                        Ok(kind) if kind.is_dir() => children.push(entry.path()),
                        Ok(_) => {}
                        Err(e) => record_error(
                            scan,
                            format!("Could not inspect {}: {e}", entry.path().display()),
                        ),
                    },
                    Ok(_) => {}
                    Err(e) => record_error(scan, format!("Could not scan {}: {e}", root.display())),
                }
            }
            children.sort();
            for child in children {
                if scan.cancelled.load(Ordering::Relaxed) {
                    return;
                }
                inspect_repository(&git, &child, scan);
            }
        }
        Err(e) => record_error(scan, format!("Could not scan {}: {e}", root.display())),
    }
    scan.result.lock().unwrap().done = true;
}
fn inspect_repository(git: &Path, directory: &Path, scan: &Scan) {
    if scan.cancelled.load(Ordering::Relaxed) {
        return;
    }
    let output = crate::git::run_capture(
        git,
        Some(directory),
        &["rev-parse", "--show-toplevel"],
        std::time::Duration::from_secs(3),
        8192,
    );
    match output {
        Ok(output) if output.status.success() && !output.truncated => {
            let path = String::from_utf8_lossy(&output.stdout)
                .trim_end_matches(['\r', '\n'])
                .to_string();
            let path = crate::git::canonical_path(Path::new(&path));
            // Git also resolves ordinary subdirectories to their enclosing repo.
            // Only report candidates that are themselves working-tree roots.
            if path == crate::git::canonical_path(directory) {
                let mut result = scan.result.lock().unwrap();
                let path = display(&path);
                if !result.paths.contains(&path) {
                    result.paths.push(path);
                }
            }
        }
        Ok(output) => {
            let message = crate::git::stderr_message(&output);
            if !message.contains("not a git repository")
                && !message.contains("must be run in a work tree")
            {
                record_error(
                    scan,
                    format!("Cannot inspect {}: {message}", directory.display()),
                );
            }
        }
        Err(error) => record_error(
            scan,
            format!("Cannot inspect {}: {error}", directory.display()),
        ),
    }
}
fn record_error(scan: &Scan, error: String) {
    scan.result.lock().unwrap().error.get_or_insert(error);
}
fn display(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn git(root: &Path, args: &[&str]) {
        let output = std::process::Command::new(crate::git::git_path().unwrap())
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    #[test]
    fn verifies_only_root_and_direct_children_with_git() {
        let temp = std::env::temp_dir().join(format!("termlevel-discovery-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        for path in ["z", "a/nested", "fake/.git", "empty"] {
            std::fs::create_dir_all(temp.join(path)).unwrap();
        }
        let temp = temp.canonicalize().unwrap();
        for path in ["a", "z", "a/nested"] {
            git(&temp.join(path), &["init", "-q"]);
        }
        let scan = Scan {
            result: Mutex::new(Repositories::default()),
            cancelled: AtomicBool::new(false),
        };
        discover_sync(temp.to_str().unwrap(), &scan);
        assert_eq!(
            scan.result.lock().unwrap().paths,
            ["a", "z"].map(|p| display(&temp.join(p)))
        );
        assert!(scan.result.lock().unwrap().done);
        // Root first, normal subdirectories do not become duplicate repositories.
        git(&temp, &["init", "-q"]);
        git(
            &temp,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "--allow-empty",
                "-qm",
                "init",
            ],
        );
        git(&temp, &["worktree", "add", "--detach", "linked", "HEAD"]);
        let scan = Scan {
            result: Mutex::new(Repositories::default()),
            cancelled: AtomicBool::new(false),
        };
        discover_sync(temp.to_str().unwrap(), &scan);
        assert_eq!(
            scan.result.lock().unwrap().paths,
            [
                temp.clone(),
                temp.join("a"),
                temp.join("linked"),
                temp.join("z")
            ]
            .map(|p| display(&p))
        );
        assert!(scan.result.lock().unwrap().error.is_none());
        std::fs::remove_dir_all(temp).unwrap();
    }
}
