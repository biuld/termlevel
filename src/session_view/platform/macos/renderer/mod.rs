//! macOS CoreText/CoreGraphics drawing and bounded main-thread caches.
use crate::session_view::paint::{self, PaintConfig, Scene, image_rect};
use objc2::rc::Retained;
use objc2_app_kit::{NSFont, NSFontWeightRegular};
use objc2_core_foundation::{
    CFArray, CFAttributedString, CFBoolean, CFData, CFDictionary, CFMutableAttributedString,
    CFNumber, CFRange, CFRetained, CFString, CFType, CGAffineTransform, CGPoint, CGRect, CGSize,
};
use objc2_core_graphics::{
    CGBitmapInfo, CGColorRenderingIntent, CGColorSpace, CGContext, CGDataProvider, CGImage,
    CGImageAlphaInfo, CGImageByteOrderInfo, CGMutablePath, CGPath,
};
use objc2_core_text::{
    CTFont, CTFontDescriptor, CTFontSymbolicTraits, CTLine, CTRun, kCTFontAttributeName,
    kCTFontCascadeListAttribute, kCTFontFamilyNameAttribute,
    kCTForegroundColorFromContextAttributeName, kCTLigatureAttributeName,
};
use std::{collections::HashMap, ptr::NonNull, rc::Rc, sync::Arc, time::Instant};

/// Only immutable layout is cached: selection, color and search are painted
/// from the current scene. Bounded by both entry count and source bytes.
struct Shaped {
    line: CFRetained<CTLine>,
    graphics_path: Option<CFRetained<CGPath>>,
    baseline: f64,
    horizontal: (f64, f64),
    advance: f64,
    fallback: bool,
}
#[derive(Hash, PartialEq, Eq, Clone)]
struct TextKey {
    text: String,
    face: u8,
}
struct CachedText {
    shaped: Rc<Shaped>,
    used: u64,
}
#[derive(Clone, Copy)]
struct GraphicsPlanes {
    blocks: CGRect,
    boxes: CGRect,
}
#[derive(Default)]
pub(super) struct Renderer {
    font_key: Option<(String, u32)>,
    fonts: HashMap<u8, CFRetained<CTFont>>,
    lines: HashMap<TextKey, CachedText>,
    graphics_planes: Option<GraphicsPlanes>,
    text_bytes: usize,
    clock: u64,
    images: Vec<(Arc<termlevel_ghostty_vt::Pixels>, CFRetained<CGImage>)>,
    frames: usize,
    elapsed: f64,
    maximum: f64,
    report: Option<Instant>,
}
impl Renderer {
    pub(super) fn draw(&mut self, context: &CGContext, scene: &Scene) {
        let start = Instant::now();
        let cfg = &scene.config;
        let snap = &scene.snap;
        let key = (cfg.family.clone(), cfg.size.to_bits());
        if self.font_key.as_ref() != Some(&key) {
            self.font_key = Some(key);
            self.fonts.clear();
            self.lines.clear();
            self.graphics_planes = None;
            self.text_bytes = 0;
        }
        let c = Some(context);
        CGContext::save_g_state(c);
        CGContext::clip_to_rect(
            c,
            rect(
                0.0,
                0.0,
                snap.frame.cols as f64 * cfg.cw as f64,
                snap.frame.rows as f64 * cfg.lh as f64,
            ),
        );
        // Default background is the parent NSH view's background. Layer 0 must
        // stay below explicit cell backgrounds, not below an opaque clear.
        CGContext::clear_rect(
            c,
            rect(
                0.0,
                0.0,
                snap.frame.cols as f64 * cfg.cw as f64,
                snap.frame.rows as f64 * cfg.lh as f64,
            ),
        );
        self.images
            .retain(|(pixels, _)| snap.images.iter().any(|p| Arc::ptr_eq(pixels, &p.pixels)));
        self.draw_images(context, scene, 0);
        // Background geometry is independent of text shaping. Fractional cell
        // widths and separately antialiased run edges otherwise expose seams,
        // especially where each wide grapheme owns a separate text run.
        CGContext::save_g_state(c);
        CGContext::set_should_antialias(c, false);
        for (row, spans) in snap.rows.iter().enumerate() {
            let mut spans = spans.iter().peekable();
            while let Some(span) = spans.next() {
                let bg = paint::run_background(span, cfg);
                if let Some(bg) = bg {
                    let mut end = span.col + span.cells;
                    while let Some(next) = spans.peek()
                        && next.col == end
                        && paint::run_background(next, cfg) == Some(bg)
                    {
                        end += next.cells;
                        spans.next();
                    }
                    let x = span.col as f64 * cfg.cw as f64;
                    let right = end as f64 * cfg.cw as f64;
                    fill(
                        context,
                        bg,
                        1.0,
                        pixel_rect(
                            context,
                            rect(x, row as f64 * cfg.lh as f64, right - x, cfg.lh as f64),
                        ),
                    );
                }
            }
        }
        CGContext::restore_g_state(c);
        self.draw_images(context, scene, 1);
        for (index, hit) in scene.hits.iter().enumerate() {
            if hit.line >= snap.origin && hit.line < snap.origin + snap.rows.len() {
                fill(
                    context,
                    cfg.search,
                    if index == scene.current { 0.28 } else { 0.12 },
                    rect(
                        hit.col as f64 * cfg.cw as f64,
                        (hit.line - snap.origin) as f64 * cfg.lh as f64,
                        hit.cols as f64 * cfg.cw as f64,
                        cfg.lh as f64,
                    ),
                );
            }
        }
        for (row, spans) in snap.rows.iter().enumerate() {
            for span in spans {
                let style = span.run.style;
                if style.hidden || span.run.text.is_empty() || span.cells == 0 {
                    continue;
                }
                let shaped = self.shape(cfg, &span.run.text, style.bold, style.italic);
                let plane = shaped
                    .graphics_path
                    .as_ref()
                    .and_then(|_| self.graphics_plane(cfg, &span.run.text));
                let (x, y, w) = (
                    span.col as f64 * cfg.cw as f64,
                    row as f64 * cfg.lh as f64,
                    span.cells as f64 * cfg.cw as f64,
                );
                let cell = if plane.is_some() {
                    pixel_rect(context, rect(x, y, w, cfg.lh as f64))
                } else {
                    rect(x, y, w, cfg.lh as f64)
                };
                CGContext::save_g_state(c);
                CGContext::clip_to_rect(c, cell);
                set_color(
                    context,
                    cfg.colors(style).0,
                    if style.dim { 0.6 } else { 1.0 },
                );
                CGContext::save_g_state(c);
                if let Some(plane) = plane {
                    let block = span
                        .run
                        .text
                        .chars()
                        .next()
                        .is_some_and(|ch| ch >= '\u{2580}');
                    // AppKit rasterizes a standalone straight outline differently
                    // from junction outlines when antialiasing is disabled.
                    // Blocks must remain pixel-solid at their shared edges.
                    CGContext::set_should_antialias(c, !block);
                    Renderer::map_graphics(context, cell, plane, block);
                } else {
                    let (scale, offset) = shaped.fit(w);
                    CGContext::translate_ctm(
                        c,
                        x + offset,
                        y + cfg.lh as f64 / 2.0 + shaped.baseline,
                    );
                    CGContext::scale_ctm(c, scale, -scale);
                }
                CGContext::set_text_matrix(
                    c,
                    CGAffineTransform {
                        a: 1.0,
                        b: 0.0,
                        c: 0.0,
                        d: 1.0,
                        tx: 0.0,
                        ty: 0.0,
                    },
                );
                CGContext::set_text_position(c, 0.0, 0.0);
                if plane.is_some()
                    && let Some(path) = &shaped.graphics_path
                {
                    // Draw outlines without the text rasterizer's hinting,
                    // which can shrink ink away from an exact cell edge.
                    CGContext::begin_path(c);
                    CGContext::add_path(c, Some(path));
                    CGContext::fill_path(c);
                } else {
                    // SAFETY: immutable CTLine and live AppKit drawing context.
                    unsafe {
                        shaped.line.draw(context);
                    }
                }
                CGContext::restore_g_state(c);
                if style.underline || span.hyperlink.is_some() {
                    CGContext::fill_rect(c, rect(x, y + (cfg.lh as f64 - 1.0).max(0.0), w, 1.0));
                }
                if style.strikethrough {
                    CGContext::fill_rect(c, rect(x, y + cfg.lh as f64 / 2.0 - 0.5, w, 1.0));
                }
                CGContext::restore_g_state(c);
            }
        }
        self.draw_images(context, scene, 2);
        CGContext::restore_g_state(c);
        self.frames += 1;
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        self.elapsed += ms;
        self.maximum = self.maximum.max(ms);
        let report = self.report.get_or_insert(start);
        if report.elapsed().as_secs_f32() >= 1.0 {
            #[cfg(debug_assertions)]
            {
                let line = format!(
                    "terminal native draw: {} frames, mean {:.2} ms, max {:.2} ms, {} cached lines / {} fonts",
                    self.frames,
                    self.elapsed / self.frames as f64,
                    self.maximum,
                    self.lines.len(),
                    self.fonts.len()
                );
                eprintln!("{line}");
                crate::session_view::diag::line(&line);
            }
            self.frames = 0;
            self.elapsed = 0.0;
            self.maximum = 0.0;
            *report = Instant::now();
        }
    }
    fn draw_images(&mut self, context: &CGContext, scene: &Scene, layer: u32) {
        for p in scene.snap.images.iter().filter(|p| p.layer == layer) {
            let Some([x, y, w, h]) = image_rect(&scene.snap, &scene.config, p) else {
                continue;
            };
            let image = if let Some((_, image)) = self
                .images
                .iter()
                .find(|(pixels, _)| Arc::ptr_eq(pixels, &p.pixels))
            {
                image.clone()
            } else {
                let Some(image) = rgba_image(&p.pixels) else {
                    continue;
                };
                self.images.push((p.pixels.clone(), image.clone()));
                image
            };
            let sx = w / p.source_width as f64;
            let sy = h / p.source_height as f64;
            let c = Some(context);
            CGContext::save_g_state(c);
            CGContext::clip_to_rect(c, rect(x, y, w, h));
            let ih = p.pixels.height as f64 * sy;
            CGContext::translate_ctm(
                c,
                x - p.source_x as f64 * sx,
                y - p.source_y as f64 * sy + ih,
            );
            CGContext::scale_ctm(c, 1.0, -1.0);
            CGContext::draw_image(
                c,
                rect(0.0, 0.0, p.pixels.width as f64 * sx, ih),
                Some(&image),
            );
            CGContext::restore_g_state(c);
        }
    }
}
fn rect(x: f64, y: f64, w: f64, h: f64) -> CGRect {
    CGRect::new(CGPoint::new(x, y), CGSize::new(w, h))
}
/// Round shared edges, not widths, in backing pixels (including the view's
/// fractional device origin). Adjacent colors then cover each pixel once.
fn pixel_rect(context: &CGContext, r: CGRect) -> CGRect {
    let device = CGContext::convert_rect_to_device_space(Some(context), r);
    let x = device.origin.x.round();
    let y = device.origin.y.round();
    let right = (device.origin.x + device.size.width).round();
    let bottom = (device.origin.y + device.size.height).round();
    CGContext::convert_rect_to_user_space(Some(context), rect(x, y, right - x, bottom - y))
}
fn set_color(c: &CGContext, color: nsh_core::Hsla, alpha: f64) {
    let color = nsh_core::Rgba::from(color);
    CGContext::set_rgb_fill_color(
        Some(c),
        color.r as f64,
        color.g as f64,
        color.b as f64,
        color.a as f64 * alpha,
    );
}
fn fill(c: &CGContext, color: nsh_core::Hsla, alpha: f64, r: CGRect) {
    set_color(c, color, alpha);
    CGContext::fill_rect(Some(c), r);
}

fn rgba_image(pixels: &termlevel_ghostty_vt::Pixels) -> Option<CFRetained<CGImage>> {
    let bytes = CFData::from_bytes(&pixels.rgba);
    let provider = CGDataProvider::with_cf_data(Some(&bytes))?;
    let space = CGColorSpace::new_device_rgb()?;
    // Ghostty's owned pixels are straight RGBA. CGImage accepts straight alpha;
    // no repeated per-frame premultiplication or full terminal raster upload.
    unsafe {
        CGImage::new(
            pixels.width as usize,
            pixels.height as usize,
            8,
            32,
            pixels.width as usize * 4,
            Some(&space),
            CGBitmapInfo(CGImageAlphaInfo::Last.0 | CGImageByteOrderInfo::Order32Big.0),
            Some(&provider),
            std::ptr::null(),
            true,
            CGColorRenderingIntent::RenderingIntentDefault,
        )
    }
}

mod shaping;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod glyph_tests;

#[cfg(test)]
mod bundled_fonts;
