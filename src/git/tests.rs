use super::*;
#[test]
fn reads_the_version_line() {
    assert_eq!(
        parse_version("git version 2.39.3 (Apple Git-146)\n"),
        Some((2, 39, 3))
    );
    assert_eq!(
        parse_version("git version 2.40.0.windows.1\n"),
        Some((2, 40, 0))
    );
    assert_eq!(parse_version("git version 2.17.1\n"), Some((2, 17, 1)));
    assert_eq!(parse_version("not git"), None);
}

#[test]
fn parses_porcelain_v2_records() {
    let mut input = Vec::new();
    for record in [
        "# branch.oid abcdef1234567890",
        "# branch.head main",
        "# branch.upstream origin/main",
        "# branch.ab +2 -1",
        "1 .M N... 100644 100644 100644 aaaaaaa bbbbbbb docs/notes.md",
        "2 R. N... 100644 100644 100644 aaaaaaa bbbbbbb R100 ui/new.ts",
        "ui/old.ts",
        "u UU N... 100644 100644 100644 100644 aaaaaaa bbbbbbb ccccccc src/conflict.ts",
        "? ui/dashboard/Dashboard.tsx",
    ] {
        input.extend_from_slice(record.as_bytes());
        input.push(0);
    }
    let status = parse_status(&input);
    assert_eq!(status.branch.as_deref(), Some("main"));
    assert_eq!(status.upstream.as_deref(), Some("origin/main"));
    assert_eq!((status.ahead, status.behind), (2, 1));
    assert!(!status.detached);
    assert_eq!(status.changes.len(), 4);
    let ordinary = &status.changes[0];
    assert_eq!(ordinary.path, "docs/notes.md");
    assert_eq!(ordinary.status, "M");
    assert!(!ordinary.staged);
    assert!(ordinary.unstaged);
    let renamed = &status.changes[1];
    assert_eq!(renamed.path, "ui/new.ts");
    assert_eq!(renamed.old_path.as_deref(), Some("ui/old.ts"));
    assert_eq!(renamed.status, "R");
    assert!(renamed.staged);
    let conflict = &status.changes[2];
    assert!(conflict.conflicted);
    assert_eq!(conflict.status, "U");
    let untracked = &status.changes[3];
    assert!(untracked.untracked);
    assert_eq!(untracked.status, "?");
}

#[test]
fn detached_head_uses_a_short_oid() {
    let mut input = Vec::new();
    for record in ["# branch.oid abcdef1234567890", "# branch.head (detached)"] {
        input.extend_from_slice(record.as_bytes());
        input.push(0);
    }
    let status = parse_status(&input);
    assert!(status.detached);
    assert_eq!(status.branch.as_deref(), Some("abcdef1"));
}

#[test]
fn parses_branch_records() {
    let input = concat!(
        "refs/heads/main\0main\0aaa\0*\0origin/main\n",
        "refs/heads/topic\0topic\0bbb\0 \0\n",
        "refs/remotes/origin/main\0origin/main\0aaa\0 \0\n",
        "refs/remotes/origin/HEAD\0origin/HEAD\0aaa\0 \0\n",
        "refs/tags/v1\0v1\0ccc\0\0\n",
    );
    let branches = parse_branches(input.as_bytes());
    assert_eq!(branches.len(), 4);
    assert_eq!(branches[0].name, "main");
    assert!(branches[0].current);
    assert_eq!(branches[0].upstream.as_deref(), Some("origin/main"));
    assert_eq!(branches[1].name, "topic");
    assert_eq!(branches[2].kind, "remote");
    assert_eq!(branches[3].kind, "tag");
}

#[test]
fn parses_worktree_records() {
    let input = concat!(
        "worktree /repo\nHEAD aaa\nbranch refs/heads/main\n\n",
        "worktree /repo-side\nHEAD bbb\nbranch refs/heads/side\nlocked in use\n\n",
        "worktree /repo-detached\nHEAD ccc\ndetached\nprunable gitdir file points to non-existent location\n",
    );
    let worktrees = parse_worktrees(input.as_bytes());
    assert_eq!(worktrees.len(), 3);
    assert!(worktrees[0].main);
    assert_eq!(worktrees[0].branch.as_deref(), Some("main"));
    assert_eq!(worktrees[1].locked.as_deref(), Some("in use"));
    assert!(worktrees[2].detached);
    assert_eq!(
        worktrees[2].prunable.as_deref(),
        Some("gitdir file points to non-existent location")
    );
}

#[test]
fn parses_log_records() {
    let input = concat!(
        "aaaaaaaa\0aaaaaaa\0bbbbbbbb cccccccc\0Ada\0ada@example.com\0",
        "1700000000\0HEAD -> main, tag: v1\0Add history\n",
        "bbbbbbbb\0bbbbbbb\0\0Grace\0grace@example.com\0",
        "1600000000\0\0First commit\n",
    );
    let commits = parse_log(input.as_bytes());
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].short, "aaaaaaa");
    assert_eq!(commits[0].parents, vec!["bbbbbbbb", "cccccccc"]);
    assert_eq!(commits[0].refs, vec!["HEAD -> main", "tag: v1"]);
    assert_eq!(commits[0].subject, "Add history");
    assert_eq!(commits[0].timestamp, 1_700_000_000);
    assert!(commits[1].parents.is_empty());
}

#[test]
fn parses_name_status_records() {
    let input = "M\tsrc/a.rs\nR100\tsrc/old.rs\tsrc/new.rs\nA\tnew.txt\n";
    let files = parse_name_status(input.as_bytes());
    assert_eq!(files.len(), 3);
    assert_eq!(files[0].status, "M");
    assert_eq!(files[1].old_path.as_deref(), Some("src/old.rs"));
    assert_eq!(files[1].path, "src/new.rs");
}

#[test]
fn rejects_paths_outside_the_repository() {
    assert!(safe_relative("docs/notes.md").is_ok());
    assert!(safe_relative("../notes.md").is_err());
    assert!(safe_relative("/etc/passwd").is_err());
    assert!(safe_relative("a/../../b").is_err());
}

#[test]
fn reads_a_merge_commit_against_its_first_parent() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "termlevel-git-merge-test-{}-{stamp}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let git = git_path().expect("git is available");
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
    std::fs::write(dir.join("a.txt"), "a\n").unwrap();
    run(&["add", "a.txt"]);
    run(&["commit", "-qm", "init"]);
    run(&["checkout", "-qb", "side"]);
    std::fs::write(dir.join("b.txt"), "b\n").unwrap();
    run(&["add", "b.txt"]);
    run(&["commit", "-qm", "side"]);
    run(&["checkout", "-q", "@{-1}"]);
    std::fs::write(dir.join("c.txt"), "c\n").unwrap();
    run(&["add", "c.txt"]);
    run(&["commit", "-qm", "main"]);
    run(&["merge", "--no-ff", "-qm", "merge side", "side"]);
    let head = Command::new(&git)
        .arg("-C")
        .arg(&dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let oid = String::from_utf8_lossy(&head.stdout).trim().to_string();
    let detail = commit_sync(dir.to_str().unwrap(), &oid);
    assert!(detail.error.is_none(), "{:?}", detail.error);
    assert!(detail.files.iter().any(|file| file.path == "b.txt"));
    assert!(detail.patch.contains("+b"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn reports_a_real_repository() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir =
        std::env::temp_dir().join(format!("termlevel-git-test-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let git = git_path().expect("git is available");
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
    std::fs::write(dir.join("tracked.txt"), "one\ntwo\n").unwrap();
    std::fs::write(dir.join("new.txt"), "new\n").unwrap();

    let status = status_sync(dir.to_str().unwrap());
    assert!(status.repo, "{:?}", status.error);
    assert!(status.root.is_some());
    assert!(
        status
            .changes
            .iter()
            .any(|entry| entry.path == "tracked.txt")
    );
    assert!(
        status
            .changes
            .iter()
            .any(|entry| entry.path == "new.txt" && entry.untracked)
    );

    let diff = diff_sync(dir.to_str().unwrap(), "tracked.txt", "unstaged").unwrap();
    assert!(diff.contains("+two"));
    let patch = diff_sync(dir.to_str().unwrap(), "new.txt", "untracked").unwrap();
    assert!(patch.contains("+new"));

    let branches = branches_sync(dir.to_str().unwrap());
    assert!(branches.error.is_none(), "{:?}", branches.error);
    assert!(
        branches
            .branches
            .iter()
            .any(|branch| branch.kind == "local" && branch.current)
    );
    let worktrees = worktrees_sync(dir.to_str().unwrap());
    assert!(worktrees.error.is_none(), "{:?}", worktrees.error);
    assert_eq!(worktrees.worktrees.len(), 1);
    assert!(worktrees.worktrees[0].main);
    assert!(worktrees.worktrees[0].current);
    let log = log_sync(dir.to_str().unwrap(), "", 10, 0);
    assert!(log.error.is_none(), "{:?}", log.error);
    let commit = log
        .commits
        .iter()
        .find(|commit| commit.subject == "init")
        .expect("the init commit is in the log");
    let detail = commit_sync(dir.to_str().unwrap(), &commit.oid);
    assert!(detail.error.is_none(), "{:?}", detail.error);
    assert!(detail.files.iter().any(|file| file.path == "tracked.txt"));
    assert!(detail.patch.contains("+one"));

    let _ = std::fs::remove_dir_all(&dir);
}
