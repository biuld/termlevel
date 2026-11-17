use super::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::mpsc,
    time::Instant,
};
static WORKER: OnceLock<mpsc::Sender<Arc<Scope>>> = OnceLock::new();

pub(super) fn schedule(scope: &Arc<Scope>, full: bool) {
    let mut plan = scope.plan.lock().unwrap();
    plan.full |= full || scope.running_full.load(Ordering::Relaxed);
    plan.cancel.store(true, Ordering::Relaxed);
    if plan.queued {
        return;
    }
    plan.queued = true;
    let sender = WORKER.get_or_init(|| {
        let (sender, receiver) = mpsc::channel::<Arc<Scope>>();
        std::thread::Builder::new()
            .name("git-catalog".into())
            .spawn(move || {
                let mut states: HashMap<String, Source> = HashMap::new();
                for scope in receiver {
                    let (full, cancel) = {
                        let mut plan = scope.plan.lock().unwrap();
                        plan.queued = false;
                        plan.cancel = Arc::new(AtomicBool::new(false));
                        (std::mem::take(&mut plan.full), plan.cancel.clone())
                    };
                    scope.running_full.store(full, Ordering::Relaxed);
                    let source = states.entry(scope.root.clone()).or_default();
                    source.used = Instant::now();
                    if scope.subscribers().is_empty() {
                        source.watcher = None;
                    } else {
                        crate::git::requests::cancellable(cancel.clone(), || {
                            source.scan(&scope, full, &cancel)
                        });
                    }
                    scope.running_full.store(false, Ordering::Relaxed);
                    // Bound inactive caches. Dropping snapshots and watchers stays on this worker.
                    if states.len() > 8 {
                        let registry = scopes().lock().unwrap();
                        let mut inactive: Vec<_> = states
                            .iter()
                            .filter(|(root, _)| {
                                registry
                                    .get(*root)
                                    .is_none_or(|scope| scope.subscribers().is_empty())
                            })
                            .map(|(root, state)| (root.clone(), state.used))
                            .collect();
                        inactive.sort_by_key(|(_, used)| *used);
                        let remove: Vec<_> = inactive
                            .into_iter()
                            .take(states.len() - 8)
                            .map(|(root, _)| root)
                            .collect();
                        drop(registry);
                        for root in remove {
                            let removed = {
                                let mut registry = scopes().lock().unwrap();
                                if registry
                                    .get(&root)
                                    .is_some_and(|scope| scope.subscribers().is_empty())
                                {
                                    registry.remove(&root);
                                    true
                                } else {
                                    false
                                }
                            };
                            if removed {
                                states.remove(&root);
                            }
                        }
                    }
                }
            })
            .expect("Git catalog worker");
        sender
    });
    let _ = sender.send(scope.clone());
}

struct Source {
    base: BTreeMap<PathBuf, Repository>,
    contexts: BTreeMap<String, Repository>,
    attempted: BTreeSet<String>,
    root_repository: Option<String>,
    candidates: Vec<PathBuf>,
    revision: u32,
    error: Option<String>,
    watcher: Option<watcher::Watches>,
    used: Instant,
}
impl Default for Source {
    fn default() -> Self {
        Self {
            base: BTreeMap::new(),
            contexts: BTreeMap::new(),
            attempted: BTreeSet::new(),
            root_repository: None,
            candidates: Vec::new(),
            revision: 0,
            error: None,
            watcher: None,
            used: Instant::now(),
        }
    }
}
impl Source {
    fn inspect(&mut self, path: &std::path::Path, enclosing: bool) {
        match resolve_repository(&path.to_string_lossy()) {
            Ok(Some(repo))
                if enclosing || crate::git::canonical_path(path).to_string_lossy() == repo.root =>
            {
                self.base.insert(path.into(), repo);
            }
            Ok(_) => {
                self.base.remove(path);
            }
            Err(error) => {
                self.error.get_or_insert(error);
            }
        }
    }
    fn scan(&mut self, scope: &Arc<Scope>, full: bool, cancel: &AtomicBool) {
        self.error = None;
        self.publish(scope, true);
        let root = crate::git::expand_path(&scope.root);
        // Contextual repositories are cheap and have priority over the directory container.
        let subscriptions = scope.subscribers();
        let mut contexts = Vec::new();
        for sub in &subscriptions {
            if let Some(first) = sub.contexts.lock().unwrap().first() {
                contexts.push(first.clone());
            }
        }
        for sub in &subscriptions {
            contexts.extend(sub.contexts.lock().unwrap().clone());
        }
        let mut seen = BTreeSet::new();
        contexts.retain(|path| seen.insert(path.clone()));
        self.contexts.retain(|path, _| seen.contains(path));
        self.attempted.clear();
        for (index, context) in contexts.into_iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            let path = crate::git::expand_path(&context);
            let path = if path.is_absolute() {
                path
            } else {
                root.join(path)
            };
            self.attempted.insert(context.clone());
            match resolve_repository(&path.to_string_lossy()) {
                Ok(Some(repo)) => {
                    self.contexts.insert(context, repo);
                }
                Ok(None) => {
                    self.contexts.remove(&context);
                }
                Err(error) => {
                    self.error.get_or_insert(error);
                }
            }
            if index < 4 || index % 16 == 0 {
                self.publish(scope, true);
            }
        }
        self.publish(scope, true);
        if full || self.candidates.is_empty() {
            self.inspect(&root, true);
            self.root_repository = self.base.get(&root).map(|repo| repo.root.clone());
            self.publish(scope, true);
            let mut candidates = vec![root.clone()];
            match std::fs::read_dir(&root) {
                Ok(entries) => {
                    let mut children = Vec::new();
                    let mut complete = true;
                    for entry in entries {
                        match entry {
                            Ok(entry) if entry.file_name() != ".git" => match entry.file_type() {
                                Ok(kind) if kind.is_dir() => children.push(entry.path()),
                                Ok(_) => {}
                                Err(error) => {
                                    complete = false;
                                    self.error.get_or_insert(error.to_string());
                                }
                            },
                            Ok(_) => {}
                            Err(error) => {
                                complete = false;
                                self.error.get_or_insert(error.to_string());
                            }
                        }
                    }
                    children.sort();
                    candidates.extend(children);
                    if complete {
                        let known: BTreeSet<_> = candidates.iter().cloned().collect();
                        self.base.retain(|path, _| known.contains(path));
                    }
                    for (index, child) in candidates.iter().skip(1).enumerate() {
                        if cancel.load(Ordering::Relaxed) {
                            return;
                        }
                        self.inspect(child, false);
                        if index % 64 == 0 {
                            self.publish(scope, true);
                        }
                    }
                }
                Err(error) => {
                    if error.kind() == std::io::ErrorKind::NotFound {
                        self.base.clear();
                        self.root_repository = None;
                    }
                    self.error
                        .get_or_insert(format!("Cannot scan {}: {error}", root.display()));
                }
            }
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            self.candidates = candidates;
            match watcher::Watches::new(scope, &self.watch_paths(), self.watcher.as_ref()) {
                Ok(Some(watches)) => {
                    self.watcher = Some(watches);
                }
                Ok(None) => {}
                Err(error) => {
                    self.error.get_or_insert(error);
                }
            }
        }
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        if !full && !self.candidates.is_empty() {
            match watcher::Watches::new(scope, &self.watch_paths(), self.watcher.as_ref()) {
                Ok(Some(watches)) => self.watcher = Some(watches),
                Ok(None) => {}
                Err(error) => {
                    self.error.get_or_insert(error);
                }
            }
        }
        self.publish(scope, false);
    }
    fn watch_paths(&self) -> Vec<PathBuf> {
        let mut paths = self.candidates.clone();
        for repo in self.base.values().chain(self.contexts.values()) {
            let root = PathBuf::from(&repo.root);
            if let Some(parent) = root.parent() {
                paths.push(parent.to_path_buf());
            }
            paths.push(root);
        }
        paths
    }
    fn publish(&mut self, scope: &Scope, scanning: bool) {
        self.revision = self.revision.wrapping_add(1).max(1);
        for sub in scope.subscribers() {
            let contexts = sub.contexts.lock().unwrap().clone();
            let preferred = contexts
                .first()
                .and_then(|path| self.contexts.get(path))
                .map(|repo| repo.root.clone())
                .or_else(|| {
                    if contexts
                        .first()
                        .is_none_or(|path| self.attempted.contains(path))
                    {
                        self.root_repository.clone()
                    } else {
                        None
                    }
                });
            let mut entries: BTreeMap<_, _> = self
                .base
                .values()
                .map(|repo| (repo.root.clone(), repo.clone()))
                .collect();
            for repo in contexts.iter().filter_map(|path| self.contexts.get(path)) {
                entries.insert(repo.root.clone(), repo.clone());
            }
            let mut entries: Vec<_> = entries.into_values().collect();
            identity::disambiguate(&mut entries);
            entries.sort_by_key(|repo| repo.label.to_lowercase());
            let snapshot = Snapshot {
                entries: Arc::new(entries),
                revision: self.revision,
                scanning,
                preferred_root: preferred,
                error: self.error.clone(),
                context_roots: contexts
                    .iter()
                    .filter_map(|path| {
                        self.contexts.get(path).map(|repo| ContextRoot {
                            path: path.clone(),
                            root: repo.root.clone(),
                        })
                    })
                    .collect(),
            };
            let previous = std::mem::replace(&mut *sub.snapshot.lock().unwrap(), snapshot);
            crate::git::requests::release(previous);
        }
    }
}
