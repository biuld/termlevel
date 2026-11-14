//! Fixed-key state access. Grid references are resolved, read and discarded
//! under one exclusive borrow; no pointer into a native page escapes.
use super::{Cell, Color, Error, Result, Style, Terminal, check, ptr, sys};

pub(super) struct Tracked(sys::GhosttyTrackedGridRef);
impl Drop for Tracked {
    fn drop(&mut self) {
        // SAFETY: owns this tracked ref; freeing after terminal destruction is allowed.
        unsafe { sys::ghostty_tracked_grid_ref_free(self.0) }
    }
}
struct LiveSearch {
    handle: sys::GhosttySearch,
    needle: String,
}
impl Drop for LiveSearch {
    fn drop(&mut self) {
        // SAFETY: owned search, freed with its exclusively accessed terminal.
        unsafe { sys::ghostty_search_free(self.handle) }
    }
}
pub(super) struct Retention {
    limit: usize,
    floor: [Option<Tracked>; 2],
    scroll_anchor: Option<Tracked>,
    search: Option<LiveSearch>,
}
impl Default for Retention {
    fn default() -> Self {
        Self {
            limit: usize::MAX,
            floor: [None, None],
            scroll_anchor: None,
            search: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Colors {
    pub foreground: Color,
    pub background: Color,
    pub palette: [Color; 256],
}

/// Owned screen coordinates, including history. End column is exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    pub start_row: usize,
    pub start_column: u16,
    pub end_row: usize,
    pub end_column: u16,
}

impl From<Color> for sys::GhosttyColorRgb {
    fn from(color: Color) -> Self {
        Self {
            r: color.r,
            g: color.g,
            b: color.b,
        }
    }
}
impl TryFrom<sys::GhosttyStyle> for Style {
    type Error = Error;
    fn try_from(style: sys::GhosttyStyle) -> Result<Self> {
        Ok(Self {
            foreground: style.fg_color.try_into()?,
            background: style.bg_color.try_into()?,
            underline_color: style.underline_color.try_into()?,
            flags: style.flags,
        })
    }
}

impl Terminal {
    /// Whether the current parser screen is the alternate screen.
    pub fn alternate_screen(&self) -> Result<bool> {
        let mut screen = sys::GHOSTTY_TERMINAL_SCREEN_PRIMARY;
        // SAFETY: ACTIVE_SCREEN writes its documented enum.
        check(unsafe {
            sys::ghostty_terminal_get(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_DATA_ACTIVE_SCREEN,
                (&mut screen as *mut sys::GhosttyTerminalScreen).cast(),
            )
        })?;
        Ok(screen == sys::GHOSTTY_TERMINAL_SCREEN_ALTERNATE)
    }

    /// One host viewport anchor, owned and accessed with the terminal. Set only
    /// on the primary screen; it survives alternate-screen use and reflow.
    pub fn set_scroll_anchor(&mut self, row: Option<usize>) -> Result<()> {
        let Some(row) = row else {
            self.retention.scroll_anchor = None;
            return Ok(());
        };
        if self.alternate_screen()? {
            return Err(Error(sys::GHOSTTY_INVALID_VALUE));
        }
        let point = sys::GhosttyPoint {
            tag: sys::GHOSTTY_POINT_TAG_SCREEN,
            value: sys::GhosttyPointValue {
                coordinate: sys::GhosttyPointCoordinate {
                    x: 0,
                    y: u32::try_from(row).map_err(|_| Error(sys::GHOSTTY_INVALID_VALUE))?,
                },
            },
        };
        let mut reference = ptr::null_mut();
        // SAFETY: the new owned reference never escapes this terminal.
        check(unsafe {
            sys::ghostty_terminal_grid_ref_track(self.handle.0, point, &mut reference)
        })?;
        self.retention.scroll_anchor = Some(Tracked(reference));
        Ok(())
    }

    /// Current full-screen row of the anchor, or None after its content was lost.
    pub fn scroll_anchor_row(&self) -> Result<Option<usize>> {
        let Some(anchor) = &self.retention.scroll_anchor else {
            return Ok(None);
        };
        let mut point = sys::GhosttyPointCoordinate::default();
        // SAFETY: owned reference; all access is serialized with the terminal.
        let code = unsafe {
            sys::ghostty_tracked_grid_ref_point(anchor.0, sys::GHOSTTY_POINT_TAG_SCREEN, &mut point)
        };
        if code == sys::GHOSTTY_NO_VALUE {
            return Ok(None);
        }
        check(code)?;
        Ok(Some(point.y as usize))
    }

    /// Native one-shot search, ASCII case-insensitive and otherwise byte-exact.
    /// Does not change the viewport or retain references in the returned values.
    pub fn find_all(&mut self, query: &str) -> Result<Vec<Match>> {
        struct Search(sys::GhosttySearch);
        impl Drop for Search {
            fn drop(&mut self) {
                unsafe { sys::ghostty_search_free(self.0) }
            }
        }
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let mut search = ptr::null_mut();
        // SAFETY: search is scoped to the exclusively borrowed terminal.
        check(unsafe { sys::ghostty_search_new(ptr::null(), &mut search, self.handle.0) })?;
        let search = Search(search);
        let needle = sys::GhosttyString {
            ptr: query.as_ptr(),
            len: query.len(),
        };
        // SAFETY: the needle is copied; run is the one-shot convenience.
        unsafe {
            check(sys::ghostty_search_set(
                search.0,
                sys::GHOSTTY_SEARCH_OPT_NEEDLE,
                (&needle as *const sys::GhosttyString).cast(),
            ))?;
            check(sys::ghostty_search_run(search.0))?;
        }
        self.search_matches(search.0)
    }

    /// Release the incremental search and its tracked native results.
    pub fn clear_search(&mut self) {
        self.retention.search = None;
    }

    /// Make one bounded feed and at most 32 ticks. None means more calls are
    /// needed. Handles and tracked results stay owned by this terminal.
    pub fn search_step(&mut self, query: &str) -> Result<Option<Vec<Match>>> {
        if self.retention.search.is_none() {
            let mut handle = ptr::null_mut();
            // SAFETY: search is owned and serialized with this terminal.
            check(unsafe { sys::ghostty_search_new(ptr::null(), &mut handle, self.handle.0) })?;
            self.retention.search = Some(LiveSearch {
                handle,
                needle: String::new(),
            });
        }
        let search = self.retention.search.as_mut().unwrap();
        if search.needle != query {
            let needle = sys::GhosttyString {
                ptr: query.as_ptr(),
                len: query.len(),
            };
            // SAFETY: native code copies needle during the call.
            check(unsafe {
                sys::ghostty_search_set(
                    search.handle,
                    sys::GHOSTTY_SEARCH_OPT_NEEDLE,
                    (&needle as *const sys::GhosttyString).cast(),
                )
            })?;
            search.needle = query.to_owned();
        }
        let handle = search.handle;
        // SAFETY: each feed copies a bounded native chunk under exclusive access.
        check(unsafe { sys::ghostty_search_feed(handle) })?;
        let mut status = sys::GHOSTTY_SEARCH_STATUS_RUNNING;
        for _ in 0..32 {
            check(unsafe { sys::ghostty_search_tick(handle, &mut status) })?;
            if status != sys::GHOSTTY_SEARCH_STATUS_RUNNING {
                break;
            }
        }
        if status == sys::GHOSTTY_SEARCH_STATUS_COMPLETE {
            self.search_matches(handle).map(Some)
        } else {
            Ok(None)
        }
    }

    fn search_matches(&self, handle: sys::GhosttySearch) -> Result<Vec<Match>> {
        let mut matches = sys::GhosttySelectionBuffer::default();
        // SAFETY: exact output struct; first query obtains required capacity.
        let code = unsafe {
            sys::ghostty_search_get(
                handle,
                sys::GHOSTTY_SEARCH_DATA_MATCHES,
                (&mut matches as *mut sys::GhosttySelectionBuffer).cast(),
            )
        };
        if code != sys::GHOSTTY_OUT_OF_SPACE {
            check(code)?;
        }
        let mut selections = vec![sys::GhosttySelection::default(); matches.len];
        matches.ptr = selections.as_mut_ptr();
        matches.cap = selections.len();
        // SAFETY: allocated exact capacity and terminal hasn't changed.
        check(unsafe {
            sys::ghostty_search_get(
                handle,
                sys::GHOSTTY_SEARCH_DATA_MATCHES,
                (&mut matches as *mut sys::GhosttySelectionBuffer).cast(),
            )
        })?;
        let mut output = Vec::with_capacity(matches.len);
        for selection in selections.into_iter().take(matches.len) {
            let (mut start, mut end) = (
                sys::GhosttyPointCoordinate::default(),
                sys::GhosttyPointCoordinate::default(),
            );
            let mut cell = 0;
            let mut wide = sys::GHOSTTY_CELL_WIDE_NARROW;
            // SAFETY: selections originate from this search/terminal, no mutation
            // occurs until all coordinates are copied and the search is dropped.
            unsafe {
                check(sys::ghostty_terminal_point_from_grid_ref(
                    self.handle.0,
                    &selection.start,
                    sys::GHOSTTY_POINT_TAG_SCREEN,
                    &mut start,
                ))?;
                check(sys::ghostty_terminal_point_from_grid_ref(
                    self.handle.0,
                    &selection.end,
                    sys::GHOSTTY_POINT_TAG_SCREEN,
                    &mut end,
                ))?;
                check(sys::ghostty_grid_ref_cell(&selection.end, &mut cell))?;
                check(sys::ghostty_cell_get(
                    cell,
                    sys::GHOSTTY_CELL_DATA_WIDE,
                    (&mut wide as *mut sys::GhosttyCellWide).cast(),
                ))?;
            }
            let width = if wide == sys::GHOSTTY_CELL_WIDE_WIDE {
                2
            } else {
                1
            };
            output.push(Match {
                start_row: start.y as usize,
                start_column: start.x,
                end_row: end.y as usize,
                end_column: end.x.saturating_add(width),
            });
        }
        Ok(output)
    }

    pub fn total_rows(&self) -> Result<usize> {
        let mut rows = 0usize;
        // SAFETY: TOTAL_ROWS writes size_t; access to this !Sync handle is serialized.
        check(unsafe {
            sys::ghostty_terminal_get(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_DATA_TOTAL_ROWS,
                (&mut rows as *mut usize).cast(),
            )
        })?;
        Ok(rows)
    }

    /// Exact logical history window over page-granular storage. Tracked floor
    /// coordinates follow resize/reflow/pruning, so increasing the limit never
    /// resurrects rows previously discarded by the application. Screens have
    /// separate floors. Callers address rows in this returned native range.
    pub fn retained_range(&mut self) -> Result<std::ops::Range<usize>> {
        let total = self.total_rows()?;
        let (_, rows) = self.size()?;
        let mut screen = sys::GHOSTTY_TERMINAL_SCREEN_PRIMARY;
        // SAFETY: ACTIVE_SCREEN writes a GhosttyTerminalScreen enum.
        check(unsafe {
            sys::ghostty_terminal_get(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_DATA_ACTIVE_SCREEN,
                (&mut screen as *mut sys::GhosttyTerminalScreen).cast(),
            )
        })?;
        let screen = match screen {
            sys::GHOSTTY_TERMINAL_SCREEN_PRIMARY => 0,
            sys::GHOSTTY_TERMINAL_SCREEN_ALTERNATE => 1,
            _ => return Err(Error(sys::GHOSTTY_UNSUPPORTED)),
        };
        let mut previous = 0usize;
        if let Some(reference) = &self.retention.floor[screen] {
            let mut point = sys::GhosttyPointCoordinate::default();
            // SAFETY: this tracked ref belongs to this terminal's corresponding screen.
            let code = unsafe {
                sys::ghostty_tracked_grid_ref_point(
                    reference.0,
                    sys::GHOSTTY_POINT_TAG_SCREEN,
                    &mut point,
                )
            };
            if code != sys::GHOSTTY_NO_VALUE {
                check(code)?;
                previous = point.y as usize;
            }
        }
        let history = total.saturating_sub(rows as usize);
        let start = previous
            .max(history.saturating_sub(self.retention.limit))
            .min(history);
        let point = sys::GhosttyPoint {
            tag: sys::GHOSTTY_POINT_TAG_SCREEN,
            value: sys::GhosttyPointValue {
                coordinate: sys::GhosttyPointCoordinate {
                    x: 0,
                    y: u32::try_from(start).map_err(|_| Error(sys::GHOSTTY_INVALID_VALUE))?,
                },
            },
        };
        if self.retention.floor[screen].is_none() || start != previous {
            let mut reference = ptr::null_mut();
            // SAFETY: create a tracked ref at the validated retained start row.
            check(unsafe {
                sys::ghostty_terminal_grid_ref_track(self.handle.0, point, &mut reference)
            })?;
            self.retention.floor[screen] = Some(Tracked(reference));
        }
        Ok(start..total)
    }

    /// Active grid coordinates even when the application hides the cursor.
    pub fn cursor_position(&self) -> Result<(u16, u16)> {
        let (mut x, mut y) = (0u16, 0u16);
        // SAFETY: fixed keys and exact u16 storage; native active coordinates,
        // independent of whether the application hides the output cursor.
        unsafe {
            check(sys::ghostty_terminal_get(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_DATA_CURSOR_X,
                (&mut x as *mut u16).cast(),
            ))?;
            check(sys::ghostty_terminal_get(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_DATA_CURSOR_Y,
                (&mut y as *mut u16).cast(),
            ))?;
        }
        Ok((x, y))
    }

    /// Page-granular native history bound. Embedders can expose an exact smaller
    /// logical window; native pruning may retain an additional page of old rows.
    pub fn set_scrollback(&mut self, lines: usize) -> Result<()> {
        let bytes = if lines == 0 { 0usize } else { usize::MAX };
        // SAFETY: both options take size_t. Disable the default byte cap so the
        // requested line cap, rather than the built-in 10KB limit, takes effect.
        unsafe {
            check(sys::ghostty_terminal_set(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_OPT_SCROLLBACK_MAX_BYTES,
                (&bytes as *const usize).cast(),
            ))?;
            check(sys::ghostty_terminal_set(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_OPT_SCROLLBACK_MAX_LINES,
                (&lines as *const usize).cast(),
            ))?;
        }
        self.retention.limit = lines;
        self.retained_range()?;
        Ok(())
    }

    /// Change defaults without overwriting colors set by program OSC sequences.
    pub fn set_default_colors(&mut self, foreground: Color, background: Color) -> Result<()> {
        let foreground = sys::GhosttyColorRgb::from(foreground);
        let background = sys::GhosttyColorRgb::from(background);
        // SAFETY: COLOR defaults take pointers to these exact RGB value structs.
        unsafe {
            check(sys::ghostty_terminal_set(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_OPT_COLOR_FOREGROUND,
                (&foreground as *const sys::GhosttyColorRgb).cast(),
            ))?;
            check(sys::ghostty_terminal_set(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_OPT_COLOR_BACKGROUND,
                (&background as *const sys::GhosttyColorRgb).cast(),
            ))?;
        }
        Ok(())
    }

    pub fn set_default_palette(&mut self, palette: [Color; 256]) -> Result<()> {
        let palette = palette.map(sys::GhosttyColorRgb::from);
        // SAFETY: exactly 256 RGB entries; native code copies them immediately.
        check(unsafe {
            sys::ghostty_terminal_set(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_OPT_COLOR_PALETTE,
                palette.as_ptr().cast(),
            )
        })
    }

    pub fn colors(&self) -> Result<Colors> {
        let mut foreground = sys::GhosttyColorRgb::default();
        let mut background = sys::GhosttyColorRgb::default();
        let mut palette = [sys::GhosttyColorRgb::default(); 256];
        // SAFETY: keys have the exact fixed RGB/RGB[256] output types below.
        unsafe {
            check(sys::ghostty_terminal_get(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_DATA_COLOR_FOREGROUND,
                (&mut foreground as *mut sys::GhosttyColorRgb).cast(),
            ))?;
            check(sys::ghostty_terminal_get(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_DATA_COLOR_BACKGROUND,
                (&mut background as *mut sys::GhosttyColorRgb).cast(),
            ))?;
            check(sys::ghostty_terminal_get(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_DATA_COLOR_PALETTE,
                palette.as_mut_ptr().cast(),
            ))?;
        }
        Ok(Colors {
            foreground: foreground.into(),
            background: background.into(),
            palette: palette.map(Into::into),
        })
    }

    /// Read one row anywhere in active-screen history. For active rendering use
    /// RenderState; this path is intended for infrequent historical row access.
    pub fn screen_row(&mut self, row: usize) -> Result<Vec<Cell>> {
        let row = u32::try_from(row).map_err(|_| Error(sys::GHOSTTY_INVALID_VALUE))?;
        let (cols, _) = self.size()?;
        let mut cells = Vec::with_capacity(cols as usize);
        for column in 0..cols {
            let point = sys::GhosttyPoint {
                tag: sys::GHOSTTY_POINT_TAG_SCREEN,
                value: sys::GhosttyPointValue {
                    coordinate: sys::GhosttyPointCoordinate { x: column, y: row },
                },
            };
            let mut reference = sys::GhosttyGridRef::default();
            let mut raw_cell = 0;
            let mut wide = sys::GHOSTTY_CELL_WIDE_NARROW;
            let mut style = sys::GhosttyStyle::default();
            let mut stack = [0u32; 16];
            let mut count = 0;
            // SAFETY: ref is resolved from this live terminal and used before
            // any mutation. All keys and destination types match their headers.
            let code = unsafe {
                check(sys::ghostty_terminal_grid_ref(
                    self.handle.0,
                    point,
                    &mut reference,
                ))?;
                check(sys::ghostty_grid_ref_cell(&reference, &mut raw_cell))?;
                check(sys::ghostty_cell_get(
                    raw_cell,
                    sys::GHOSTTY_CELL_DATA_WIDE,
                    (&mut wide as *mut sys::GhosttyCellWide).cast(),
                ))?;
                check(sys::ghostty_grid_ref_style(&reference, &mut style))?;
                sys::ghostty_grid_ref_graphemes(
                    &reference,
                    stack.as_mut_ptr(),
                    stack.len(),
                    &mut count,
                )
            };
            let codepoints = if code == sys::GHOSTTY_OUT_OF_SPACE {
                let mut values = vec![0u32; count];
                // SAFETY: exact required capacity; ref remains valid.
                check(unsafe {
                    sys::ghostty_grid_ref_graphemes(
                        &reference,
                        values.as_mut_ptr(),
                        values.len(),
                        &mut count,
                    )
                })?;
                values.truncate(count);
                values
            } else {
                check(code)?;
                stack[..count].to_vec()
            };
            let text: String = codepoints
                .into_iter()
                .map(|c| char::from_u32(c).unwrap_or(char::REPLACEMENT_CHARACTER))
                .collect();
            let hyperlink = unsafe {
                super::selection::link_at(
                    self.handle.0,
                    column,
                    row as usize,
                    sys::GHOSTTY_POINT_TAG_SCREEN,
                )
            }?;
            let selected = if let Some(selection) = self.selection()? {
                let mut value = false;
                check(unsafe {
                    sys::ghostty_terminal_selection_contains(
                        self.handle.0,
                        &selection,
                        point,
                        &mut value,
                    )
                })?;
                value
            } else {
                false
            };
            cells.push(Cell {
                column,
                text,
                wide,
                style: Style::try_from(style)?.with_cell_background(raw_cell)?,
                selected,
                hyperlink,
            });
        }
        Ok(cells)
    }
}
