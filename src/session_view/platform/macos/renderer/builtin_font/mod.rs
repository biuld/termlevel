// Adapted from Alacritty builtin_font.rs, Apache-2.0; see NOTICE.md.
use std::{cmp, mem, ops};
mod canvas;
mod lines;
mod shapes;
#[cfg(test)]
mod tests;
use canvas::*;

#[derive(Clone, Copy)]
struct Metrics {
    line_height: f64,
    average_advance: f64,
    descent: f64,
    stroke_width: usize,
}
struct Delta<T> {
    x: T,
    y: T,
}
enum BitmapBuffer {
    Rgb(Vec<u8>),
}
pub(super) struct RasterizedGlyph {
    pub(super) width: i32,
    pub(super) height: i32,
    pub(super) top: i32,
    pub(super) left: i32,
    buffer: BitmapBuffer,
}
impl RasterizedGlyph {
    pub(super) fn rgb(&self) -> &[u8] {
        match &self.buffer {
            BitmapBuffer::Rgb(bytes) => bytes,
        }
    }
}
const COLOR_FILL_ALPHA_STEP_1: Pixel = Pixel {
    _r: 192,
    _g: 192,
    _b: 192,
};
const COLOR_FILL_ALPHA_STEP_2: Pixel = Pixel {
    _r: 128,
    _g: 128,
    _b: 128,
};
const COLOR_FILL_ALPHA_STEP_3: Pixel = Pixel {
    _r: 64,
    _g: 64,
    _b: 64,
};

/// Default color used for filling.
const COLOR_FILL: Pixel = Pixel {
    _r: 255,
    _g: 255,
    _b: 255,
};

const POWERLINE_TRIANGLE_LTR: char = '\u{e0b0}';
const POWERLINE_ARROW_LTR: char = '\u{e0b1}';
const POWERLINE_TRIANGLE_RTL: char = '\u{e0b2}';
const POWERLINE_ARROW_RTL: char = '\u{e0b3}';
fn builtin_glyph(
    character: char,
    metrics: &Metrics,
    offset: &Delta<i8>,
    glyph_offset: &Delta<i8>,
) -> Option<RasterizedGlyph> {
    let mut glyph = match character {
        // Box drawing characters and block elements.
        '\u{2500}'..='\u{259f}' | '\u{1fb00}'..='\u{1fb3b}' | '\u{1fb82}'..='\u{1fb8b}' => {
            box_drawing(character, metrics, offset)
        }
        // Powerline symbols: '','','',''
        POWERLINE_TRIANGLE_LTR..=POWERLINE_ARROW_RTL => {
            powerline_drawing(character, metrics, offset)?
        }
        _ => return None,
    };

    // Since we want to ignore `glyph_offset` for the built-in font, subtract it to compensate its
    // addition when loading glyphs in the renderer.
    glyph.left -= glyph_offset.x as i32;
    glyph.top -= glyph_offset.y as i32;

    Some(glyph)
}
fn box_drawing(character: char, metrics: &Metrics, offset: &Delta<i8>) -> RasterizedGlyph {
    // Ensure that width and height is at least one.
    let height = (metrics.line_height as i32 + offset.y as i32).max(1) as usize;
    let width = (metrics.average_advance as i32 + offset.x as i32).max(1) as usize;
    let stroke_size = calculate_stroke_size(metrics.stroke_width);

    // Certain symbols require larger canvas than the cell itself, since for proper contiguous
    // lines they require drawing on neighbour cells. So treat them specially early on and handle
    // 'normal' characters later.
    let mut canvas = match character {
        // Diagonals: '╱', '╲', '╳'.
        '\u{2571}'..='\u{2573}' => {
            // Last coordinates.
            let x_end = width as f32;
            let mut y_end = height as f32;

            let top = height as i32 + metrics.descent as i32 + stroke_size as i32;
            let height = height + 2 * stroke_size;
            let mut canvas = Canvas::new(width, height + 2 * stroke_size);

            // The offset that we should take into account when drawing, since we've enlarged
            // buffer vertically by twice of that amount.
            let y_offset = stroke_size as f32;
            y_end += y_offset;

            let k = y_end / x_end;
            let f_x = |x: f32, h: f32| -> f32 { -k * x + h + y_offset };
            let g_x = |x: f32, h: f32| -> f32 { k * x + h + y_offset };

            let from_x = 0.;
            let to_x = x_end + 1.;
            for stroke_size in 0..2 * stroke_size {
                let stroke_size = stroke_size as f32 / 2.;
                if character == '\u{2571}' || character == '\u{2573}' {
                    let h = y_end - stroke_size;
                    let from_y = f_x(from_x, h);
                    let to_y = f_x(to_x, h);
                    canvas.draw_line(from_x, from_y, to_x, to_y);
                }
                if character == '\u{2572}' || character == '\u{2573}' {
                    let from_y = g_x(from_x, stroke_size);
                    let to_y = g_x(to_x, stroke_size);
                    canvas.draw_line(from_x, from_y, to_x, to_y);
                }
            }

            let buffer = BitmapBuffer::Rgb(canvas.into_raw());
            return RasterizedGlyph {
                top,
                left: 0,
                height: height as i32,
                width: width as i32,
                buffer,
            };
        }
        _ => Canvas::new(width, height),
    };

    if !lines::draw(character, &mut canvas, width, height, stroke_size)
        && !shapes::draw(character, &mut canvas, width, height, stroke_size)
    {
        unreachable!();
    }
    let top = height as i32 + metrics.descent as i32;
    let buffer = BitmapBuffer::Rgb(canvas.into_raw());
    RasterizedGlyph {
        top,
        left: 0,
        height: height as i32,
        width: width as i32,
        buffer,
    }
}
fn powerline_drawing(
    character: char,
    metrics: &Metrics,
    offset: &Delta<i8>,
) -> Option<RasterizedGlyph> {
    let height = (metrics.line_height as i32 + offset.y as i32) as usize;
    let width = (metrics.average_advance as i32 + offset.x as i32) as usize;
    let extra_thickness = calculate_stroke_size(metrics.stroke_width) as i32 - 1;

    let mut canvas = Canvas::new(width, height);

    let slope = 1;
    let top_y = 1;
    let bottom_y = height as i32 - top_y - 1;

    // Start with offset `1` and draw until the intersection of the f(x) = slope * x + 1 and
    // g(x) = H - slope * x - 1 lines. The intersection happens when f(x) = g(x), which is at
    // x = (H - 2) / (2 * slope).
    let x_intersection = (height as i32 + 1) / 2 - 1;

    // Don't use built-in font if we'd cut the tip too much, for example when the font is really
    // narrow.
    if x_intersection - width as i32 > 1 {
        return None;
    }

    let top_line = (0..x_intersection).map(|x| line_equation(slope, x, top_y));
    let bottom_line = (0..x_intersection).map(|x| line_equation(-slope, x, bottom_y));

    // Inner lines to make arrows thicker.
    let mut top_inner_line = (0..x_intersection - extra_thickness)
        .map(|x| line_equation(slope, x, top_y + extra_thickness));
    let mut bottom_inner_line = (0..x_intersection - extra_thickness)
        .map(|x| line_equation(-slope, x, bottom_y - extra_thickness));

    // NOTE: top_line and bottom_line have the same amount of iterations.
    for (p1, p2) in top_line.zip(bottom_line) {
        if character == POWERLINE_TRIANGLE_LTR || character == POWERLINE_TRIANGLE_RTL {
            canvas.draw_rect(0., p1.1, p1.0 + 1., 1., COLOR_FILL);
            canvas.draw_rect(0., p2.1, p2.0 + 1., 1., COLOR_FILL);
        } else if character == POWERLINE_ARROW_LTR || character == POWERLINE_ARROW_RTL {
            let p3 = top_inner_line.next().unwrap_or(p2);
            let p4 = bottom_inner_line.next().unwrap_or(p1);

            // If we can't fit the entire arrow in the cell, we cut off the tip of the arrow by
            // drawing a rectangle between the two lines.
            if p1.0 as usize + 1 == width {
                canvas.draw_rect(p1.0, p1.1, 1., p2.1 - p1.1 + 1., COLOR_FILL);
                break;
            } else {
                canvas.draw_rect(p1.0, p1.1, 1., p3.1 - p1.1 + 1., COLOR_FILL);
                canvas.draw_rect(p4.0, p4.1, 1., p2.1 - p4.1 + 1., COLOR_FILL);
            }
        }
    }

    if character == POWERLINE_TRIANGLE_RTL || character == POWERLINE_ARROW_RTL {
        canvas.flip_horizontal();
    }

    let top = height as i32 + metrics.descent as i32;
    let buffer = BitmapBuffer::Rgb(canvas.into_raw());
    Some(RasterizedGlyph {
        top,
        left: 0,
        height: height as i32,
        width: width as i32,
        buffer,
    })
}

pub(super) fn glyph(
    character: char,
    width: usize,
    height: usize,
    stroke_width: usize,
) -> Option<RasterizedGlyph> {
    let metrics = Metrics {
        line_height: height as f64,
        average_advance: width as f64,
        descent: 0.0,
        stroke_width,
    };
    builtin_glyph(
        character,
        &metrics,
        &Delta { x: 0, y: 0 },
        &Delta { x: 0, y: 0 },
    )
}

pub(super) fn supports(character: char) -> bool {
    matches!(character, '\u{2500}'..='\u{259f}' | '\u{1fb00}'..='\u{1fb3b}' | '\u{1fb82}'..='\u{1fb8b}' | '\u{e0b0}'..='\u{e0b3}')
}
