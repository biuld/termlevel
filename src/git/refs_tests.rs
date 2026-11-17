use super::*;

#[test]
fn twenty_thousand_long_remote_refs_are_paged_and_unchanged_reads_send_no_rows() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("termlevel-refs-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let git = git_path().unwrap();
    let run = |args: &[&str]| {
        let output = Command::new(&git)
            .arg("-C")
            .arg(&dir)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    };
    run(&["init", "-q", "-b", "main"]);
    run(&[
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.com",
        "commit",
        "--allow-empty",
        "-qm",
        "init",
    ]);
    let oid = run(&["rev-parse", "HEAD"]);
    let mut packed = "# pack-refs with: peeled fully-peeled sorted\n".to_string();
    for i in 0..20_000 {
        packed.push_str(&format!("{oid} refs/remotes/origin/feature/large-repository-integration-with-a-long-name-{i:05}\n"));
    }
    assert!(packed.len() > 2 * 1024 * 1024);
    std::fs::write(dir.join(".git/packed-refs"), packed).unwrap();
    // Annotated tags must decorate their peeled commit, not the tag object's OID.
    run(&[
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.com",
        "tag",
        "-am",
        "release",
        "v1",
    ]);
    let owner = requests::Owner::default();
    let first = branch_snapshot(&owner, dir.to_str().unwrap(), "");
    assert!(first.error.is_none(), "{:?}", first.error);
    assert_eq!(first.total, 20_002);
    assert_eq!(first.branches.len(), 256);
    let mut total = first.branches.len();
    while total < first.total as usize {
        let page = branch_page(&owner, &first.revision, total as u32);
        assert!(page.error.is_none());
        assert!(!page.branches.is_empty() && page.branches.len() <= 256);
        total += page.branches.len();
    }
    assert_eq!(total, first.total as usize);
    let same = branch_snapshot(&owner, dir.to_str().unwrap(), &first.revision);
    assert!(same.error.is_none(), "{:?}", same.error);
    assert!(same.unchanged);
    assert!(same.branches.is_empty());
    let mut log = log_sync(dir.to_str().unwrap(), "", 200, 0);
    assert!(log.error.is_none());
    assert_eq!(log.commits.len(), 1);
    decorations::apply(&owner, &mut log.commits[0]);
    assert!(log.commits[0].refs.len() <= 8);
    assert_eq!(log.commits[0].ref_count, 20_002);
    owner
        .graph
        .lock()
        .unwrap()
        .apply(dir.to_str().unwrap(), "", 0, &mut log)
        .unwrap();
    assert_eq!(
        match_refs(&owner, "long-name-19999").unwrap(),
        vec![oid.clone()]
    );
    let last = branch_page(&owner, &first.revision, first.total - 1);
    let tag = last
        .branches
        .iter()
        .find(|branch| branch.name == "v1")
        .unwrap();
    assert_eq!(tag.target, oid);
    assert!(branch_page(&owner, "stale", 0).error.is_some());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn pinned_pages_keep_the_original_history_when_a_branch_moves() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("termlevel-pinned-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let git = git_path().unwrap();
    let run = |args: &[&str]| {
        let output = Command::new(&git)
            .arg("-C")
            .arg(&dir)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    run(&["init", "-q", "-b", "main"]);
    run(&["config", "user.name", "Test"]);
    run(&["config", "user.email", "test@example.com"]);
    for subject in ["one", "two", "three", "four"] {
        run(&["commit", "--allow-empty", "-qm", subject]);
    }
    let root = dir.to_str().unwrap();
    let owner = requests::Owner::default();
    assert!(branch_snapshot(&owner, root, "").error.is_none());
    let tips = history_tips(&owner, root, "").unwrap();
    let first = log_sync_pinned(root, "", 2, 0, Some(&tips));
    assert_eq!(
        first
            .commits
            .iter()
            .map(|commit| commit.subject.as_str())
            .collect::<Vec<_>>(),
        ["four", "three"]
    );
    run(&["commit", "--allow-empty", "-qm", "five"]);
    let second = log_sync_pinned(root, "", 2, 2, Some(&tips));
    assert_eq!(
        second
            .commits
            .iter()
            .map(|commit| commit.subject.as_str())
            .collect::<Vec<_>>(),
        ["two", "one"]
    );
    assert_eq!(log_sync(root, "", 2, 0).commits[0].subject, "five");
    std::fs::remove_dir_all(dir).unwrap();
}
