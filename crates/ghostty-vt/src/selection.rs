use super::{Error, Result, Terminal, buffer, check, ptr, sys};

fn point(column: u16, row: usize, tag: sys::GhosttyPointTag) -> Result<sys::GhosttyPoint> {
    Ok(sys::GhosttyPoint {
        tag,
        value: sys::GhosttyPointValue {
            coordinate: sys::GhosttyPointCoordinate {
                x: column,
                y: u32::try_from(row).map_err(|_| Error(sys::GHOSTTY_INVALID_VALUE))?,
            },
        },
    })
}

pub(super) unsafe fn link_at(
    terminal: sys::GhosttyTerminal,
    column: u16,
    row: usize,
    tag: sys::GhosttyPointTag,
) -> Result<Option<String>> {
    let mut reference = sys::GhosttyGridRef::default();
    // SAFETY: caller serializes access; ref is not used across a mutation.
    check(unsafe {
        sys::ghostty_terminal_grid_ref(terminal, point(column, row, tag)?, &mut reference)
    })?;
    let bytes = buffer(|data, len, written| unsafe {
        sys::ghostty_grid_ref_hyperlink_uri(&reference, data, len, written)
    })?;
    if bytes.is_empty() {
        Ok(None)
    } else {
        Ok(Some(
            String::from_utf8(bytes).map_err(|_| Error(sys::GHOSTTY_INVALID_VALUE))?,
        ))
    }
}

impl Terminal {
    pub fn hyperlink(&mut self, column: u16, row: usize) -> Result<Option<String>> {
        // SAFETY: exclusively borrowed terminal, no native reference escapes.
        unsafe { link_at(self.handle.0, column, row, sys::GHOSTTY_POINT_TAG_SCREEN) }
    }
    pub fn select(
        &mut self,
        start: (u16, usize),
        end: (u16, usize),
        rectangle: bool,
    ) -> Result<()> {
        let mut selection = sys::GhosttySelection {
            rectangle,
            ..Default::default()
        };
        unsafe {
            // SAFETY: both refs resolved and copied into tracked selection before mutation.
            check(sys::ghostty_terminal_grid_ref(
                self.handle.0,
                point(start.0, start.1, sys::GHOSTTY_POINT_TAG_SCREEN)?,
                &mut selection.start,
            ))?;
            check(sys::ghostty_terminal_grid_ref(
                self.handle.0,
                point(end.0, end.1, sys::GHOSTTY_POINT_TAG_SCREEN)?,
                &mut selection.end,
            ))?;
            check(sys::ghostty_terminal_set(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_OPT_SELECTION,
                (&selection as *const sys::GhosttySelection).cast(),
            ))
        }
    }
    pub fn select_word(&mut self, column: u16, row: usize) -> Result<()> {
        let mut reference = sys::GhosttyGridRef::default();
        let mut selection = sys::GhosttySelection::default();
        unsafe {
            // SAFETY: sized options, borrowed ref and output types match the C API.
            check(sys::ghostty_terminal_grid_ref(
                self.handle.0,
                point(column, row, sys::GHOSTTY_POINT_TAG_SCREEN)?,
                &mut reference,
            ))?;
            let options = sys::GhosttyTerminalSelectWordOptions {
                size: size_of::<sys::GhosttyTerminalSelectWordOptions>(),
                ref_: reference,
                ..Default::default()
            };
            check(sys::ghostty_terminal_select_word(
                self.handle.0,
                &options,
                &mut selection,
            ))?;
            check(sys::ghostty_terminal_set(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_OPT_SELECTION,
                (&selection as *const sys::GhosttySelection).cast(),
            ))
        }
    }
    /// Extend the terminal-owned tracked start, including after scroll/reflow.
    pub fn extend_selection(&mut self, end: (u16, usize), rectangle: bool) -> Result<()> {
        let Some(mut selection) = self.selection()? else {
            return Ok(());
        };
        selection.rectangle = rectangle;
        unsafe {
            check(sys::ghostty_terminal_grid_ref(
                self.handle.0,
                point(end.0, end.1, sys::GHOSTTY_POINT_TAG_SCREEN)?,
                &mut selection.end,
            ))?;
            check(sys::ghostty_terminal_set(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_OPT_SELECTION,
                (&selection as *const sys::GhosttySelection).cast(),
            ))
        }
    }
    pub fn select_all(&mut self) -> Result<()> {
        let mut selection = sys::GhosttySelection::default();
        unsafe {
            // SAFETY: live owned terminal and correctly typed selection storage.
            check(sys::ghostty_terminal_select_all(
                self.handle.0,
                &mut selection,
            ))?;
            check(sys::ghostty_terminal_set(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_OPT_SELECTION,
                (&selection as *const sys::GhosttySelection).cast(),
            ))
        }
    }
    pub fn clear_selection(&mut self) -> Result<()> {
        // SAFETY: null resets the selection option.
        check(unsafe {
            sys::ghostty_terminal_set(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_OPT_SELECTION,
                ptr::null(),
            )
        })
    }
    pub(super) fn selection(&self) -> Result<Option<sys::GhosttySelection>> {
        let mut selection = sys::GhosttySelection::default();
        // SAFETY: fixed key uses this exact struct; caller cannot mutate while borrowed.
        let code = unsafe {
            sys::ghostty_terminal_get(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_DATA_SELECTION,
                (&mut selection as *mut sys::GhosttySelection).cast(),
            )
        };
        if code == sys::GHOSTTY_NO_VALUE {
            Ok(None)
        } else {
            check(code)?;
            Ok(Some(selection))
        }
    }
    pub fn selection_text(&mut self) -> Result<Option<String>> {
        let Some(selection) = self.selection()? else {
            return Ok(None);
        };
        struct Formatter(sys::GhosttyFormatter);
        impl Drop for Formatter {
            fn drop(&mut self) {
                unsafe { sys::ghostty_formatter_free(self.0) }
            }
        }
        let mut handle = ptr::null_mut();
        // SAFETY: formatter cannot outlive this exclusive terminal borrow.
        check(unsafe {
            sys::ghostty_formatter_terminal_new(ptr::null(), &mut handle, self.handle.0)
        })?;
        let formatter = Formatter(handle);
        let yes = true;
        unsafe {
            check(sys::ghostty_formatter_set(
                formatter.0,
                sys::GHOSTTY_FORMATTER_OPT_SELECTION,
                (&selection as *const sys::GhosttySelection).cast(),
            ))?;
            check(sys::ghostty_formatter_set(
                formatter.0,
                sys::GHOSTTY_FORMATTER_OPT_UNWRAP,
                (&yes as *const bool).cast(),
            ))?;
            check(sys::ghostty_formatter_set(
                formatter.0,
                sys::GHOSTTY_FORMATTER_OPT_TRIM,
                (&yes as *const bool).cast(),
            ))?;
        }
        let bytes = buffer(|data, len, written| unsafe {
            sys::ghostty_formatter_format_buf(formatter.0, data, len, written)
        })?;
        Ok(Some(
            String::from_utf8(bytes).map_err(|_| Error(sys::GHOSTTY_INVALID_VALUE))?,
        ))
    }
}
