use termlevel_ghostty_vt::{raw as ffi, *};

#[test]
fn terminal_and_renderer_can_move_between_serialized_threads() -> Result<()> {
    use std::sync::{Arc, Mutex};
    let terminal = Arc::new(Mutex::new(Terminal::new(80, 24)?));
    let writer = terminal.clone();
    std::thread::spawn(move || {
        writer
            .lock()
            .unwrap()
            .write("worker 中文".as_bytes())
            .unwrap()
    })
    .join()
    .unwrap();
    let mut render = RenderState::new()?;
    let frame = render.capture(&mut terminal.lock().unwrap())?;
    let reader = terminal.clone();
    std::thread::spawn(move || {
        let mut terminal = reader.lock().unwrap();
        terminal.resize(90, 30, 8, 16).unwrap();
        assert!(terminal.text().unwrap().contains("worker 中文"));
        render.capture(&mut terminal).unwrap();
    })
    .join()
    .unwrap();
    assert_eq!(frame.cols, 80);
    Ok(())
}

#[test]
fn pinned_version_split_utf8_and_split_escape_sequences() -> Result<()> {
    assert_eq!(library_version(), "1.0.1");
    let mut terminal = Terminal::new(80, 24)?;
    // Feed every byte separately, as a PTY reader is allowed to do.
    for byte in "hello 中文 e\u{301} \x1b[31mred\x1b[0m".as_bytes() {
        terminal.write(&[*byte])?;
    }
    assert_eq!(terminal.text()?, "hello 中文 e\u{301} red");
    assert_eq!(terminal.size()?, (80, 24));
    Ok(())
}

#[test]
fn replies_and_metadata_are_owned_after_moves_and_drop() -> Result<()> {
    let terminal = Terminal::new(20, 4)?;
    // Move the Rust object: userdata must still point at the same allocation.
    let mut moved = vec![terminal].pop().unwrap();
    moved.write(b"\x1b]2;first\x07\x07\x1b]7;file://localhost/tmp\x1b\\")?;
    moved.write(b"abc\x1b[6n")?;
    let events = moved.take_events();
    moved.write(b"\x1b]2;second\x07")?;
    drop(moved);
    assert!(
        events.contains(&Event::TitleChanged("first".into())),
        "{events:?}"
    );
    assert!(
        events.contains(&Event::WorkingDirectoryChanged(
            "file://localhost/tmp".into()
        )),
        "{events:?}"
    );
    assert!(events.contains(&Event::Bell));
    assert!(
        events.contains(&Event::WritePty(b"\x1b[1;4R".to_vec())),
        "{events:?}"
    );
    Ok(())
}

#[test]
fn resize_alternate_screen_reset_and_independent_terminals() -> Result<()> {
    let mut first = Terminal::new(8, 3)?;
    let mut second = Terminal::new(8, 3)?;
    first.write(b"primary")?;
    second.write(b"other")?;
    first.write(b"\x1b[?1049hsecondary")?;
    assert!(!first.text()?.contains("primary"));
    first.write(b"\x1b[?1049l")?;
    assert!(first.text()?.contains("primary"));
    first.resize(12, 5, 8, 16)?;
    assert_eq!(first.size()?, (12, 5));
    assert!(first.text()?.contains("primary"));
    first.reset()?;
    assert_eq!(first.text()?, "");
    first.write(b"\x1bc")?;
    assert!(first.take_events().contains(&Event::Reset));
    assert_eq!(second.text()?, "other");
    Ok(())
}

#[test]
fn text_includes_history_and_resize_reflows_soft_wrapped_lines() -> Result<()> {
    let mut terminal = Terminal::new(4, 2)?;
    terminal.write(b"abcdef")?;
    assert_eq!(terminal.text()?, "abcd\nef");
    terminal.resize(8, 2, 8, 16)?;
    assert_eq!(terminal.text()?, "abcdef");
    terminal.write(b"\r\nnew\r\nlast")?;
    let text = terminal.text()?;
    assert!(text.contains("abcdef"), "{text:?}");
    assert!(text.contains("last"), "{text:?}");
    Ok(())
}

#[test]
fn frame_preserves_graphemes_width_truecolor_underlines_and_cursor() -> Result<()> {
    let mut terminal = Terminal::new(20, 3)?;
    terminal.write("\x1b[38;2;12;34;56;4:3m中e\u{301}\x1b[0m\x1b[6 q".as_bytes())?;
    let mut render = RenderState::new()?;
    let frame = render.capture(&mut terminal)?;
    let line = &frame.lines[0];
    assert_eq!(line.cells[0].text, "中");
    assert_eq!(line.cells[0].wide, ffi::GHOSTTY_CELL_WIDE_WIDE);
    assert_eq!(line.cells[1].wide, ffi::GHOSTTY_CELL_WIDE_SPACER_TAIL);
    assert_eq!(line.cells[2].text, "e\u{301}");
    assert_eq!(
        line.cells[0].style.foreground,
        StyleColor::Rgb(Color {
            r: 12,
            g: 34,
            b: 56
        })
    );
    assert_eq!(
        (line.cells[0].style.flags & ffi::GHOSTTY_STYLE_UNDERLINE_MASK)
            >> ffi::GHOSTTY_STYLE_UNDERLINE_SHIFT,
        ffi::GHOSTTY_SGR_UNDERLINE_CURLY as u16
    );
    assert_eq!(frame.cursor.position, Some((3, 0)));
    assert_eq!(frame.cursor.style, ffi::GHOSTTY_CURSOR_STYLE_BAR);
    assert_eq!(frame.lines.len(), 3);
    assert_eq!(line.cells.len(), 20);
    assert!(line.dirty);
    let clean = render.capture(&mut terminal)?;
    assert_eq!(clean.dirty, ffi::GHOSTTY_RENDER_STATE_DIRTY_FALSE);
    terminal.resize(30, 5, 8, 16)?;
    drop(terminal);
    drop(render);
    assert_eq!(frame.lines[0].cells[0].text, "中");
    // Owned frames may be sent to a renderer thread even though handles may not.
    std::thread::spawn(move || assert_eq!(frame.cols, 20))
        .join()
        .unwrap();
    Ok(())
}

#[test]
fn render_state_switches_terminal_without_stale_rows() -> Result<()> {
    let mut first = Terminal::new(10, 2)?;
    let mut second = Terminal::new(10, 2)?;
    first.write(b"one")?;
    second.write(b"two")?;
    let mut render = RenderState::new()?;
    assert_eq!(render.capture(&mut first)?.lines[0].cells[0].text, "o");
    assert_eq!(render.capture(&mut second)?.lines[0].cells[0].text, "t");
    assert_eq!(render.capture(&mut first)?.lines[0].cells[0].text, "o");
    Ok(())
}

#[test]
fn keys_follow_legacy_application_and_kitty_modes() -> Result<()> {
    let mut terminal = Terminal::new(80, 24)?;
    let up = KeyEvent::press(ffi::GHOSTTY_KEY_ARROW_UP, "");
    assert_eq!(terminal.encode_key(&up, false)?, b"\x1b[A");
    terminal.write(b"\x1b[?1h")?;
    assert_eq!(terminal.encode_key(&up, false)?, b"\x1bOA");
    let mut ctrl_c = KeyEvent::press(ffi::GHOSTTY_KEY_C, "c");
    ctrl_c.unshifted_codepoint = Some('c');
    ctrl_c.modifiers = ffi::GHOSTTY_MODS_CTRL as u16;
    assert_eq!(terminal.encode_key(&ctrl_c, false)?, b"\x03");
    terminal.write(b"\x1b[>1u")?;
    assert_eq!(terminal.encode_key(&ctrl_c, false)?, b"\x1b[99;5u");
    // Encoding returns bytes and never writes them back into the output parser.
    assert_eq!(terminal.text()?, "");
    Ok(())
}

#[test]
fn mouse_focus_and_paste_respect_negotiated_modes() -> Result<()> {
    let mut terminal = Terminal::new(80, 24)?;
    let click = MouseEvent {
        action: ffi::GHOSTTY_MOUSE_ACTION_PRESS,
        button: Some(ffi::GHOSTTY_MOUSE_BUTTON_LEFT),
        modifiers: 0,
        x: 16.0,
        y: 16.0,
        any_button_pressed: true,
    };
    assert!(terminal.encode_mouse(click, 8, 16)?.is_empty());
    assert!(terminal.encode_focus(true)?.is_empty());
    assert_eq!(terminal.encode_paste("a\nb")?, b"a\rb");
    terminal.write(b"\x1b[?1000h\x1b[?1006h\x1b[?1004h\x1b[?2004h")?;
    assert_eq!(terminal.encode_mouse(click, 8, 16)?, b"\x1b[<0;3;2M");
    assert_eq!(terminal.encode_focus(true)?, b"\x1b[I");
    assert_eq!(terminal.encode_focus(false)?, b"\x1b[O");
    assert_eq!(terminal.encode_paste("a\nb")?, b"\x1b[200~a\nb\x1b[201~");
    Ok(())
}

#[test]
fn invalid_dimensions_keys_and_coordinates_are_errors() -> Result<()> {
    assert!(Terminal::new(0, 24).is_err());
    let mut terminal = Terminal::new(80, 24)?;
    assert!(terminal.resize(0, 24, 8, 16).is_err());
    assert_eq!(terminal.size()?, (80, 24));
    assert!(
        terminal
            .encode_key(&KeyEvent::press(u32::MAX, ""), false)
            .is_err()
    );
    assert!(
        terminal
            .encode_key(&KeyEvent::press(ffi::GHOSTTY_KEY_C, "\x03"), false)
            .is_err()
    );
    assert!(terminal.dec_mode(32768).is_err());
    let mouse = MouseEvent {
        action: ffi::GHOSTTY_MOUSE_ACTION_PRESS,
        button: None,
        modifiers: 0,
        x: f32::NAN,
        y: 0.0,
        any_button_pressed: false,
    };
    assert!(terminal.encode_mouse(mouse, 8, 16).is_err());
    Ok(())
}
