//! Ghostty C ABI behind both the real Rust session surface and explicit preview.
//! SessionManager owns the PTY; this adapter only owns terminal emulation state.
use nsh_base::{
    TerminalColor, TerminalCursor, TerminalEmulator, TerminalEmulatorEvent,
    TerminalEmulatorFactory, TerminalLine, TerminalModes, TerminalRun, TerminalSize, TerminalStyle,
};
use nsh_core::prelude::*;
use std::{
    cell::RefCell,
    ops::Range,
    sync::Arc,
    time::{Duration, Instant},
};
pub(crate) use termlevel_ghostty_vt::Color as Rgb;
use termlevel_ghostty_vt::{
    Cell as GridCell, Colors, Frame, RenderState, StyleColor, Terminal, raw,
};

#[derive(Debug, Clone, PartialEq, Props)]
pub struct TerminalEngineProps {
    /// Workspace content using the Ghostty preview adapter.
    pub children: Prop<Children>,
}
/// Preview provider; creating a view never creates a process.
#[component]
pub fn TerminalEngine(props: TerminalEngineProps, _cx: Scope<NoEvent>) -> impl IntoElement {
    provide_context(TerminalEmulatorFactory::new(|size, scrollback| {
        Box::new(GhosttyEmulator::new(size, scrollback))
    }));
    view().flex_1().child(props.children)
}

#[derive(Clone, PartialEq)]
pub(crate) struct GridRun {
    pub col: usize,
    pub cells: usize,
    pub run: TerminalRun,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SearchHit {
    pub line: usize,
    pub col: usize,
    pub cols: usize,
}
#[derive(Debug)]
pub(crate) enum BackendEvent {
    Title(String),
    Bell,
    Reply(Vec<u8>),
    Error(String),
}
struct State {
    terminal: Terminal,
    renderer: RenderState,
    frame: Option<Arc<Frame>>,
    colors: Colors,
    range: Range<usize>,
    size: TerminalSize,
    foreground: Rgb,
    background: Rgb,
    errors: Vec<String>,
}
impl State {
    fn update(&mut self) -> Result<(), String> {
        let actual = self.terminal.retained_range().map_err(|e| e.to_string())?;
        if let Some(held) = self.terminal.render_hold() {
            let history = held.total_rows.saturating_sub(held.frame.rows as usize);
            // Output before the hold boundary may already have filled history
            // in this same chunk. Keep the frozen screen's logical window capped.
            self.range = self
                .range
                .start
                .max(held.total_rows.saturating_sub(actual.len()))
                .min(history)..held.total_rows;
            self.colors = Colors {
                foreground: held.frame.foreground,
                background: held.frame.background,
                palette: held.frame.palette,
            };
        } else {
            self.range = actual;
            self.frame = None;
            self.colors = self.terminal.colors().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    fn frame(&mut self) -> Result<Arc<Frame>, String> {
        if let Some(held) = self.terminal.render_hold() {
            return Ok(held.frame.clone());
        }
        if self.frame.is_none() {
            self.frame = Some(Arc::new(
                self.renderer
                    .capture(&mut self.terminal)
                    .map_err(|e| e.to_string())?,
            ));
        }
        Ok(self.frame.as_ref().unwrap().clone())
    }
    fn cells(&mut self, index: usize) -> Result<Vec<GridCell>, String> {
        let native = self.range.start + index;
        if native >= self.range.end {
            return Ok(Vec::new());
        }
        let active = self.range.end.saturating_sub(self.size.rows as usize);
        if native >= active {
            let frame = self.frame()?;
            Ok(frame
                .lines
                .get(native - active)
                .map(|r| r.cells.clone())
                .unwrap_or_default())
        } else {
            self.terminal.screen_row(native).map_err(|e| e.to_string())
        }
    }
}
/// All native state is serialized by Session.emulator. RefCell only supports
/// NSH's immutable line()/cursor() trait while filling the Rust frame cache.
pub(crate) struct GhosttyEmulator {
    state: RefCell<State>,
}
pub(crate) fn indexed_color(index: u8) -> Rgb {
    let hex = match index {
        0..=15 => [
            0x000000, 0xcd3131, 0x0dbc79, 0xe5e510, 0x2472c8, 0xbc3fbc, 0x11a8cd, 0xe5e5e5,
            0x666666, 0xf14c4c, 0x23d18b, 0xf5f543, 0x3b8eea, 0xd670d6, 0x29b8db, 0xffffff,
        ][index as usize],
        16..=231 => {
            let n = index as u32 - 16;
            let level = |v| if v == 0 { 0 } else { 55 + v * 40 };
            level(n / 36) << 16 | level(n / 6 % 6) << 8 | level(n % 6)
        }
        _ => {
            let v = 8 + (index as u32 - 232) * 10;
            v << 16 | v << 8 | v
        }
    };
    Rgb {
        r: (hex >> 16) as u8,
        g: (hex >> 8) as u8,
        b: hex as u8,
    }
}

/// Default lines kept above the screen. Older lines drop off the top.
pub(crate) const SCROLLBACK_LIMIT: usize = 10_000;
/// Smallest scrollback the settings control can apply.
pub(crate) const SCROLLBACK_MIN: usize = 1_000;
/// Largest scrollback the settings control can apply.
pub(crate) const SCROLLBACK_MAX: usize = 100_000;
/// Settings changes the limit by this many lines.
pub(crate) const SCROLLBACK_STEP: usize = 1_000;

pub(crate) fn clamp_scrollback(lines: usize) -> usize {
    lines.clamp(SCROLLBACK_MIN, SCROLLBACK_MAX)
}

impl GhosttyEmulator {
    pub(crate) fn try_new(size: TerminalSize, scrollback: usize) -> Result<Self, String> {
        let result = (|| -> termlevel_ghostty_vt::Result<Self> {
            let mut terminal = Terminal::new(size.cols.max(1), size.rows.max(1))?;
            let foreground = Rgb {
                r: 0xd7,
                g: 0xd9,
                b: 0xe1,
            };
            let background = Rgb {
                r: 0x19,
                g: 0x1a,
                b: 0x1e,
            };
            terminal.set_default_colors(foreground, background)?;
            terminal.set_default_palette(std::array::from_fn(|n| indexed_color(n as u8)))?;
            terminal.set_scrollback(scrollback)?;
            let colors = terminal.colors()?;
            let range = terminal.retained_range()?;
            Ok(Self {
                state: RefCell::new(State {
                    terminal,
                    renderer: RenderState::new()?,
                    frame: None,
                    colors,
                    range,
                    size,
                    foreground,
                    background,
                    errors: Vec::new(),
                }),
            })
        })();
        result.map_err(|e| format!("Starting Ghostty: {e}"))
    }
    pub(crate) fn new(size: TerminalSize, scrollback: usize) -> Self {
        Self::try_new(size, scrollback).expect("create Ghostty preview emulator")
    }
    pub(crate) fn set_scrollback(&mut self, lines: usize) {
        let state = self.state.get_mut();
        if let Err(error) = state
            .terminal
            .set_scrollback(lines)
            .map_err(|e| e.to_string())
            .and_then(|_| state.update())
        {
            state.errors.push(error);
        }
    }
    pub(crate) fn set_colors(&mut self, foreground: Option<Rgb>, background: Option<Rgb>) -> bool {
        let state = self.state.get_mut();
        let fg = foreground.unwrap_or(state.foreground);
        let bg = background.unwrap_or(state.background);
        if fg == state.foreground && bg == state.background {
            return false;
        }
        state.foreground = fg;
        state.background = bg;
        if let Err(error) = state
            .terminal
            .set_default_colors(fg, bg)
            .map_err(|e| e.to_string())
            .and_then(|_| state.update())
        {
            state.errors.push(error);
        }
        true
    }
    pub(crate) fn backend_events(&mut self) -> Vec<BackendEvent> {
        use termlevel_ghostty_vt::Event;
        let state = self.state.get_mut();
        let mut events: Vec<_> = state
            .terminal
            .take_events()
            .into_iter()
            .filter_map(|event| match event {
                Event::TitleChanged(title) => Some(BackendEvent::Title(title)),
                Event::Reset => Some(BackendEvent::Title(String::new())),
                Event::Bell => Some(BackendEvent::Bell),
                Event::WritePty(bytes) => Some(BackendEvent::Reply(bytes)),
                Event::WorkingDirectoryChanged(_) => None,
            })
            .collect();
        events.extend(state.errors.drain(..).map(BackendEvent::Error));
        events
    }
    pub(crate) fn sync_deadline(&self) -> Option<Instant> {
        self.state
            .borrow()
            .terminal
            .render_hold()
            .map(|hold| hold.since + Duration::from_millis(150))
    }
    pub(crate) fn flush_sync(&mut self, now: Instant, force: bool) -> bool {
        let Some(deadline) = self.sync_deadline() else {
            return false;
        };
        if !force && now < deadline {
            return false;
        }
        let state = self.state.get_mut();
        if let Err(error) = state
            .terminal
            .release_render_hold()
            .map_err(|e| e.to_string())
            .and_then(|_| state.update())
        {
            state.errors.push(error);
        }
        true
    }
    pub(crate) fn screen_rows(&self) -> usize {
        self.state.borrow().size.rows as usize
    }
    pub(crate) fn input_cursor(&self) -> TerminalCursor {
        let state = self.state.borrow();
        let position = state
            .terminal
            .render_hold()
            .map(|h| h.frame.cursor.position)
            .unwrap_or_else(|| state.terminal.cursor_position().ok());
        let (col, row) = position.unwrap_or_default();
        let mut cursor = TerminalCursor::default();
        cursor.col = col;
        cursor.row = row;
        cursor
    }
    pub(crate) fn cursor_style(&self) -> raw::GhosttyCursorStyle {
        self.state
            .borrow_mut()
            .frame()
            .map(|f| f.cursor.style)
            .unwrap_or(raw::GHOSTTY_CURSOR_STYLE_BLOCK)
    }
    pub(crate) fn encode_key(
        &mut self,
        event: &termlevel_ghostty_vt::KeyEvent<'_>,
    ) -> Result<Vec<u8>, String> {
        self.state
            .get_mut()
            .terminal
            .encode_key(event, true)
            .map_err(|e| e.to_string())
    }
    pub(crate) fn encode_text(&mut self, text: &str) -> Result<Vec<u8>, String> {
        let state = self.state.get_mut();
        let mut out = Vec::new();
        for ch in text.chars() {
            if ch.is_ascii_control() {
                out.push(ch as u8);
                continue;
            }
            let mut bytes = [0; 4];
            let mut event = termlevel_ghostty_vt::KeyEvent::press(
                raw::GHOSTTY_KEY_UNIDENTIFIED,
                ch.encode_utf8(&mut bytes),
            );
            event.unshifted_codepoint = Some(ch);
            out.extend(
                state
                    .terminal
                    .encode_key(&event, true)
                    .map_err(|e| e.to_string())?,
            );
        }
        Ok(out)
    }
    pub(crate) fn encode_paste(&self, text: &str) -> Result<Vec<u8>, String> {
        self.state
            .borrow()
            .terminal
            .encode_paste(text)
            .map_err(|e| e.to_string())
    }
    pub(crate) fn encode_focus(&self, focused: bool) -> Result<Vec<u8>, String> {
        self.state
            .borrow()
            .terminal
            .encode_focus(focused)
            .map_err(|e| e.to_string())
    }
    pub(crate) fn search(&self, query: &str) -> Vec<SearchHit> {
        if query.is_empty() {
            return Vec::new();
        }
        if query.is_ascii() && self.sync_deadline().is_none() {
            let mut state = self.state.borrow_mut();
            let range = state.range.clone();
            let cols = state.size.cols as usize;
            match state.terminal.find_all(query) {
                Ok(matches) => {
                    let mut hits = Vec::new();
                    for hit in matches {
                        for line in hit.start_row..=hit.end_row {
                            if !range.contains(&line) {
                                continue;
                            }
                            let col = if line == hit.start_row {
                                hit.start_column as usize
                            } else {
                                0
                            };
                            let end = if line == hit.end_row {
                                hit.end_column as usize
                            } else {
                                cols
                            };
                            hits.push(SearchHit {
                                line: line - range.start,
                                col,
                                cols: end.saturating_sub(col),
                            });
                        }
                    }
                    hits.sort_by_key(|hit| (hit.line, hit.col));
                    return hits;
                }
                Err(error) => state.errors.push(error.to_string()),
            }
        }
        // Preserve Unicode case folding for non-ASCII queries; native search
        // currently folds ASCII only. All matching and grid reads stay in Rust.
        let needle: Vec<_> = query.chars().flat_map(char::to_lowercase).collect();
        let mut hits = Vec::new();
        for line in 0..self.line_count() {
            let mut chars = Vec::new();
            let mut owners = Vec::new();
            for run in self.grid_runs(line) {
                for (offset, ch) in run.run.text.chars().enumerate() {
                    let col = run.col + if run.run.text.is_ascii() { offset } else { 0 };
                    let width = if run.run.text.is_ascii() {
                        1
                    } else {
                        run.cells
                    };
                    for lower in ch.to_lowercase() {
                        chars.push(lower);
                        owners.push((col, width));
                    }
                }
            }
            let mut at = 0;
            while at + needle.len() <= chars.len() {
                if chars[at..at + needle.len()] == needle {
                    let (col, _) = owners[at];
                    let (end, width) = owners[at + needle.len() - 1];
                    hits.push(SearchHit {
                        line,
                        col,
                        cols: end + width - col,
                    });
                    at += needle.len();
                } else {
                    at += 1;
                }
            }
        }
        hits
    }
    pub(crate) fn grid_runs(&self, index: usize) -> Vec<GridRun> {
        let mut state = self.state.borrow_mut();
        let cells = match state.cells(index) {
            Ok(cells) => cells,
            Err(error) => {
                state.errors.push(error);
                return Vec::new();
            }
        };
        let mut spans: Vec<GridRun> = Vec::new();
        for cell in cells {
            if matches!(
                cell.wide,
                raw::GHOSTTY_CELL_WIDE_SPACER_TAIL | raw::GHOSTTY_CELL_WIDE_SPACER_HEAD
            ) {
                continue;
            }
            let mut style = TerminalStyle::default();
            let convert = |color, effective: Rgb, default: Rgb| match color {
                StyleColor::Default => {
                    if effective != default {
                        Some(TerminalColor::Rgb(effective.r, effective.g, effective.b))
                    } else {
                        None
                    }
                }
                StyleColor::Palette(n) => {
                    let rgb = state.colors.palette[n as usize];
                    if rgb == indexed_color(n) {
                        Some(TerminalColor::Indexed(n))
                    } else {
                        Some(TerminalColor::Rgb(rgb.r, rgb.g, rgb.b))
                    }
                }
                StyleColor::Rgb(rgb) => Some(TerminalColor::Rgb(rgb.r, rgb.g, rgb.b)),
            };
            style.fg = convert(
                cell.style.foreground,
                state.colors.foreground,
                state.foreground,
            );
            style.bg = convert(
                cell.style.background,
                state.colors.background,
                state.background,
            );
            let flags = cell.style.flags;
            style.bold = flags & raw::GHOSTTY_STYLE_FLAG_BOLD != 0;
            style.dim = flags & raw::GHOSTTY_STYLE_FLAG_FAINT != 0;
            style.italic = flags & raw::GHOSTTY_STYLE_FLAG_ITALIC != 0;
            style.underline = flags & raw::GHOSTTY_STYLE_UNDERLINE_MASK != 0;
            style.strikethrough = flags & raw::GHOSTTY_STYLE_FLAG_STRIKETHROUGH != 0;
            style.inverse = flags & raw::GHOSTTY_STYLE_FLAG_INVERSE != 0;
            style.hidden = flags & raw::GHOSTTY_STYLE_FLAG_INVISIBLE != 0;
            let col = cell.column as usize;
            let width = if cell.wide == raw::GHOSTTY_CELL_WIDE_WIDE {
                2
            } else {
                1
            };
            let text = if cell.text.is_empty() {
                " ".to_owned()
            } else {
                cell.text
            };
            if text.is_ascii()
                && let Some(last) = spans.last_mut()
                && last.run.text.is_ascii()
                && last.run.style == style
                && last.col + last.cells == col
            {
                last.run.text.push_str(&text);
                last.cells += width;
            } else {
                spans.push(GridRun {
                    col,
                    cells: width,
                    run: TerminalRun::new(text, style),
                });
            }
        }
        if let Some(last) = spans.last_mut()
            && last.run.style == TerminalStyle::default()
        {
            let trimmed = last.run.text.trim_end_matches(' ').len();
            last.cells -= last.run.text.len() - trimmed;
            last.run.text.truncate(trimmed);
            if trimmed == 0 {
                spans.pop();
            }
        }
        spans
    }
}
impl TerminalEmulator for GhosttyEmulator {
    fn feed(&mut self, bytes: &[u8]) {
        let state = self.state.get_mut();
        if let Err(error) = state
            .terminal
            .write(bytes)
            .map_err(|e| e.to_string())
            .and_then(|_| state.update())
        {
            state.errors.push(error);
        }
    }
    fn resize(&mut self, size: TerminalSize) {
        let state = self.state.get_mut();
        state.size = size;
        if let Err(error) = state
            .terminal
            .resize(size.cols.max(1), size.rows.max(1), 0, 0)
            .map_err(|e| e.to_string())
            .and_then(|_| state.update())
        {
            state.errors.push(error);
        }
    }
    fn line_count(&self) -> usize {
        self.state.borrow().range.len()
    }
    fn line(&self, index: usize) -> TerminalLine {
        let mut runs: Vec<TerminalRun> = Vec::new();
        for span in self.grid_runs(index) {
            if let Some(last) = runs.last_mut()
                && last.style == span.run.style
            {
                last.text.push_str(&span.run.text);
            } else {
                runs.push(span.run);
            }
        }
        TerminalLine::new(runs)
    }
    fn cursor(&self) -> Option<TerminalCursor> {
        let mut state = self.state.borrow_mut();
        if !state.frame().ok()?.cursor.visible {
            return None;
        }
        drop(state);
        Some(self.input_cursor())
    }
    fn take_events(&mut self) -> Vec<TerminalEmulatorEvent> {
        self.backend_events()
            .into_iter()
            .filter_map(|event| match event {
                BackendEvent::Title(title) => Some(TerminalEmulatorEvent::Title(title)),
                BackendEvent::Bell => Some(TerminalEmulatorEvent::Bell),
                BackendEvent::Reply(bytes) => Some(TerminalEmulatorEvent::Reply(bytes)),
                BackendEvent::Error(error) => {
                    eprintln!("Ghostty preview: {error}");
                    None
                }
            })
            .collect()
    }
    fn modes(&self) -> TerminalModes {
        let state = self.state.borrow();
        let mut modes = TerminalModes::default();
        modes.application_cursor = state.terminal.dec_mode(1).unwrap_or(false);
        modes.bracketed_paste = state.terminal.dec_mode(2004).unwrap_or(false);
        modes
    }
}
#[cfg(test)]
mod palette_tests;
#[cfg(test)]
mod tests;
