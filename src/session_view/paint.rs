//! Owned rendering inputs shared by the engine and the AppKit surface.
//! No parser, PTY, or framework drawing commands cross this boundary.
use crate::terminal::{SearchHit, WindowSnap};
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
