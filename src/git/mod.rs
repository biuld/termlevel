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

mod cli;
mod decorations;
mod diff;
mod graph;
mod history;
mod refs;
#[cfg(test)]
mod refs_tests;
pub(crate) mod requests;
mod status;
#[cfg(test)]
mod tests;
mod types;

#[cfg(test)]
use cli::parse_version;
use cli::run_capture_input;
pub(crate) use cli::{Captured, git_path, run_capture, run_capture_mutating};
use diff::*;
pub(crate) use diff::{expand_path, resolve_root};
pub use graph::GraphRow;
use history::*;
use refs::*;
pub(crate) use refs::{canonical_path, list_worktrees, ref_exists, repository_root};
pub(crate) use status::stderr_message;
use status::*;
pub use types::*;

/// Read-only background queries from the system Git CLI.
#[module]
pub struct Git;

#[module]
impl Git {
    /// Branch, upstream, ahead/behind, operation and changed paths for a group.
    pub async fn status(root: String, owner: u32) -> RepoStatus {
        match requests::run(owner, "status", move |_| status_sync(&root)).await {
            Ok(status) => status,
            Err(_) => RepoStatus::failed("The Git status task failed.".into()),
        }
    }

    /// A unified patch for one changed path. `mode` is staged, unstaged, head, or untracked.
    pub async fn diff(
        root: String,
        path: String,
        mode: String,
        owner: u32,
    ) -> Result<String, String> {
        match requests::run(owner, "diff", move |_| diff_sync(&root, &path, &mode)).await {
            Ok(result) => result,
            Err(_) => Err("The Git diff task failed.".into()),
        }
    }

    /// Local branches, remote-tracking branches and tags.
    pub async fn branches(root: String, revision: String, owner: u32) -> RepoBranches {
        match requests::run(owner, "branches", move |owner| {
            branch_snapshot(owner, &root, &revision)
        })
        .await
        {
            Ok(result) => result,
            Err(_) => RepoBranches {
                branches: Vec::new(),
                error: Some("The Git branch task failed.".into()),
                ..Default::default()
            },
        }
    }

    /// Another bounded page from the branch snapshot read by this view.
    pub async fn branch_page(revision: String, offset: u32, owner: u32) -> RepoBranches {
        requests::run(owner, "branch-page", move |owner| {
            branch_page(owner, &revision, offset)
        })
        .await
        .unwrap_or_else(RepoBranches::failed)
    }

    /// Worktrees of the repository that contains `root`.
    pub async fn worktrees(root: String, owner: u32) -> RepoWorktrees {
        match requests::run(owner, "worktrees", move |_| worktrees_sync(&root)).await {
            Ok(result) => result,
            Err(_) => RepoWorktrees {
                worktrees: Vec::new(),
                error: Some("The Git worktree task failed.".into()),
            },
        }
    }

    /// A page of commits. `scope` is a full ref, or empty for every ref.
    pub async fn log(root: String, scope: String, limit: u32, skip: u32, owner: u32) -> RepoLog {
        match requests::run(owner, "log", move |owner| {
            let mut graph = owner.graph.lock().unwrap();
            if skip == 0 {
                graph.pin(history_tips(owner, &root, &scope));
            }
            let mut log = log_sync_pinned(&root, &scope, limit, skip, graph.tips());
            for commit in &mut log.commits {
                decorations::apply(owner, commit);
            }
            if log.error.is_none()
                && let Err(error) = graph.apply(&root, &scope, skip, &mut log)
            {
                log.error = Some(error);
                log.commits.clear();
                log.has_more = false;
            }
            log
        })
        .await
        {
            Ok(result) => result,
            Err(_) => RepoLog {
                commits: Vec::new(),
                has_more: false,
                truncated: false,
                error: Some("The Git log task failed.".into()),
                ..Default::default()
            },
        }
    }

    /// Checks a retained history cursor before a remounted view appends another page.
    pub async fn resume_history(root: String, scope: String, count: u32, owner: u32) -> bool {
        requests::run(owner, "resume", move |owner| {
            owner.graph.lock().unwrap().matches(&root, &scope, count)
        })
        .await
        .unwrap_or(false)
    }

    /// Loaded commits whose complete ref names match a search query.
    pub async fn match_refs(query: String, owner: u32) -> Result<Vec<String>, String> {
        requests::run(owner, "search", move |owner| match_refs(owner, &query)).await?
    }

    /// One commit's metadata, changed paths and patch against its first parent.
    pub async fn commit(root: String, oid: String, owner: u32) -> RepoCommitDetail {
        let fallback = oid.clone();
        match requests::run(owner, "commit", move |owner| {
            let mut detail = commit_sync(&root, &oid);
            decorations::apply(owner, &mut detail.commit);
            detail
        })
        .await
        {
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
