use super::*;
use termlevel_ghostty_vt::{MouseShape, ProgramStatusState, ProgressState, SemanticPromptKind};

fn emulator(cols: u16, rows: u16, history: usize) -> GhosttyEmulator {
    GhosttyEmulator::new(TerminalSize::new(cols, rows), history)
}

fn lines(term: &GhosttyEmulator) -> Vec<String> {
    (0..term.line_count())
        .map(|i| term.line(i).text())
        .collect()
}

#[test]
fn split_utf8_and_escape_sequences_keep_parser_state() {
    let mut term = emulator(20, 3, 10);
    for byte in "中e\u{301}\x1b[38;2;1;2;3m文".as_bytes() {
        term.feed(&[*byte]);
    }
    assert_eq!(term.line(0).text(), "中e\u{301}文");
    assert_eq!(
        term.line(0).runs[1].style.fg,
        Some(TerminalColor::Rgb(1, 2, 3))
    );
    assert_eq!(term.cursor().unwrap().col, 5);
}

#[test]
fn styles_and_background_spaces_survive_conversion() {
    let mut term = emulator(20, 2, 0);
    term.feed(b"\x1b[1;2;3;4;7;9;31;48;5;200mA \x1b[0mB");
    let line = term.line(0);
    assert_eq!(line.text(), "A B");
    let style = line.runs[0].style;
    assert_eq!(style.fg, Some(TerminalColor::Indexed(1)));
    assert_eq!(style.bg, Some(TerminalColor::Indexed(200)));
    assert!(style.bold && style.dim && style.italic && style.underline);
    assert!(style.inverse && style.strikethrough);
    assert_eq!(line.runs[1].style, TerminalStyle::default());
    term.feed(b"\r\n\x1b[44m  ");
    assert!(term.line(1).text().starts_with("  "));
    assert_eq!(
        term.line(1).runs[0].style.bg,
        Some(TerminalColor::Indexed(4))
    );
}

#[test]
fn search_matches_case_insensitively_in_order() {
    let mut term = emulator(20, 4, 10);
    term.feed(b"Hello\r\nhello\r\nnone");
    let hits = term.search("HELLO");
    assert_eq!(hits.len(), 2);
    assert_eq!(
        hits[0],
        SearchHit {
            line: 0,
            col: 0,
            cols: 5
        }
    );
    assert_eq!(hits[1].line, 1);
    assert!(term.search("").is_empty());
    assert!(term.search("missing").is_empty());
}

#[test]
fn search_wide_glyph_uses_its_column_span() {
    let mut term = emulator(20, 2, 4);
    term.feed("a中b".as_bytes());
    assert_eq!(
        term.search("中"),
        vec![SearchHit {
            line: 0,
            col: 1,
            cols: 2
        }]
    );
}

#[test]
fn scrollback_limit_keeps_ten_thousand_lines() {
    assert_eq!(SCROLLBACK_LIMIT, 10_000);
    assert_eq!(clamp_scrollback(1), SCROLLBACK_MIN);
    assert_eq!(clamp_scrollback(usize::MAX), SCROLLBACK_MAX);
}

#[test]
fn set_scrollback_drops_the_oldest_lines() {
    let mut term = emulator(10, 2, 4);
    term.feed(b"zero\r\none\r\ntwo\r\nthree\r\nfour");
    assert_eq!(lines(&term), ["zero", "one", "two", "three", "four"]);
    term.set_scrollback(2);
    assert_eq!(lines(&term), ["one", "two", "three", "four"]);
    term.set_scrollback(20);
    assert_eq!(
        lines(&term),
        ["one", "two", "three", "four"],
        "growing the limit cannot resurrect discarded rows"
    );
    assert!(term.backend_events().is_empty());
}

#[test]
fn history_is_bounded_and_ordered_oldest_first() {
    let mut term = emulator(10, 2, 2);
    term.feed(b"zero\r\none\r\ntwo\r\nthree\r\nfour");
    assert_eq!(lines(&term), ["one", "two", "three", "four"]);
    assert!(term.line(4).runs.is_empty());
    assert_eq!(term.cursor().unwrap().row, 1);
}

#[test]
fn clear_erases_the_screen_and_keeps_scrolled_history() {
    let mut term = emulator(20, 3, 10);
    term.feed(b"one\r\ntwo\r\nthree\r\nfour\r\nfive\r\n");
    term.feed(b"\x1b[H\x1b[2Jnew prompt");
    assert_eq!(lines(&term), ["one", "two", "three", "new prompt", "", ""]);
    assert_eq!(term.cursor().unwrap().row, 0);
}

#[test]
fn alternate_screen_restores_primary_and_has_no_history() {
    let mut term = emulator(20, 2, 10);
    term.feed(b"history\r\nshell\r\nprompt");
    let before = lines(&term);
    term.feed(b"\x1b[?1049h\x1b[2J\x1b[Hfull screen");
    assert_eq!(term.line_count(), 2);
    assert_eq!(term.line(0).text(), "full screen");
    term.feed(b"\x1b[?1049l");
    assert_eq!(lines(&term), before);
}

#[test]
fn shrinking_then_growing_reflows_primary_content() {
    let mut term = emulator(12, 3, 10);
    term.feed(b"abcdefghijkl");
    term.resize(TerminalSize::new(6, 3));
    assert_eq!(&lines(&term)[..2], ["abcdef", "ghijkl"]);
    term.resize(TerminalSize::new(20, 3));
    assert_eq!(term.line(0).text(), "abcdefghijkl");
}

#[test]
fn modes_title_bell_and_query_replies_are_forwarded_once() {
    let mut term = emulator(20, 3, 0);
    term.feed(b"\x1b]0;build\x07\x1b[?1h\x1b[?2004h\x1b[?25lab\x1b[6n\x07");
    assert!(term.modes().application_cursor && term.modes().bracketed_paste);
    assert!(term.cursor().is_none());
    assert_eq!(
        term.take_events(),
        [
            TerminalEmulatorEvent::Title("build".into()),
            TerminalEmulatorEvent::Reply(b"\x1b[1;3R".to_vec()),
            TerminalEmulatorEvent::Bell,
        ]
    );
    assert!(term.take_events().is_empty());
    term.feed(b"\x1b[?1l\x1b[?2004l\x1b[?25h");
    assert!(!term.modes().application_cursor && !term.modes().bracketed_paste);
    assert_eq!(term.cursor().unwrap().col, 2);
}

#[test]
fn progress_and_program_status_map_to_backend_events_not_preview_events() {
    let mut term = emulator(20, 3, 0);
    term.feed(b"\x1b]9;4;1;42\x07\x1b]7501;state=working:app=cargo:progress=40\x07");
    let events = term.backend_events();
    assert!(matches!(
        events.as_slice(),
        [
            BackendEvent::Progress(ProgressReport {
                state: ProgressState::Set,
                percent: Some(42),
            }),
            BackendEvent::ProgramStatus(ProgramStatusReport {
                state: ProgramStatusState::Working,
                percent: Some(40),
                app,
                ..
            }),
        ] if app == "cargo"
    ));
    assert!(term.take_events().is_empty());
}

#[test]
fn pwd_prompt_and_mouse_shape_are_mapped_to_backend_events() {
    let mut term = emulator(40, 6, 0);
    term.feed(b"\x1b]7;file://localhost/tmp\x07\x1b]133;A\x07\x1b]22;pointer\x07");
    let events = term.backend_events();
    assert!(events.iter().any(|event| matches!(
        event,
        BackendEvent::WorkingDirectoryChanged(value) if value == "file://localhost/tmp"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        BackendEvent::SemanticPrompt(report)
            if report.kind == SemanticPromptKind::PromptStart && report.cursor_at_prompt
    )));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, BackendEvent::MouseShape(MouseShape::Pointer)))
    );
    term.feed(b"\x1b]22;text\x07");
    assert!(
        term.backend_events()
            .iter()
            .any(|event| matches!(event, BackendEvent::MouseShape(MouseShape::Text)))
    );
}

#[test]
fn palette_changes_and_resets_are_reflected_in_existing_cells() {
    let mut term = emulator(10, 2, 0);
    term.feed(b"\x1b[31mA");
    assert_eq!(
        term.line(0).runs[0].style.fg,
        Some(TerminalColor::Indexed(1))
    );
    term.feed(b"\x1b]4;1;rgb:12/34/56\x07");
    assert_eq!(
        term.line(0).runs[0].style.fg,
        Some(TerminalColor::Rgb(0x12, 0x34, 0x56))
    );
    term.feed(b"\x1b]104;1\x07");
    assert_eq!(
        term.line(0).runs[0].style.fg,
        Some(TerminalColor::Indexed(1))
    );
}

#[test]
fn independent_emulators_and_reset_do_not_leak_output() {
    let mut first = emulator(20, 3, 10);
    let mut second = emulator(20, 3, 10);
    first.feed(b"first\r\noutput");
    second.feed(b"second");
    first.feed(b"\x1bc");
    assert_eq!(lines(&first), ["", "", ""]);
    assert_eq!(second.line(0).text(), "second");
    first.resize(TerminalSize::new(1, 1));
    first.feed(b"ab");
    assert!(first.line_count() >= 1);
}

#[test]
fn synchronized_output_is_atomic_and_has_a_timeout() {
    let mut term = emulator(20, 3, 0);
    term.feed(b"A\x1b[?2026hB");
    assert_eq!(term.line(0).text(), "A");
    let deadline = term.sync_deadline().unwrap();
    assert!(term.flush_sync(deadline + std::time::Duration::from_nanos(1), false));
    assert_eq!(term.line(0).text(), "AB");
    term.feed(b"\x1b[?2026hC\x1b[?2026l");
    assert_eq!(term.line(0).text(), "ABC");
    assert!(!term.flush_sync(deadline, true));
}

#[test]
fn synchronized_image_replacement_keeps_the_previous_pixels_until_commit() {
    let mut term = GhosttyEmulator::new(TerminalSize::new(80, 24), 100);
    term.resize_cells(TerminalSize::new(80, 24), 8, 16);
    let mut window = crate::session_view::window::Window::default();
    term.feed(b"\x1b_Ga=T,f=32,s=1,v=1,c=2,r=2,i=49374;/wAA/w==\x1b\\");
    let mut previous = term
        .begin_window(&mut window, false, None)
        .unwrap()
        .finish();
    assert_eq!(previous.images.len(), 1);
    for n in 0..20 {
        term.feed(b"\x1b[?2026h");
        term.feed(b"\x1b_Ga=d,d=I,i=49374,q=2;\x1b\\");
        let held = term
            .begin_window(&mut window, false, Some(&previous))
            .unwrap()
            .finish();
        assert_eq!(held.images.len(), 1, "held replacement {n}");
        assert_eq!(
            &*held.images[0].pixels.rgba,
            &*previous.images[0].pixels.rgba
        );
        let payload = if n % 2 == 0 { "AP8A/w==" } else { "/wAA/w==" };
        term.feed(
            format!("\x1b[H\x1b_Ga=T,f=32,s=1,v=1,c=2,r=2,i=49374;{payload}\x1b\\\x1b[?2026l")
                .as_bytes(),
        );
        previous = term
            .begin_window(&mut window, false, Some(&held))
            .unwrap()
            .finish();
        assert_eq!(previous.images.len(), 1, "committed replacement {n}");
        let expected = if n % 2 == 0 {
            [0, 255, 0, 255]
        } else {
            [255, 0, 0, 255]
        };
        assert_eq!(
            &*previous.images[0].pixels.rgba, &expected,
            "replacement {n}"
        );
    }
}

#[test]
fn active_chunked_image_transfer_survives_the_initial_sync_deadline() {
    use crate::session_view::window::Window;
    let mut term = emulator(20, 3, 0);
    term.resize_cells(TerminalSize::new(20, 3), 8, 16);
    term.feed(b"\x1b_Ga=T,f=32,s=1,v=1,c=1,r=1,i=42;/wAA/w==\x1b\\\x1b[H");
    let mut window = Window::default();
    let previous = term.capture_window(&mut window, false, None).unwrap();
    term.feed(b"\x1b[?2026h\x1b[?25l\x1b_Ga=d,d=I,i=42;\x1b\\");
    let since = term.state.borrow().terminal.render_hold().unwrap().since;
    let original_deadline = term.sync_deadline().unwrap();
    // Like Codex, delete the old image and stream the replacement in Kitty
    // chunks while the cursor is temporarily hidden. The total transfer lasts
    // longer than 150 ms, but every inter-chunk gap is shorter than that.
    for (ms, chunk) in [
        (
            100,
            &b"\x1b_Ga=T,f=32,s=1,v=1,c=1,r=1,i=42,m=1;AP8A\x1b\\"[..],
        ),
        (200, &b"\x1b_Gm=1;/w==\x1b\\"[..]),
    ] {
        let now = since + Duration::from_millis(ms);
        term.feed_at(chunk, now);
        assert!(!term.flush_sync(now, false));
        let held = term
            .capture_window(&mut window, false, Some(&previous))
            .unwrap();
        assert_eq!(&*held.images[0].pixels.rgba, &[255, 0, 0, 255]);
        assert!(held.frame.cursor.visible);
    }
    assert!(term.sync_deadline().unwrap() > original_deadline);
    term.feed_at(
        b"\x1b_Gm=0;\x1b\\\x1b[H\x1b[?25h\x1b[?2026l",
        since + Duration::from_millis(250),
    );
    let next = term
        .capture_window(&mut window, false, Some(&previous))
        .unwrap();
    assert_eq!(&*next.images[0].pixels.rgba, &[0, 255, 0, 255]);
    assert!(next.frame.cursor.visible);
    assert!(term.sync_deadline().is_none());
}

#[test]
fn sync_watchdog_flushes_idle_output_and_bounds_continuous_output() {
    let mut term = emulator(20, 3, 0);
    term.feed(b"\x1b[?2026hA");
    let since = term.state.borrow().terminal.render_hold().unwrap().since;
    term.feed_at(b"B", since + Duration::from_millis(100));
    assert!(!term.flush_sync(since + Duration::from_millis(200), false));
    assert!(term.flush_sync(since + Duration::from_millis(250), false));
    assert_eq!(term.line(0).text(), "AB");

    term.feed(b"\x1b[?2026h");
    let since = term.state.borrow().terminal.render_hold().unwrap().since;
    for ms in (100..=1000).step_by(100) {
        let now = since + Duration::from_millis(ms);
        term.feed_at(b"C", now);
        assert_eq!(term.flush_sync(now, false), ms == 1000);
    }
    assert!(!term.render_held());
    assert!(term.line(0).text().ends_with("CCCCCCCCCC"));
}

#[test]
fn synchronized_snapshot_does_not_consume_another_renderers_dirty_rows() {
    let mut term = emulator(20, 3, 10);
    term.feed(b"old");
    assert_eq!(term.line(0).text(), "old");
    // The hold callback captures `oldA`; no text is written after that capture.
    // A renderer with its own incremental cache still has to pick up A.
    term.feed(b"A\x1b[?2026h\x1b[?2026l");
    assert_eq!(term.line(0).text(), "oldA");
    term.feed(b"\x1b[?2026h\x1b[31mB");
    assert_eq!(term.line(0).text(), "oldA");
    assert!(term.flush_sync(Instant::now(), true));
    assert_eq!(term.line(0).text(), "oldAB");
}

#[test]
fn a_hold_started_after_output_in_the_same_chunk_keeps_history_bounded() {
    let mut term = emulator(20, 2, 2);
    term.feed(b"zero\r\none\r\ntwo\r\nthree\r\nfour\x1b[?2026hheld");
    assert_eq!(term.line_count(), 4);
    assert_eq!(lines(&term), ["one", "two", "three", "four"]);
    assert!(term.flush_sync(Instant::now(), true));
    assert_eq!(lines(&term), ["one", "two", "three", "fourheld"]);
}

#[test]
fn ghostty_encodes_committed_unicode_and_function_keys() {
    let mut term = emulator(80, 24, 100);
    assert_eq!(
        term.encode_text("中文e\u{301}").unwrap(),
        "中文e\u{301}".as_bytes()
    );
    let event = termlevel_ghostty_vt::KeyEvent::press(raw::GHOSTTY_KEY_F5, "");
    assert_eq!(term.encode_key(&event).unwrap(), b"\x1b[15~");
    term.feed(b"\x1b[>8u");
    assert_eq!(term.encode_text("a").unwrap(), b"\x1b[97u");
}

#[test]
fn native_spans_anchor_wide_and_combining_text_to_grid_columns() {
    let mut emu = GhosttyEmulator::new(TerminalSize::new(20, 4), 10);
    emu.feed("ab中文e\u{301}Z".as_bytes());
    let runs = emu.grid_runs(0);
    let spans: Vec<_> = runs
        .iter()
        .map(|r| (r.col, r.cells, r.run.text.as_str()))
        .collect();
    assert_eq!(
        spans,
        [
            (0, 2, "ab"),
            (2, 2, "中"),
            (4, 2, "文"),
            (6, 1, "e\u{301}"),
            (7, 1, "Z")
        ]
    );
    assert_eq!(emu.input_cursor().col, 8);
    emu.feed(b"\x1b[?25l");
    assert!(emu.cursor().is_none());
    assert_eq!(
        emu.input_cursor().col,
        8,
        "IME anchor remains valid with hidden TUI cursor"
    );
}

fn window_text(snap: &WindowSnap, row: usize) -> String {
    snap.rows[row]
        .iter()
        .map(|run| run.run.text.as_str())
        .collect()
}

#[test]
fn grid_window_uses_frame_rows_and_clear_never_reveals_history() {
    use crate::session_view::window::Window;
    let mut term = emulator(20, 3, 10);
    term.feed(b"one\r\ntwo\r\nthree\r\nfour\r\nfive\r\n");
    let mut w = Window::default();
    let before = term.capture_window(&mut w, false, None).unwrap();
    term.feed(b"\x1b[H\x1b[2Jnew prompt");
    let snap = term.capture_window(&mut w, false, Some(&before)).unwrap();
    assert_eq!(snap.origin, snap.count - 3);
    assert_eq!(window_text(&snap, 0), "new prompt");
    assert!(window_text(&snap, 1).is_empty());
    let repeated = term.capture_window(&mut w, false, Some(&snap)).unwrap();
    assert!(Arc::ptr_eq(&snap, &repeated));
}

#[test]
fn grid_history_anchor_follows_reflow_and_survives_alternate_screen() {
    use crate::session_view::window::Window;
    let mut term = emulator(12, 3, 20);
    term.feed(b"abcdefghijkl\r\nTARGET\r\nnext\r\nlast\r\nprompt");
    let mut w = Window::default();
    term.capture_window(&mut w, false, None).unwrap();
    w.follow_user(1);
    let snap = term.capture_window(&mut w, true, None).unwrap();
    assert_eq!(window_text(&snap, 0), "TARGET");
    term.resize(TerminalSize::new(6, 3));
    let snap = term.capture_window(&mut w, false, Some(&snap)).unwrap();
    assert_eq!(window_text(&snap, 0), "TARGET");
    assert!(snap.origin > 1, "preceding soft line gained a physical row");
    term.feed(b"\x1b[?1049hFULL SCREEN");
    let alt = term.capture_window(&mut w, false, Some(&snap)).unwrap();
    assert!(alt.alternate);
    assert_eq!(alt.origin, 0);
    term.feed(b"\x1b[2J\x1b[?1049l");
    let restored = term.capture_window(&mut w, false, Some(&alt)).unwrap();
    assert!(!restored.alternate);
    assert_eq!(window_text(&restored, 0), "TARGET");
    term.resize(TerminalSize::new(20, 3));
    let wide = term.capture_window(&mut w, false, Some(&restored)).unwrap();
    assert_eq!(window_text(&wide, 0), "TARGET");
}

#[test]
fn consecutive_sync_transactions_publish_completed_boundary_without_an_unheld_tick() {
    use crate::session_view::window::Window;
    let mut term = emulator(20, 3, 20);
    term.feed(b"\x1b[?1049hZERO");
    let mut window = Window::default();
    let mut shown = term.capture_window(&mut window, false, None).unwrap();
    term.feed(b"\x1b[?2026h\x1b[HFRAME_000");
    for n in 1..=160 {
        // The reader completes one transaction and starts the next before the
        // display gets its lock. A poll can never observe an unheld terminal.
        term.feed(format!("\x1b[?2026l\x1b[?2026h\x1b[HFRAME_{n:03}").as_bytes());
        assert!(term.render_held());
        let next = term
            .capture_window(&mut window, false, Some(&shown))
            .unwrap();
        assert_eq!(window_text(&next, 0), format!("FRAME_{:03}", n - 1));
        assert!(!Arc::ptr_eq(&next, &shown));
        let repeat = term
            .capture_window(&mut window, false, Some(&next))
            .unwrap();
        assert!(Arc::ptr_eq(&repeat, &next), "same boundary must be cached");
        shown = next;
    }
    term.feed(b"\x1b[?2026l");
    let final_frame = term
        .capture_window(&mut window, false, Some(&shown))
        .unwrap();
    assert_eq!(window_text(&final_frame, 0), "FRAME_160");
}

#[test]
fn held_active_boundary_never_reads_partial_live_screen_or_size() {
    use crate::session_view::window::Window;
    let mut term = emulator(12, 3, 20);
    term.feed(b"READY");
    let mut window = Window::default();
    let before = term.capture_window(&mut window, false, None).unwrap();
    term.feed(b"\x1b[?2026h\x1b[?1049hSECRET");
    let held = term
        .capture_window(&mut window, false, Some(&before))
        .unwrap();
    assert_eq!(window_text(&held, 0), "READY");
    assert!(!held.alternate);
    assert_eq!((held.frame.cols, held.frame.rows), (12, 3));
    term.feed(b" MORE PARTIAL");
    let same = term
        .capture_window(&mut window, false, Some(&held))
        .unwrap();
    assert!(Arc::ptr_eq(&same, &held));
    term.feed(b"\x1b[?2026l\x1b[?2026h\x1b[2JINCOMPLETE");
    let committed = term
        .capture_window(&mut window, false, Some(&same))
        .unwrap();
    assert!(committed.alternate);
    assert!(committed.text.contains("SECRET"));
    assert!(!committed.text.contains("INCOMPLETE"));
}

#[test]
fn held_window_freezes_screen_identity_until_native_resize_commits() {
    use crate::session_view::window::Window;
    let mut term = emulator(12, 3, 20);
    term.feed(b"old0\r\nold1\r\nold2\r\nold3\r\nprompt");
    let mut w = Window::default();
    term.capture_window(&mut w, false, None).unwrap();
    w.follow_user(0);
    let before = term.capture_window(&mut w, true, None).unwrap();
    term.feed(b"\x1b[?2026h\x1b[?1049hSECRET");
    assert!(term.render_held(), "hold survived screen switch");
    let held = term.capture_window(&mut w, false, Some(&before)).unwrap();
    assert!(Arc::ptr_eq(&held, &before));
    assert_eq!(
        (held.frame.cols, held.frame.rows, held.alternate),
        (12, 3, false)
    );
    assert_eq!(w.origin, before.origin);
    let mut initial = Window::default();
    let initial = term.capture_window(&mut initial, false, None).unwrap();
    assert_eq!(
        (initial.frame.cols, initial.frame.rows, initial.alternate),
        (12, 3, false)
    );
    term.resize(TerminalSize::new(30, 8));
    assert!(
        !term.render_held(),
        "libghostty-vt 1.0.1 resize commits the hold"
    );
    let released = term.capture_window(&mut w, false, Some(&held)).unwrap();
    assert_eq!(
        (released.frame.cols, released.frame.rows, released.alternate),
        (30, 8, true)
    );
}

#[test]
fn grid_selection_changes_snapshot_even_when_text_does_not_change() {
    use crate::session_view::window::Window;
    let mut term = emulator(20, 3, 10);
    term.feed(b"select me");
    let mut w = Window::default();
    let before = term.capture_window(&mut w, false, None).unwrap();
    term.select((0, 0), (5, 0), false, false);
    let selected = term.capture_window(&mut w, false, Some(&before)).unwrap();
    assert!(!Arc::ptr_eq(&before.frame, &selected.frame));
    assert!(selected.rows[0].iter().any(|run| run.selected));
}

#[test]
fn grid_image_snapshot_uses_active_frame_and_animation_idle_keeps_identity() {
    use crate::session_view::window::Window;
    let mut term = emulator(10, 3, 10);
    term.resize_cells(TerminalSize::new(10, 3), 8, 16);
    term.feed(b"\x1b_Ga=T,f=32,s=1,v=1,c=1,r=1,i=42;/wAA/w==\x1b\\");
    let mut w = Window::default();
    let before = term.capture_window(&mut w, false, None).unwrap();
    assert_eq!(before.images.len(), 1);
    assert!(Arc::ptr_eq(
        &before.images[0].pixels,
        &before.frame.images[0].pixels
    ));
    assert!(!term.tick_images(1));
    let after = term.capture_window(&mut w, false, Some(&before)).unwrap();
    assert!(Arc::ptr_eq(&before, &after));
}

#[test]
fn window_second_phase_uses_only_owned_data_after_terminal_mutation_and_drop() {
    use crate::session_view::window::Window;
    let mut term = emulator(20, 3, 10);
    term.feed(b"old\x1b[31mRED");
    let mut w = Window::default();
    let capture = term.begin_window(&mut w, false, None).unwrap();
    term.feed(b"\x1b[2J\x1b[HNEW\x1b]4;1;rgb:00/ff/00\x07");
    drop(term);
    let snap = capture.finish();
    assert_eq!(window_text(&snap, 0), "oldRED");
    assert_eq!(
        snap.rows[0][1].run.style.fg,
        Some(TerminalColor::Indexed(1))
    );
    assert!(snap.text.starts_with("oldRED\n"));
}
