use super::*;

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
