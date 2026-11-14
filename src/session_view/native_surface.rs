//! Termlevel's renderer: an actual NSView embedded through NSH's public hook.
//! The engine publishes owned snapshots; only the main thread owns CoreText/CG
//! objects. The scalar prop is an invalidation token, never serialized cells.
use super::paint::{PaintConfig, Scene, image_rect};
use nsh_core::JsonValue;
use nsh_platform_macos::{Components, NativeComponent};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained};
use objc2_app_kit::{NSFont, NSFontWeightRegular, NSGraphicsContext, NSView};
use objc2_core_foundation::{
    CFAttributedString, CFBoolean, CFData, CFDictionary, CFMutableAttributedString, CFNumber,
    CFRange, CFRetained, CFString, CFType, CGAffineTransform, CGPoint, CGRect, CGSize,
};
use objc2_core_graphics::{
    CGBitmapInfo, CGColorRenderingIntent, CGColorSpace, CGContext, CGDataProvider, CGImage,
    CGImageAlphaInfo, CGImageByteOrderInfo,
};
use objc2_core_text::{
    CTFont, CTFontDescriptor, CTFontSymbolicTraits, CTLine, kCTFontAttributeName,
    kCTFontFamilyNameAttribute, kCTForegroundColorFromContextAttributeName,
    kCTLigatureAttributeName,
};
use objc2_foundation::{NSObjectProtocol, NSPoint, NSRect};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    ptr::NonNull,
    rc::Rc,
    sync::{
        Arc, Mutex, OnceLock, Weak,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

const NAME: &str = "termlevel.terminal-surface";
type Slots = HashMap<u64, Weak<Slot>>;
fn slots() -> &'static Mutex<Slots> {
    static SLOTS: OnceLock<Mutex<Slots>> = OnceLock::new();
    SLOTS.get_or_init(Mutex::default)
}
struct Slot {
    id: u64,
    scene: Mutex<Option<Arc<Scene>>>,
    revision: AtomicU64,
}
impl Drop for Slot {
    fn drop(&mut self) {
        slots().lock().unwrap().remove(&self.id);
    }
}
#[derive(Clone)]
pub(super) struct Publisher(Arc<Slot>);
impl Publisher {
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let slot = Arc::new(Slot {
            id: NEXT.fetch_add(1, Ordering::Relaxed),
            scene: Mutex::new(None),
            revision: AtomicU64::new(0),
        });
        slots()
            .lock()
            .unwrap()
            .insert(slot.id, Arc::downgrade(&slot));
        Self(slot)
    }
    pub fn publish(&self, scene: Scene) -> JsonValue {
        *self.0.scene.lock().unwrap() = Some(Arc::new(scene));
        let rev = self.0.revision.fetch_add(1, Ordering::Relaxed) + 1;
        JsonValue::Array(vec![self.0.id.into(), rev.into()])
    }
}

pub(crate) fn register(components: &Components) {
    components.register(
        NAME,
        Rc::new(|_| {
            let mtm = MainThreadMarker::new().expect("native components mount on main thread");
            let view = Surface::alloc(mtm).set_ivars(Ivars::default());
            // SAFETY: initialize the allocated NSView subclass once.
            let view: Retained<Surface> =
                unsafe { msg_send![super(view), initWithFrame: NSRect::ZERO] };
            view.setWantsLayer(true);
            Box::new(Component(view))
        }),
    );
}
pub(super) fn element() -> nsh_core::element::NativeView {
    nsh_core::element::native(NAME)
}
struct Component(Retained<Surface>);
impl NativeComponent for Component {
    fn view(&self) -> Retained<NSView> {
        self.0.clone().into_super()
    }
    fn set_prop(&mut self, name: &str, value: Option<&JsonValue>) {
        if name != "frame" {
            return;
        }
        let id = value.and_then(|v| v.get(0)).and_then(JsonValue::as_u64);
        let slot = id.and_then(|id| slots().lock().unwrap().get(&id).and_then(Weak::upgrade));
        *self.0.ivars().slot.borrow_mut() = slot;
        self.0.setNeedsDisplay(true);
    }
}
#[derive(Default)]
struct Ivars {
    frames: Cell<u32>,
    slot: RefCell<Option<Arc<Slot>>>,
    renderer: RefCell<Renderer>,
}
define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "TermlevelTerminalSurface"]
    #[ivars = Ivars]
    struct Surface;
    unsafe impl NSObjectProtocol for Surface {}
    impl Surface {
        #[unsafe(method(isFlipped))]
        fn flipped(&self) -> bool { true }
        // The NSH grid parent owns all pointer handling. This drawing view must
        // not become a second mouse responder or steal the field editor.
        #[unsafe(method_id(hitTest:))]
        fn hit_test(&self, _point: NSPoint) -> Option<Retained<NSView>> { None }
        #[unsafe(method(drawRect:))]
        fn draw(&self, _rect: NSRect) {
            let scene = self.ivars().slot.borrow().as_ref()
                .and_then(|s| s.scene.lock().unwrap().clone());
            let (Some(scene), Some(context)) = (scene, NSGraphicsContext::currentContext()) else { return; };
            self.ivars().renderer.borrow_mut().draw(&context.CGContext(), &scene);
            self.ivars().frames.set(self.ivars().frames.get().saturating_add(1));
        }
    }
);

/// Only immutable layout is cached: selection, color and search are painted
/// from the current scene. Bounded by both entry count and source bytes.
struct Shaped {
    line: CFRetained<CTLine>,
    baseline: f64,
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
#[derive(Default)]
struct Renderer {
    font_key: Option<(String, u32)>,
    fonts: HashMap<u8, CFRetained<CTFont>>,
    lines: HashMap<TextKey, CachedText>,
    text_bytes: usize,
    clock: u64,
    images: Vec<(Arc<termlevel_ghostty_vt::Pixels>, CFRetained<CGImage>)>,
    frames: usize,
    elapsed: f64,
    maximum: f64,
    report: Option<Instant>,
}
impl Renderer {
    fn shape(&mut self, cfg: &PaintConfig, text: &str, bold: bool, italic: bool) -> Rc<Shaped> {
        let face = u8::from(bold) | (u8::from(italic) << 1);
        self.clock += 1;
        let key = TextKey {
            text: text.into(),
            face,
        };
        if let Some(cached) = self.lines.get_mut(&key) {
            cached.used = self.clock;
            return cached.shaped.clone();
        }
        let font = self
            .fonts
            .entry(face)
            .or_insert_with(|| make_font(cfg, bold, italic));
        let string = CFMutableAttributedString::new(None, 0).expect("attributed string");
        // SAFETY: all attributes have CoreText's documented value types and
        // range covers the UTF-16 attributed string. CoreText handles fallback.
        let line = unsafe {
            CFMutableAttributedString::replace_string(
                Some(&string),
                CFRange::new(0, 0),
                Some(&CFString::from_str(text)),
            );
            let range = CFRange::new(0, CFAttributedString::length(&string));
            CFMutableAttributedString::set_attribute(
                Some(&string),
                range,
                Some(kCTFontAttributeName),
                Some(font.as_ref()),
            );
            CFMutableAttributedString::set_attribute(
                Some(&string),
                range,
                Some(kCTForegroundColorFromContextAttributeName),
                Some(CFBoolean::new(true).as_ref()),
            );
            // Terminal cell boundaries take priority over discretionary ligatures.
            CFMutableAttributedString::set_attribute(
                Some(&string),
                range,
                Some(kCTLigatureAttributeName),
                Some(CFNumber::new_i32(0).as_ref()),
            );
            CTLine::with_attributed_string(&string)
        };
        // A single primary-font baseline keeps fallback glyphs on the same row.
        let baseline = unsafe { (font.ascent() - font.descent()) / 2.0 };
        let shaped = Rc::new(Shaped { line, baseline });
        // A pathological single grapheme can exceed the whole cache budget.
        // Draw it once without retaining it or evicting otherwise useful lines.
        if text.len() > 256 * 1024 {
            return shaped;
        }
        while self.lines.len() >= 2048 || self.text_bytes + text.len() > 256 * 1024 {
            let Some(oldest) = self
                .lines
                .iter()
                .min_by_key(|(_, v)| v.used)
                .map(|(k, _)| k.clone())
            else {
                break;
            };
            self.text_bytes -= oldest.text.len();
            self.lines.remove(&oldest);
        }
        self.text_bytes += text.len();
        self.lines.insert(
            key,
            CachedText {
                shaped: shaped.clone(),
                used: self.clock,
            },
        );
        shaped
    }
    fn draw(&mut self, context: &CGContext, scene: &Scene) {
        let start = Instant::now();
        let cfg = &scene.config;
        let snap = &scene.snap;
        let key = (cfg.family.clone(), cfg.size.to_bits());
        if self.font_key.as_ref() != Some(&key) {
            self.font_key = Some(key);
            self.fonts.clear();
            self.lines.clear();
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
                let bg = background(span, cfg);
                if let Some(bg) = bg {
                    let mut end = span.col + span.cells;
                    while let Some(next) = spans.peek()
                        && next.col == end
                        && background(next, cfg) == Some(bg)
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
                let (x, y, w) = (
                    span.col as f64 * cfg.cw as f64,
                    row as f64 * cfg.lh as f64,
                    span.cells as f64 * cfg.cw as f64,
                );
                CGContext::save_g_state(c);
                CGContext::clip_to_rect(c, rect(x, y, w, cfg.lh as f64));
                set_color(
                    context,
                    cfg.colors(style).0,
                    if style.dim { 0.6 } else { 1.0 },
                );
                CGContext::save_g_state(c);
                CGContext::translate_ctm(c, x, y + cfg.lh as f64 / 2.0 + shaped.baseline);
                CGContext::scale_ctm(c, 1.0, -1.0);
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
                // SAFETY: immutable CTLine and live AppKit drawing context.
                unsafe {
                    shaped.line.draw(context);
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
                super::diag::line(&line);
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
fn background(span: &crate::terminal::GridRun, cfg: &PaintConfig) -> Option<nsh_core::Hsla> {
    if span.selected {
        Some(cfg.selection)
    } else {
        cfg.colors(span.run.style).1
    }
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

fn make_font(cfg: &PaintConfig, bold: bool, italic: bool) -> CFRetained<CTFont> {
    let size = cfg.size as f64;
    // Match NSH's family-list interpretation, retaining the chosen font once.
    let font = nsh_core::font_families(&cfg.family)
        .find_map(|family| match family {
            nsh_core::FamilyName::Named(name) => {
                let name = CFString::from_str(name);
                let attrs = CFDictionary::<CFString, CFType>::from_slices(
                    &[unsafe { kCTFontFamilyNameAttribute }],
                    &[name.as_ref()],
                );
                // SAFETY: a family descriptor, with no transform.
                let font = unsafe {
                    CTFont::with_font_descriptor(
                        &CTFontDescriptor::with_attributes(attrs.as_opaque()),
                        size,
                        std::ptr::null(),
                    )
                };
                (unsafe { font.family_name() }
                    .to_string()
                    .eq_ignore_ascii_case(&name.to_string()))
                .then_some(font)
            }
            _ => Some(system_mono(size)),
        })
        .unwrap_or_else(|| system_mono(size));
    let mut traits = CTFontSymbolicTraits::empty();
    if bold {
        traits |= CTFontSymbolicTraits::TraitBold;
    }
    if italic {
        traits |= CTFontSymbolicTraits::TraitItalic;
    }
    // SAFETY: valid font, size 0 and null matrix preserve its metrics.
    unsafe { font.copy_with_symbolic_traits(0.0, std::ptr::null(), traits, traits) }.unwrap_or(font)
}
fn system_mono(size: f64) -> CFRetained<CTFont> {
    let font = NSFont::monospacedSystemFontOfSize_weight(size, unsafe { NSFontWeightRegular });
    // SAFETY: NSFont is toll-free bridged to CTFont; transfer its +1 reference.
    unsafe { CFRetained::from_raw(NonNull::new(Retained::into_raw(font).cast()).unwrap()) }
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

#[cfg(debug_assertions)]
pub(super) fn frames(view: &NSView) -> Option<u32> {
    if let Some(surface) = view.downcast_ref::<Surface>() {
        return Some(surface.ivars().frames.get());
    }
    let children = view.subviews();
    (0..children.count()).find_map(|i| frames(&children.objectAtIndex(i)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{session_view::window::Window, terminal::GhosttyEmulator};
    use nsh_base::{TerminalEmulator, TerminalSize};
    use objc2_core_graphics::{CGBitmapContextCreate, CGBitmapContextGetData};

    fn config() -> PaintConfig {
        PaintConfig {
            cw: 10.0,
            lh: 20.0,
            family: "Menlo".into(),
            size: 13.0,
            foreground: nsh_core::color::rgb(0xffffff).into(),
            background: nsh_core::color::rgb(0).into(),
            selection: nsh_core::color::rgb(0xffff00).into(),
            search: nsh_core::color::rgb(0xff00ff).into(),
        }
    }
    fn scene() -> Scene {
        let mut emulator = GhosttyEmulator::new(TerminalSize::new(4, 1), 10);
        emulator.feed(b"\x1b[48;2;255;0;0m  A \x1b[0m");
        let snap = emulator
            .begin_window(&mut Window::default(), false, None)
            .unwrap()
            .finish();
        Scene {
            snap,
            config: config(),
            hits: Vec::new(),
            current: 0,
        }
    }
    fn bitmap() -> CFRetained<CGContext> {
        let space = CGColorSpace::new_device_rgb().unwrap();
        // SAFETY: CoreGraphics owns the allocated 40x20 RGBA buffer.
        let context = unsafe {
            CGBitmapContextCreate(
                std::ptr::null_mut(),
                40,
                20,
                8,
                160,
                Some(&space),
                CGImageAlphaInfo::PremultipliedLast.0 | CGImageByteOrderInfo::Order32Big.0,
            )
        }
        .unwrap();
        CGContext::translate_ctm(Some(&context), 0.0, 20.0);
        CGContext::scale_ctm(Some(&context), 1.0, -1.0);
        context
    }
    fn pixel(context: &CGContext, x: usize, y: usize) -> [u8; 4] {
        // SAFETY: the known context above owns 40*20*4 initialized bytes.
        unsafe {
            std::slice::from_raw_parts(
                CGBitmapContextGetData(Some(context)).cast::<u8>(),
                40 * 20 * 4,
            )[(y * 40 + x) * 4..(y * 40 + x + 1) * 4]
                .try_into()
                .unwrap()
        }
    }
    fn placement(col: i32, layer: u32) -> termlevel_ghostty_vt::ImagePlacement {
        termlevel_ghostty_vt::ImagePlacement {
            id: col as u32,
            generation: 1,
            layer,
            z: 0,
            column: col,
            row: 0,
            columns: 1,
            rows: 1,
            offset_x: 0,
            offset_y: 0,
            width: 10,
            height: 20,
            source_x: 0,
            source_y: 0,
            source_width: 1,
            source_height: 1,
            pixels: Arc::new(termlevel_ghostty_vt::Pixels {
                width: 1,
                height: 1,
                rgba: Arc::from([0, 0, 255, 255]),
            }),
        }
    }

    #[test]
    fn fractional_backgrounds_are_solid_across_wide_runs_rows_and_backing_scales() {
        for scale in [1.0, 2.0] {
            for mode in ["48;2;25;100;200", "7;38;2;25;100;200", "0"] {
                let mut emulator = GhosttyEmulator::new(TerminalSize::new(4, 2), 10);
                emulator.feed(format!("\x1b[{mode};8m中ab\r\n\x1b[1m中ab").as_bytes());
                let mut snap = emulator
                    .begin_window(&mut Window::default(), false, None)
                    .unwrap()
                    .finish();
                if mode == "0" {
                    for row in &mut Arc::make_mut(&mut snap).rows {
                        for span in row {
                            span.selected = true;
                        }
                    }
                }
                assert!(snap.rows.iter().all(|row| row.len() >= 2));
                let mut cfg = config();
                cfg.cw = 7.7 / scale as f32;
                cfg.lh = 9.3 / scale as f32;
                cfg.selection = nsh_core::color::rgb(0x1964c8).into();
                let scene = Scene {
                    snap,
                    config: cfg,
                    hits: Vec::new(),
                    current: 0,
                };
                let context = bitmap();
                CGContext::translate_ctm(Some(&context), 0.27, 0.27);
                CGContext::scale_ctm(Some(&context), scale, scale);
                Renderer::default().draw(&context, &scene);
                // Hidden glyphs isolate the backgrounds. Check every interior
                // pixel, including run boundaries and the shared row edge.
                for y in 2..18 {
                    for x in 1..31 {
                        assert_eq!(
                            pixel(&context, x, y),
                            [25, 100, 200, 255],
                            "gap at {x},{y}, scale={scale}, mode={mode}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn adjacent_background_colors_share_one_device_pixel_edge() {
        for scale in [1.0, 2.0] {
            let mut emulator = GhosttyEmulator::new(TerminalSize::new(4, 2), 10);
            emulator.feed(b"\x1b[8;48;2;255;0;0mab\x1b[48;2;0;0;255mcd\r\n\x1b[48;2;255;0;0mab\x1b[48;2;0;0;255mcd");
            let snap = emulator
                .begin_window(&mut Window::default(), false, None)
                .unwrap()
                .finish();
            let mut cfg = config();
            cfg.cw = 7.7 / scale as f32;
            cfg.lh = 9.3 / scale as f32;
            let scene = Scene {
                snap,
                config: cfg,
                hits: Vec::new(),
                current: 0,
            };
            let context = bitmap();
            CGContext::translate_ctm(Some(&context), 0.27, 0.27);
            CGContext::scale_ctm(Some(&context), scale, scale);
            Renderer::default().draw(&context, &scene);
            for y in 2..18 {
                for x in 1..31 {
                    let expected = if x < 16 {
                        [255, 0, 0, 255]
                    } else {
                        [0, 0, 255, 255]
                    };
                    assert_eq!(
                        pixel(&context, x, y),
                        expected,
                        "edge at {x},{y}, scale={scale}"
                    );
                }
            }
        }
    }

    #[test]
    fn native_renderer_keeps_image_layers_source_crop_and_cell_backgrounds() {
        let mut scene = scene();
        let snap = Arc::make_mut(&mut scene.snap);
        snap.images = vec![placement(0, 0), placement(1, 1), placement(2, 2)];
        let mut crop = placement(3, 2);
        crop.source_x = 1;
        crop.pixels = Arc::new(termlevel_ghostty_vt::Pixels {
            width: 2,
            height: 1,
            rgba: Arc::from([255, 0, 0, 255, 0, 255, 0, 255]),
        });
        snap.images.push(crop);
        let context = bitmap();
        Renderer::default().draw(&context, &scene);
        assert_eq!(
            pixel(&context, 5, 10),
            [255, 0, 0, 255],
            "layer 0 is below the explicit background"
        );
        assert_eq!(
            pixel(&context, 15, 10),
            [0, 0, 255, 255],
            "layer 1 is above the background"
        );
        assert_eq!(
            pixel(&context, 25, 10),
            [0, 0, 255, 255],
            "layer 2 covers the glyph"
        );
        assert_eq!(
            pixel(&context, 35, 10),
            [0, 255, 0, 255],
            "source crop must not squeeze the complete image"
        );
    }

    #[test]
    fn native_text_layout_survives_repaint_and_theme_but_not_font_change() {
        let mut scene = scene();
        let context = bitmap();
        let mut renderer = Renderer::default();
        renderer.draw(&context, &scene);
        let a = renderer.shape(&scene.config, "A", false, false);
        scene.config.foreground = nsh_core::color::rgb(0x00ff00).into();
        renderer.draw(&context, &scene);
        assert!(Rc::ptr_eq(
            &a,
            &renderer.shape(&scene.config, "A", false, false)
        ));
        assert!(!Rc::ptr_eq(
            &a,
            &renderer.shape(&scene.config, "A", true, false)
        ));
        scene.config.size = 16.0;
        renderer.draw(&context, &scene);
        assert!(!Rc::ptr_eq(
            &a,
            &renderer.shape(&scene.config, "A", false, false)
        ));
        assert_eq!(renderer.fonts.len(), 1);
    }

    #[test]
    fn native_text_cache_is_bounded_and_retains_recent_entries() {
        let cfg = config();
        let mut renderer = Renderer::default();
        let hot = renderer.shape(&cfg, "hot", false, false);
        for i in 0..2100 {
            renderer.shape(&cfg, &format!("scroll row {i}"), false, false);
            renderer.shape(&cfg, "hot", false, false);
        }
        assert_eq!(renderer.lines.len(), 2048);
        assert!(renderer.text_bytes <= 256 * 1024);
        assert!(Rc::ptr_eq(&hot, &renderer.shape(&cfg, "hot", false, false)));
        assert!(!renderer.lines.contains_key(&TextKey {
            text: "scroll row 0".into(),
            face: 0
        }));
        assert_eq!(renderer.fonts.len(), 1);
    }

    #[test]
    fn publisher_is_latest_frame_only_and_lifetime_does_not_own_a_session() {
        let publisher = Publisher::new();
        let id = publisher.0.id;
        let first = publisher.publish(scene());
        let old = publisher
            .0
            .scene
            .lock()
            .unwrap()
            .as_ref()
            .map(Arc::downgrade)
            .unwrap();
        let next = publisher.publish(scene());
        assert_eq!(first[0], next[0]);
        assert_ne!(first[1], next[1]);
        assert!(old.upgrade().is_none(), "pending frames must coalesce");
        drop(publisher);
        assert!(!slots().lock().unwrap().contains_key(&id));
    }

    #[test]
    fn semantic_adapter_records_no_drawing_operations() {
        let scene = scene();
        let drawing = super::super::paint::semantics(&scene.snap, 10.0, 20.0).finish();
        assert!(drawing.ops.is_empty());
        assert!(!drawing.nodes.is_empty());
    }
}
