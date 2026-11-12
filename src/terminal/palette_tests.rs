use super::*;

#[test]
fn query_colors_match_nsh_palette_and_program_overrides() {
    let mut emulator = AlacrittyEmulator::new(TerminalSize::new(20, 3), 0);
    let foreground = Rgb { r: 1, g: 2, b: 3 };
    let background = Rgb { r: 4, g: 5, b: 6 };
    assert_eq!(
        emulator.query_color(1, foreground, background),
        Rgb {
            r: 0xcd,
            g: 0x31,
            b: 0x31
        }
    );
    assert_eq!(
        emulator.query_color(196, foreground, background),
        Rgb { r: 255, g: 0, b: 0 }
    );
    assert_eq!(
        emulator.query_color(232, foreground, background),
        Rgb { r: 8, g: 8, b: 8 }
    );
    assert_eq!(
        emulator.query_color(NamedColor::Foreground as usize, foreground, background),
        foreground
    );
    assert_eq!(
        emulator.query_color(NamedColor::Background as usize, foreground, background),
        background
    );
    emulator.feed(b"\x1b]4;1;rgb:aa/bb/cc\x07");
    assert_eq!(
        emulator.query_color(1, foreground, background),
        Rgb {
            r: 0xaa,
            g: 0xbb,
            b: 0xcc
        }
    );
}
