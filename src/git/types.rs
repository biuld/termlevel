use super::*;

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
    pub(super) fn empty() -> Self {
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

    pub(super) fn failed(message: String) -> Self {
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
    pub commit_target: bool,
}

/// Branches and tags, with why Git could not list them.
#[derive(Clone, Debug, Default, PartialEq, JsObject)]
pub struct RepoBranches {
    pub revision: String,
    pub unchanged: bool,
    pub total: u32,
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
    pub ref_count: u32,
    pub subject: String,
}

impl RepoCommit {
    pub(super) fn empty(oid: String) -> Self {
        Self {
            short: oid.chars().take(7).collect(),
            oid,
            parents: Vec::new(),
            author: String::new(),
            email: String::new(),
            timestamp: 0,
            refs: Vec::new(),
            ref_count: 0,
            subject: String::new(),
        }
    }
}

/// A page of commits.
#[derive(Clone, Debug, Default, PartialEq, JsObject)]
pub struct RepoLog {
    pub graph: Vec<GraphRow>,
    pub graph_max_lanes: u32,
    pub graph_clipped: bool,
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

impl RepoBranches {
    pub(super) fn failed(error: String) -> Self {
        Self {
            error: Some(error),
            ..Default::default()
        }
    }
}
