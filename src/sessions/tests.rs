#![cfg(unix)]

use super::*;
use std::time::Instant;

fn wait_for(mut condition: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(8);
    while !condition() {
        assert!(Instant::now() < until, "timed out waiting for PTY state");
        thread::sleep(Duration::from_millis(10));
    }
}

fn output(session: &Session) -> String {
    let emulator = session.emulator.lock().unwrap();
    (0..emulator.line_count())
        .map(|i| emulator.line(i).text())
        .collect::<Vec<_>>()
        .join("\n")
}

fn shell(manager: &mut SessionManager) -> Arc<Session> {
    let mut command = CommandBuilder::new("/bin/sh");
    command.args([
        "-c",
        "stty -echo; printf 'SESSION_READY\\n'; exec /bin/sh -i",
    ]);
    command.env("PS1", "");
    let info = manager.open("/tmp".into(), command);
    assert_eq!(info.state, "running", "{:?}", info.error);
    let session = manager.get(&info.id).unwrap();
    wait_for(|| output(&session).contains("SESSION_READY"));
    session
}

fn finished(session: &Session) -> bool {
    session
        .workers
        .lock()
        .unwrap()
        .iter()
        .all(thread::JoinHandle::is_finished)
}

#[test]
fn real_shell_utf8_resize_title_and_exit_status() {
    let mut manager = SessionManager::default();
    let session = shell(&mut manager);
    session.resize(TerminalSize::new(91, 31));
    session
        .input("printf 'UTF_%s\\n' '中文'; stty size; printf '\\033]0;Real PTY\\007'\n".as_bytes());
    wait_for(|| output(&session).contains("UTF_中文") && output(&session).contains("31 91"));
    wait_for(|| session.info().title == "Real PTY");
    session.input(b"printf '\\007'\n");
    wait_for(|| session.bells.load(Ordering::Acquire) == 1);
    session.input(b"sleep 60\n");
    let shell_pid = session.info().pid.unwrap() as libc::pid_t;
    let foreground = || {
        session
            .control
            .lock()
            .unwrap()
            .master
            .as_ref()
            .and_then(|master| master.process_group_leader())
    };
    wait_for(|| foreground().is_some_and(|pid| pid != shell_pid));
    session.input(b"\x03");
    wait_for(|| foreground() == Some(shell_pid));
    assert_eq!(session.info().state, "running");
    session.input(b"printf 'FINAL_%s\\n' OUTPUT; exit 7\n");
    wait_for(|| session.info().exit_code == Some(7) && output(&session).contains("FINAL_OUTPUT"));
    // Exit retains the screen and identity; reattaching is not a restart.
    let rebound = manager.get(&session.info().id).unwrap();
    assert!(Arc::ptr_eq(&session, &rebound));
    assert_eq!(rebound.info().state, "exited");
    assert!(output(&rebound).contains("UTF_中文"));
    manager.shutdown();
    wait_for(|| finished(&session));
}

#[cfg(target_os = "macos")]
#[test]
fn zsh_runs_interactively_with_utf8_and_exit_status() {
    let mut manager = SessionManager::default();
    let mut command = CommandBuilder::new("/bin/zsh");
    command.args(["-f", "-i"]);
    command.env("HISTFILE", "/dev/null");
    let info = manager.open("/tmp".into(), command);
    let session = manager.get(&info.id).unwrap();
    session.input("printf 'ZSH_%s\\n' '中文'; exit 5\n".as_bytes());
    wait_for(|| output(&session).contains("ZSH_中文") && session.info().exit_code == Some(5));
    manager.shutdown();
    wait_for(|| finished(&session));
}

#[test]
fn replies_go_directly_from_parser_to_real_pty() {
    let mut manager = SessionManager::default();
    let mut command = CommandBuilder::new("/bin/sh");
    command.args([
        "-c",
        "stty raw -echo; printf '\\033[6n'; dd bs=1 count=6 2>/dev/null | od -An -tx1",
    ]);
    let info = manager.open("/tmp".into(), command);
    let session = manager.get(&info.id).unwrap();
    wait_for(|| {
        output(&session).split_whitespace().collect::<Vec<_>>()
            == ["1b", "5b", "31", "3b", "31", "52"]
    });
    wait_for(|| session.info().exit_code == Some(0));
    manager.shutdown();
    wait_for(|| finished(&session));
}

#[test]
fn ghostty_encoded_keys_text_paste_and_focus_reach_the_real_pty() {
    use termlevel_ghostty_vt::{KeyEvent, raw};
    let expected = [
        b"\x1bOA\x1b[15;6~".as_slice(),
        "中文".as_bytes(),
        b"\x1b[200~hello\nworld\x1b[201~\x1b[I",
    ]
    .concat();
    let mut manager = SessionManager::default();
    let mut command = CommandBuilder::new("/bin/sh");
    let script = format!(
        "stty raw -echo; printf '\\033[?1h\\033[?2004h\\033[?1004hRAW_READY\\r\\n'; dd bs=1 count={} 2>/dev/null | od -An -tx1",
        expected.len()
    );
    command.args(["-c", &script]);
    let info = manager.open("/tmp".into(), command);
    let session = manager.get(&info.id).unwrap();
    wait_for(|| output(&session).contains("RAW_READY"));
    session.key_input(&KeyEvent::press(raw::GHOSTTY_KEY_ARROW_UP, ""));
    let mut f5 = KeyEvent::press(raw::GHOSTTY_KEY_F5, "");
    f5.modifiers = (raw::GHOSTTY_MODS_SHIFT | raw::GHOSTTY_MODS_CTRL) as u16;
    session.key_input(&f5);
    session.text_input("中文");
    session.paste("hello\nworld");
    session.focus_input(true);
    wait_for(|| session.info().exit_code == Some(0));
    // Reaping the child can precede draining its PTY. Wait for the reader too;
    // od's whitespace and raw-mode line wrapping are not part of the encoding.
    wait_for(|| finished(&session));
    let actual: Vec<_> = output(&session)
        .split_whitespace()
        .filter(|token| token.len() == 2 && token.bytes().all(|c| c.is_ascii_hexdigit()))
        .map(|token| u8::from_str_radix(token, 16).unwrap())
        .collect();
    assert_eq!(actual, expected);
    manager.shutdown();
    wait_for(|| finished(&session));
}

#[test]
fn real_pty_color_queries_use_the_ui_theme_before_first_paint() {
    let mut manager = SessionManager::default();
    let expected = b"\x1b]11;rgb:1010/2020/3030\x1b\\";
    let mut command = CommandBuilder::new("/bin/sh");
    command.args(["-c", &format!("stty raw -echo; printf '\\033]11;?\\033\\\\'; dd bs=1 count={} 2>/dev/null | od -An -tx1", expected.len())]);
    let info = manager.open_with_colors("/tmp".into(), command, (Some("#abcdef"), Some("#102030")));
    let session = manager.get(&info.id).unwrap();
    let bytes = expected
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>();
    wait_for(|| output(&session).split_whitespace().collect::<Vec<_>>() == bytes);
    wait_for(|| session.info().exit_code == Some(0));
    manager.shutdown();
    wait_for(|| finished(&session));
}

#[test]
fn sessions_survive_detach_and_are_isolated_until_explicit_close() {
    let mut manager = SessionManager::default();
    let first = shell(&mut manager);
    let second = shell(&mut manager);
    let id = first.info().id;
    first.input(b"printf 'ONLY_%s\\n' FIRST\n");
    wait_for(|| output(&first).contains("ONLY_FIRST"));
    assert!(!output(&second).contains("ONLY_FIRST"));
    drop(first); // No view/reference remains; manager is the owner.
    let rebound = manager.get(&id).unwrap();
    assert!(output(&rebound).contains("ONLY_FIRST"));
    rebound.input(b"printf 'STILL_%s\\n' ALIVE\n");
    wait_for(|| output(&rebound).contains("STILL_ALIVE"));
    manager.close(&id);
    manager.close(&id);
    assert!(manager.get(&id).is_err());
    assert_eq!(rebound.info().state, "closed");
    wait_for(|| finished(&rebound));
    assert_eq!(second.info().state, "running");
    manager.shutdown();
    wait_for(|| finished(&second));
}

#[test]
fn closing_kills_foreground_job_and_reaps_shell() {
    let mut manager = SessionManager::default();
    let session = shell(&mut manager);
    let shell_pid = session.info().pid.unwrap() as libc::pid_t;
    session.input(b"sleep 60\n");
    let foreground = || {
        session
            .control
            .lock()
            .unwrap()
            .master
            .as_ref()
            .and_then(|master| master.process_group_leader())
    };
    wait_for(|| foreground().is_some_and(|pid| pid != shell_pid));
    let job_pid = foreground().unwrap();
    manager.close(&session.info().id);
    wait_for(|| finished(&session));
    wait_for(|| unsafe { libc::kill(shell_pid, 0) != 0 && libc::kill(job_pid, 0) != 0 });
    assert!(session.info().exit_code.is_some());
}

#[test]
fn closing_allows_shell_to_hang_up_background_jobs() {
    let mut manager = SessionManager::default();
    let session = shell(&mut manager);
    session.input(b"sleep 60 & printf 'BACKGROUND_%s\\n' $!\n");
    let job = || {
        output(&session).lines().find_map(|line| {
            line.strip_prefix("BACKGROUND_")
                .and_then(|pid| pid.parse::<i32>().ok())
        })
    };
    wait_for(|| job().is_some());
    let job = job().unwrap();
    manager.close(&session.info().id);
    wait_for(|| finished(&session));
    let until = Instant::now() + Duration::from_secs(3);
    while unsafe { libc::kill(job, 0) == 0 } {
        if Instant::now() > until {
            unsafe {
                libc::kill(job, libc::SIGKILL);
            }
            panic!("background job survived closing its terminal");
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(manager.settled());
}

#[test]
fn startup_failures_have_no_process_and_do_not_reopen_on_attach() {
    let mut manager = SessionManager::default();
    let invalid_dir = manager.open(
        "/termlevel-nonexistent-directory".into(),
        CommandBuilder::new("/bin/sh"),
    );
    assert_eq!(invalid_dir.state, "error");
    assert!(invalid_dir.error.unwrap().contains("Cannot open directory"));
    assert!(invalid_dir.pid.is_none());
    assert!(manager.get(&invalid_dir.id).is_err());
    let invalid_command = manager.open(
        "/tmp".into(),
        CommandBuilder::new("/termlevel-no-such-program"),
    );
    assert_eq!(invalid_command.state, "error");
    assert!(invalid_command.pid.is_none());
    assert_eq!(manager.list().len(), 2);
    manager.close(&invalid_dir.id);
    assert!(manager.get(&invalid_dir.id).is_err());
}

#[test]
fn large_output_stays_bounded_and_finishes_before_view_attaches() {
    let mut manager = SessionManager::default();
    // Exercise eviction at an explicit limit, independently of the UI default.
    let history_limit = 3000;
    manager.set_scrollback(history_limit);
    let mut command = CommandBuilder::new("/bin/sh");
    command.args(["-c", "i=0; while [ $i -lt 4000 ]; do printf 'line_%s\\r\\n' $i; i=$((i+1)); done; printf 'BULK_DONE\\r\\n'"]);
    let info = manager.open("/tmp".into(), command);
    let session = manager.get(&info.id).unwrap();
    wait_for(|| session.info().exit_code == Some(0) && output(&session).contains("BULK_DONE"));
    {
        let emulator = session.emulator.lock().unwrap();
        assert!(emulator.line_count() <= history_limit + emulator.screen_rows());
    }
    assert!(!output(&session).contains("line_0\n"));
    assert!(output(&session).contains("line_3999"));
    manager.shutdown();
    wait_for(|| finished(&session));
}

#[test]
fn synchronized_output_flushes_on_idle_and_eof() {
    let mut manager = SessionManager::default();
    for script in [
        "printf '\\033[?2026hSYNC_IDLE'; read line",
        "printf '\\033[?2026hSYNC_EOF'",
    ] {
        let mut command = CommandBuilder::new("/bin/sh");
        command.args(["-c", script]);
        let info = manager.open("/tmp".into(), command);
        let session = manager.get(&info.id).unwrap();
        wait_for(|| output(&session).contains("SYNC_"));
        if script.contains("SYNC_IDLE") {
            assert!(session.info().exit_code.is_none());
        }
        manager.close(&info.id);
        wait_for(|| finished(&session));
    }
}

#[test]
fn manager_shutdown_terminates_all_owned_processes() {
    let (first, second) = {
        let mut manager = SessionManager::default();
        (shell(&mut manager), shell(&mut manager))
    };
    wait_for(|| finished(&first) && finished(&second));
    assert_eq!(first.info().state, "closed");
    assert_eq!(second.info().state, "closed");
}
