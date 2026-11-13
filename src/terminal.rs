//! Alacritty's byte parser and grid behind NSH's native terminal view.
//!
//! Real sessions project cell spans into our Rust view; the preview uses
//! the NSH TerminalEmulator adapter.
//! The preview has view-scoped lifetime. Real PTYs and their emulator state
//! belong to the independent Rust SessionManager in sessions.rs.

use std::sync::{Arc, Mutex};

use alacritty_terminal::{
    Term,
    event::{Event, EventListener},
    grid::Dimensions,
    index::{Column, Line},
    term::{Config, Osc52, TermMode, cell::Flags},
    vte::ansi::{Color, NamedColor, Processor, Rgb},
};
use nsh_base::{
    TerminalColor, TerminalCursor, TerminalEmulator, TerminalEmulatorEvent,
    TerminalEmulatorFactory, TerminalLine, TerminalModes, TerminalRun, TerminalSize, TerminalStyle,
};
use nsh_core::prelude::*;

#[derive(Debug, Clone, PartialEq, Props)]
pub struct TerminalEngineProps {
    /// Workspace content whose terminal views use the Alacritty adapter.
    pub children: Prop<Children>,
}

/// Uses Alacritty for every NSH Terminal below this native scope.
/// Creating the scope or changing the theme never executes an OS command.
#[component]
pub fn TerminalEngine(props: TerminalEngineProps, _cx: Scope<NoEvent>) -> impl IntoElement {
    provide_context(TerminalEmulatorFactory::new(|size, scrollback| {
        Box::new(AlacrittyEmulator::new(size, scrollback))
    }));
    view().flex_1().child(props.children)
}

#[derive(Clone, Default)]
struct Events(Arc<Mutex<Vec<Event>>>);

impl EventListener for Events {
    fn send_event(&self, event: Event) {
        self.0.lock().unwrap().push(event);
    }
}

struct Size(TerminalSize);

impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        self.screen_lines()
    }

    fn screen_lines(&self) -> usize {
        self.0.rows as usize
    }

    fn columns(&self) -> usize {
        self.0.cols as usize
    }
}

pub(crate) struct AlacrittyEmulator {
    term: Term<Events>,
    parser: Processor,
    events: Events,
}

/// A text span positioned by Alacritty cells, rather than glyph advance.
#[derive(Clone, PartialEq)]
pub(crate) struct GridRun {
    pub col: usize,
    pub cells: usize,
    pub run: TerminalRun,
}

/// Shared fallback palette for painting and OSC color-query replies.
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

fn emulator_config(scrollback: usize) -> Config {
    Config {
        scrolling_history: scrollback,
        // NSH's key encoder has no Kitty protocol support. Clipboard
        // access is likewise outside this adapter's present contract.
        kitty_keyboard: false,
        osc52: Osc52::Disabled,
        ..Default::default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SearchHit {
    pub line: usize,
    pub col: usize,
    pub cols: usize,
}

impl AlacrittyEmulator {
    pub(crate) fn new(size: TerminalSize, scrollback: usize) -> Self {
        let events = Events::default();
        Self {
            term: Term::new(emulator_config(scrollback), &Size(size), events.clone()),
            parser: Processor::new(),
            events,
        }
    }

    /// Changes how many lines stay above the screen. A smaller limit drops the oldest lines.
    pub(crate) fn set_scrollback(&mut self, lines: usize) {
        self.term.set_options(emulator_config(lines));
        // set_options announces the current title. That event is not new program output.
        let _ = self.backend_events();
    }

    pub(crate) fn backend_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut *self.events.0.lock().unwrap())
    }

    pub(crate) fn flush_sync(&mut self, now: std::time::Instant, force: bool) -> bool {
        if self
            .parser
            .sync_timeout()
            .sync_timeout()
            .is_some_and(|deadline| force || now >= deadline)
        {
            self.parser.stop_sync(&mut self.term);
            true
        } else {
            false
        }
    }

    /// NSH 0.5's fixed ANSI palette, with Alacritty's program overrides.
    pub(crate) fn query_color(&self, index: usize, foreground: Rgb, background: Rgb) -> Rgb {
        if let Some(rgb) = self.term.colors()[index] {
            return rgb;
        }
        match index {
            0..=255 => indexed_color(index as u8),
            i if i == NamedColor::Background as usize => background,
            _ => foreground,
        }
    }

    pub(crate) fn screen_rows(&self) -> usize {
        self.term.grid().screen_lines()
    }

    pub(crate) fn input_cursor(&self) -> TerminalCursor {
        let point = self.term.grid().cursor.point;
        let mut cursor = TerminalCursor::default();
        cursor.col = point.column.0 as u16;
        cursor.row = point.line.0 as u16;
        cursor
    }

    /// Case-insensitive matches in scrollback and the screen, in reading order.
    pub(crate) fn search(&self, query: &str) -> Vec<SearchHit> {
        let needle: Vec<char> = query.chars().flat_map(|ch| ch.to_lowercase()).collect();
        if needle.is_empty() {
            return Vec::new();
        }
        let mut hits = Vec::new();
        for line in 0..self.line_count() {
            let runs = self.grid_runs(line);
            let mut glyphs: Vec<(usize, usize, String)> = Vec::new();
            for run in &runs {
                if run.run.text.is_ascii() {
                    for (offset, ch) in run.run.text.chars().enumerate() {
                        glyphs.push((run.col + offset, 1, ch.to_string()));
                    }
                } else if !run.run.text.is_empty() {
                    glyphs.push((run.col, run.cells, run.run.text.clone()));
                }
            }
            let mut flat = String::new();
            let mut owners = Vec::new();
            for (index, (_, _, text)) in glyphs.iter().enumerate() {
                for _ in text.chars() {
                    owners.push(index);
                }
                flat.push_str(text);
            }
            let folded: Vec<char> = flat.chars().flat_map(|ch| ch.to_lowercase()).collect();
            if folded.len() != owners.len() || folded.len() < needle.len() {
                continue;
            }
            let mut at = 0;
            while at + needle.len() <= folded.len() {
                if folded[at..at + needle.len()] == needle {
                    let start = owners[at];
                    let end = owners[at + needle.len() - 1];
                    hits.push(SearchHit {
                        line,
                        col: glyphs[start].0,
                        cols: glyphs[end].0 + glyphs[end].1 - glyphs[start].0,
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
        let grid = self.term.grid();
        if index >= grid.total_lines() {
            return Vec::new();
        }
        let row = &grid[Line(index as i32 - grid.history_size() as i32)];
        let mut spans: Vec<GridRun> = Vec::new();
        for col in 0..grid.columns() {
            let cell = &row[Column(col)];
            if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                continue;
            }
            let style = self.cell_style(cell);
            let cells = if cell.flags.contains(Flags::WIDE_CHAR) {
                2
            } else {
                1
            };
            let mut text = cell.c.to_string();
            if let Some(combining) = cell.zerowidth() {
                text.extend(combining);
            }
            // Shape ASCII runs together; non-ASCII graphemes retain their
            // explicit grid anchor regardless of fallback-font advances.
            if text.is_ascii()
                && let Some(last) = spans.last_mut()
                && last.run.text.is_ascii()
                && last.run.style == style
                && last.col + last.cells == col
            {
                last.run.text.push_str(&text);
                last.cells += cells;
            } else {
                spans.push(GridRun {
                    col,
                    cells,
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

    fn cell_style(&self, cell: &alacritty_terminal::term::cell::Cell) -> TerminalStyle {
        let mut style = TerminalStyle::default();
        style.fg = self.color(cell.fg);
        style.bg = self.color(cell.bg);
        style.bold = cell.flags.contains(Flags::BOLD);
        style.dim = cell.flags.contains(Flags::DIM);
        style.italic = cell.flags.contains(Flags::ITALIC);
        style.underline = cell.flags.intersects(Flags::ALL_UNDERLINES);
        style.strikethrough = cell.flags.contains(Flags::STRIKEOUT);
        style.inverse = cell.flags.contains(Flags::INVERSE);
        style.hidden = cell.flags.contains(Flags::HIDDEN);
        style
    }

    fn color(&self, color: Color) -> Option<TerminalColor> {
        let index = match color {
            Color::Spec(rgb) => return Some(TerminalColor::Rgb(rgb.r, rgb.g, rgb.b)),
            Color::Indexed(index) => index as usize,
            Color::Named(name) => name as usize,
        };
        // Honor palette/default-color overrides already parsed by Alacritty.
        if let Some(rgb) = self.term.colors()[index] {
            return Some(TerminalColor::Rgb(rgb.r, rgb.g, rgb.b));
        }
        match color {
            Color::Indexed(index) => Some(TerminalColor::Indexed(index)),
            Color::Named(name) if index < 16 => Some(TerminalColor::Indexed(name as u8)),
            // Unspecified defaults remain theme-dependent in NSH.
            _ => None,
        }
    }
}

impl TerminalEmulator for AlacrittyEmulator {
    fn feed(&mut self, bytes: &[u8]) {
        // One parser lives for the entire emulator: split UTF-8 and split
        // escape sequences are not decoded or reset at chunk boundaries.
        self.parser.advance(&mut self.term, bytes);
    }

    fn resize(&mut self, size: TerminalSize) {
        self.term.resize(Size(size));
    }

    fn line_count(&self) -> usize {
        self.term.grid().total_lines()
    }

    fn line(&self, index: usize) -> TerminalLine {
        let grid = self.term.grid();
        if index >= grid.total_lines() {
            return TerminalLine::default();
        }
        // Alacritty numbers history negatively, with screen row zero at 0.
        let row = &grid[Line(index as i32 - grid.history_size() as i32)];
        let mut runs: Vec<TerminalRun> = Vec::new();
        for col in 0..grid.columns() {
            let cell = &row[Column(col)];
            if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                continue;
            }
            let style = self.cell_style(cell);
            if runs.last().is_none_or(|run| run.style != style) {
                runs.push(TerminalRun::new(String::new(), style));
            }
            let text = &mut runs.last_mut().expect("a run was just created").text;
            text.push(cell.c);
            if let Some(combining) = cell.zerowidth() {
                text.extend(combining);
            }
        }
        // Keep styled spaces: their background and attributes are visible.
        if let Some(last) = runs.last_mut()
            && last.style == TerminalStyle::default()
        {
            last.text.truncate(last.text.trim_end_matches(' ').len());
            if last.text.is_empty() {
                runs.pop();
            }
        }
        TerminalLine::new(runs)
    }

    fn cursor(&self) -> Option<TerminalCursor> {
        if !self.term.mode().contains(TermMode::SHOW_CURSOR) {
            return None;
        }
        let point = self.term.grid().cursor.point;
        let mut cursor = TerminalCursor::default();
        cursor.col = point.column.0 as u16;
        cursor.row = point.line.0 as u16;
        Some(cursor)
    }

    fn take_events(&mut self) -> Vec<TerminalEmulatorEvent> {
        self.backend_events()
            .into_iter()
            .filter_map(|event| match event {
                Event::Title(title) => Some(TerminalEmulatorEvent::Title(title)),
                Event::ResetTitle => Some(TerminalEmulatorEvent::Title(String::new())),
                Event::Bell => Some(TerminalEmulatorEvent::Bell),
                Event::PtyWrite(data) => Some(TerminalEmulatorEvent::Reply(data.into_bytes())),
                _ => None,
            })
            .collect()
    }

    fn modes(&self) -> TerminalModes {
        let mut modes = TerminalModes::default();
        modes.application_cursor = self.term.mode().contains(TermMode::APP_CURSOR);
        modes.bracketed_paste = self.term.mode().contains(TermMode::BRACKETED_PASTE);
        modes
    }
}

#[cfg(test)]
mod palette_tests;
#[cfg(test)]
mod tests;
