use super::*;
use std::{sync::atomic::AtomicUsize, task::Wake};

fn token() -> Arc<Token> {
    Arc::new(Token {
        id: NEXT.fetch_add(1, Ordering::Relaxed),
        cancelled: AtomicBool::new(false),
        waiter: Mutex::new(None),
    })
}
struct Counter(AtomicUsize);
impl Wake for Counter {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn read_admission_yields_at_limit_and_cancelled_waiters_wake() {
    let counter = Arc::new(Counter(AtomicUsize::new(0)));
    let waker = Waker::from(counter.clone());
    let mut cx = Context::from_waker(&waker);
    let mut permits = Vec::new();
    for _ in 0..READERS {
        let mut admission = Box::pin(Admission(token()));
        let Poll::Ready(Ok(permit)) = admission.as_mut().poll(&mut cx) else {
            panic!("capacity available");
        };
        permits.push(permit);
    }
    // Superseding a queued read cancels it without allocating a blocking worker.
    let owner = GitQueries::open();
    let mut older = Box::pin(run(owner, "log", |_| 1));
    assert!(older.as_mut().poll(&mut cx).is_pending());
    let mut newer = Box::pin(run(owner, "log", |_| 2));
    assert!(newer.as_mut().poll(&mut cx).is_pending());
    assert!(matches!(older.as_mut().poll(&mut cx), Poll::Ready(Err(_))));
    GitQueries::close(owner);
    assert!(matches!(newer.as_mut().poll(&mut cx), Poll::Ready(Err(_))));
    let cancelled = token();
    let mut waiting = Box::pin(Admission(cancelled.clone()));
    assert!(waiting.as_mut().poll(&mut cx).is_pending());
    cancelled.cancel();
    assert!(counter.0.load(Ordering::Relaxed) > 0);
    assert!(matches!(
        waiting.as_mut().poll(&mut cx),
        Poll::Ready(Err(_))
    ));
    let mut next = Box::pin(Admission(token()));
    assert!(next.as_mut().poll(&mut cx).is_pending());
    drop(permits.pop());
    assert!(matches!(next.as_mut().poll(&mut cx), Poll::Ready(Ok(_))));
    drop(permits);
}

#[test]
fn closing_a_view_cancels_its_reads_but_leaves_other_views_alive() {
    let first = GitQueries::open();
    let second = GitQueries::open();
    let a = token();
    let b = token();
    owners().lock().unwrap()[&first]
        .jobs
        .lock()
        .unwrap()
        .insert("log", a.clone());
    owners().lock().unwrap()[&second]
        .jobs
        .lock()
        .unwrap()
        .insert("log", b.clone());
    GitQueries::close(first);
    assert!(a.check().is_err());
    assert!(b.check().is_ok());
    assert!(!owners().lock().unwrap().contains_key(&first));
    GitQueries::close(second);
}

#[cfg(unix)]
#[test]
fn cancelling_a_running_read_kills_and_reaps_its_process_group() {
    let token = token();
    let worker_token = token.clone();
    let marker = std::env::temp_dir().join(format!(
        "termlevel-cancel-{}-{}",
        std::process::id(),
        token.id
    ));
    let script = format!("echo $$ > '{}'; sleep 30", marker.display());
    let worker = std::thread::spawn(move || {
        let _current = Current(CURRENT.with(|current| current.replace(Some(worker_token))));
        super::super::run_capture(
            Path::new("/bin/sh"),
            None,
            &["-c", &script],
            Duration::from_secs(10),
            4096,
        )
    });
    let deadline = Instant::now() + Duration::from_secs(2);
    while !marker.exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }
    assert!(marker.exists(), "child started");
    let started = Instant::now();
    token.cancel();
    let result = worker.join().unwrap();
    assert!(result.err().unwrap().contains("cancelled"));
    assert!(started.elapsed() < Duration::from_secs(1));
    let pid: i32 = std::fs::read_to_string(&marker)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1, "child was reaped");
    let _ = std::fs::remove_file(marker);
}

#[cfg(unix)]
#[test]
fn deadline_covers_helpers_that_hold_pipes_after_git_exits() {
    let started = Instant::now();
    let result = super::super::run_capture(
        Path::new("/bin/sh"),
        None,
        &["-c", "sleep 30 & exit 0"],
        Duration::from_millis(80),
        4096,
    );
    assert!(result.err().unwrap().contains("timed out"));
    assert!(started.elapsed() < Duration::from_secs(1));
}
