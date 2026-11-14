use super::{Error, Result, Terminal, buffer, check, handle, ptr, sys};
use std::sync::{Arc, Weak};

handle!(
    State,
    sys::GhosttyRenderState,
    sys::ghostty_render_state_new,
    sys::ghostty_render_state_free
);
handle!(
    Rows,
    sys::GhosttyRenderStateRowIterator,
    sys::ghostty_render_state_row_iterator_new,
    sys::ghostty_render_state_row_iterator_free
);
handle!(
    Cells,
    sys::GhosttyRenderStateRowCells,
    sys::ghostty_render_state_row_cells_new,
    sys::ghostty_render_state_row_cells_free
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
impl From<sys::GhosttyColorRgb> for Color {
    fn from(rgb: sys::GhosttyColorRgb) -> Self {
        Self {
            r: rgb.r,
            g: rgb.g,
            b: rgb.b,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleColor {
    Default,
    Palette(u8),
    Rgb(Color),
}
impl TryFrom<sys::GhosttyStyleColor> for StyleColor {
    type Error = Error;
    fn try_from(color: sys::GhosttyStyleColor) -> Result<Self> {
        // SAFETY: read the C union only through its documented active tag.
        unsafe {
            match color.tag as u32 {
                sys::GHOSTTY_STYLE_COLOR_NONE => Ok(Self::Default),
                sys::GHOSTTY_STYLE_COLOR_PALETTE => Ok(Self::Palette(color.value.palette)),
                sys::GHOSTTY_STYLE_COLOR_RGB => Ok(Self::Rgb(color.value.rgb.into())),
                _ => Err(Error(sys::GHOSTTY_UNSUPPORTED)),
            }
        }
    }
}

/// Original colors and every style bit are preserved, including underline kind.
/// Cell snapshots fold background-only content tags into `background` as well.
/// Flags use the `GHOSTTY_STYLE_FLAG_*` masks in [`crate::raw`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    pub foreground: StyleColor,
    pub background: StyleColor,
    pub underline_color: StyleColor,
    pub flags: u16,
}

impl Style {
    /// Empty cells can store their background in the packed content instead
    /// of the style table (notably BCE/erase operations). Preserve palette
    /// indices so later palette changes still apply to historical cells.
    pub(super) fn with_cell_background(mut self, cell: sys::GhosttyCell) -> Result<Self> {
        let mut tag = sys::GHOSTTY_CELL_CONTENT_CODEPOINT;
        // SAFETY: the packed cell is copied from a serialized native snapshot;
        // every key below writes its documented fixed type.
        check(unsafe {
            sys::ghostty_cell_get(
                cell,
                sys::GHOSTTY_CELL_DATA_CONTENT_TAG,
                (&mut tag as *mut sys::GhosttyCellContentTag).cast(),
            )
        })?;
        match tag {
            sys::GHOSTTY_CELL_CONTENT_BG_COLOR_PALETTE => {
                let mut index: sys::GhosttyColorPaletteIndex = 0;
                check(unsafe {
                    sys::ghostty_cell_get(
                        cell,
                        sys::GHOSTTY_CELL_DATA_COLOR_PALETTE,
                        (&mut index as *mut sys::GhosttyColorPaletteIndex).cast(),
                    )
                })?;
                self.background = StyleColor::Palette(index);
            }
            sys::GHOSTTY_CELL_CONTENT_BG_COLOR_RGB => {
                let mut rgb = sys::GhosttyColorRgb::default();
                check(unsafe {
                    sys::ghostty_cell_get(
                        cell,
                        sys::GHOSTTY_CELL_DATA_COLOR_RGB,
                        (&mut rgb as *mut sys::GhosttyColorRgb).cast(),
                    )
                })?;
                self.background = StyleColor::Rgb(rgb.into());
            }
            _ => {}
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// Exactly one grid column, including wide-character spacer columns.
    pub column: u16,
    /// Full grapheme cluster; empty for cells without text.
    pub text: String,
    /// `GHOSTTY_CELL_WIDE_*` classification; never infer width from UTF-8 length.
    pub wide: sys::GhosttyCellWide,
    pub style: Style,
    pub selected: bool,
    pub hyperlink: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub viewport_y: i32,
    pub id: [u64; 2],
    pub dirty: bool,
    pub cells: Vec<Cell>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub position: Option<(u16, u16)>,
    pub visible: bool,
    pub blinking: bool,
    pub wide_tail: bool,
    pub password_input: bool,
    /// `GHOSTTY_CURSOR_STYLE_*` value.
    pub style: sys::GhosttyCursorStyle,
}

/// A fully owned viewport copy; survives writes, resize and terminal destruction.
/// Includes owned image pixels, hyperlink targets and selection flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub cols: u16,
    pub rows: u16,
    pub dirty: sys::GhosttyRenderStateDirty,
    pub cursor: Cursor,
    pub foreground: Color,
    pub background: Color,
    pub cursor_color: Option<Color>,
    pub palette: [Color; 256],
    pub lines: Vec<Row>,
    pub images: Vec<super::ImagePlacement>,
}

/// Persistent native render snapshot. Use one per terminal. Capturing a different
/// terminal starts a fresh snapshot (including when an old address is reused).
/// This first safe API copies the full viewport, not just the dirty rows.
pub struct RenderState {
    state: State,
    owner: Weak<()>,
    hold_generation: u64,
    image_cache: std::collections::HashMap<(u32, u64), Arc<super::Pixels>>,
}
// SAFETY: render snapshots and the default allocator have no thread affinity;
// all mutation is exclusive. No iterator or borrowed C row escapes this owner.
unsafe impl Send for RenderState {}
impl RenderState {
    pub fn new() -> Result<Self> {
        Ok(Self {
            state: State::new()?,
            owner: Weak::new(),
            hold_generation: 0,
            image_cache: Default::default(),
        })
    }

    pub fn capture(&mut self, terminal: &mut Terminal) -> Result<Frame> {
        if let Some(held) = terminal.render_hold() {
            return Ok((*held.frame).clone());
        }
        let owner = Arc::downgrade(&terminal.identity);
        let generation = terminal.render_hold_generation();
        if !self.owner.ptr_eq(&owner) || self.hold_generation != generation {
            self.state = State::new()?;
            if !self.owner.ptr_eq(&owner) {
                self.image_cache.clear();
            }
            self.owner = owner;
            self.hold_generation = generation;
        }
        // SAFETY: exclusive terminal and render state access for the snapshot.
        check(unsafe { sys::ghostty_render_state_update(self.state.0, terminal.handle.0) })?;
        let frame = self.read_frame(terminal.handle.0);
        // Native updates consume terminal dirty flags. Other render-state
        // owners must rebuild before their next capture; this one stays warm.
        terminal.events.get_mut().hold_generation =
            terminal.events.get_mut().hold_generation.wrapping_add(1);
        self.hold_generation = terminal.render_hold_generation();
        frame
    }

    /// Capture history images at an absolute native screen row, then restore
    /// the prior parser viewport even if rendering fails. References stay local.
    pub fn images_at(
        &mut self,
        terminal: &mut Terminal,
        row: usize,
    ) -> Result<Vec<super::ImagePlacement>> {
        let mut scrollbar = sys::GhosttyTerminalScrollbar::default();
        check(unsafe {
            sys::ghostty_terminal_get(
                terminal.handle.0,
                sys::GHOSTTY_TERMINAL_DATA_SCROLLBAR,
                (&mut scrollbar as *mut sys::GhosttyTerminalScrollbar).cast(),
            )
        })?;
        let old = scrollbar.offset as usize;
        let scroll = |row| sys::GhosttyTerminalScrollViewport {
            tag: sys::GHOSTTY_TERMINAL_SCROLL_VIEWPORT_ROW,
            value: sys::GhosttyTerminalScrollViewportValue { row },
        };
        check(unsafe { sys::ghostty_terminal_scroll_viewport(terminal.handle.0, scroll(row)) })?;
        let result = self.capture(terminal).map(|f| f.images);
        let restore = if scrollbar.offset + scrollbar.len >= scrollbar.total {
            sys::GhosttyTerminalScrollViewport {
                tag: sys::GHOSTTY_TERMINAL_SCROLL_VIEWPORT_BOTTOM,
                value: Default::default(),
            }
        } else {
            scroll(old)
        };
        let restored =
            check(unsafe { sys::ghostty_terminal_scroll_viewport(terminal.handle.0, restore) });
        terminal.events.get_mut().hold_generation =
            terminal.events.get_mut().hold_generation.wrapping_add(1);
        restored?;
        result
    }
    fn read_frame(&mut self, terminal: sys::GhosttyTerminal) -> Result<Frame> {
        let (mut cols, mut rows) = (0u16, 0u16);
        let mut dirty = sys::GHOSTTY_RENDER_STATE_DIRTY_FALSE;
        let mut cursor = sys::GhosttyRenderStateCursor {
            size: size_of::<sys::GhosttyRenderStateCursor>(),
            ..Default::default()
        };
        let mut colors = sys::GhosttyRenderStateColors {
            size: size_of::<sys::GhosttyRenderStateColors>(),
            ..Default::default()
        };
        let mut iter = Rows::new()?;
        let mut cells = Cells::new()?;
        // SAFETY: every key below is paired with the exact documented output type.
        unsafe {
            check(sys::ghostty_render_state_get(
                self.state.0,
                sys::GHOSTTY_RENDER_STATE_DATA_COLS,
                (&mut cols as *mut u16).cast(),
            ))?;
            check(sys::ghostty_render_state_get(
                self.state.0,
                sys::GHOSTTY_RENDER_STATE_DATA_ROWS,
                (&mut rows as *mut u16).cast(),
            ))?;
            check(sys::ghostty_render_state_get(
                self.state.0,
                sys::GHOSTTY_RENDER_STATE_DATA_DIRTY,
                (&mut dirty as *mut sys::GhosttyRenderStateDirty).cast(),
            ))?;
            check(sys::ghostty_render_state_get(
                self.state.0,
                sys::GHOSTTY_RENDER_STATE_DATA_CURSOR,
                (&mut cursor as *mut sys::GhosttyRenderStateCursor).cast(),
            ))?;
            check(sys::ghostty_render_state_get(
                self.state.0,
                sys::GHOSTTY_RENDER_STATE_DATA_COLORS,
                (&mut colors as *mut sys::GhosttyRenderStateColors).cast(),
            ))?;
            check(sys::ghostty_render_state_get(
                self.state.0,
                sys::GHOSTTY_RENDER_STATE_DATA_ROW_ITERATOR,
                (&mut iter.0 as *mut sys::GhosttyRenderStateRowIterator).cast(),
            ))?;
        }
        let mut lines = Vec::with_capacity(rows as usize);
        // SAFETY: iter is bound to this state, which is not updated during traversal.
        while unsafe { sys::ghostty_render_state_row_iterator_next(iter.0) } {
            let mut y = 0i32;
            let mut id = sys::GhosttyRenderStateRowId::default();
            let mut row_dirty = false;
            // SAFETY: current row is selected; key/output types match headers.
            unsafe {
                check(sys::ghostty_render_state_row_get(
                    iter.0,
                    sys::GHOSTTY_RENDER_STATE_ROW_DATA_VIEWPORT_Y,
                    (&mut y as *mut i32).cast(),
                ))?;
                check(sys::ghostty_render_state_row_get(
                    iter.0,
                    sys::GHOSTTY_RENDER_STATE_ROW_DATA_ID,
                    (&mut id as *mut sys::GhosttyRenderStateRowId).cast(),
                ))?;
                check(sys::ghostty_render_state_row_get(
                    iter.0,
                    sys::GHOSTTY_RENDER_STATE_ROW_DATA_DIRTY,
                    (&mut row_dirty as *mut bool).cast(),
                ))?;
                check(sys::ghostty_render_state_row_get(
                    iter.0,
                    sys::GHOSTTY_RENDER_STATE_ROW_DATA_CELLS,
                    (&mut cells.0 as *mut sys::GhosttyRenderStateRowCells).cast(),
                ))?;
            }
            let mut row = Vec::with_capacity(cols as usize);
            // SAFETY: populated for the current row; no state update in this loop.
            while unsafe { sys::ghostty_render_state_row_cells_next(cells.0) } {
                let mut style = sys::GhosttyStyle::default();
                let mut selected = false;
                let mut raw_cell = 0;
                let mut wide = sys::GHOSTTY_CELL_WIDE_NARROW;
                // SAFETY: each typed destination matches its fixed key.
                unsafe {
                    check(sys::ghostty_render_state_row_cells_get(
                        cells.0,
                        sys::GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_STYLE,
                        (&mut style as *mut sys::GhosttyStyle).cast(),
                    ))?;
                    check(sys::ghostty_render_state_row_cells_get(
                        cells.0,
                        sys::GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_SELECTED,
                        (&mut selected as *mut bool).cast(),
                    ))?;
                    check(sys::ghostty_render_state_row_cells_get(
                        cells.0,
                        sys::GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_RAW,
                        (&mut raw_cell as *mut sys::GhosttyCell).cast(),
                    ))?;
                    check(sys::ghostty_cell_get(
                        raw_cell,
                        sys::GHOSTTY_CELL_DATA_WIDE,
                        (&mut wide as *mut sys::GhosttyCellWide).cast(),
                    ))?;
                }
                // SAFETY: UTF8 key fills a GhosttyBuffer, borrowing no Rust data
                // beyond this synchronous call. The iterator is not advanced.
                let bytes = buffer(|data, cap, written| unsafe {
                    let mut buf = sys::GhosttyBuffer {
                        ptr: data,
                        cap,
                        len: 0,
                    };
                    let code = sys::ghostty_render_state_row_cells_get(
                        cells.0,
                        sys::GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_UTF8,
                        (&mut buf as *mut sys::GhosttyBuffer).cast(),
                    );
                    *written = buf.len;
                    code
                })?;
                let column =
                    u16::try_from(row.len()).map_err(|_| Error(sys::GHOSTTY_INVALID_VALUE))?;
                let mut has_link = false;
                // SAFETY: fixed bool key and a cell from this render state.
                check(unsafe {
                    sys::ghostty_cell_get(
                        raw_cell,
                        sys::GHOSTTY_CELL_DATA_HAS_HYPERLINK,
                        (&mut has_link as *mut bool).cast(),
                    )
                })?;
                let hyperlink = if has_link {
                    // SAFETY: terminal is exclusively borrowed for this frame.
                    unsafe {
                        super::selection::link_at(
                            terminal,
                            column,
                            y as usize,
                            sys::GHOSTTY_POINT_TAG_VIEWPORT,
                        )
                    }?
                } else {
                    None
                };
                row.push(Cell {
                    column,
                    text: String::from_utf8(bytes)
                        .map_err(|_| Error(sys::GHOSTTY_INVALID_VALUE))?,
                    wide,
                    selected,
                    hyperlink,
                    style: Style::try_from(style)?.with_cell_background(raw_cell)?,
                });
            }
            lines.push(Row {
                viewport_y: y,
                id: id.bits,
                dirty: row_dirty,
                cells: row,
            });
        }
        let frame = Frame {
            cols,
            rows,
            dirty,
            cursor: Cursor {
                position: cursor
                    .viewport_has_value
                    .then_some((cursor.viewport_x, cursor.viewport_y)),
                visible: cursor.visible,
                blinking: cursor.blinking,
                wide_tail: cursor.wide_tail,
                password_input: cursor.password_input,
                style: cursor.visual_style,
            },
            foreground: colors.foreground.into(),
            background: colors.background.into(),
            cursor_color: if colors.cursor_has_value {
                Some(colors.cursor.into())
            } else {
                None
            },
            palette: colors.palette.map(Into::into),
            lines,
            // SAFETY: state and terminal are live, serialized, and not mutated
            // while image pixels and placement metadata are copied.
            images: unsafe {
                super::graphics::read_images(self.state.0, terminal, &mut self.image_cache)
            }?,
        };
        // SAFETY: all data has been copied successfully; no partial read is marked clean.
        check(unsafe { sys::ghostty_render_state_clean(self.state.0) })?;
        Ok(frame)
    }
}

/// Read-only native capture used inside the synchronous render-hold callback.
pub(super) unsafe fn capture_raw(terminal: sys::GhosttyTerminal) -> Result<Frame> {
    let mut state = RenderState::new()?;
    // SAFETY: caller owns serialized access to the live terminal.
    check(unsafe { sys::ghostty_render_state_update(state.state.0, terminal) })?;
    state.read_frame(terminal)
}
