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

#[test]
fn hyperlinks_selection_and_owned_images() -> Result<()> {
    let mut t = Terminal::new(12, 4)?;
    t.resize(12, 4, 8, 16)?;
    t.write(b"\x1b]8;;https://example.com\x1b\\hello\x1b]8;;\x1b\\ world")?;
    assert_eq!(t.hyperlink(0, 0)?.as_deref(), Some("https://example.com"));
    assert_eq!(t.hyperlink(8, 0)?, None);
    t.select((0, 0), (4, 0), false)?;
    assert_eq!(t.selection_text()?.as_deref(), Some("hello"));
    let mut r = RenderState::new()?;
    let frame = r.capture(&mut t)?;
    assert!(frame.lines[0].cells[0].selected);
    assert_eq!(
        frame.lines[0].cells[0].hyperlink.as_deref(),
        Some("https://example.com")
    );
    t.clear_selection()?;
    t.select_word(7, 0)?;
    assert_eq!(t.selection_text()?.as_deref(), Some("world"));
    // A single opaque red pixel, displayed over one cell via inline Kitty APC.
    t.write(b"\x1b_Ga=T,f=32,s=1,v=1,c=1,r=1,i=42;/wAA/w==\x1b\\")?;
    let f = r.capture(&mut t)?;
    assert_eq!(f.images.len(), 1, "{:?}", t.take_events());
    assert_eq!(&*f.images[0].pixels.rgba, &[255, 0, 0, 255]);
    t.write(b"\x1b_Ga=d,d=A\x1b\\")?;
    assert!(r.capture(&mut t)?.images.is_empty());
    assert_eq!(&*f.images[0].pixels.rgba, &[255, 0, 0, 255]);
    Ok(())
}

#[test]
fn osc52_is_synchronous_and_denied_reads_do_not_leak() -> Result<()> {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    };
    #[derive(Default)]
    struct Host {
        text: Mutex<String>,
        allow: AtomicBool,
    }
    impl Clipboard for Host {
        fn read(&self) -> Result<String> {
            if self.allow.load(Ordering::Acquire) {
                Ok(self.text.lock().unwrap().clone())
            } else {
                Err(Error(ffi::GHOSTTY_REJECTED))
            }
        }
        fn write(&self, text: &str) -> Result<()> {
            *self.text.lock().unwrap() = text.to_owned();
            Ok(())
        }
    }
    let host = Arc::new(Host::default());
    let mut t = Terminal::new(20, 3)?;
    t.set_clipboard(host.clone())?;
    for b in b"\x1b]52;c;aGVsbG8=\x07" {
        t.write(&[*b])?;
    }
    assert_eq!(*host.text.lock().unwrap(), "hello");
    t.write(b"\x1b]52;c;?\x07")?;
    assert!(
        !t.take_events()
            .iter()
            .any(|e| matches!(e,Event::WritePty(bytes) if bytes.windows(8).any(|s|s==b"aGVsbG8=")))
    );
    host.allow.store(true, Ordering::Release);
    t.write(b"\x1b]52;c;?\x07")?;
    assert!(
        t.take_events()
            .iter()
            .any(|e| matches!(e,Event::WritePty(bytes) if bytes.windows(8).any(|s|s==b"aGVsbG8=")))
    );
    t.write(b"\x1b]52;c;\x07")?;
    assert_eq!(*host.text.lock().unwrap(), "");
    Ok(())
}

#[test]
fn png_images_and_history_placements_survive_viewport_capture() -> Result<()> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[255, 0, 0, 255, 0, 255, 0, 128])
            .unwrap();
    }
    let mut t = Terminal::new(10, 3)?;
    t.resize(10, 3, 8, 16)?;
    t.write(
        format!(
            "\x1b_Ga=T,f=100,c=2,r=1,i=99;{}\x1b\\",
            STANDARD.encode(png)
        )
        .as_bytes(),
    )?;
    let mut r = RenderState::new()?;
    let f = r.capture(&mut t)?;
    assert_eq!(&*f.images[0].pixels.rgba, &[255, 0, 0, 255, 0, 255, 0, 128]);
    t.write(b"\r\n1\r\n2\r\n3\r\n4")?;
    assert!(r.capture(&mut t)?.images.is_empty());
    let history = r.images_at(&mut t, 0)?;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].row, 0);
    assert!(r.capture(&mut t)?.images.is_empty());
    Ok(())
}
