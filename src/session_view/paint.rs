//! Owned rendering inputs shared by the engine and the AppKit surface.
//! No parser, PTY, or framework drawing commands cross this boundary.
use crate::terminal::{GridRun, SearchHit, WindowSnap};
use nsh_base::{TerminalColor, TerminalStyle};
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone, PartialEq)]
pub(super) struct PaintConfig {
    pub cw: f32,
    pub lh: f32,
    pub family: String,
    pub size: f32,
    pub foreground: Hsla,
    pub background: Hsla,
    pub selection: Hsla,
    pub search: Hsla,
}
impl PaintConfig {
    pub fn colors(&self, style: TerminalStyle) -> (Hsla, Option<Hsla>) {
        let fg = style.fg.map(resolve).unwrap_or(self.foreground);
        let bg = style.bg.map(resolve);
        if style.inverse {
            (bg.unwrap_or(self.background), Some(fg))
        } else {
            (fg, bg)
        }
    }
}
/// Solid fill for one grid run. `None` means the terminal default shows through.
pub(super) fn run_background(span: &GridRun, cfg: &PaintConfig) -> Option<Hsla> {
    if span.selected {
        Some(cfg.selection)
    } else {
        cfg.colors(span.run.style).1
    }
}

/// Opaque field-editor fill while composing. The editor covers the composing
/// text, so it uses the cursor cell's painted background.
pub(super) fn composition_background(snap: &WindowSnap, cfg: &PaintConfig) -> Hsla {
    let Some((col, row)) = snap.frame.cursor.position else {
        return cfg.background;
    };
    let col = col as usize;
    snap.rows
        .get(row as usize)
        .and_then(|runs| {
            runs.iter()
                .find(|run| run.col <= col && col < run.col + run.cells)
        })
        .and_then(|run| run_background(run, cfg))
        .unwrap_or(cfg.background)
}

fn resolve(color: TerminalColor) -> Hsla {
    let (r, g, b) = match color {
        TerminalColor::Rgb(r, g, b) => (r, g, b),
        TerminalColor::Indexed(n) => {
            let c = crate::terminal::indexed_color(n);
            (c.r, c.g, c.b)
        }
    };
    nsh_core::color::rgb((r as u32) << 16 | (g as u32) << 8 | b as u32).into()
}

pub(super) struct Scene {
    pub snap: Arc<WindowSnap>,
    pub config: PaintConfig,
    pub hits: Vec<SearchHit>,
    pub current: usize,
}

/// Placement in points; shared with clipping tests and the native renderer.
pub(super) fn image_rect(
    snap: &WindowSnap,
    cfg: &PaintConfig,
    p: &termlevel_ghostty_vt::ImagePlacement,
) -> Option<[f64; 4]> {
    if p.source_width == 0 || p.source_height == 0 {
        return None;
    }
    let dx = cfg.cw / cfg.cw.round().max(1.0);
    let dy = cfg.lh / cfg.lh.round().max(1.0);
    let x = p.column as f32 * cfg.cw + p.offset_x as f32 * dx;
    let y = (p.row as f32 - snap.origin as f32) * cfg.lh + p.offset_y as f32 * dy;
    let w = p.width as f32 * dx;
    let h = p.height as f32 * dy;
    (x < snap.frame.cols as f32 * cfg.cw
        && y < snap.frame.rows as f32 * cfg.lh
        && x + w > 0.0
        && y + h > 0.0)
        .then_some([x as f64, y as f64, w as f64, h as f64])
}

/// Metadata only: no glyphs or paint operations. NSH's model/test agent can
/// still target visible terminal rows although pixels belong to our NSView.
pub(super) fn semantics(snap: &WindowSnap, cw: f32, lh: f32) -> Painter {
    let mut metadata = Painter::new();
    for (row, label) in snap.text.split('\n').enumerate() {
        if label.is_empty() {
            continue;
        }
        let width = snap.rows[row]
            .iter()
            .rev()
            .find(|span| !span.run.style.hidden && !span.run.text.trim().is_empty())
            .map_or(1, |span| span.col + span.cells);
        metadata = metadata.node(nsh_core::canvas::CanvasNode::new(
            Bounds {
                origin: Point::new(Points(0.0), Points(row as f32 * lh)),
                size: Size::new(Points(width as f32 * cw), Points(lh)),
            },
            AccessibilityRole::Text,
            label,
        ));
    }
    metadata
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{session_view::window::Window, terminal::GhosttyEmulator};
    use nsh_base::{TerminalColor, TerminalEmulator, TerminalRun, TerminalSize, TerminalStyle};

    fn config() -> PaintConfig {
        PaintConfig {
            cw: 8.0,
            lh: 21.0,
            family: "Menlo".into(),
            size: 13.0,
            foreground: nsh_core::color::rgb(0xeeeeee).into(),
            background: nsh_core::color::rgb(0x18191b).into(),
            selection: nsh_core::color::rgb(0x0870e4).into(),
            search: nsh_core::color::rgb(0xffffff).into(),
        }
    }

    fn run(text: &str, style: TerminalStyle, selected: bool) -> GridRun {
        GridRun {
            col: 0,
            cells: text.chars().count(),
            selected,
            hyperlink: None,
            run: TerminalRun::new(text, style),
        }
    }

    #[test]
    fn run_background_uses_the_cell_color_unless_selected() {
        let cfg = config();
        let mut gray = TerminalStyle::default();
        gray.bg = Some(TerminalColor::Rgb(40, 42, 46));
        assert_eq!(
            run_background(&run(" ", gray, false), &cfg),
            Some(resolve(TerminalColor::Rgb(40, 42, 46)))
        );
        assert_eq!(
            run_background(&run(" ", TerminalStyle::default(), false), &cfg),
            None
        );
        assert_eq!(
            run_background(&run(" ", gray, true), &cfg),
            Some(cfg.selection)
        );
    }

    #[test]
    fn composition_fill_follows_the_cursor_cell() {
        let cfg = config();
        let mut emulator = GhosttyEmulator::new(TerminalSize::new(12, 1), 10);
        // Gray across the row, then leave the cursor inside that run.
        emulator.feed(b"\x1b[48;2;40;42;46m> prompt    \x1b[4D");
        let snap = emulator
            .begin_window(&mut Window::default(), false, None)
            .unwrap()
            .finish();
        assert_eq!(
            nsh_core::Rgba::from(composition_background(&snap, &cfg)),
            nsh_core::Rgba::from(resolve(TerminalColor::Rgb(40, 42, 46)))
        );

        let mut plain = GhosttyEmulator::new(TerminalSize::new(12, 1), 10);
        plain.feed(b"hi");
        let snap = plain
            .begin_window(&mut Window::default(), false, None)
            .unwrap()
            .finish();
        assert_eq!(
            nsh_core::Rgba::from(composition_background(&snap, &cfg)),
            nsh_core::Rgba::from(cfg.background)
        );
    }
}
