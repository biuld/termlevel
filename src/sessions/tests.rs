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

#[test]
fn a_missing_or_legacy_locale_becomes_utf8_and_an_existing_one_stays() {
    assert_eq!(
        utf8_locale_env(None, None, None),
        vec![("LANG", "en_US.UTF-8")]
    );
    assert_eq!(
        utf8_locale_env(Some("C".into()), Some("C".into()), None),
        vec![
            ("LC_ALL", "en_US.UTF-8"),
            ("LC_CTYPE", "en_US.UTF-8"),
            ("LANG", "en_US.UTF-8")
        ]
    );
    assert!(utf8_locale_env(None, None, Some("zh_CN.UTF-8".into())).is_empty());
    assert!(utf8_locale_env(Some("en_US.UTF-8".into()), Some("C".into()), None).is_empty());
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

fn status_record(state: ProgramStatusState, app: &str, updated: u64) -> StatusRecord {
    StatusRecord {
        state,
        kind: ProgramStatusKind::None,
        percent: None,
        app: app.into(),
        title: String::new(),
        message: String::new(),
        updated,
        version: updated,
        acknowledged: false,
    }
}

#[test]
fn program_status_aggregate_prefers_actionable_records_and_inherits_app() {
    let mut records = BTreeMap::new();
    records.insert(
        String::new(),
        status_record(ProgramStatusState::Working, "deploy", 1),
    );
    records.insert(
        "us-east".into(),
        status_record(ProgramStatusState::Blocked, "", 2),
    );
    let activity = aggregate(&records).unwrap();
    assert_eq!(activity.state, "blocked");
    assert_eq!(activity.app.as_deref(), Some("deploy"));
}

#[test]
fn clearing_one_status_record_removes_children_and_keeps_siblings() {
    let mut records = BTreeMap::new();
    records.insert(
        "build".into(),
        status_record(ProgramStatusState::Working, "", 1),
    );
    records.insert(
        "build/test".into(),
        status_record(ProgramStatusState::Working, "", 2),
    );
    records.insert(
        "deploy".into(),
        status_record(ProgramStatusState::Done, "", 3),
    );
    clear_status_subtree(&mut records, "build");
    assert!(!records.contains_key("build"));
    assert!(!records.contains_key("build/test"));
    assert!(records.contains_key("deploy"));
}

#[test]
fn real_shell_progress_and_program_status_update_info_and_acknowledge() {
    let mut manager = SessionManager::default();
    let session = shell(&mut manager);
    session.input(
        b"printf '\\033]9;4;1;42\\007\\033]7501;state=working:app=cargo:progress=40\\007'\n",
    );
    wait_for(|| {
        let info = session.info();
        info.progress
            .as_ref()
            .is_some_and(|progress| progress.percent == Some(42))
            && info
                .activity
                .as_ref()
                .is_some_and(|activity| activity.state == "working")
    });
    let activity = session.info().activity.unwrap();
    assert_eq!(activity.app.as_deref(), Some("cargo"));
    assert_eq!(activity.percent, Some(40));

    session.input(
        b"printf '\\033]7501;state=blocked:kind=permission:app=terraform:msg=QXBwbHk/\\007'\n",
    );
    wait_for(|| {
        session
            .info()
            .activity
            .as_ref()
            .is_some_and(|activity| activity.state == "blocked")
    });
    session.acknowledge(session.info().status_revision);
    let activity = session.info().activity.unwrap();
    assert_eq!(activity.state, "blocked");
    assert!(!activity.unread);
    session.input(b"printf '\\033]7501;state=clear\\007'\n");
    wait_for(|| session.info().activity.is_none());

    session.input(b"printf '\\033]9;4;1;55\\007\\033]7501;state=working:id=build\\007'\n");
    wait_for(|| {
        session
            .info()
            .activity
            .as_ref()
            .is_some_and(|activity| activity.state == "working")
    });
    session.input(b"exit 7\n");
    wait_for(|| session.info().exit_code == Some(7));
    let info = session.info();
    assert!(info.progress.is_none());
    assert!(
        info.activity.is_none(),
        "working records are removed on exit"
    );
    manager.shutdown();
    wait_for(|| finished(&session));
}

#[test]
fn pwd_uris_and_bare_paths_are_decoded_and_validated() {
    assert_eq!(
        decode_pwd("file://localhost/tmp/a%20b"),
        Some("/tmp/a b".into())
    );
    assert_eq!(decode_pwd("file:///tmp/x"), Some("/tmp/x".into()));
    assert_eq!(decode_pwd("/tmp/x"), Some("/tmp/x".into()));
    assert_eq!(decode_pwd("file://server/tmp"), None);
    assert_eq!(decode_pwd("http://example.com/x"), None);
    assert_eq!(decode_pwd("relative/path"), None);
    assert_eq!(decode_pwd(""), None);
    assert_eq!(decode_pwd("/tmp/\n"), None);
}

#[test]
fn real_shell_pwd_prompt_and_mouse_shape_reach_info() {
    let mut manager = SessionManager::default();
    let session = shell(&mut manager);
    session.input(b"printf '\\033]7;file://localhost/tmp\\007'\n");
    wait_for(|| session.info().current_cwd.as_deref() == Some("/tmp"));
    session.input(b"printf '\\033]22;pointer\\007'\n");
    wait_for(|| session.info().mouse_shape == "pointer");
    session.input(b"printf '\\033]7501;state=working:id=build\\007'\n");
    wait_for(|| {
        session
            .info()
            .activity
            .as_ref()
            .is_some_and(|activity| activity.state == "working")
    });
    session.input(b"printf '\\033]133;A\\007'\n");
    wait_for(|| session.info().at_prompt);
    assert!(session.info().activity.is_none());
    session.input(b"printf '\\033]133;C\\007'\n");
    wait_for(|| !session.info().at_prompt);
    session.input(b"printf '\\033]7;\\007'\n");
    wait_for(|| session.info().current_cwd.is_none());
    session.input(b"exit 7\n");
    wait_for(|| session.info().exit_code == Some(7));
    let info = session.info();
    assert!(info.current_cwd.is_none());
    assert!(!info.at_prompt);
    assert_eq!(info.mouse_shape, "text");
    manager.shutdown();
    wait_for(|| finished(&session));
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
    let info = manager.open_with_colors(
        "/tmp".into(),
        command,
        (Some("#abcdef"), Some("#102030")),
        None,
    );
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

#[test]
fn terminal_identity_is_per_session_and_does_not_claim_a_ghostty_version() {
    let mut manager = SessionManager::default();
    let open = |manager: &mut SessionManager, identity| {
        let mut command = CommandBuilder::new("/bin/sh");
        command.args(["-c", "stty -echo; exec /bin/sh -i"]);
        command.env("PS1", "");
        // A launcher's identity must not leak through either profile.
        command.env("TERM_PROGRAM", "inherited-terminal");
        command.env("TERM_PROGRAM_VERSION", "999.0");
        let info = manager.open_with_colors("/tmp".into(), command, (None, None), identity);
        assert_eq!(info.state, "running", "{:?}", info.error);
        manager.get(&info.id).unwrap()
    };
    let native = open(&mut manager, None);
    let compatibility = open(&mut manager, Some("ghostty"));
    let fallback = open(&mut manager, Some("unknown"));
    let report = b"printf 'IDENTITY=%s;VERSION=%s;TERM=%s;COLOR=%s\\n' \"$TERM_PROGRAM\" \"${TERM_PROGRAM_VERSION-unset}\" \"$TERM\" \"$COLORTERM\"\n";
    // Report from the old session after opening the compatibility session.
    native.input(report);
    compatibility.input(report);
    fallback.input(report);
    let native_report = format!(
        "IDENTITY=termlevel;VERSION={};TERM=xterm-256color;COLOR=truecolor",
        env!("CARGO_PKG_VERSION")
    );
    wait_for(|| output(&native).contains(&native_report));
    wait_for(|| output(&fallback).contains(&native_report));
    wait_for(|| {
        output(&compatibility)
            .contains("IDENTITY=ghostty;VERSION=unset;TERM=xterm-256color;COLOR=truecolor")
    });
    assert_ne!(native.info().id, compatibility.info().id);
    manager.shutdown();
    wait_for(|| finished(&native) && finished(&compatibility) && finished(&fallback));
}

#[test]
fn reply_storm_survives_a_client_that_reads_only_after_writing_all_queries() {
    let mut manager = SessionManager::default();
    let mut command = CommandBuilder::new("/usr/bin/python3");
    command.arg(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/probes/input_backpressure_probe.py"
    ));
    let info = manager.open("/tmp".into(), command);
    let session = manager.get(&info.id).unwrap();
    wait_for(|| session.info().exit_code.is_some());
    assert_eq!(session.info().exit_code, Some(0), "{}", output(&session));
    wait_for(|| output(&session).contains("INPUT_REPLIES_OK count=20000"));
    assert_eq!(session.info().error, None);
    assert_eq!(session.info().input_warning, None);
    manager.shutdown();
    wait_for(|| finished(&session));
}

#[test]
fn rejected_paste_is_a_warning_and_a_later_delivered_input_clears_it() {
    let mut manager = SessionManager::default();
    let session = shell(&mut manager);
    session.input(&vec![b'x'; input_queue::INPUT_LIMIT + 1]);
    assert_eq!(session.info().state, "running");
    assert_eq!(session.info().error, None);
    assert!(
        session
            .info()
            .input_warning
            .unwrap()
            .contains("paste exceeds")
    );
    session.input(b"printf 'AFTER_REJECT_%s\\n' OK\n");
    wait_for(|| output(&session).contains("AFTER_REJECT_OK"));
    wait_for(|| session.info().input_warning.is_none());
    assert_eq!(session.info().state, "running");
    manager.shutdown();
    wait_for(|| finished(&session));
}

#[test]
fn notifications_survive_session_close_without_a_mounted_view() {
    let mut manager = SessionManager::default();
    let session = shell(&mut manager);
    let id = session.info().id;
    session.input(b"printf '\\033]777;notify;SESSION_NOTICE;Background done\\007'\n");
    wait_for(|| {
        crate::notifications::Notifications::list()
            .iter()
            .any(|entry| {
                entry.session_id.as_deref() == Some(&id) && entry.title == "SESSION_NOTICE"
            })
    });
    manager.close(&id);
    wait_for(|| finished(&session));
    assert!(
        crate::notifications::Notifications::list()
            .iter()
            .any(
                |entry| entry.session_id.as_deref() == Some(&id) && entry.body == "Background done"
            )
    );
}

fn report(id: &str, state: ProgramStatusState) -> ProgramStatusReport {
    ProgramStatusReport {
        id: id.into(),
        state,
        kind: ProgramStatusKind::Permission,
        percent: None,
        app: "agent".into(),
        title: String::new(),
        message: "Apply?".into(),
    }
}

#[test]
fn acknowledgement_preserves_facts_and_cannot_consume_later_results() {
    let mut manager = SessionManager::default();
    let session = shell(&mut manager);
    assert_eq!(session.info().shell_phase, "unknown");
    session.apply_program_status(report("task", ProgramStatusState::Blocked));
    let seen = session.info().status_revision;
    session.acknowledge(seen);
    assert_eq!(session.info().status.state, "blocked");
    assert!(!session.info().activity.unwrap().unread);
    session.apply_program_status(report("task", ProgramStatusState::Blocked));
    assert!(
        !session.info().activity.unwrap().unread,
        "same state stays read"
    );
    session.apply_program_status(report("task", ProgramStatusState::Working));
    session.apply_program_status(report("task", ProgramStatusState::Blocked));
    session.acknowledge(seen);
    assert!(
        session.info().activity.unwrap().unread,
        "old snapshot cannot consume new generation"
    );
    session.apply_program_status(report("task", ProgramStatusState::Error));
    let error_seen = session.info().status_revision;
    session.apply_program_status(report("next", ProgramStatusState::Done));
    session.acknowledge(error_seen);
    assert_eq!(
        session.info().status.state,
        "done",
        "later result is still pending"
    );
    session.acknowledge(session.info().status_revision);
    assert!(session.info().activity.is_none());
    assert_eq!(
        session.status.lock().unwrap().records.len(),
        2,
        "ack does not delete reports"
    );
    session.apply_program_status(report("task", ProgramStatusState::Error));
    assert!(
        session.info().activity.is_none(),
        "identical repeated error stays acknowledged"
    );
    session.apply_program_status(report("task", ProgramStatusState::Working));
    assert_eq!(session.info().status.state, "working");
    // Root clear and RIS must not reuse a version accepted before the reset.
    session.apply_program_status(report("", ProgramStatusState::Clear));
    session.handle_events(vec![BackendEvent::Reset]);
    session.apply_program_status(report("task", ProgramStatusState::Error));
    session.acknowledge(error_seen);
    assert_eq!(session.info().status.state, "error");
    manager.shutdown();
    wait_for(|| finished(&session));
}

#[test]
fn record_transitions_notify_even_when_aggregate_state_is_unchanged() {
    let mut manager = SessionManager::default();
    let session = shell(&mut manager);
    let id = session.info().id;
    let notices = || {
        crate::notifications::Notifications::list()
            .into_iter()
            .filter(|n| n.session_id.as_deref() == Some(&id) && n.program.is_some())
            .collect::<Vec<_>>()
    };
    session.apply_program_status(report("a", ProgramStatusState::Blocked));
    session.apply_program_status(report("b", ProgramStatusState::Blocked));
    assert_eq!(
        notices().len(),
        2,
        "two tasks with identical text do not coalesce"
    );
    assert_eq!(session.info().status.pending_count, 1);
    assert_eq!(session.info().status.label, "Needs approval");
    let program = notices()[0].program.clone().unwrap();
    assert_eq!(program.kind.as_deref(), Some("permission"));
    assert_eq!(program.app.as_deref(), Some("agent"));
    assert_eq!(program.message.as_deref(), Some("Apply?"));
    session.acknowledge(session.info().status_revision);
    session.apply_program_status(report("b", ProgramStatusState::Blocked));
    assert_eq!(notices().len(), 2, "ack + repeated report never renotifies");
    session.apply_program_status(report("a", ProgramStatusState::Working));
    session.apply_program_status(report("a", ProgramStatusState::Blocked));
    assert_eq!(notices().len(), 3, "new blocked generation notifies again");
    session.apply_program_status(report("error", ProgramStatusState::Error));
    assert_eq!(
        notices().len(),
        4,
        "error cannot be hidden by blocked aggregation"
    );
    assert!(notices().iter().any(|n| n.title.starts_with("Task failed")));
    session.apply_program_status(report("b", ProgramStatusState::Clear));
    assert_eq!(
        notices().len(),
        4,
        "revealing an old state is not a new transition"
    );
    manager.shutdown();
    wait_for(|| finished(&session));
}

#[test]
fn resolved_progress_and_lifecycle_do_not_invent_completion() {
    let mut manager = SessionManager::default();
    let session = shell(&mut manager);
    session.apply_program_status(ProgramStatusReport {
        percent: Some(40),
        ..report("", ProgramStatusState::Working)
    });
    assert_eq!(session.info().status.progress.unwrap().percent, Some(40));
    session.apply_progress(ProgressReport {
        state: ProgressState::Set,
        percent: Some(42),
    });
    let resolved = session.info().status.progress.unwrap();
    assert_eq!(resolved.source, "osc9");
    assert_eq!(resolved.label, "Terminal progress 42%");
    session.apply_program_status(report("", ProgramStatusState::Blocked));
    assert_eq!(session.info().status.progress.unwrap().percent, Some(42));
    session.apply_progress(ProgressReport {
        state: ProgressState::Remove,
        percent: None,
    });
    assert!(
        session.info().status.progress.is_none(),
        "blocked never invents a line"
    );
    for state in [
        ProgressState::Pause,
        ProgressState::Error,
        ProgressState::Indeterminate,
    ] {
        session.apply_progress(ProgressReport {
            state,
            percent: None,
        });
        assert!(session.info().status.progress.is_none());
    }
    session.apply_progress(ProgressReport {
        state: ProgressState::Indeterminate,
        percent: Some(91),
    });
    session.apply_program_status(ProgramStatusReport {
        percent: Some(40),
        ..report("", ProgramStatusState::Working)
    });
    let fallback = session.info().status.progress.unwrap();
    assert_eq!(fallback.source, "osc7501");
    assert_eq!(fallback.percent, Some(40));
    assert!(!fallback.indeterminate && !fallback.animate);
    session.apply_progress(ProgressReport {
        state: ProgressState::Set,
        percent: Some(100),
    });
    assert_eq!(session.info().status.state, "working");
    session.input(b"printf '\\033]133;A\\007'\n");
    wait_for(|| session.info().shell_phase == "prompt");
    assert!(session.info().status.progress.is_none());
    assert!(session.info().activity.is_none());
    session.apply_program_status(report("result", ProgramStatusState::Done));
    session.input(b"exit 9\n");
    wait_for(|| session.info().exit_code == Some(9));
    assert_eq!(session.info().status.label, "Exited (code 9)");
    session.acknowledge(session.info().status_revision);
    assert_eq!(session.info().status.state, "exited");
    manager.shutdown();
    wait_for(|| finished(&session));
}

#[test]
fn record_replacement_and_eviction_keep_only_bounded_current_facts() {
    let mut manager = SessionManager::default();
    let session = shell(&mut manager);
    for index in 0..STATUS_RECORD_LIMIT {
        session.apply_program_status(report(
            &format!("task/{index}"),
            ProgramStatusState::Working,
        ));
    }
    session.apply_program_status(ProgramStatusReport {
        app: String::new(),
        title: String::new(),
        message: String::new(),
        ..report("task/0", ProgramStatusState::Working)
    });
    let activity = session.info().activity.unwrap();
    assert!(
        activity.app.is_none() && activity.message.is_none(),
        "replacement drops omitted display fields"
    );
    session.apply_program_status(report("new", ProgramStatusState::Working));
    {
        let status = session.status.lock().unwrap();
        assert_eq!(status.records.len(), STATUS_RECORD_LIMIT);
        assert!(status.records.contains_key("task/0"));
        assert!(
            !status.records.contains_key("task/1"),
            "evict least recently updated record"
        );
    }
    session.apply_program_status(report("task", ProgramStatusState::Clear));
    assert_eq!(session.status.lock().unwrap().records.len(), 1);
    manager.shutdown();
    wait_for(|| finished(&session));
}
