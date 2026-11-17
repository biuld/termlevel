use super::*;

pub(crate) fn repository_root(git: &Path, root: &Path) -> Result<PathBuf, String> {
    let output = run_capture(
        git,
        Some(root),
        &["rev-parse", "--show-toplevel"],
        STATUS_TIMEOUT,
        8192,
    )?;
    if !output.status.success() {
        return Err(stderr_message(&output));
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        return Err("Git did not report a repository root.".into());
    }
    Ok(PathBuf::from(path))
}

pub(super) fn open_repository(root: &str) -> Result<(PathBuf, PathBuf), String> {
    let git = git_path()?;
    let root = resolve_root(root)?;
    let repo_root = repository_root(&git, &root)?;
    Ok((git, repo_root))
}

pub(crate) fn ref_exists(git: &Path, root: &Path, name: &str) -> Result<bool, String> {
    let output = run_capture(
        git,
        Some(root),
        &["rev-parse", "--verify", "--quiet", name],
        STATUS_TIMEOUT,
        4096,
    )?;
    Ok(output.status.success())
}

pub(super) fn branches_sync(root: &str) -> RepoBranches {
    let (git, repo_root) = match open_repository(root) {
        Ok(repository) => repository,
        Err(error) => {
            return RepoBranches {
                branches: Vec::new(),
                error: Some(error),
                ..Default::default()
            };
        }
    };
    let output = match run_capture(
        &git,
        Some(&repo_root),
        &[
            "for-each-ref",
            "--format=%(refname)%00%(refname:short)%00%(objectname)%00%(HEAD)%00%(upstream:short)%00%(*objectname)%00%(objecttype)%00%(*objecttype)",
            "refs/heads",
            "refs/remotes",
            "refs/tags",
        ],
        STATUS_TIMEOUT,
        32 * 1024 * 1024,
    ) {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            return RepoBranches {
                branches: Vec::new(),
                error: Some(stderr_message(&output)),
                ..Default::default()
            };
        }
        Err(error) => {
            return RepoBranches {
                branches: Vec::new(),
                error: Some(error),
                ..Default::default()
            };
        }
    };
    if output.truncated {
        return RepoBranches::failed(
            "Branch list exceeded the 32 MiB query limit. Use the terminal to inspect all refs."
                .into(),
        );
    }
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    output.stdout.hash(&mut hash);
    let branches = parse_branches(&output.stdout);
    RepoBranches {
        revision: format!("{:016x}", hash.finish()),
        total: branches.len() as u32,
        branches,
        error: None,
        ..Default::default()
    }
}

pub(super) fn parse_branches(bytes: &[u8]) -> Vec<RepoBranch> {
    let text = String::from_utf8_lossy(bytes);
    let mut branches = Vec::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.split('\0').collect();
        if fields.len() < 5 {
            continue;
        }
        let full_name = fields[0].trim();
        let name = fields[1].trim();
        let target = fields[2].trim();
        let head = fields[3].trim();
        let upstream = fields[4].trim();
        if full_name.is_empty() || name.is_empty() || target.is_empty() {
            continue;
        }
        let kind = if full_name.starts_with("refs/heads/") {
            "local"
        } else if full_name.starts_with("refs/remotes/") {
            "remote"
        } else if full_name.starts_with("refs/tags/") {
            "tag"
        } else {
            continue;
        };
        // origin/HEAD is a symbolic convenience ref, not a branch to switch to.
        if kind == "remote" && name.ends_with("/HEAD") {
            continue;
        }
        branches.push(RepoBranch {
            name: name.to_string(),
            full_name: full_name.to_string(),
            kind: kind.to_string(),
            current: kind == "local" && head == "*",
            upstream: (!upstream.is_empty()).then(|| upstream.to_string()),
            commit_target: fields.get(6).is_none_or(|kind| *kind == "commit")
                || fields.get(7).is_some_and(|kind| *kind == "commit"),
            target: if kind == "tag" {
                fields
                    .get(5)
                    .filter(|oid| !oid.is_empty())
                    .copied()
                    .unwrap_or(target)
            } else {
                target
            }
            .to_string(),
        });
    }
    branches.sort_by(|a, b| {
        let rank = |kind: &str| match kind {
            "local" => 0,
            "remote" => 1,
            _ => 2,
        };
        (rank(&a.kind), !a.current, a.name.to_lowercase()).cmp(&(
            rank(&b.kind),
            !b.current,
            b.name.to_lowercase(),
        ))
    });
    branches
}

pub(super) fn worktrees_sync(root: &str) -> RepoWorktrees {
    match list_worktrees(root) {
        Ok(worktrees) => RepoWorktrees {
            worktrees,
            error: None,
        },
        Err(error) => RepoWorktrees {
            worktrees: Vec::new(),
            error: Some(error),
        },
    }
}

/// Worktrees with `current` resolved against the repository that holds `root`.
pub(crate) fn list_worktrees(root: &str) -> Result<Vec<RepoWorktree>, String> {
    let (git, repo_root) = open_repository(root)?;
    let group_root = resolve_root(root)?;
    let output = run_capture(
        &git,
        Some(&repo_root),
        &["worktree", "list", "--porcelain"],
        STATUS_TIMEOUT,
        MAX_STATUS_BYTES,
    )?;
    if !output.status.success() {
        return Err(stderr_message(&output));
    }
    if output.truncated {
        return Err("Worktree list exceeded the output limit.".into());
    }
    let mut worktrees = parse_worktrees(&output.stdout);
    mark_current_worktree(&mut worktrees, &group_root);
    Ok(worktrees)
}

pub(super) fn parse_worktrees(bytes: &[u8]) -> Vec<RepoWorktree> {
    let text = String::from_utf8_lossy(bytes);
    let mut worktrees = Vec::new();
    let mut block: Vec<&str> = Vec::new();
    let mut main = true;
    for line in text.lines().chain(std::iter::once("")) {
        if line.is_empty() {
            if !block.is_empty() {
                if let Some(worktree) = parse_worktree_block(&block, main) {
                    worktrees.push(worktree);
                    main = false;
                }
                block.clear();
            }
        } else {
            block.push(line);
        }
    }
    worktrees
}

pub(super) fn parse_worktree_block(lines: &[&str], main: bool) -> Option<RepoWorktree> {
    let mut path = None;
    let mut branch = None;
    let mut head = None;
    let mut bare = false;
    let mut detached = false;
    let mut locked = None;
    let mut prunable = None;
    for line in lines {
        if let Some(value) = line.strip_prefix("worktree ") {
            path = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("HEAD ") {
            head = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("branch ") {
            branch = Some(
                value
                    .trim()
                    .strip_prefix("refs/heads/")
                    .unwrap_or(value.trim())
                    .to_string(),
            );
        } else if *line == "bare" {
            bare = true;
        } else if *line == "detached" {
            detached = true;
        } else if let Some(reason) = flag_reason(line, "locked") {
            locked = Some(reason);
        } else if let Some(reason) = flag_reason(line, "prunable") {
            prunable = Some(reason);
        }
    }
    Some(RepoWorktree {
        path: path?,
        branch,
        head,
        bare,
        detached,
        locked,
        prunable,
        main,
        current: false,
    })
}

pub(super) fn flag_reason(line: &str, flag: &str) -> Option<String> {
    let rest = line.strip_prefix(flag)?;
    if rest.is_empty() || rest.starts_with(' ') {
        let reason = rest.trim();
        Some(if reason.is_empty() {
            flag.to_string()
        } else {
            reason.to_string()
        })
    } else {
        None
    }
}

pub(super) fn mark_current_worktree(worktrees: &mut [RepoWorktree], group_root: &Path) {
    let group = canonical_path(group_root);
    let mut best: Option<(usize, usize)> = None;
    for (index, worktree) in worktrees.iter().enumerate() {
        let path = canonical_path(Path::new(&worktree.path));
        if group == path || group.starts_with(&path) {
            let length = path.as_os_str().len();
            if best.is_none_or(|(_, current)| length > current) {
                best = Some((index, length));
            }
        }
    }
    if let Some((index, _)) = best {
        worktrees[index].current = true;
    }
}

pub(crate) fn canonical_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// One small batch crosses the engine boundary at a time; the complete snapshot stays here.
const BRANCH_PAGE: usize = 256;
pub(super) struct BranchSnapshot {
    root: String,
    head: String,
    revision: String,
    branches: Vec<RepoBranch>,
    pub decorations: std::collections::HashMap<String, super::decorations::Decoration>,
}
pub(super) fn branch_snapshot(owner: &requests::Owner, root: &str, revision: &str) -> RepoBranches {
    let mut result = branches_sync(root);
    if result.error.is_some() {
        return result;
    }
    if let Err(error) = requests::check() {
        return RepoBranches::failed(error);
    }
    let (head, decorations) = match super::decorations::index(root, &mut result) {
        Ok(index) => index,
        Err(error) => return RepoBranches::failed(error),
    };
    if revision == result.revision {
        return RepoBranches {
            revision: result.revision,
            unchanged: true,
            total: result.total,
            ..Default::default()
        };
    }
    let revision = result.revision;
    *owner.branches.lock().unwrap() = Some(BranchSnapshot {
        root: root.into(),
        head,
        revision: revision.clone(),
        branches: result.branches,
        decorations,
    });
    branch_page(owner, &revision, 0)
}
pub(super) fn branch_page(owner: &requests::Owner, revision: &str, offset: u32) -> RepoBranches {
    let snapshot = owner.branches.lock().unwrap();
    let Some(snapshot) = snapshot
        .as_ref()
        .filter(|snapshot| snapshot.revision == revision)
    else {
        return RepoBranches::failed("Branch snapshot changed. Refresh the branch list.".into());
    };
    RepoBranches {
        revision: revision.into(),
        total: snapshot.branches.len() as u32,
        branches: snapshot
            .branches
            .iter()
            .skip(offset as usize)
            .take(BRANCH_PAGE)
            .cloned()
            .collect(),
        ..Default::default()
    }
}
/// A traversal keeps immutable tips across pages even if fetch/commit moves refs.
pub(super) fn history_tips(
    owner: &requests::Owner,
    root: &str,
    scope: &str,
) -> Option<Vec<String>> {
    let snapshot = owner.branches.lock().unwrap();
    let snapshot = snapshot.as_ref().filter(|snapshot| snapshot.root == root)?;
    let mut tips: Vec<String> = snapshot
        .branches
        .iter()
        .filter(|branch| branch.commit_target && (scope.is_empty() || branch.full_name == scope))
        .map(|branch| branch.target.clone())
        .collect();
    if scope.is_empty() && !snapshot.head.is_empty() {
        tips.push(snapshot.head.clone());
    }
    tips.sort_unstable();
    tips.dedup();
    Some(tips)
}
/// Match full names including refs omitted from the bounded chip preview.
pub(super) fn match_refs(owner: &requests::Owner, query: &str) -> Result<Vec<String>, String> {
    let query = query.to_lowercase();
    let loaded = owner.graph.lock().unwrap().loaded.clone();
    let snapshot = owner.branches.lock().unwrap();
    let mut matches = std::collections::HashSet::new();
    if let Some(snapshot) = snapshot.as_ref() {
        for branch in &snapshot.branches {
            requests::check()?;
            if loaded.contains(&branch.target) && branch.name.to_lowercase().contains(&query) {
                matches.insert(branch.target.clone());
            }
        }
    }
    Ok(matches.into_iter().collect())
}
