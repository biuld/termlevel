use super::*;
use notify::{
    EventKind, RecommendedWatcher, RecursiveMode, Watcher,
    event::{CreateKind, ModifyKind, RemoveKind},
};
use std::collections::HashSet;

pub(super) struct Watches {
    _watcher: RecommendedWatcher,
    paths: Vec<PathBuf>,
}
impl Watches {
    pub fn new(
        scope: &Arc<Scope>,
        candidates: &[PathBuf],
        previous: Option<&Self>,
    ) -> Result<Option<Self>, String> {
        let root = crate::git::expand_path(&scope.root);
        let mut paths = candidates.to_vec();
        if let Some(parent) = root.parent() {
            paths.push(parent.to_path_buf());
        }
        for sub in scope.subscribers() {
            for context in sub.contexts.lock().unwrap().iter() {
                let path = crate::git::expand_path(context);
                paths.push(if path.is_absolute() {
                    path
                } else {
                    root.join(path)
                });
            }
        }
        paths = paths
            .into_iter()
            .map(|path| crate::git::canonical_path(&path))
            .collect();
        paths.sort();
        paths.dedup();
        paths.retain(|path| path.is_dir());
        if paths.len() > 1024 {
            return Err(
                "Too many directories to watch. Use Rescan to discover newly created repositories."
                    .into(),
            );
        }
        if previous.is_some_and(|watcher| watcher.paths == paths) {
            return Ok(None);
        }
        let known: HashSet<_> = paths.iter().cloned().collect();
        let weak = Arc::downgrade(scope);
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                let structural = event.as_ref().is_ok_and(|event| match event.kind {
                    EventKind::Create(CreateKind::Folder)
                    | EventKind::Remove(RemoveKind::Folder) => true,
                    EventKind::Modify(ModifyKind::Data(_)) => event
                        .paths
                        .iter()
                        .any(|path| known.contains(&crate::git::canonical_path(path))),
                    EventKind::Modify(ModifyKind::Name(_)) => event.paths.iter().any(|path| {
                        known.contains(&crate::git::canonical_path(path))
                            || path.file_name().is_some_and(|name| name == ".git")
                    }),
                    EventKind::Create(_) | EventKind::Remove(_) => event
                        .paths
                        .iter()
                        .any(|path| path.file_name().is_some_and(|name| name == ".git")),
                    _ => event.need_rescan(),
                });
                if (structural || event.is_err())
                    && let Some(scope) = weak.upgrade()
                {
                    worker::schedule(&scope, true);
                }
            })
            .map_err(|error| format!("Repository watching is unavailable: {error}. Use Rescan."))?;
        for path in &paths {
            if path.is_dir() {
                watcher
                    .watch(path, RecursiveMode::NonRecursive)
                    .map_err(|error| {
                        format!("Cannot watch {}: {error}. Use Rescan.", path.display())
                    })?;
            }
        }
        Ok(Some(Self {
            _watcher: watcher,
            paths,
        }))
    }
}
