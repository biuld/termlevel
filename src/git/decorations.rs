//! A bounded preview of decorations; the complete ref list is queried separately.
use super::*;
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};

const PREVIEW: usize = 8;
#[derive(Default)]
pub(super) struct Decoration {
    names: Vec<String>,
    total: u32,
}
fn add(index: &mut HashMap<String, Decoration>, oid: &str, name: String) {
    if oid.is_empty() {
        return;
    }
    let entry = index.entry(oid.into()).or_default();
    entry.total += 1;
    if entry.names.len() < PREVIEW {
        entry.names.push(name);
    }
}

pub(super) fn index(
    root: &str,
    refs: &mut RepoBranches,
) -> Result<(String, HashMap<String, Decoration>), String> {
    let (git, root) = open_repository(root)?;
    let head = run_capture(
        &git,
        Some(&root),
        &["rev-parse", "--verify", "HEAD"],
        STATUS_TIMEOUT,
        8192,
    )?;
    let head = if head.status.success() {
        String::from_utf8_lossy(&head.stdout).trim().to_string()
    } else {
        String::new()
    };
    let current = refs.branches.iter().find(|branch| branch.current);
    let head_label = current.map_or_else(
        || "HEAD".to_string(),
        |branch| format!("HEAD -> {}", branch.name),
    );
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    (&refs.revision, &head, &head_label).hash(&mut hash);
    refs.revision = format!("{:016x}", hash.finish());
    let mut index = HashMap::new();
    add(&mut index, &head, head_label);
    for branch in &refs.branches {
        requests::check()?;
        if branch.current || !branch.commit_target {
            continue;
        }
        let label = if branch.kind == "tag" {
            format!("tag: {}", branch.name)
        } else {
            branch.name.clone()
        };
        add(&mut index, &branch.target, label);
    }
    Ok((head, index))
}

pub(super) fn apply(owner: &requests::Owner, commit: &mut RepoCommit) {
    let snapshot = owner.branches.lock().unwrap();
    if let Some(decoration) = snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.decorations.get(&commit.oid))
    {
        commit.refs.clone_from(&decoration.names);
        commit.ref_count = decoration.total;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn many_refs_on_one_commit_keep_metadata_bounded() {
        let mut index = HashMap::new();
        for i in 0..20_000 {
            add(&mut index, "head", format!("origin/long-branch-{i}"));
        }
        assert_eq!(index["head"].total, 20_000);
        assert_eq!(index["head"].names.len(), PREVIEW);
    }
}
