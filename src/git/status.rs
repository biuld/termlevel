use super::*;

pub(super) fn status_sync(root: &str) -> RepoStatus {
    let git = match git_path() {
        Ok(path) => path,
        Err(error) => return RepoStatus::failed(error),
    };
    let root = match resolve_root(root) {
        Ok(path) => path,
        Err(error) => return RepoStatus::failed(error),
    };
    let inside = match run_capture(
        &git,
        Some(&root),
        &["rev-parse", "--is-inside-work-tree"],
        STATUS_TIMEOUT,
        1024,
    ) {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        Ok(output) => return not_repo_or_failed(&output),
        Err(error) => return RepoStatus::failed(error),
    };
    if inside != "true" {
        return RepoStatus::empty();
    }
    let top = match run_capture(
        &git,
        Some(&root),
        &["rev-parse", "--show-toplevel"],
        STATUS_TIMEOUT,
        8192,
    ) {
        Ok(output) if output.status.success() => output.stdout,
        Ok(output) => return RepoStatus::failed(stderr_message(&output)),
        Err(error) => return RepoStatus::failed(error),
    };
    let repo_root = PathBuf::from(String::from_utf8_lossy(&top).trim().to_string());
    if repo_root.as_os_str().is_empty() {
        return RepoStatus::failed("Git did not report a repository root.".into());
    }
    let operation = git_dir_for(&repo_root)
        .as_deref()
        .and_then(operation_in_progress);
    let output = match run_capture(
        &git,
        Some(&repo_root),
        &[
            "status",
            "--porcelain=v2",
            "--branch",
            "-z",
            "--untracked-files=all",
        ],
        STATUS_TIMEOUT,
        MAX_STATUS_BYTES,
    ) {
        Ok(output) if output.status.success() => output,
        Ok(output) => return RepoStatus::failed(stderr_message(&output)),
        Err(error) => return RepoStatus::failed(error),
    };
    let mut status = parse_status(&output.stdout);
    status.repo = true;
    status.root = Some(repo_root.to_string_lossy().into_owned());
    status.operation = operation;
    status.truncated |= output.truncated;
    status
}

pub(super) fn not_repo_or_failed(output: &Captured) -> RepoStatus {
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("not a git repository") || stderr.contains("not a git work tree") {
        RepoStatus::empty()
    } else {
        RepoStatus::failed(stderr_message(output))
    }
}

pub(crate) fn stderr_message(output: &Captured) -> String {
    let text = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if text.is_empty() {
        "Git command failed.".into()
    } else {
        text.chars().take(400).collect()
    }
}

pub(super) fn parse_status(bytes: &[u8]) -> RepoStatus {
    let mut status = RepoStatus::empty();
    let mut oid = String::new();
    let mut head = String::new();
    let mut records: Vec<&[u8]> = bytes.split(|byte| *byte == 0).collect();
    // A capped read may end in the middle of a record; never parse that tail.
    if bytes.last() != Some(&0) {
        records.pop();
    }
    let mut index = 0;
    while index < records.len() {
        let record = records[index];
        index += 1;
        if record.is_empty() {
            continue;
        }
        let text = String::from_utf8_lossy(record);
        let line = text.trim_end_matches(['\n', '\r']);
        if let Some(value) = line.strip_prefix("# branch.oid ") {
            oid = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("# branch.head ") {
            head = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("# branch.upstream ") {
            status.upstream = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("# branch.ab ") {
            for token in value.split_whitespace() {
                if let Some(number) = token.strip_prefix('+') {
                    status.ahead = number.parse().unwrap_or(0);
                } else if let Some(number) = token.strip_prefix('-') {
                    status.behind = number.parse().unwrap_or(0);
                }
            }
        } else if line.starts_with("1 ") {
            if let Some(entry) = parse_ordinary(line) {
                status.changes.push(entry);
            }
        } else if line.starts_with("2 ") {
            let old_path = records.get(index).map(|next| {
                String::from_utf8_lossy(next)
                    .trim_end_matches(['\n', '\r'])
                    .to_string()
            });
            index += 1;
            if let Some(entry) = parse_renamed(line, old_path) {
                status.changes.push(entry);
            }
        } else if line.starts_with("u ") {
            if let Some(entry) = parse_unmerged(line) {
                status.changes.push(entry);
            }
        } else if let Some(path) = line.strip_prefix("? ") {
            status.changes.push(RepoEntry {
                path: path.to_string(),
                old_path: None,
                status: "?".into(),
                staged: false,
                unstaged: false,
                untracked: true,
                conflicted: false,
                kind: "untracked".into(),
            });
        }
        if status.changes.len() >= MAX_ENTRIES {
            status.truncated = true;
            break;
        }
    }
    if head == "(detached)" {
        status.detached = true;
        status.branch = (!oid.is_empty()).then(|| oid.chars().take(7).collect());
    } else if !head.is_empty() {
        status.branch = Some(head);
    }
    status
}

pub(super) fn parse_ordinary(line: &str) -> Option<RepoEntry> {
    let fields: Vec<&str> = line.splitn(9, ' ').collect();
    if fields.len() < 9 {
        return None;
    }
    let xy = fields[1].as_bytes();
    let index = xy.first().copied().unwrap_or(b'.') as char;
    let worktree = xy.get(1).copied().unwrap_or(b'.') as char;
    let letter = if index == '.' { worktree } else { index };
    Some(RepoEntry {
        path: fields[8].to_string(),
        old_path: None,
        status: letter.to_string(),
        staged: index != '.',
        unstaged: worktree != '.',
        untracked: false,
        conflicted: false,
        kind: if fields[2].starts_with('S') {
            "submodule"
        } else {
            "ordinary"
        }
        .into(),
    })
}

pub(super) fn parse_renamed(line: &str, old_path: Option<String>) -> Option<RepoEntry> {
    let fields: Vec<&str> = line.splitn(10, ' ').collect();
    if fields.len() < 10 {
        return None;
    }
    let xy = fields[1].as_bytes();
    let index = xy.first().copied().unwrap_or(b'.') as char;
    let worktree = xy.get(1).copied().unwrap_or(b'.') as char;
    let letter = fields[8].chars().next().unwrap_or('R');
    Some(RepoEntry {
        path: fields[9].to_string(),
        old_path,
        status: letter.to_string(),
        staged: index != '.',
        unstaged: worktree != '.',
        untracked: false,
        conflicted: false,
        kind: if letter == 'C' { "copied" } else { "renamed" }.into(),
    })
}

pub(super) fn parse_unmerged(line: &str) -> Option<RepoEntry> {
    let fields: Vec<&str> = line.splitn(11, ' ').collect();
    if fields.len() < 11 {
        return None;
    }
    Some(RepoEntry {
        path: fields[10].to_string(),
        old_path: None,
        status: "U".into(),
        staged: false,
        unstaged: false,
        untracked: false,
        conflicted: true,
        kind: "unmerged".into(),
    })
}

pub(super) fn git_dir_for(repo_root: &Path) -> Option<PathBuf> {
    let dot_git = repo_root.join(".git");
    if dot_git.is_dir() {
        return Some(dot_git);
    }
    if dot_git.is_file() {
        let text = std::fs::read_to_string(&dot_git).ok()?;
        let path = PathBuf::from(text.strip_prefix("gitdir:")?.trim());
        return Some(if path.is_absolute() {
            path
        } else {
            repo_root.join(path)
        });
    }
    None
}

pub(super) fn operation_in_progress(git_dir: &Path) -> Option<String> {
    let has = |name: &str| git_dir.join(name).exists();
    if has("rebase-merge") || has("rebase-apply") {
        Some("rebase".into())
    } else if has("MERGE_HEAD") {
        Some("merge".into())
    } else if has("CHERRY_PICK_HEAD") {
        Some("cherry-pick".into())
    } else if has("REVERT_HEAD") {
        Some("revert".into())
    } else if has("BISECT_LOG") {
        Some("bisect".into())
    } else {
        None
    }
}
