//! Conventional xterm input encoding. Text itself belongs to native Input.
use nsh_base::TerminalModes;

#[derive(Clone, Copy)]
pub(super) enum Bytes {
    Fixed(&'static str),
    Cursor(char),
}
impl Bytes {
    pub fn encode(self, modes: TerminalModes) -> String {
        match self {
            Self::Fixed(s) => s.to_owned(),
            Self::Cursor(c) => format!(
                "\x1b{}{c}",
                if modes.application_cursor { 'O' } else { '[' }
            ),
        }
    }
}

pub(super) fn bindings(apple: bool) -> Vec<(String, Bytes)> {
    use Bytes::*;
    let mut out: Vec<_> = [
        ("enter", Fixed("\r")),
        ("backspace", Fixed("\x7f")),
        ("tab", Fixed("\t")),
        ("shift+tab", Fixed("\x1b[Z")),
        ("escape", Fixed("\x1b")),
        ("up", Cursor('A')),
        ("down", Cursor('B')),
        ("right", Cursor('C')),
        ("left", Cursor('D')),
        ("home", Cursor('H')),
        ("end", Cursor('F')),
        ("delete", Fixed("\x1b[3~")),
        ("pageup", Fixed("\x1b[5~")),
        ("pagedown", Fixed("\x1b[6~")),
        ("alt+left", Fixed("\x1bb")),
        ("alt+right", Fixed("\x1bf")),
        ("alt+backspace", Fixed("\x1b\x7f")),
    ]
    .into_iter()
    .map(|(k, b)| (k.to_owned(), b))
    .collect();
    const CTRL: [&str; 26] = [
        "\x01", "\x02", "\x03", "\x04", "\x05", "\x06", "\x07", "\x08", "\x09", "\x0a", "\x0b",
        "\x0c", "\x0d", "\x0e", "\x0f", "\x10", "\x11", "\x12", "\x13", "\x14", "\x15", "\x16",
        "\x17", "\x18", "\x19", "\x1a",
    ];
    for (n, bytes) in CTRL.into_iter().enumerate() {
        if !apple && n == 16 {
            continue;
        }
        out.push((format!("ctrl+{}", (b'a' + n as u8) as char), Fixed(bytes)));
    }
    if apple {
        for (key, bytes) in [
            ("cmd+backspace", "\x15"),
            ("cmd+left", "\x01"),
            ("cmd+right", "\x05"),
        ] {
            out.push((key.to_owned(), Fixed(bytes)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bindings_parse_and_cursor_keys_follow_application_mode() {
        for apple in [true, false] {
            for (key, _) in bindings(apple) {
                nsh_core::keymap::Keystroke::parse_with(&key, apple).unwrap();
            }
        }
        assert_eq!(
            Bytes::Cursor('A').encode(TerminalModes::default()),
            "\x1b[A"
        );
        let mut modes = TerminalModes::default();
        modes.application_cursor = true;
        assert_eq!(Bytes::Cursor('A').encode(modes), "\x1bOA");
    }
}
