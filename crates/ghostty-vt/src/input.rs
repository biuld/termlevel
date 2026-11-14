use super::{Error, Result, Terminal, buffer, check, handle, ptr, sys};

handle!(
    KeyEncoder,
    sys::GhosttyKeyEncoder,
    sys::ghostty_key_encoder_new,
    sys::ghostty_key_encoder_free
);
handle!(
    MouseEncoder,
    sys::GhosttyMouseEncoder,
    sys::ghostty_mouse_encoder_new,
    sys::ghostty_mouse_encoder_free
);
handle!(
    KeyHandle,
    sys::GhosttyKeyEvent,
    sys::ghostty_key_event_new,
    sys::ghostty_key_event_free
);
handle!(
    MouseHandle,
    sys::GhosttyMouseEvent,
    sys::ghostty_mouse_event_new,
    sys::ghostty_mouse_event_free
);

/// An input event. Numeric key/action/modifier values are the `GHOSTTY_KEY_*`,
/// `GHOSTTY_KEY_ACTION_*` and `GHOSTTY_MODS_*` constants from [`crate::raw`].
/// Unknown keys/actions return an error, never become unchecked Rust enums.
#[derive(Debug, Clone)]
pub struct KeyEvent<'a> {
    pub key: sys::GhosttyKey,
    pub action: sys::GhosttyKeyAction,
    pub modifiers: u16,
    pub consumed_modifiers: u16,
    /// Layout-produced text before Ctrl/Meta transforms. Empty for function keys.
    pub text: &'a str,
    /// The platform's base character for this key, needed for Kitty encoding.
    /// Supply it for character keys; shifted text alone cannot reconstruct it.
    pub unshifted_codepoint: Option<char>,
    pub composing: bool,
}
impl<'a> KeyEvent<'a> {
    pub fn press(key: sys::GhosttyKey, text: &'a str) -> Self {
        Self {
            key,
            action: sys::GHOSTTY_KEY_ACTION_PRESS,
            modifiers: 0,
            consumed_modifiers: 0,
            text,
            unshifted_codepoint: None,
            composing: false,
        }
    }
}

/// Coordinates are pixels relative to the unpadded terminal content origin.
#[derive(Debug, Clone, Copy)]
pub struct MouseEvent {
    pub action: sys::GhosttyMouseAction,
    pub button: Option<sys::GhosttyMouseButton>,
    pub modifiers: u16,
    pub x: f32,
    pub y: f32,
    pub any_button_pressed: bool,
}

impl Terminal {
    /// Encode for the terminal's current legacy/Kitty mode without sending it.
    /// `option_as_alt` is an explicit host preference, not terminal state.
    pub fn encode_key(&mut self, event: &KeyEvent<'_>, option_as_alt: bool) -> Result<Vec<u8>> {
        if event
            .text
            .chars()
            .any(|c| c <= '\u{1f}' || c == '\u{7f}' || ('\u{f700}'..='\u{f8ff}').contains(&c))
        {
            return Err(Error(sys::GHOSTTY_INVALID_VALUE));
        }
        let native = KeyHandle::new()?;
        // SAFETY: live owned handles; the event's text is borrowed only until
        // native is dropped, before this call returns. Setters validate values.
        unsafe {
            check(sys::ghostty_key_event_set_key(native.0, event.key))?;
            check(sys::ghostty_key_event_set_action(native.0, event.action))?;
            check(sys::ghostty_key_event_set_mods(native.0, event.modifiers))?;
            check(sys::ghostty_key_event_set_consumed_mods(
                native.0,
                event.consumed_modifiers,
            ))?;
            check(sys::ghostty_key_event_set_composing(
                native.0,
                event.composing,
            ))?;
            check(sys::ghostty_key_event_set_unshifted_codepoint(
                native.0,
                event.unshifted_codepoint.map_or(0, |c| c as u32),
            ))?;
            check(sys::ghostty_key_event_set_utf8(
                native.0,
                sys::GhosttyString {
                    ptr: event.text.as_ptr(),
                    len: event.text.len(),
                },
            ))?;
            check(sys::ghostty_key_encoder_set_from_terminal(
                self.key_encoder.0,
                self.handle.0,
            ))?;
            let option = if option_as_alt {
                sys::GHOSTTY_OPTION_AS_ALT_TRUE
            } else {
                sys::GHOSTTY_OPTION_AS_ALT_FALSE
            };
            check(sys::ghostty_key_encoder_set(
                self.key_encoder.0,
                sys::GHOSTTY_KEY_ENCODER_OPT_MACOS_OPTION_AS_ALT,
                (&option as *const sys::GhosttyOptionAsAlt).cast(),
            ))?;
        }
        // SAFETY: size query and encode use the same unmodified event and encoder.
        buffer(|data, len, written| unsafe {
            sys::ghostty_key_encoder_encode(self.key_encoder.0, native.0, data, len, written)
        })
    }

    /// Encode a mouse event for the currently negotiated protocol. The caller
    /// supplies actual cell pixel dimensions; unknown dimensions are rejected.
    /// Motion deduplication is disabled in this first wrapper.
    pub fn encode_mouse(
        &mut self,
        event: MouseEvent,
        cell_width: u32,
        cell_height: u32,
    ) -> Result<Vec<u8>> {
        if !event.x.is_finite()
            || !event.y.is_finite()
            || event.x < 0.0
            || event.y < 0.0
            || cell_width == 0
            || cell_height == 0
        {
            return Err(Error(sys::GHOSTTY_INVALID_VALUE));
        }
        let (cols, rows) = self.size()?;
        let size = sys::GhosttyMouseEncoderSize {
            size: size_of::<sys::GhosttyMouseEncoderSize>(),
            screen_width: u32::from(cols)
                .checked_mul(cell_width)
                .ok_or(Error(sys::GHOSTTY_INVALID_VALUE))?,
            screen_height: u32::from(rows)
                .checked_mul(cell_height)
                .ok_or(Error(sys::GHOSTTY_INVALID_VALUE))?,
            cell_width,
            cell_height,
            ..Default::default()
        };
        let native = MouseHandle::new()?;
        // SAFETY: matching option types and valid owned handles. C validates enum values.
        unsafe {
            check(sys::ghostty_mouse_event_set_action(native.0, event.action))?;
            if let Some(button) = event.button {
                check(sys::ghostty_mouse_event_set_button(native.0, button))?;
            }
            check(sys::ghostty_mouse_event_set_mods(native.0, event.modifiers))?;
            check(sys::ghostty_mouse_event_set_position(
                native.0,
                sys::GhosttyMousePosition {
                    x: event.x,
                    y: event.y,
                },
            ))?;
            check(sys::ghostty_mouse_encoder_set_from_terminal(
                self.mouse_encoder.0,
                self.handle.0,
            ))?;
            check(sys::ghostty_mouse_encoder_set(
                self.mouse_encoder.0,
                sys::GHOSTTY_MOUSE_ENCODER_OPT_SIZE,
                (&size as *const sys::GhosttyMouseEncoderSize).cast(),
            ))?;
            check(sys::ghostty_mouse_encoder_set(
                self.mouse_encoder.0,
                sys::GHOSTTY_MOUSE_ENCODER_OPT_ANY_BUTTON_PRESSED,
                (&event.any_button_pressed as *const bool).cast(),
            ))?;
        }
        // Encode once: no size-query call that could consume motion state.
        let mut output = [0u8; 128];
        let mut written = 0;
        // SAFETY: output is a writable buffer of the advertised capacity.
        check(unsafe {
            sys::ghostty_mouse_encoder_encode(
                self.mouse_encoder.0,
                native.0,
                output.as_mut_ptr(),
                output.len(),
                &mut written,
            )
        })?;
        output
            .get(..written)
            .map(<[u8]>::to_vec)
            .ok_or(Error(sys::GHOSTTY_INVALID_VALUE))
    }

    /// Check a DEC private mode by number, e.g. 1004 (focus), 2004 (paste).
    pub fn dec_mode(&self, number: u16) -> Result<bool> {
        if number > 32767 {
            return Err(Error(sys::GHOSTTY_INVALID_VALUE));
        }
        let mut config = sys::GhosttyTerminalModeConfig {
            mode: sys::ghostty_mode_new(number, false),
            value: false,
        };
        // SAFETY: MODE uses this exact initialized in/out struct.
        check(unsafe {
            sys::ghostty_terminal_get(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_DATA_MODE,
                (&mut config as *mut sys::GhosttyTerminalModeConfig).cast(),
            )
        })?;
        Ok(config.value)
    }

    /// Return nothing unless the program requested focus reports (mode 1004).
    pub fn encode_focus(&self, focused: bool) -> Result<Vec<u8>> {
        if !self.dec_mode(1004)? {
            return Ok(Vec::new());
        }
        let event = if focused {
            sys::GHOSTTY_FOCUS_GAINED
        } else {
            sys::GHOSTTY_FOCUS_LOST
        };
        // SAFETY: valid event and buffer storage supplied by buffer().
        buffer(|data, len, written| unsafe { sys::ghostty_focus_encode(event, data, len, written) })
    }

    /// Sanitize and frame a plain-text paste according to mode 2004. This does
    /// not access the system clipboard or implement Kitty clipboard events.
    pub fn encode_paste(&self, text: &str) -> Result<Vec<u8>> {
        let bracketed = self.dec_mode(2004)?;
        let mut input = text.as_bytes().to_vec();
        // SAFETY: upstream documents in-place sanitization as idempotent, so
        // repeating it after the size query is valid. No pointers are retained.
        buffer(|data, len, written| unsafe {
            sys::ghostty_paste_encode(
                input.as_mut_ptr().cast(),
                input.len(),
                bracketed,
                data,
                len,
                written,
            )
        })
    }
}
