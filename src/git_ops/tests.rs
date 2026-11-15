use super::*;
use std::process::Command;

fn temp_repo(name: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "termlevel-git-ops-{name}-{}-{stamp}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let git = git::git_path().expect("git is available");
    let run = |args: &[&str]| {
        let output = Command::new(&git)
            .arg("-C")
            .arg(&dir)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    run(&["init", "-q"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test"]);
    std::fs::write(dir.join("tracked.txt"), "one\n").unwrap();
    run(&["add", "tracked.txt"]);
    run(&["commit", "-qm", "init"]);
    dir
}

#[test]
fn creates_and_switches_branches() {
    let dir = temp_repo("branch");
    let root = dir.to_str().unwrap();
    create_branch_sync(root, "feature", "", true).unwrap();
    let branches = git::list_worktrees(root).unwrap();
    assert!(
        branches
            .iter()
            .any(|worktree| worktree.branch.as_deref() == Some("feature"))
    );
    checkout_sync(root, "main").unwrap();
    let branches = git::list_worktrees(root).unwrap();
    assert!(
        branches
            .iter()
            .any(|worktree| worktree.branch.as_deref() == Some("main"))
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn deletes_a_merged_branch_and_refuses_the_current_one() {
    let dir = temp_repo("delete");
    let root = dir.to_str().unwrap();
    create_branch_sync(root, "merged", "", false).unwrap();
    delete_branch_sync(root, "merged").unwrap();
    let git = git::git_path().unwrap();
    let output = Command::new(&git)
        .arg("-C")
        .arg(&dir)
        .args(["branch", "--list", "merged"])
        .output()
        .unwrap();
    assert!(output.stdout.is_empty());
    assert!(delete_branch_sync(root, "main").is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn adds_and_removes_a_worktree() {
    let dir = temp_repo("worktree");
    let root = dir.to_str().unwrap();
    create_branch_sync(root, "side", "", false).unwrap();
    let path = dir.join("side-worktree");
    let added = add_worktree_sync(root, path.to_str().unwrap(), "side").unwrap();
    assert!(Path::new(&added).is_dir());
    let worktrees = git::list_worktrees(root).unwrap();
    let added_path = git::canonical_path(Path::new(&added));
    assert!(
        worktrees
            .iter()
            .any(|worktree| git::canonical_path(Path::new(&worktree.path)) == added_path)
    );
    remove_worktree_sync(root, &added).unwrap();
    assert!(!Path::new(&added).exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn fetch_and_fast_forward_pull_use_the_current_worktree_upstream() {
    let dir = temp_repo("remote");
    let remote = dir.with_extension("bare");
    let peer = dir.with_extension("peer");
    let git = git::git_path().unwrap();
    let invoke = |cwd: &Path, args: &[&str]| -> String {
        let output = Command::new(&git)
            .arg("-C")
            .arg(cwd)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    };
    let parent = dir.parent().unwrap();
    invoke(
        parent,
        &[
            "clone",
            "--bare",
            dir.to_str().unwrap(),
            remote.to_str().unwrap(),
        ],
    );
    invoke(
        parent,
        &["clone", remote.to_str().unwrap(), peer.to_str().unwrap()],
    );
    invoke(&peer, &["config", "user.email", "test@example.com"]);
    invoke(&peer, &["config", "user.name", "Test"]);
    invoke(&dir, &["remote", "add", "origin", remote.to_str().unwrap()]);
    fetch_sync(dir.to_str().unwrap()).unwrap();
    let branch = invoke(&dir, &["branch", "--show-current"]);
    invoke(
        &dir,
        &["branch", "--set-upstream-to", &format!("origin/{branch}")],
    );
    let old_head = invoke(&dir, &["rev-parse", "HEAD"]);
    std::fs::write(peer.join("tracked.txt"), "remote\n").unwrap();
    invoke(&peer, &["commit", "-am", "remote update"]);
    invoke(&peer, &["push"]);
    fetch_sync(dir.to_str().unwrap()).unwrap();
    assert_eq!(invoke(&dir, &["rev-parse", "HEAD"]), old_head);
    assert_ne!(invoke(&dir, &["rev-parse", "@{upstream}"]), old_head);
    std::fs::write(dir.join("tracked.txt"), "uncommitted\n").unwrap();
    assert!(pull_sync(dir.to_str().unwrap()).is_err());
    assert_eq!(invoke(&dir, &["rev-parse", "HEAD"]), old_head);
    assert_eq!(
        std::fs::read_to_string(dir.join("tracked.txt")).unwrap(),
        "uncommitted\n"
    );
    invoke(&dir, &["checkout", "--", "tracked.txt"]);
    pull_sync(dir.to_str().unwrap()).unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.join("tracked.txt")).unwrap(),
        "remote\n"
    );
    std::fs::write(dir.join("local.txt"), "local\n").unwrap();
    invoke(&dir, &["add", "local.txt"]);
    invoke(&dir, &["commit", "-m", "local update"]);
    std::fs::write(peer.join("remote.txt"), "remote\n").unwrap();
    invoke(&peer, &["add", "remote.txt"]);
    invoke(&peer, &["commit", "-m", "another remote update"]);
    invoke(&peer, &["push"]);
    let local_head = invoke(&dir, &["rev-parse", "HEAD"]);
    assert!(pull_sync(dir.to_str().unwrap()).is_err());
    assert_eq!(invoke(&dir, &["rev-parse", "HEAD"]), local_head);
    for path in [&dir, &remote, &peer] {
        let _ = std::fs::remove_dir_all(path);
    }
}

#[test]
fn push_updates_only_the_current_upstream_and_rejects_divergence() {
    let dir = temp_repo("push");
    let remote = dir.with_extension("bare");
    let peer = dir.with_extension("peer");
    let git = git::git_path().unwrap();
    let invoke = |cwd: &Path, args: &[&str]| -> String {
        let output = Command::new(&git)
            .arg("-C")
            .arg(cwd)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    };
    let parent = dir.parent().unwrap();
    invoke(
        parent,
        &[
            "clone",
            "--bare",
            dir.to_str().unwrap(),
            remote.to_str().unwrap(),
        ],
    );
    invoke(
        parent,
        &["clone", remote.to_str().unwrap(), peer.to_str().unwrap()],
    );
    invoke(&peer, &["config", "user.email", "test@example.com"]);
    invoke(&peer, &["config", "user.name", "Test"]);
    invoke(&dir, &["remote", "add", "origin", remote.to_str().unwrap()]);
    assert!(
        push_sync(dir.to_str().unwrap())
            .unwrap_err()
            .contains("no upstream")
    );
    fetch_sync(dir.to_str().unwrap()).unwrap();
    let branch = invoke(&dir, &["branch", "--show-current"]);
    invoke(
        &dir,
        &["branch", "--set-upstream-to", &format!("origin/{branch}")],
    );
    invoke(
        &dir,
        &["config", "remote.origin.push", "+refs/heads/*:refs/heads/*"],
    );
    invoke(&dir, &["branch", "unrelated"]);
    std::fs::write(dir.join("local.txt"), "local\n").unwrap();
    invoke(&dir, &["add", "local.txt"]);
    invoke(&dir, &["commit", "-m", "local change"]);
    push_sync(dir.to_str().unwrap()).unwrap();
    assert_eq!(
        invoke(&remote, &["rev-parse", &format!("refs/heads/{branch}")]),
        invoke(&dir, &["rev-parse", "HEAD"])
    );
    let unrelated = Command::new(&git)
        .arg("-C")
        .arg(&remote)
        .args(["show-ref", "--verify", "--quiet", "refs/heads/unrelated"])
        .status()
        .unwrap();
    assert!(
        !unrelated.success(),
        "configured wildcard must not push another branch"
    );

    invoke(&peer, &["pull", "--ff-only"]);
    std::fs::write(peer.join("remote.txt"), "remote\n").unwrap();
    invoke(&peer, &["add", "remote.txt"]);
    invoke(&peer, &["commit", "-m", "remote change"]);
    invoke(&peer, &["push"]);
    let remote_head = invoke(&remote, &["rev-parse", &format!("refs/heads/{branch}")]);
    std::fs::write(dir.join("local2.txt"), "local again\n").unwrap();
    invoke(&dir, &["add", "local2.txt"]);
    invoke(&dir, &["commit", "-m", "divergent local change"]);
    assert!(push_sync(dir.to_str().unwrap()).is_err());
    assert_eq!(
        invoke(&remote, &["rev-parse", &format!("refs/heads/{branch}")]),
        remote_head
    );
    invoke(&dir, &["checkout", "--detach"]);
    assert!(
        push_sync(dir.to_str().unwrap())
            .unwrap_err()
            .contains("detached HEAD")
    );
    for path in [&dir, &remote, &peer] {
        let _ = std::fs::remove_dir_all(path);
    }
}
