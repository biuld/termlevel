use super::*;

#[test]
fn query_colors_match_nsh_palette_and_program_overrides() {
    let mut emulator = GhosttyEmulator::new(TerminalSize::new(20, 3), 0);
    emulator.set_colors(
        Some(Rgb { r: 1, g: 2, b: 3 }),
        Some(Rgb { r: 4, g: 5, b: 6 }),
    );
    emulator.feed(
        b"\x1b]4;1;?\x1b\\\x1b]4;196;?\x1b\\\x1b]4;232;?\x1b\\\x1b]10;?\x1b\\\x1b]11;?\x1b\\",
    );
    let replies: Vec<_> = emulator
        .take_events()
        .into_iter()
        .filter_map(|event| {
            if let TerminalEmulatorEvent::Reply(bytes) = event {
                Some(String::from_utf8(bytes).unwrap())
            } else {
                None
            }
        })
        .collect();
    assert!(
        replies.iter().any(|s| s.contains("4;1;rgb:cdcd/3131/3131")),
        "{replies:?}"
    );
    assert!(
        replies
            .iter()
            .any(|s| s.contains("4;196;rgb:ffff/0000/0000")),
        "{replies:?}"
    );
    assert!(
        replies
            .iter()
            .any(|s| s.contains("4;232;rgb:0808/0808/0808")),
        "{replies:?}"
    );
    assert!(
        replies.iter().any(|s| s.contains("10;rgb:0101/0202/0303")),
        "{replies:?}"
    );
    assert!(
        replies.iter().any(|s| s.contains("11;rgb:0404/0505/0606")),
        "{replies:?}"
    );
    emulator.feed(b"\x1b]4;1;rgb:aa/bb/cc\x1b\\\x1b]4;1;?\x1b\\");
    assert!(emulator.take_events().iter().any(|event| matches!(event,
        TerminalEmulatorEvent::Reply(bytes) if String::from_utf8_lossy(bytes).contains("aaaa/bbbb/cccc"))));
}
