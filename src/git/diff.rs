use super::*;

pub(super) fn diff_sync(root: &str, path: &str, mode: &str) -> Result<String, String> {
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
        "staged" | "unstaged" | "head" => {
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
            // Working tree against HEAD. The added side matches the open file,
            // including changes that are already staged.
            if mode == "head" {
                args.push("HEAD");
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

pub(super) fn untracked_patch(repo_root: &Path, path: &str) -> Result<String, String> {
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

pub(super) fn safe_relative(path: &str) -> Result<String, String> {
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
