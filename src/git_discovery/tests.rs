use super::*;
use std::{
    path::Path,
    process::Command,
    time::{Duration, Instant},
};
fn git(root: &Path, args: &[&str]) {
    let output = Command::new(crate::git::git_path().unwrap())
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
fn wait(id: u32) -> Vec<Repository> {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let first = GitDiscovery::poll(id, 0, 0);
        if first.done {
            assert!(first.error.is_none(), "{:?}", first.error);
            let revision = first.revision;
            let total = first.total;
            let mut entries = first.repositories;
            while entries.len() < total as usize {
                let page = GitDiscovery::poll(id, revision, entries.len() as u32);
                if page.reset {
                    break;
                }
                entries.extend(page.repositories);
            }
            if entries.len() == total as usize {
                return entries;
            }
        }
        assert!(Instant::now() < until, "catalog completed");
        std::thread::sleep(Duration::from_millis(20));
    }
}
#[test]
fn catalog_resolves_enclosing_roots_contexts_worktrees_and_aliases() {
    let root = std::env::temp_dir().join(format!("termlevel-catalog-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for path in ["project/src", "container/a", "container/deep/b/src"] {
        std::fs::create_dir_all(root.join(path)).unwrap();
    }
    for path in ["project", "container/a", "container/deep/b"] {
        git(&root.join(path), &["init", "-q", "-b", "main"]);
        git(
            &root.join(path),
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
    }
    let sub = GitDiscovery::start(
        root.join("project/src").to_string_lossy().into_owned(),
        vec![],
    );
    let entries = wait(sub);
    assert_eq!(entries.len(), 1);
    assert!(entries[0].root.ends_with("/project"));
    let inside = resolve_repository(root.join("project/src").to_str().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(inside.id, entries[0].id);
    GitDiscovery::cancel(sub);
    let cwd = root
        .join("container/deep/b/src")
        .to_string_lossy()
        .into_owned();
    let sub = GitDiscovery::start(
        root.join("container").to_string_lossy().into_owned(),
        vec![cwd],
    );
    let entries = wait(sub);
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().any(|repo| repo.root.ends_with("/deep/b")));
    git(
        &root.join("project"),
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            root.join("linked").to_str().unwrap(),
        ],
    );
    let linked = resolve_repository(root.join("linked").to_str().unwrap())
        .unwrap()
        .unwrap();
    assert_ne!(inside.id, linked.id);
    assert_eq!(inside.common_dir, linked.common_dir);
    assert_eq!(linked.kind, "linked");
    GitDiscovery::cancel(sub);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn watches_repository_creation_and_preserves_other_subscribers() {
    let root = std::env::temp_dir().join(format!("termlevel-catalog-watch-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let first = GitDiscovery::start(root.to_string_lossy().into_owned(), vec![]);
    let second = GitDiscovery::start(root.to_string_lossy().into_owned(), vec![]);
    assert!(wait(first).is_empty());
    assert!(wait(second).is_empty());
    GitDiscovery::cancel(first);
    std::fs::create_dir(root.join("new")).unwrap();
    git(&root.join("new"), &["init", "-q"]);
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let page = GitDiscovery::poll(second, 0, 0);
        if page.done
            && page
                .repositories
                .iter()
                .any(|repo| repo.root.ends_with("/new"))
        {
            break;
        }
        assert!(
            Instant::now() < until,
            "watcher discovered the new repository"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    GitDiscovery::cancel(second);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn pages_are_bounded_and_new_revisions_restart_assembly() {
    let scope = Arc::new(Scope {
        root: "/not-used".into(),
        subscribers: Mutex::new(HashMap::new()),
        plan: Mutex::new(Plan::default()),
        running_full: AtomicBool::new(false),
    });
    let entries: Vec<_> = (0..600)
        .map(|index| Repository {
            id: index,
            root: format!("/repo-{index}"),
            git_dir: String::new(),
            common_dir: String::new(),
            label: index.to_string(),
            kind: "main".into(),
        })
        .collect();
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let subscription = Arc::new(Subscription {
        scope,
        contexts: Mutex::new(vec![]),
        snapshot: Mutex::new(Snapshot {
            entries: Arc::new(entries),
            revision: 7,
            scanning: false,
            ..Default::default()
        }),
    });
    subscriptions()
        .lock()
        .unwrap()
        .insert(id, subscription.clone());
    assert_eq!(GitDiscovery::poll(id, 0, 0).repositories.len(), 256);
    assert!(
        GitDiscovery::poll(id, 7, 0).repositories.is_empty(),
        "unchanged revision sends no rows"
    );
    let next = GitDiscovery::poll(id, 7, 256);
    assert_eq!(next.repositories[0].id, 256);
    subscription.snapshot.lock().unwrap().revision = 8;
    let changed = GitDiscovery::poll(id, 7, 512);
    assert!(changed.reset);
    assert_eq!(changed.repositories[0].id, 0);
    subscriptions().lock().unwrap().remove(&id);
}
