//! Explicit Git operations for the history panel.
//!
//! Reads stay in `git`; this module only switches branches, creates branches
//! and manages worktrees. Merges, rebases and conflict resolution are
//! deliberately absent.

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
mod tests {
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
}
