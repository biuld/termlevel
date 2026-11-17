//! Shared refs and worktree mutations are keyed by the canonical common directory.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock, Weak},
};
static GATES: OnceLock<Mutex<HashMap<String, Weak<Mutex<()>>>>> = OnceLock::new();
pub(super) fn run(
    root: &str,
    action: impl FnOnce() -> Result<String, String>,
) -> Result<String, String> {
    let repository = crate::git_discovery::resolve_repository(root)?
        .ok_or("No Git working tree at this path.")?;
    let lock = {
        let mut gates = GATES.get_or_init(Default::default).lock().unwrap();
        gates.retain(|_, gate| gate.strong_count() > 0);
        let gate = gates.entry(repository.common_dir).or_default();
        match gate.upgrade() {
            Some(lock) => lock,
            None => {
                let lock = Arc::new(Mutex::new(()));
                *gate = Arc::downgrade(&lock);
                lock
            }
        }
    };
    let _guard = lock.try_lock().map_err(|_| {
        "Another Git operation is in progress for this repository or one of its worktrees."
            .to_string()
    })?;
    action()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linked_worktrees_share_the_mutation_gate() {
        let root =
            std::env::temp_dir().join(format!("termlevel-common-gate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let git = |args: &[&str]| {
            assert!(
                std::process::Command::new(crate::git::git_path().unwrap())
                    .arg("-C")
                    .arg(&root)
                    .args(args)
                    .status()
                    .unwrap()
                    .success()
            );
        };
        git(&["init", "-q"]);
        git(&[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "--allow-empty",
            "-qm",
            "init",
        ]);
        git(&["worktree", "add", "--detach", "-q", "linked", "HEAD"]);
        let (started, ready) = std::sync::mpsc::channel();
        let (release, wait) = std::sync::mpsc::channel();
        let path = root.to_string_lossy().into_owned();
        let worker = std::thread::spawn(move || {
            run(&path, || {
                started.send(()).unwrap();
                wait.recv().unwrap();
                Ok(String::new())
            })
        });
        ready.recv().unwrap();
        assert!(
            run(root.join("linked").to_str().unwrap(), || Ok(String::new()))
                .unwrap_err()
                .contains("Another Git operation")
        );
        release.send(()).unwrap();
        worker.join().unwrap().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
