//! Paths the file view is allowed to touch. A relative path stays under the
//! group directory, including through a symlink.

use std::path::{Component, Path, PathBuf};

/// A path the view already named, resolved against its group directory.
pub(crate) struct Located {
    pub root: PathBuf,
    /// The path to operate on. A final symlink is the link itself.
    pub path: PathBuf,
}

pub(crate) fn within(root: &Path, path: &Path) -> bool {
    path == root || path.starts_with(root)
}

fn user_path(root: &str) -> PathBuf {
    let home = crate::sessions::home_directory();
    if root == "~" {
        return home;
    }
    if let Some(rest) = root.strip_prefix("~/") {
        return home.join(rest);
    }
    PathBuf::from(root)
}

fn parts(relative: &str) -> Result<Vec<String>, String> {
    if relative.is_empty() {
        return Ok(Vec::new());
    }
    if relative.contains('\0') {
        return Err("The path is not valid.".into());
    }
    let path = Path::new(relative);
    if path.is_absolute() {
        return Err("The path leaves this folder.".into());
    }
    let mut names = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(name) => names.push(name.to_string_lossy().into_owned()),
            Component::CurDir => {}
            _ => return Err("The path leaves this folder.".into()),
        }
    }
    Ok(names)
}

/// Canonical group directory, then the relative path beneath it.
pub(crate) fn locate(root: &str, relative: &str) -> Result<Located, String> {
    let root = user_path(root)
        .canonicalize()
        .map_err(|_| "The folder is unavailable.".to_owned())?;
    if !root.is_dir() {
        return Err("The folder is unavailable.".into());
    }
    let mut current = root.clone();
    let names = parts(relative)?;
    if names.is_empty() {
        return Ok(Located {
            root,
            path: current,
        });
    }
    for (index, name) in names.iter().enumerate() {
        let next = current.join(name);
        let meta =
            std::fs::symlink_metadata(&next).map_err(|_| "The path is unavailable.".to_owned())?;
        let last = index + 1 == names.len();
        if meta.file_type().is_symlink() {
            if last {
                return Ok(Located { root, path: next });
            }
            let target = next
                .canonicalize()
                .map_err(|_| "The link is unavailable.".to_owned())?;
            if !within(&root, &target) {
                return Err("The link leaves this folder.".into());
            }
            current = target;
        } else if meta.is_dir() {
            current = next
                .canonicalize()
                .map_err(|_| "The folder is unavailable.".to_owned())?;
            if !within(&root, &current) {
                return Err("The path leaves this folder.".into());
            }
        } else if last {
            return Ok(Located { path: next, root });
        } else {
            return Err("The path is not a folder.".into());
        }
    }
    Ok(Located {
        root,
        path: current,
    })
}

/// Directory a new terminal should start in for this entry.
pub(crate) fn terminal_directory(root: &str, relative: &str) -> Result<String, String> {
    let located = locate(root, relative)?;
    let meta = std::fs::symlink_metadata(&located.path)
        .map_err(|_| "The path is unavailable.".to_owned())?;
    let directory = if meta.is_dir() {
        located.path.clone()
    } else if meta.file_type().is_symlink() {
        match located.path.canonicalize() {
            Ok(target) if within(&located.root, &target) && target.is_dir() => located.path,
            _ => parent_dir(&located),
        }
    } else {
        parent_dir(&located)
    };
    Ok(directory.to_string_lossy().into_owned())
}

fn parent_dir(located: &Located) -> PathBuf {
    located
        .path
        .parent()
        .filter(|parent| within(&located.root, parent))
        .unwrap_or(&located.root)
        .to_path_buf()
}

pub(crate) fn display_path(root: &str, relative: &str) -> Result<String, String> {
    let located = locate(root, relative)?;
    Ok(located.path.to_string_lossy().into_owned())
}
