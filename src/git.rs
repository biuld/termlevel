//! Read-only Git data for a group's overview.
//!
//! The system `git` CLI is the interface: porcelain v2 status and unified
//! patches. Nothing here writes to the repository or runs hooks.

use nsh_core::prelude::*;
use std::{
    collections::HashSet,
    io::Read,
    path::{Component, Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::OnceLock,
    thread,
    time::{Duration, Instant},
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

/// Oldest Git the status format and flags below support.
const MIN_GIT: (u32, u32, u32) = (2, 20, 0);
const STATUS_TIMEOUT: Duration = Duration::from_secs(3);
const DIFF_TIMEOUT: Duration = Duration::from_secs(3);
const LOG_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_LOG_PAGE: u32 = 500;
const VERSION_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_STATUS_BYTES: usize = 2 * 1024 * 1024;
const MAX_DIFF_BYTES: usize = 2 * 1024 * 1024;
const MAX_STDERR_BYTES: usize = 64 * 1024;
const MAX_UNTRACKED_BYTES: u64 = 512 * 1024;
const MAX_UNTRACKED_LINES: usize = 5000;
const MAX_ENTRIES: usize = 5000;

#[derive(Clone, Debug, PartialEq, JsObject)]
pub struct RepoEntry {
    /// Repository-root-relative path.
    pub path: String,
    /// The previous path of a rename or copy.
    pub old_path: Option<String>,
    /// One status letter: M, A, D, R, C, T, U or ?.
    pub status: String,
    /// The index differs from HEAD.
    pub staged: bool,
    /// The worktree differs from the index.
    pub unstaged: bool,
    /// Git does not track the path.
    pub untracked: bool,
    /// The path has an unmerged index entry.
    pub conflicted: bool,
    /// ordinary, renamed, copied, unmerged, untracked or submodule.
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, JsObject)]
pub struct RepoStatus {
    pub repo: bool,
    pub root: Option<String>,
    pub branch: Option<String>,
    pub detached: bool,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    /// merge, rebase, cherry-pick, revert or bisect while one is in progress.
    pub operation: Option<String>,
    pub changes: Vec<RepoEntry>,
    /// The command output or the entry list was capped.
    pub truncated: bool,
    /// Why Git could not answer; absent when the folder is simply not a repo.
    pub error: Option<String>,
}

impl RepoStatus {
    fn empty() -> Self {
        Self {
            repo: false,
            root: None,
            branch: None,
            detached: false,
            upstream: None,
            ahead: 0,
            behind: 0,
            operation: None,
            changes: Vec::new(),
            truncated: false,
            error: None,
        }
    }

    fn failed(message: String) -> Self {
        Self {
            error: Some(message),
            ..Self::empty()
        }
    }
}

/// One local branch, remote-tracking branch or tag.
#[derive(Clone, Debug, PartialEq, JsObject)]
pub struct RepoBranch {
    /// The short name: `main`, `origin/main` or a tag name.
    pub name: String,
    /// The full ref: `refs/heads/main`, `refs/remotes/origin/main` or `refs/tags/v1`.
    pub full_name: String,
    /// local, remote or tag.
    pub kind: String,
    /// The branch this worktree has checked out.
    pub current: bool,
    /// The upstream short name, when the branch tracks one.
    pub upstream: Option<String>,
    /// The object the ref points at.
    pub target: String,
}

/// Branches and tags, with why Git could not list them.
#[derive(Clone, Debug, PartialEq, JsObject)]
pub struct RepoBranches {
    pub branches: Vec<RepoBranch>,
    pub error: Option<String>,
}

/// One worktree of the repository.
#[derive(Clone, Debug, PartialEq, JsObject)]
pub struct RepoWorktree {
    pub path: String,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub bare: bool,
    pub detached: bool,
    pub locked: Option<String>,
    pub prunable: Option<String>,
    /// The repository's first worktree.
    pub main: bool,
    /// The worktree that holds the folder the overview asked about.
    pub current: bool,
}

/// Worktrees, with why Git could not list them.
#[derive(Clone, Debug, PartialEq, JsObject)]
pub struct RepoWorktrees {
    pub worktrees: Vec<RepoWorktree>,
    pub error: Option<String>,
}

/// One commit in the history list.
#[derive(Clone, Debug, PartialEq, JsObject)]
pub struct RepoCommit {
    pub oid: String,
    pub short: String,
    pub parents: Vec<String>,
    pub author: String,
    pub email: String,
    /// Author time, seconds since the Unix epoch.
    pub timestamp: i64,
    /// Decorations such as `HEAD -> main`, `origin/main` or `tag: v1`.
    pub refs: Vec<String>,
    pub subject: String,
}

impl RepoCommit {
    fn empty(oid: String) -> Self {
        Self {
            short: oid.chars().take(7).collect(),
            oid,
            parents: Vec::new(),
            author: String::new(),
            email: String::new(),
            timestamp: 0,
            refs: Vec::new(),
            subject: String::new(),
        }
    }
}

/// A page of commits.
#[derive(Clone, Debug, PartialEq, JsObject)]
pub struct RepoLog {
    pub commits: Vec<RepoCommit>,
    pub has_more: bool,
    pub truncated: bool,
    pub error: Option<String>,
}

/// One path changed by a commit.
#[derive(Clone, Debug, PartialEq, JsObject)]
pub struct RepoCommitFile {
    pub path: String,
    pub old_path: Option<String>,
    /// One status letter: M, A, D, R, C, T or U.
    pub status: String,
}

/// One commit's metadata, changed paths and patch.
#[derive(Clone, Debug, PartialEq, JsObject)]
pub struct RepoCommitDetail {
    pub commit: RepoCommit,
    pub body: String,
    pub files: Vec<RepoCommitFile>,
    /// A unified patch against the first parent.
    pub patch: String,
    pub truncated: bool,
    pub error: Option<String>,
}

/// Read-only Git status and patches from the system CLI.
#[module]
pub struct Git;

#[module]
impl Git {
    /// Branch, upstream, ahead/behind, operation and changed paths for a group.
    pub async fn status(root: String) -> RepoStatus {
        match blocking(move || status_sync(&root)).await {
            Ok(status) => status,
            Err(_) => RepoStatus::failed("The Git status task failed.".into()),
        }
    }

    /// A unified patch for one changed path. `mode` is staged, unstaged or untracked.
    pub async fn diff(root: String, path: String, mode: String) -> Result<String, String> {
        match blocking(move || diff_sync(&root, &path, &mode)).await {
            Ok(result) => result,
            Err(_) => Err("The Git diff task failed.".into()),
        }
    }

    /// Local branches, remote-tracking branches and tags.
    pub async fn branches(root: String) -> RepoBranches {
        match blocking(move || branches_sync(&root)).await {
            Ok(result) => result,
            Err(_) => RepoBranches {
                branches: Vec::new(),
                error: Some("The Git branch task failed.".into()),
            },
        }
    }

    /// Worktrees of the repository that contains `root`.
    pub async fn worktrees(root: String) -> RepoWorktrees {
        match blocking(move || worktrees_sync(&root)).await {
            Ok(result) => result,
            Err(_) => RepoWorktrees {
                worktrees: Vec::new(),
                error: Some("The Git worktree task failed.".into()),
            },
        }
    }

    /// A page of commits. `scope` is a full ref, or empty for every ref.
    pub async fn log(root: String, scope: String, limit: u32, skip: u32) -> RepoLog {
        match blocking(move || log_sync(&root, &scope, limit, skip)).await {
            Ok(result) => result,
            Err(_) => RepoLog {
                commits: Vec::new(),
                has_more: false,
                truncated: false,
                error: Some("The Git log task failed.".into()),
            },
        }
    }

    /// One commit's metadata, changed paths and patch against its first parent.
    pub async fn commit(root: String, oid: String) -> RepoCommitDetail {
        let fallback = oid.clone();
        match blocking(move || commit_sync(&root, &oid)).await {
            Ok(result) => result,
            Err(_) => RepoCommitDetail {
                commit: RepoCommit::empty(fallback),
                body: String::new(),
                files: Vec::new(),
                patch: String::new(),
                truncated: false,
                error: Some("The Git commit task failed.".into()),
            },
        }
    }
}

pub(crate) struct Captured {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
    pub(crate) truncated: bool,
}

pub(crate) fn git_path() -> Result<PathBuf, String> {
    static PATH: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    PATH.get_or_init(resolve_git).clone()
}

fn resolve_git() -> Result<PathBuf, String> {
    let path = find_git().ok_or_else(|| {
        "Git is not installed or is not on the application's PATH. \
         Install Git 2.20 or newer, then reopen this overview."
            .to_string()
    })?;
    let output = run_capture(&path, None, &["--version"], VERSION_TIMEOUT, 4096)
        .map_err(|error| format!("Git could not be started: {error}"))?;
    if !output.status.success() {
        return Err(
            "Git could not be started. Run `git --version` in a terminal to check it.".into(),
        );
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let version = parse_version(&text)
        .ok_or_else(|| format!("Git version could not be read: {}", text.trim()))?;
    if version < MIN_GIT {
        return Err(format!(
            "Git {}.{}.{} is too old. Termlevel needs Git 2.20 or newer.",
            version.0, version.1, version.2
        ));
    }
    Ok(path)
}

fn find_git() -> Option<PathBuf> {
    let name = if cfg!(windows) { "git.exe" } else { "git" };
    let mut seen = HashSet::new();
    let mut dirs = Vec::new();
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }
    #[cfg(unix)]
    dirs.extend(
        ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"]
            .into_iter()
            .map(PathBuf::from),
    );
    for dir in dirs {
        if !seen.insert(dir.clone()) {
            continue;
        }
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn parse_version(text: &str) -> Option<(u32, u32, u32)> {
    let token = text
        .strip_prefix("git version ")?
        .split_whitespace()
        .next()?;
    let mut parts = token.split('.');
    Some((
        leading_number(parts.next()?)?,
        leading_number(parts.next().unwrap_or("0"))?,
        leading_number(parts.next().unwrap_or("0"))?,
    ))
}

fn leading_number(text: &str) -> Option<u32> {
    let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

fn status_sync(root: &str) -> RepoStatus {
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

fn not_repo_or_failed(output: &Captured) -> RepoStatus {
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

fn parse_status(bytes: &[u8]) -> RepoStatus {
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

fn parse_ordinary(line: &str) -> Option<RepoEntry> {
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

fn parse_renamed(line: &str, old_path: Option<String>) -> Option<RepoEntry> {
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

fn parse_unmerged(line: &str) -> Option<RepoEntry> {
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

fn git_dir_for(repo_root: &Path) -> Option<PathBuf> {
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

fn operation_in_progress(git_dir: &Path) -> Option<String> {
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

fn open_repository(root: &str) -> Result<(PathBuf, PathBuf), String> {
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

fn branches_sync(root: &str) -> RepoBranches {
    let (git, repo_root) = match open_repository(root) {
        Ok(repository) => repository,
        Err(error) => {
            return RepoBranches {
                branches: Vec::new(),
                error: Some(error),
            };
        }
    };
    let output = match run_capture(
        &git,
        Some(&repo_root),
        &[
            "for-each-ref",
            "--format=%(refname)%00%(refname:short)%00%(objectname)%00%(HEAD)%00%(upstream:short)",
            "refs/heads",
            "refs/remotes",
            "refs/tags",
        ],
        STATUS_TIMEOUT,
        MAX_STATUS_BYTES,
    ) {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            return RepoBranches {
                branches: Vec::new(),
                error: Some(stderr_message(&output)),
            };
        }
        Err(error) => {
            return RepoBranches {
                branches: Vec::new(),
                error: Some(error),
            };
        }
    };
    RepoBranches {
        branches: parse_branches(&output.stdout),
        error: None,
    }
}

fn parse_branches(bytes: &[u8]) -> Vec<RepoBranch> {
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
            target: target.to_string(),
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

fn worktrees_sync(root: &str) -> RepoWorktrees {
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
    let mut worktrees = parse_worktrees(&output.stdout);
    mark_current_worktree(&mut worktrees, &group_root);
    Ok(worktrees)
}

fn parse_worktrees(bytes: &[u8]) -> Vec<RepoWorktree> {
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

fn parse_worktree_block(lines: &[&str], main: bool) -> Option<RepoWorktree> {
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

fn flag_reason(line: &str, flag: &str) -> Option<String> {
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

fn mark_current_worktree(worktrees: &mut [RepoWorktree], group_root: &Path) {
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

fn log_sync(root: &str, scope: &str, limit: u32, skip: u32) -> RepoLog {
    let (git, repo_root) = match open_repository(root) {
        Ok(repository) => repository,
        Err(error) => {
            return RepoLog {
                commits: Vec::new(),
                has_more: false,
                truncated: false,
                error: Some(error),
            };
        }
    };
    if !scope.is_empty()
        && (!scope.starts_with("refs/") || scope.chars().any(|character| character.is_whitespace()))
    {
        return RepoLog {
            commits: Vec::new(),
            has_more: false,
            truncated: false,
            error: Some("The history scope must be a full Git ref.".into()),
        };
    }
    let limit = limit.clamp(1, MAX_LOG_PAGE);
    let format_arg = "--format=%H%x00%h%x00%P%x00%an%x00%ae%x00%at%x00%D%x00%s";
    let mut args: Vec<String> = vec![
        "log".into(),
        "--no-color".into(),
        "--no-show-signature".into(),
        "--topo-order".into(),
        format!("--max-count={}", limit + 1),
        format!("--skip={skip}"),
        format_arg.into(),
    ];
    if scope.is_empty() {
        args.push("--all".into());
    } else {
        args.push(scope.to_string());
    }
    let arguments: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = match run_capture(
        &git,
        Some(&repo_root),
        &arguments,
        LOG_TIMEOUT,
        MAX_DIFF_BYTES,
    ) {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            let message = stderr_message(&output);
            let empty = message.contains("does not have any commits yet")
                || message.contains("bad revision");
            return RepoLog {
                commits: Vec::new(),
                has_more: false,
                truncated: false,
                error: (!empty).then_some(message),
            };
        }
        Err(error) => {
            return RepoLog {
                commits: Vec::new(),
                has_more: false,
                truncated: false,
                error: Some(error),
            };
        }
    };
    let mut commits = parse_log(&output.stdout);
    let has_more = commits.len() > limit as usize;
    if has_more {
        commits.truncate(limit as usize);
    }
    RepoLog {
        commits,
        has_more: has_more || output.truncated,
        truncated: output.truncated,
        error: None,
    }
}

fn parse_log(bytes: &[u8]) -> Vec<RepoCommit> {
    let text = String::from_utf8_lossy(bytes);
    let mut commits = Vec::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.split('\0').collect();
        if fields.len() < 8 {
            continue;
        }
        let oid = fields[0].trim();
        if oid.is_empty() {
            continue;
        }
        commits.push(RepoCommit {
            oid: oid.to_string(),
            short: fields[1].trim().to_string(),
            parents: fields[2].split_whitespace().map(str::to_string).collect(),
            author: fields[3].to_string(),
            email: fields[4].to_string(),
            timestamp: fields[5].trim().parse().unwrap_or(0),
            refs: parse_refs(fields[6]),
            subject: fields[7].to_string(),
        });
    }
    commits
}

fn parse_refs(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

fn commit_sync(root: &str, oid: &str) -> RepoCommitDetail {
    let (git, repo_root) = match open_repository(root) {
        Ok(repository) => repository,
        Err(error) => {
            return RepoCommitDetail {
                commit: RepoCommit::empty(oid.to_string()),
                body: String::new(),
                files: Vec::new(),
                patch: String::new(),
                truncated: false,
                error: Some(error),
            };
        }
    };
    if oid.is_empty()
        || oid.len() > 64
        || !oid.chars().all(|character| character.is_ascii_hexdigit())
    {
        return RepoCommitDetail {
            commit: RepoCommit::empty(oid.to_string()),
            body: String::new(),
            files: Vec::new(),
            patch: String::new(),
            truncated: false,
            error: Some("Not a commit hash.".into()),
        };
    }
    let format_arg = "--format=%H%x00%h%x00%P%x00%an%x00%ae%x00%at%x00%D%x00%s%x00%b";
    let meta = match run_capture(
        &git,
        Some(&repo_root),
        &[
            "show",
            "-s",
            "--no-color",
            "--no-show-signature",
            format_arg,
            oid,
        ],
        STATUS_TIMEOUT,
        MAX_STDERR_BYTES,
    ) {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            return RepoCommitDetail {
                commit: RepoCommit::empty(oid.to_string()),
                body: String::new(),
                files: Vec::new(),
                patch: String::new(),
                truncated: false,
                error: Some(stderr_message(&output)),
            };
        }
        Err(error) => {
            return RepoCommitDetail {
                commit: RepoCommit::empty(oid.to_string()),
                body: String::new(),
                files: Vec::new(),
                patch: String::new(),
                truncated: false,
                error: Some(error),
            };
        }
    };
    let meta_text = String::from_utf8_lossy(&meta.stdout);
    let fields: Vec<&str> = meta_text.split('\0').collect();
    let commit = if fields.len() >= 9 {
        RepoCommit {
            oid: fields[0].trim().to_string(),
            short: fields[1].trim().to_string(),
            parents: fields[2].split_whitespace().map(str::to_string).collect(),
            author: fields[3].to_string(),
            email: fields[4].to_string(),
            timestamp: fields[5].trim().parse().unwrap_or(0),
            refs: parse_refs(fields[6]),
            subject: fields[7].to_string(),
        }
    } else {
        RepoCommit::empty(oid.to_string())
    };
    let body = fields
        .get(8)
        .map(|value| value.trim_end_matches(['\n', '\r']).to_string())
        .unwrap_or_default();
    let files = run_capture(
        &git,
        Some(&repo_root),
        &[
            "show",
            "--no-color",
            "--format=",
            "--name-status",
            "--find-renames",
            "--first-parent",
            oid,
        ],
        STATUS_TIMEOUT,
        MAX_STATUS_BYTES,
    )
    .map(|output| {
        if output.status.success() {
            parse_name_status(&output.stdout)
        } else {
            Vec::new()
        }
    })
    .unwrap_or_default();
    let patch = match run_capture(
        &git,
        Some(&repo_root),
        &[
            "show",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "--format=",
            "--unified=3",
            "--find-renames",
            "--first-parent",
            oid,
        ],
        DIFF_TIMEOUT,
        MAX_DIFF_BYTES,
    ) {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            return RepoCommitDetail {
                commit,
                body,
                files,
                patch: String::new(),
                truncated: false,
                error: Some(stderr_message(&output)),
            };
        }
        Err(error) => {
            return RepoCommitDetail {
                commit,
                body,
                files,
                patch: String::new(),
                truncated: false,
                error: Some(error),
            };
        }
    };
    RepoCommitDetail {
        commit,
        body,
        files,
        patch: String::from_utf8_lossy(&patch.stdout).into_owned(),
        truncated: patch.truncated,
        error: None,
    }
}

fn parse_name_status(bytes: &[u8]) -> Vec<RepoCommitFile> {
    let text = String::from_utf8_lossy(bytes);
    let mut files = Vec::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 2 {
            continue;
        }
        let status = fields[0].chars().next().unwrap_or('?').to_string();
        if (status == "R" || status == "C") && fields.len() >= 3 {
            files.push(RepoCommitFile {
                path: fields[2].to_string(),
                old_path: Some(fields[1].to_string()),
                status,
            });
        } else {
            files.push(RepoCommitFile {
                path: fields[1].to_string(),
                old_path: None,
                status,
            });
        }
    }
    files
}

fn diff_sync(root: &str, path: &str, mode: &str) -> Result<String, String> {
    let git = git_path()?;
    let root = resolve_root(root)?;
    let top = run_capture(
        &git,
        Some(&root),
        &["rev-parse", "--show-toplevel"],
        DIFF_TIMEOUT,
        8192,
    )?;
    if !top.status.success() {
        return Err(stderr_message(&top));
    }
    let repo_root = PathBuf::from(String::from_utf8_lossy(&top.stdout).trim().to_string());
    let path = safe_relative(path)?;
    match mode {
        "staged" | "unstaged" => {
            let mut args = vec![
                "diff",
                "--no-ext-diff",
                "--no-textconv",
                "--no-color",
                "--unified=3",
                "--submodule=short",
            ];
            if mode == "staged" {
                args.push("--cached");
            }
            args.push("--");
            args.push(path.as_str());
            let output = run_capture(&git, Some(&repo_root), &args, DIFF_TIMEOUT, MAX_DIFF_BYTES)?;
            if !output.status.success() {
                return Err(stderr_message(&output));
            }
            if output.truncated {
                return Err(
                    "Diff is too large to display (over 2 MB). Open it in the terminal.".into(),
                );
            }
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        }
        "untracked" => untracked_patch(&repo_root, &path),
        _ => Err("Unknown diff mode.".into()),
    }
}

fn untracked_patch(repo_root: &Path, path: &str) -> Result<String, String> {
    let file = repo_root.join(path);
    let metadata = std::fs::symlink_metadata(&file)
        .map_err(|error| format!("Could not read {path}: {error}"))?;
    if metadata.file_type().is_symlink() {
        return Err("Symbolic link. Open it in the terminal.".into());
    }
    if !metadata.is_file() {
        return Err("Not a regular file.".into());
    }
    if metadata.len() > MAX_UNTRACKED_BYTES {
        return Err("New file is too large to preview (over 512 KB).".into());
    }
    let bytes = std::fs::read(&file).map_err(|error| format!("Could not read {path}: {error}"))?;
    if bytes.iter().take(8192).any(|byte| *byte == 0) {
        return Err("Binary file. Open it in the terminal.".into());
    }
    let text = String::from_utf8_lossy(&bytes);
    let mut lines: Vec<&str> = text.lines().take(MAX_UNTRACKED_LINES).collect();
    let truncated = text.lines().nth(MAX_UNTRACKED_LINES).is_some();
    let mut patch = String::new();
    patch.push_str("--- /dev/null\n");
    patch.push_str(&format!("+++ b/{path}\n"));
    if lines.is_empty() {
        patch.push_str("@@ -0,0 +1,1 @@\n+\n");
        return Ok(patch);
    }
    patch.push_str(&format!("@@ -0,0 +1,{} @@\n", lines.len()));
    for line in lines.drain(..) {
        patch.push('+');
        patch.push_str(line);
        patch.push('\n');
    }
    if truncated {
        patch.push_str("+… (preview truncated)\n");
    } else if !text.ends_with('\n') {
        patch.push_str("\\ No newline at end of file\n");
    }
    Ok(patch)
}

fn safe_relative(path: &str) -> Result<String, String> {
    if path.is_empty() {
        return Err("Empty path.".into());
    }
    let candidate = Path::new(path);
    if candidate.is_absolute()
        || candidate
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("Path must stay inside the repository.".into());
    }
    Ok(path.to_string())
}

pub(crate) fn resolve_root(root: &str) -> Result<PathBuf, String> {
    let expanded = expand_path(root);
    let path = expanded
        .canonicalize()
        .map_err(|error| format!("Cannot open {}: {error}", expanded.display()))?;
    if !path.is_dir() {
        return Err(format!("Not a directory: {}", path.display()));
    }
    Ok(path)
}

pub(crate) fn expand_path(root: &str) -> PathBuf {
    let trimmed = root.trim();
    if trimmed == "~" {
        return crate::sessions::home_directory();
    }
    if let Some(rest) = trimmed.strip_prefix("~/") {
        return crate::sessions::home_directory().join(rest);
    }
    PathBuf::from(trimmed)
}

pub(crate) fn run_capture(
    git: &Path,
    root: Option<&Path>,
    args: &[&str],
    timeout: Duration,
    max_stdout: usize,
) -> Result<Captured, String> {
    run_capture_with(git, root, args, timeout, max_stdout, true)
}

/// Runs an explicit write operation. Unlike reads, it may take optional locks
/// and is allowed to run the repository's own hooks.
pub(crate) fn run_capture_mutating(
    git: &Path,
    root: Option<&Path>,
    args: &[&str],
    timeout: Duration,
    max_stdout: usize,
) -> Result<Captured, String> {
    run_capture_with(git, root, args, timeout, max_stdout, false)
}

fn run_capture_with(
    git: &Path,
    root: Option<&Path>,
    args: &[&str],
    timeout: Duration,
    max_stdout: usize,
    read_only: bool,
) -> Result<Captured, String> {
    let mut command = Command::new(git);
    if let Some(root) = root {
        command.arg("-C").arg(root);
    }
    command.args(args);
    if read_only {
        command.env("GIT_OPTIONAL_LOCKS", "0");
    }
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_PAGER", "cat")
        .env("LC_ALL", "C")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove("GIT_NAMESPACE")
        .env_remove("GIT_PREFIX")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command
        .spawn()
        .map_err(|error| format!("Git could not be started: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Git stdout was unavailable.".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Git stderr was unavailable.".to_string())?;
    let out_reader = thread::spawn(move || read_capped(stdout, max_stdout));
    let err_reader = thread::spawn(move || read_capped(stderr, MAX_STDERR_BYTES));
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(error) => {
                kill_tree(&mut child);
                let _ = out_reader.join();
                let _ = err_reader.join();
                return Err(error.to_string());
            }
        }
        if Instant::now() >= deadline {
            kill_tree(&mut child);
            let _ = child.wait();
            let _ = out_reader.join();
            let _ = err_reader.join();
            return Err("Git timed out.".into());
        }
        thread::sleep(Duration::from_millis(10));
    };
    let (stdout, truncated) = out_reader
        .join()
        .map_err(|_| "Git output reader failed.".to_string())?;
    let (stderr, _) = err_reader
        .join()
        .map_err(|_| "Git error reader failed.".to_string())?;
    Ok(Captured {
        status,
        stdout,
        stderr,
        truncated,
    })
}

fn read_capped(mut reader: impl Read, limit: usize) -> (Vec<u8>, bool) {
    let mut output = Vec::new();
    let mut buffer = [0u8; 8192];
    let mut truncated = false;
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                let remaining = limit.saturating_sub(output.len());
                if read > remaining {
                    output.extend_from_slice(&buffer[..remaining]);
                    truncated = true;
                } else {
                    output.extend_from_slice(&buffer[..read]);
                }
            }
            Err(_) => break,
        }
    }
    (output, truncated)
}

fn kill_tree(child: &mut Child) {
    #[cfg(unix)]
    {
        let pid = child.id() as i32;
        if pid > 0 {
            // The child leads its own process group, so helpers go with it.
            unsafe {
                libc::kill(-pid, libc::SIGKILL);
            }
        }
    }
    let _ = child.kill();
}

#[cfg(test)]
mod tests {
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
}
