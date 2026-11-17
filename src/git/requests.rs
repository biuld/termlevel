//! View-owned reads. No runtime of our own: NSH schedules the future and blocking work.
use super::*;
use std::{
    cell::RefCell,
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    task::{Context, Poll, Waker},
};

const READERS: usize = 4;
static NEXT: AtomicU32 = AtomicU32::new(1);
static OWNERS: OnceLock<Mutex<HashMap<u32, Arc<Owner>>>> = OnceLock::new();
static POOL: OnceLock<Mutex<Pool>> = OnceLock::new();
thread_local! { static EXTERNAL: RefCell<Option<Arc<AtomicBool>>> = const { RefCell::new(None) }; }
thread_local! { static CURRENT: RefCell<Option<Arc<Token>>> = const { RefCell::new(None) }; }

#[derive(Default)]
struct Pool {
    active: usize,
    waiting: HashMap<u32, Waker>,
}
fn pool() -> &'static Mutex<Pool> {
    POOL.get_or_init(Default::default)
}
fn owners() -> &'static Mutex<HashMap<u32, Arc<Owner>>> {
    OWNERS.get_or_init(Default::default)
}

#[derive(Default)]
pub(super) struct Owner {
    jobs: Mutex<HashMap<&'static str, Arc<Token>>>,
    pub graph: Mutex<super::graph::HistoryGraph>,
    pub branches: Mutex<Option<super::refs::BranchSnapshot>>,
}
// The final Arc may be released by an engine future. Move large caches off-thread
// even in that case; closing a view must not free tens of thousands of refs there.
type Retired = Box<dyn Send>;
static RETIRED: OnceLock<std::sync::mpsc::Sender<Retired>> = OnceLock::new();
pub(crate) fn release(value: impl Send + 'static) {
    let sender = RETIRED.get_or_init(|| {
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("git-cache-release".into())
            .spawn(move || {
                for retired in receiver {
                    drop(retired);
                }
            })
            .expect("Git cache cleanup thread");
        sender
    });
    let _ = sender.send(Box::new(value));
}
impl Drop for Owner {
    fn drop(&mut self) {
        let graph = std::mem::take(
            self.graph
                .get_mut()
                .unwrap_or_else(|error| error.into_inner()),
        );
        let branches = self
            .branches
            .get_mut()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        if !graph.loaded.is_empty() || branches.is_some() {
            release((graph, branches));
        }
    }
}

struct Token {
    id: u32,
    cancelled: AtomicBool,
    waiter: Mutex<Option<Waker>>,
}
impl Token {
    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
        if let Some(waker) = self.waiter.lock().unwrap().take() {
            waker.wake();
        }
    }
    fn check(&self) -> Result<(), String> {
        if self.cancelled.load(Ordering::Relaxed) {
            Err("Git query cancelled.".into())
        } else {
            Ok(())
        }
    }
}

/// Read leases are independent of session/process lifetime and Git mutations.
#[module]
pub struct GitQueries;
#[module]
impl GitQueries {
    /// Creates an independent view-owned read lease.
    pub fn open() -> u32 {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        owners()
            .lock()
            .unwrap()
            .insert(id, Arc::new(Owner::default()));
        id
    }
    /// Cancels all reads owned by a view; the lease can be reused for another repository.
    pub fn cancel(id: u32) {
        if let Some(owner) = owners().lock().unwrap().get(&id) {
            cancel_owner(owner);
        }
    }
    /// Cancels one superseded selection without interrupting the other reads.
    pub fn cancel_read(id: u32, channel: String) {
        if let Some(owner) = owners().lock().unwrap().get(&id)
            && let Some(token) = owner.jobs.lock().unwrap().remove(channel.as_str())
        {
            token.cancel();
        }
    }
    /// Cancels reads and releases cached history/refs on unmount.
    pub fn close(id: u32) {
        if let Some(owner) = owners().lock().unwrap().remove(&id) {
            cancel_owner(&owner);
        }
    }
}
fn cancel_owner(owner: &Owner) {
    for token in owner.jobs.lock().unwrap().drain().map(|(_, token)| token) {
        token.cancel();
    }
}

struct Admission(Arc<Token>);
struct Permit;
impl Future for Admission {
    type Output = Result<Permit, String>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut pool = pool().lock().unwrap();
        // Register before checking cancellation, so a concurrent cancel cannot miss us.
        *self.0.waiter.lock().unwrap() = Some(cx.waker().clone());
        if let Err(error) = self.0.check() {
            pool.waiting.remove(&self.0.id);
            return Poll::Ready(Err(error));
        }
        if pool.active < READERS {
            pool.waiting.remove(&self.0.id);
            pool.active += 1;
            Poll::Ready(Ok(Permit))
        } else {
            pool.waiting.insert(self.0.id, cx.waker().clone());
            Poll::Pending
        }
    }
}
impl Drop for Admission {
    fn drop(&mut self) {
        pool().lock().unwrap().waiting.remove(&self.0.id);
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        let waiting = {
            let mut pool = pool().lock().unwrap();
            pool.active -= 1;
            std::mem::take(&mut pool.waiting)
        };
        for (_, waker) in waiting {
            waker.wake();
        }
    }
}
struct Job {
    owner: Arc<Owner>,
    channel: &'static str,
    token: Arc<Token>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.token.cancel();
        let mut jobs = self.owner.jobs.lock().unwrap();
        if jobs
            .get(self.channel)
            .is_some_and(|next| Arc::ptr_eq(next, &self.token))
        {
            jobs.remove(self.channel);
        }
    }
}
struct Current(Option<Arc<Token>>);
impl Drop for Current {
    fn drop(&mut self) {
        CURRENT.with(|current| current.replace(self.0.take()));
    }
}

pub(super) async fn run<R: Send + 'static>(
    id: u32,
    channel: &'static str,
    f: impl FnOnce(&Owner) -> R + Send + 'static,
) -> Result<R, String> {
    let owner = owners()
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or("Git view closed.")?;
    let token = Arc::new(Token {
        id: NEXT.fetch_add(1, Ordering::Relaxed),
        cancelled: AtomicBool::new(false),
        waiter: Mutex::new(None),
    });
    if let Some(previous) = owner.jobs.lock().unwrap().insert(channel, token.clone()) {
        previous.cancel();
    }
    let _job = Job {
        owner: owner.clone(),
        channel,
        token: token.clone(),
    };
    // Admission yields on the engine thread; queued reads consume no blocking worker.
    let permit = Admission(token.clone()).await?;
    blocking(move || {
        let _permit = permit;
        let _current = Current(CURRENT.with(|current| current.replace(Some(token.clone()))));
        token.check()?;
        let result = f(&owner);
        token.check()?;
        Ok(result)
    })
    .await
    .map_err(|_| "Git read task failed.".to_string())?
}

pub(crate) fn check() -> Result<(), String> {
    if EXTERNAL.with(|current| {
        current
            .borrow()
            .as_ref()
            .is_some_and(|cancel| cancel.load(Ordering::Relaxed))
    }) {
        return Err("Git query cancelled.".into());
    }
    CURRENT.with(|current| {
        current
            .borrow()
            .as_ref()
            .map_or(Ok(()), |token| token.check())
    })
}

/// Version initialization must not cache a cancelled caller's error in OnceLock.
pub(super) fn uncancelled<R>(f: impl FnOnce() -> R) -> R {
    let _current = Current(CURRENT.with(|current| current.replace(None)));
    cancellable(Arc::new(AtomicBool::new(false)), f)
}

/// The catalog has its own bounded worker, but uses the same process cancellation checks.
pub(crate) fn cancellable<R>(cancel: Arc<AtomicBool>, f: impl FnOnce() -> R) -> R {
    struct Restore(Option<Arc<AtomicBool>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            EXTERNAL.with(|slot| slot.replace(self.0.take()));
        }
    }
    let _restore = Restore(EXTERNAL.with(|slot| slot.replace(Some(cancel))));
    f()
}

#[cfg(test)]
mod tests;
