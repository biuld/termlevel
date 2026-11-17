use super::*;
use std::{collections::BTreeMap, path::Path, time::Duration};
static IDENTITIES: OnceLock<Mutex<HashMap<(PathBuf, PathBuf), u32>>> = OnceLock::new();

/// Called only from a blocking worker, including explicit Git mutations.
pub(crate) fn resolve_repository(directory: &str) -> Result<Option<Repository>, String> {
    crate::git::requests::check()?;
    let directory = crate::git::expand_path(directory);
    match std::fs::metadata(&directory) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => return Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Cannot inspect {}: {error}", directory.display())),
    }
    let git = crate::git::git_path()?;
    let output = crate::git::run_capture(
        &git,
        Some(&directory),
        &[
            "rev-parse",
            "--show-toplevel",
            "--absolute-git-dir",
            "--git-common-dir",
            "--show-superproject-working-tree",
        ],
        Duration::from_secs(3),
        32 * 1024,
    )?;
    if !output.status.success() {
        let error = crate::git::stderr_message(&output);
        if error.contains("not a git repository") || error.contains("must be run in a work tree") {
            return Ok(None);
        }
        return Err(format!("Cannot inspect {}: {error}", directory.display()));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<_> = text.lines().collect();
    if output.truncated || !(3..=4).contains(&lines.len()) {
        return Err("Git returned an unsupported repository path.".into());
    }
    let root = crate::git::canonical_path(Path::new(lines[0]));
    let git_dir = crate::git::canonical_path(Path::new(lines[1]));
    let common = PathBuf::from(lines[2]);
    let common = crate::git::canonical_path(&if common.is_absolute() {
        common
    } else {
        directory.join(common)
    });
    let id = *IDENTITIES
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .entry((root.clone(), git_dir.clone()))
        .or_insert_with(|| NEXT.fetch_add(1, Ordering::Relaxed));
    let kind = if git_dir != common {
        "linked"
    } else if lines.get(3).is_some_and(|line| !line.is_empty()) {
        "submodule"
    } else {
        "main"
    };
    Ok(Some(Repository {
        id,
        label: root
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        root: root.to_string_lossy().into_owned(),
        git_dir: git_dir.to_string_lossy().into_owned(),
        common_dir: common.to_string_lossy().into_owned(),
        kind: kind.into(),
    }))
}

pub(super) fn disambiguate(repositories: &mut [Repository]) {
    let parts: Vec<Vec<_>> = repositories
        .iter()
        .map(|repo| {
            Path::new(&repo.root)
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .rev()
                .collect()
        })
        .collect();
    let mut depths = vec![1; repositories.len()];
    loop {
        let labels: Vec<String> = parts
            .iter()
            .zip(&depths)
            .map(|(parts, count)| {
                parts
                    .iter()
                    .take(*count)
                    .rev()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .collect();
        let mut collisions = BTreeMap::<&str, Vec<usize>>::new();
        for (index, label) in labels.iter().enumerate() {
            collisions.entry(label).or_default().push(index);
        }
        let mut changed = false;
        for indexes in collisions.values().filter(|indexes| indexes.len() > 1) {
            for index in indexes {
                if depths[*index] < parts[*index].len() {
                    depths[*index] += 1;
                    changed = true;
                }
            }
        }
        if !changed {
            for (repo, label) in repositories.iter_mut().zip(labels) {
                repo.label = label;
            }
            break;
        }
    }
}
