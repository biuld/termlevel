//! Shared repository catalog. One bounded background worker owns discovery and watchers.
use nsh_core::prelude::*;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, Mutex, OnceLock, Weak,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
};
mod identity;
#[cfg(test)]
mod tests;
mod watcher;
mod worker;
pub(crate) use identity::resolve_repository;

const PAGE: usize = 256;
static NEXT: AtomicU32 = AtomicU32::new(1);
static SUBSCRIPTIONS: OnceLock<Mutex<HashMap<u32, Arc<Subscription>>>> = OnceLock::new();
static SCOPES: OnceLock<Mutex<HashMap<String, Arc<Scope>>>> = OnceLock::new();
fn subscriptions() -> &'static Mutex<HashMap<u32, Arc<Subscription>>> {
    SUBSCRIPTIONS.get_or_init(Default::default)
}
fn scopes() -> &'static Mutex<HashMap<String, Arc<Scope>>> {
    SCOPES.get_or_init(Default::default)
}

#[derive(Clone, Debug, PartialEq, JsObject)]
pub struct Repository {
    pub id: u32,
    pub root: String,
    pub git_dir: String,
    pub common_dir: String,
    pub label: String,
    /// main, linked or submodule; nested standalone repositories remain independent.
    pub kind: String,
}
#[derive(Clone, Default, JsObject)]
pub struct ContextRoot {
    pub path: String,
    pub root: String,
}
#[derive(Clone, Default, JsObject)]
pub struct Repositories {
    pub paths: Vec<String>,
    pub context_roots: Vec<ContextRoot>,
    pub repositories: Vec<Repository>,
    pub revision: u32,
    pub total: u32,
    pub reset: bool,
    pub scanning: bool,
    pub done: bool,
    pub preferred_root: Option<String>,
    pub error: Option<String>,
}
#[derive(Clone)]
struct Snapshot {
    entries: Arc<Vec<Repository>>,
    revision: u32,
    scanning: bool,
    preferred_root: Option<String>,
    error: Option<String>,
    context_roots: Vec<ContextRoot>,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            entries: Arc::new(Vec::new()),
            revision: 0,
            scanning: true,
            preferred_root: None,
            error: None,
            context_roots: Vec::new(),
        }
    }
}
struct Subscription {
    scope: Arc<Scope>,
    contexts: Mutex<Vec<String>>,
    snapshot: Mutex<Snapshot>,
}
impl Drop for Subscription {
    fn drop(&mut self) {
        crate::git::requests::release(std::mem::take(self.snapshot.get_mut().unwrap()));
    }
}
struct Plan {
    full: bool,
    queued: bool,
    cancel: Arc<AtomicBool>,
}
impl Default for Plan {
    fn default() -> Self {
        Self {
            full: false,
            queued: false,
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}
struct Scope {
    root: String,
    subscribers: Mutex<HashMap<u32, Weak<Subscription>>>,
    plan: Mutex<Plan>,
    running_full: AtomicBool,
}
impl Scope {
    fn subscribers(&self) -> Vec<Arc<Subscription>> {
        self.subscribers
            .lock()
            .unwrap()
            .values()
            .filter_map(Weak::upgrade)
            .collect()
    }
}

/// Discovers working trees separately from activating their Git data.
#[module]
pub struct GitDiscovery;
#[module]
impl GitDiscovery {
    /// Subscribes to a group root and known terminal/explicit directories. No filesystem IO on the engine.
    pub fn start(root: String, contexts: Vec<String>) -> u32 {
        let root = crate::git::expand_path(&root)
            .to_string_lossy()
            .into_owned();
        let scope = scopes()
            .lock()
            .unwrap()
            .entry(root.clone())
            .or_insert_with(|| {
                Arc::new(Scope {
                    root,
                    subscribers: Mutex::new(HashMap::new()),
                    plan: Mutex::new(Plan::default()),
                    running_full: AtomicBool::new(false),
                })
            })
            .clone();
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let subscription = Arc::new(Subscription {
            scope: scope.clone(),
            contexts: Mutex::new(contexts),
            snapshot: Mutex::new(Snapshot::default()),
        });
        scope
            .subscribers
            .lock()
            .unwrap()
            .insert(id, Arc::downgrade(&subscription));
        subscriptions().lock().unwrap().insert(id, subscription);
        worker::schedule(&scope, true);
        id
    }
    /// Reads one bounded page; a new revision restarts assembly instead of mixing snapshots.
    pub fn poll(id: u32, revision: u32, offset: u32) -> Repositories {
        let Some(subscription) = subscriptions().lock().unwrap().get(&id).cloned() else {
            return Repositories {
                done: true,
                error: Some("Repository catalog is unavailable.".into()),
                ..Default::default()
            };
        };
        let snapshot = subscription.snapshot.lock().unwrap();
        if revision == snapshot.revision && offset == 0 {
            return Repositories {
                revision,
                total: snapshot.entries.len() as u32,
                scanning: snapshot.scanning,
                done: !snapshot.scanning,
                ..Default::default()
            };
        }
        let reset = offset > 0 && revision != snapshot.revision;
        let offset = if reset { 0 } else { offset as usize };
        let entries: Vec<_> = snapshot
            .entries
            .iter()
            .skip(offset)
            .take(PAGE)
            .cloned()
            .collect();
        Repositories {
            context_roots: snapshot.context_roots.clone(),
            paths: entries.iter().map(|repo| repo.root.clone()).collect(),
            repositories: entries,
            revision: snapshot.revision,
            total: snapshot.entries.len() as u32,
            reset,
            scanning: snapshot.scanning,
            done: !snapshot.scanning,
            preferred_root: snapshot.preferred_root.clone(),
            error: snapshot.error.clone(),
        }
    }
    /// Updates contextual roots, coalescing rapid cwd changes and cancelling stale probes.
    pub fn update(id: u32, contexts: Vec<String>) {
        if let Some(subscription) = subscriptions().lock().unwrap().get(&id).cloned() {
            *subscription.contexts.lock().unwrap() = contexts;
            worker::schedule(&subscription.scope, false);
        }
    }
    /// Explicit recovery when a watcher reports an error or a filesystem does not support watching.
    pub fn refresh(id: u32) {
        if let Some(subscription) = subscriptions().lock().unwrap().get(&id).cloned() {
            worker::schedule(&subscription.scope, true);
        }
    }
    /// Releases this subscription. Other groups/views using the same root remain subscribed.
    pub fn cancel(id: u32) {
        if let Some(subscription) = subscriptions().lock().unwrap().remove(&id) {
            let scope = &subscription.scope;
            scope.subscribers.lock().unwrap().remove(&id);
            worker::schedule(scope, false);
        }
    }
}
