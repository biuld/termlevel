//! Map NSH key bindings to Ghostty key events; no escape-sequence table.
use termlevel_ghostty_vt::{KeyEvent, raw};

#[derive(Clone, Copy)]
pub(super) struct KeySpec {
    key: raw::GhosttyKey,
    modifiers: u16,
    text: &'static str,
    character: Option<char>,
}
impl KeySpec {
    pub fn event(self) -> KeyEvent<'static> {
        let mut event = KeyEvent::press(self.key, self.text);
        event.modifiers = self.modifiers;
        event.unshifted_codepoint = self.character;
        event
    }
}
pub(super) fn bindings(apple: bool) -> Vec<(String, KeySpec)> {
    let special = [
        ("enter", raw::GHOSTTY_KEY_ENTER),
        ("backspace", raw::GHOSTTY_KEY_BACKSPACE),
        ("tab", raw::GHOSTTY_KEY_TAB),
        ("escape", raw::GHOSTTY_KEY_ESCAPE),
        ("up", raw::GHOSTTY_KEY_ARROW_UP),
        ("down", raw::GHOSTTY_KEY_ARROW_DOWN),
        ("left", raw::GHOSTTY_KEY_ARROW_LEFT),
        ("right", raw::GHOSTTY_KEY_ARROW_RIGHT),
        ("home", raw::GHOSTTY_KEY_HOME),
        ("end", raw::GHOSTTY_KEY_END),
        ("delete", raw::GHOSTTY_KEY_DELETE),
        ("insert", raw::GHOSTTY_KEY_INSERT),
        ("pageup", raw::GHOSTTY_KEY_PAGE_UP),
        ("pagedown", raw::GHOSTTY_KEY_PAGE_DOWN),
        ("f1", raw::GHOSTTY_KEY_F1),
        ("f2", raw::GHOSTTY_KEY_F2),
        ("f3", raw::GHOSTTY_KEY_F3),
        ("f4", raw::GHOSTTY_KEY_F4),
        ("f5", raw::GHOSTTY_KEY_F5),
        ("f6", raw::GHOSTTY_KEY_F6),
        ("f7", raw::GHOSTTY_KEY_F7),
        ("f8", raw::GHOSTTY_KEY_F8),
        ("f9", raw::GHOSTTY_KEY_F9),
        ("f10", raw::GHOSTTY_KEY_F10),
        ("f11", raw::GHOSTTY_KEY_F11),
        ("f12", raw::GHOSTTY_KEY_F12),
        ("f13", raw::GHOSTTY_KEY_F13),
        ("f14", raw::GHOSTTY_KEY_F14),
        ("f15", raw::GHOSTTY_KEY_F15),
        ("f16", raw::GHOSTTY_KEY_F16),
        ("f17", raw::GHOSTTY_KEY_F17),
        ("f18", raw::GHOSTTY_KEY_F18),
        ("f19", raw::GHOSTTY_KEY_F19),
        ("f20", raw::GHOSTTY_KEY_F20),
        ("f21", raw::GHOSTTY_KEY_F21),
        ("f22", raw::GHOSTTY_KEY_F22),
        ("f23", raw::GHOSTTY_KEY_F23),
        ("f24", raw::GHOSTTY_KEY_F24),
    ];
    let mut out = Vec::new();
    for (name, key) in special {
        for mask in 0u16..8 {
            let mut binding = String::new();
            let mut modifiers = 0;
            for (bit, label, flag) in [
                (1, "shift+", raw::GHOSTTY_MODS_SHIFT),
                (2, "ctrl+", raw::GHOSTTY_MODS_CTRL),
                (4, "alt+", raw::GHOSTTY_MODS_ALT),
            ] {
                if mask & bit != 0 {
                    binding.push_str(label);
                    modifiers |= flag as u16;
                }
            }
            binding.push_str(name);
            out.push((
                binding,
                KeySpec {
                    key,
                    modifiers,
                    text: "",
                    character: None,
                },
            ));
        }
    }
    let letters = [
        ("a", raw::GHOSTTY_KEY_A),
        ("b", raw::GHOSTTY_KEY_B),
        ("c", raw::GHOSTTY_KEY_C),
        ("d", raw::GHOSTTY_KEY_D),
        ("e", raw::GHOSTTY_KEY_E),
        ("f", raw::GHOSTTY_KEY_F),
        ("g", raw::GHOSTTY_KEY_G),
        ("h", raw::GHOSTTY_KEY_H),
        ("i", raw::GHOSTTY_KEY_I),
        ("j", raw::GHOSTTY_KEY_J),
        ("k", raw::GHOSTTY_KEY_K),
        ("l", raw::GHOSTTY_KEY_L),
        ("m", raw::GHOSTTY_KEY_M),
        ("n", raw::GHOSTTY_KEY_N),
        ("o", raw::GHOSTTY_KEY_O),
        ("p", raw::GHOSTTY_KEY_P),
        ("q", raw::GHOSTTY_KEY_Q),
        ("r", raw::GHOSTTY_KEY_R),
        ("s", raw::GHOSTTY_KEY_S),
        ("t", raw::GHOSTTY_KEY_T),
        ("u", raw::GHOSTTY_KEY_U),
        ("v", raw::GHOSTTY_KEY_V),
        ("w", raw::GHOSTTY_KEY_W),
        ("x", raw::GHOSTTY_KEY_X),
        ("y", raw::GHOSTTY_KEY_Y),
        ("z", raw::GHOSTTY_KEY_Z),
    ];
    for (text, key) in letters {
        if !apple && text == "q" {
            continue;
        }
        out.push((
            format!("ctrl+{text}"),
            KeySpec {
                key,
                modifiers: raw::GHOSTTY_MODS_CTRL as u16,
                text,
                character: text.chars().next(),
            },
        ));
    }
    if apple {
        for (binding, key, text) in [
            ("cmd+backspace", raw::GHOSTTY_KEY_U, "u"),
            ("cmd+left", raw::GHOSTTY_KEY_A, "a"),
            ("cmd+right", raw::GHOSTTY_KEY_E, "e"),
        ] {
            out.push((
                binding.into(),
                KeySpec {
                    key,
                    modifiers: raw::GHOSTTY_MODS_CTRL as u16,
                    text,
                    character: text.chars().next(),
                },
            ));
        }
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::GhosttyEmulator;
    use nsh_base::{TerminalEmulator, TerminalSize};
    #[test]
    fn bindings_parse_and_cursor_keys_follow_application_mode() {
        for apple in [true, false] {
            for (binding, _) in bindings(apple) {
                nsh_core::keymap::Keystroke::parse_with(&binding, apple).unwrap();
            }
        }
        let event = bindings(true)
            .into_iter()
            .find(|(key, _)| key == "up")
            .unwrap()
            .1
            .event();
        let mut terminal = GhosttyEmulator::new(TerminalSize::new(80, 24), 100);
        assert_eq!(terminal.encode_key(&event).unwrap(), b"\x1b[A");
        terminal.feed(b"\x1b[?1h");
        assert_eq!(terminal.encode_key(&event).unwrap(), b"\x1bOA");
    }
}
