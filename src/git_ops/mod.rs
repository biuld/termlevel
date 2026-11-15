//! Explicit Git operations for the history panel.
//!
//! Reads stay in `git`; this module owns explicit branch, remote and worktree
//! operations. Pull is fast-forward only; push targets the current branch upstream.

use nsh_core::prelude::*;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use crate::git;

const OP_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_OUTPUT: usize = 512 * 1024;

/// Explicit branch and worktree operations from the system Git CLI.
#[module]
pub struct GitOps;

#[module]
impl GitOps {
    /// Fetches from the default remote configured for the current branch.
    pub async fn fetch(root: String) -> Result<String, String> {
        match blocking(move || fetch_sync(&root)).await {
            Ok(result) => result,
            Err(_) => Err("The Git fetch task failed.".into()),
        }
    }

    /// Fetches and fast-forwards the current branch from its upstream.
    pub async fn pull(root: String) -> Result<String, String> {
        match blocking(move || pull_sync(&root)).await {
            Ok(result) => result,
            Err(_) => Err("The Git pull task failed.".into()),
        }
    }
    /// Pushes the current branch to its configured upstream without forcing.
    pub async fn push(root: String) -> Result<String, String> {
        match blocking(move || push_sync(&root)).await {
            Ok(result) => result,
            Err(_) => Err("The Git push task failed.".into()),
        }
    }

    /// Switches the worktree that contains `root` to a local branch, or creates
    /// a tracking branch for a remote one.
    pub async fn checkout(root: String, branch: String) -> Result<String, String> {
        match blocking(move || checkout_sync(&root, &branch)).await {
            Ok(result) => result,
            Err(_) => Err("The Git checkout task failed.".into()),
        }
    }

    /// Creates a branch from `start` (a commit, ref or `HEAD`) and optionally
    /// checks it out.
    pub async fn create_branch(
        root: String,
        name: String,
        start: String,
        checkout: bool,
    ) -> Result<String, String> {
        match blocking(move || create_branch_sync(&root, &name, &start, checkout)).await {
            Ok(result) => result,
            Err(_) => Err("The Git branch task failed.".into()),
        }
    }

    /// Deletes a local branch that Git considers merged. Git refuses a branch
    /// checked out in a worktree or not merged into its upstream/HEAD.
    pub async fn delete_branch(root: String, name: String) -> Result<String, String> {
        match blocking(move || delete_branch_sync(&root, &name)).await {
            Ok(result) => result,
            Err(_) => Err("The Git branch task failed.".into()),
        }
    }

    /// Adds a worktree at `path` for an existing local branch.
    pub async fn add_worktree(
        root: String,
        path: String,
        branch: String,
    ) -> Result<String, String> {
        match blocking(move || add_worktree_sync(&root, &path, &branch)).await {
            Ok(result) => result,
            Err(_) => Err("The Git worktree task failed.".into()),
        }
    }

    /// Removes a linked worktree without force. Dirty worktrees are refused by
    /// Git and keep their directory.
    pub async fn remove_worktree(root: String, path: String) -> Result<String, String> {
        match blocking(move || remove_worktree_sync(&root, &path)).await {
            Ok(result) => result,
            Err(_) => Err("The Git worktree task failed.".into()),
        }
    }

    /// Prevents Git from pruning or removing a worktree.
    pub async fn lock_worktree(root: String, path: String) -> Result<String, String> {
        match blocking(move || flag_worktree_sync(&root, &path, "lock")).await {
            Ok(result) => result,
            Err(_) => Err("The Git worktree task failed.".into()),
        }
    }

    /// Allows a worktree to be pruned or removed again.
    pub async fn unlock_worktree(root: String, path: String) -> Result<String, String> {
        match blocking(move || flag_worktree_sync(&root, &path, "unlock")).await {
            Ok(result) => result,
            Err(_) => Err("The Git worktree task failed.".into()),
        }
    }

    /// Removes worktree administrative records whose folders are gone.
    pub async fn prune_worktrees(root: String) -> Result<String, String> {
        match blocking(move || prune_worktrees_sync(&root)).await {
            Ok(result) => result,
            Err(_) => Err("The Git worktree task failed.".into()),
        }
    }
}

fn open(root: &str) -> Result<(PathBuf, PathBuf), String> {
    let git = git::git_path()?;
    let root = git::resolve_root(root)?;
    let repo_root = git::repository_root(&git, &root)?;
    Ok((git, repo_root))
}

fn run(git: &Path, root: &Path, args: &[&str]) -> Result<(), String> {
    let output = git::run_capture_mutating(git, Some(root), args, OP_TIMEOUT, MAX_OUTPUT)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(git::stderr_message(&output))
    }
}

fn read(git: &Path, root: &Path, args: &[&str]) -> Result<git::Captured, String> {
    git::run_capture(git, Some(root), args, OP_TIMEOUT, MAX_OUTPUT)
}

fn fetch_sync(root: &str) -> Result<String, String> {
    let (git, repo_root) = open(root)?;
    run(&git, &repo_root, &["fetch"])?;
    Ok("Fetch complete.".into())
}

fn pull_sync(root: &str) -> Result<String, String> {
    let (git, repo_root) = open(root)?;
    run(&git, &repo_root, &["pull", "--ff-only"])?;
    Ok("Pull complete (fast-forward only).".into())
}

fn push_sync(root: &str) -> Result<String, String> {
    let (git, repo_root) = open(root)?;
    let current = read(
        &git,
        &repo_root,
        &["symbolic-ref", "--quiet", "--short", "HEAD"],
    )?;
    if !current.status.success() {
        return Err("Cannot push a detached HEAD. Check out a branch first.".into());
    }
    let branch = String::from_utf8(current.stdout).map_err(|_| "Invalid branch name.")?;
    let branch = branch.trim();
    let config = |field: &str| -> Result<String, String> {
        let key = format!("branch.{branch}.{field}");
        let output = read(&git, &repo_root, &["config", "--get", &key])?;
        if !output.status.success() {
            return Err(format!(
                "Branch {branch} has no upstream. Configure one before pushing."
            ));
        }
        String::from_utf8(output.stdout)
            .map(|value| value.trim().to_owned())
            .map_err(|_| "Invalid upstream configuration.".into())
    };
    let remote = config("remote")?;
    let destination = config("merge")?;
    if remote.is_empty()
        || remote == "."
        || !destination.starts_with("refs/heads/")
        || destination.contains('\n')
    {
        return Err("The current branch does not have a remote branch as its upstream.".into());
    }
    let refspec = format!("HEAD:{destination}");
    run(
        &git,
        &repo_root,
        &[
            "push",
            "--no-force",
            "--no-follow-tags",
            "--recurse-submodules=no",
            "--",
            &remote,
            &refspec,
        ],
    )?;
    Ok("Push complete.".into())
}

fn validate_branch(git: &Path, root: &Path, name: &str) -> Result<(), String> {
    if name.is_empty() || name.starts_with('-') || name.chars().any(char::is_whitespace) {
        return Err("Enter a valid branch name.".into());
    }
    let output = read(git, root, &["check-ref-format", "--branch", name])?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!("`{name}` is not a valid branch name."))
    }
}

fn resolve_start(git: &Path, root: &Path, start: &str) -> Result<String, String> {
    let start = start.trim();
    let start = if start.is_empty() { "HEAD" } else { start };
    if start.starts_with('-') || start.chars().any(char::is_whitespace) {
        return Err("Choose a valid starting revision.".into());
    }
    let spec = format!("{start}^{{commit}}");
    let output = read(git, root, &["rev-parse", "--verify", &spec])?;
    if !output.status.success() {
        return Err(format!("`{start}` is not a commit."));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn checkout_sync(root: &str, branch: &str) -> Result<String, String> {
    let (git, repo_root) = open(root)?;
    validate_branch(&git, &repo_root, branch)?;
    let local = format!("refs/heads/{branch}");
    if git::ref_exists(&git, &repo_root, &local)? {
        run(&git, &repo_root, &["checkout", branch])?;
        return Ok(format!("Checked out {branch}."));
    }
    let remote = format!("refs/remotes/{branch}");
    if !git::ref_exists(&git, &repo_root, &remote)? {
        return Err(format!("Branch {branch} was not found."));
    }
    let local_name = branch.split_once('/').map(|(_, name)| name).unwrap_or("");
    if local_name.is_empty() {
        return Err("That remote branch has no local name.".into());
    }
    let local_ref = format!("refs/heads/{local_name}");
    if git::ref_exists(&git, &repo_root, &local_ref)? {
        run(&git, &repo_root, &["checkout", local_name])?;
        Ok(format!("Checked out {local_name}."))
    } else {
        run(&git, &repo_root, &["checkout", "--track", branch])?;
        Ok(format!("Checked out {local_name} from {branch}."))
    }
}

fn create_branch_sync(
    root: &str,
    name: &str,
    start: &str,
    checkout: bool,
) -> Result<String, String> {
    let (git, repo_root) = open(root)?;
    let name = name.trim();
    validate_branch(&git, &repo_root, name)?;
    let start = resolve_start(&git, &repo_root, start)?;
    if checkout {
        run(&git, &repo_root, &["checkout", "-b", name, &start])?;
        Ok(format!("Created and checked out {name}."))
    } else {
        run(&git, &repo_root, &["branch", name, &start])?;
        Ok(format!("Created {name}."))
    }
}

fn delete_branch_sync(root: &str, name: &str) -> Result<String, String> {
    let (git, repo_root) = open(root)?;
    let name = name.trim();
    validate_branch(&git, &repo_root, name)?;
    let branch_ref = format!("refs/heads/{name}");
    if !git::ref_exists(&git, &repo_root, &branch_ref)? {
        return Err(format!("{name} is not a local branch."));
    }
    let current = read(&git, &repo_root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let current = String::from_utf8_lossy(&current.stdout).trim().to_string();
    if current == name {
        return Err("The current branch cannot be deleted.".into());
    }
    run(&git, &repo_root, &["branch", "-d", name])?;
    Ok(format!("Deleted {name}."))
}

fn worktree_path(repo_root: &Path, path: &str) -> Result<PathBuf, String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("Enter a folder for the worktree.".into());
    }
    let expanded = git::expand_path(path);
    let absolute = if expanded.is_absolute() {
        expanded
    } else {
        repo_root.join(expanded)
    };
    if !absolute.is_absolute() {
        return Err("The worktree folder must be an absolute path.".into());
    }
    Ok(absolute)
}

fn add_worktree_sync(root: &str, path: &str, branch: &str) -> Result<String, String> {
    let (git, repo_root) = open(root)?;
    let branch = branch.trim();
    validate_branch(&git, &repo_root, branch)?;
    let branch_ref = format!("refs/heads/{branch}");
    if !git::ref_exists(&git, &repo_root, &branch_ref)? {
        return Err(format!("{branch} is not a local branch."));
    }
    let path = worktree_path(&repo_root, path)?;
    if path.exists() {
        return Err(format!("{} already exists.", path.display()));
    }
    let path_text = path.to_string_lossy().into_owned();
    run(&git, &repo_root, &["worktree", "add", &path_text, branch])?;
    Ok(path_text)
}

fn find_worktree(root: &str, path: &str) -> Result<(PathBuf, PathBuf, git::RepoWorktree), String> {
    let (git, repo_root) = open(root)?;
    let target = git::canonical_path(&worktree_path(&repo_root, path)?);
    let worktrees = git::list_worktrees(root)?;
    let worktree = worktrees
        .into_iter()
        .find(|worktree| git::canonical_path(Path::new(&worktree.path)) == target)
        .ok_or_else(|| "That folder is not a worktree of this repository.".to_string())?;
    Ok((git, repo_root, worktree))
}

fn remove_worktree_sync(root: &str, path: &str) -> Result<String, String> {
    let (git, repo_root, worktree) = find_worktree(root, path)?;
    if worktree.main {
        return Err("The main worktree cannot be removed.".into());
    }
    if worktree.locked.is_some() {
        return Err("Unlock the worktree before removing it.".into());
    }
    run(&git, &repo_root, &["worktree", "remove", &worktree.path])?;
    Ok(format!("Removed {}.", worktree.path))
}

fn flag_worktree_sync(root: &str, path: &str, action: &str) -> Result<String, String> {
    let (git, repo_root, worktree) = find_worktree(root, path)?;
    if action == "lock" && worktree.main {
        return Err("The main worktree cannot be locked.".into());
    }
    run(&git, &repo_root, &["worktree", action, &worktree.path])?;
    Ok(format!(
        "{} {}.",
        if action == "lock" {
            "Locked"
        } else {
            "Unlocked"
        },
        worktree.path
    ))
}

fn prune_worktrees_sync(root: &str) -> Result<String, String> {
    let (git, repo_root) = open(root)?;
    run(&git, &repo_root, &["worktree", "prune"])?;
    Ok("Pruned stale worktrees.".into())
}

#[cfg(test)]
mod tests;
