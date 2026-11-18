use super::*;

impl Renderer {
    /// Keep the font's shapes, but map its graphics design plane to the full
    /// terminal cell instead of using text baseline/line spacing. Reference
    /// glyphs are shared by a Unicode family: never stretch each glyph's ink
    /// bounds, which would turn a half block into a full block.
    pub(super) fn graphics_plane(&mut self, cfg: &PaintConfig, text: &str) -> Option<CGRect> {
        let ch = graphics_char(text)?;
        if self.graphics_planes.is_none() {
            let bounds = |shaped: Rc<Shaped>| {
                shaped
                    .graphics_path
                    .as_ref()
                    .map(|path| CGPath::path_bounding_box(Some(path)))
            };
            let blocks = bounds(self.shape(cfg, "█", false, false))?;
            let horizontal = bounds(self.shape(cfg, "─", false, false))?;
            let vertical = bounds(self.shape(cfg, "│", false, false))?;
            let center = horizontal.origin.y + horizontal.size.height / 2.0;
            let half_height =
                (center - vertical.origin.y).min(vertical.origin.y + vertical.size.height - center);
            let boxes = rect(
                horizontal.origin.x,
                center - half_height,
                horizontal.size.width,
                2.0 * half_height,
            );
            self.graphics_planes = Some(GraphicsPlanes { blocks, boxes });
        }
        let planes = self.graphics_planes?;
        let plane = if ch >= '\u{2580}' {
            planes.blocks
        } else {
            planes.boxes
        };
        (plane.size.width.is_finite()
            && plane.size.height.is_finite()
            && plane.size.width > 0.0
            && plane.size.height > 0.0)
            .then_some(plane)
    }

    /// Place a graphics outline in `cell`.
    ///
    /// Blocks stretch independently on each axis so a half block stays half a
    /// cell. Box drawing uses one scale, the larger of the two axes, and the
    /// cell clip drops the overflow. Stroke weight then stays the same
    /// horizontally and vertically, while a line that already reaches the font
    /// plane still reaches the cell edge.
    pub(super) fn map_graphics(context: &CGContext, cell: CGRect, plane: CGRect, block: bool) {
        let fit_x = cell.size.width / plane.size.width;
        let fit_y = cell.size.height / plane.size.height;
        let (scale_x, scale_y) = if block {
            (fit_x, fit_y)
        } else {
            let scale = fit_x.max(fit_y);
            (scale, scale)
        };
        let c = Some(context);
        CGContext::translate_ctm(
            c,
            cell.origin.x + cell.size.width / 2.0,
            cell.origin.y + cell.size.height / 2.0,
        );
        CGContext::scale_ctm(c, scale_x, -scale_y);
        CGContext::translate_ctm(
            c,
            -(plane.origin.x + plane.size.width / 2.0),
            -(plane.origin.y + plane.size.height / 2.0),
        );
    }

    pub(super) fn shape(
        &mut self,
        cfg: &PaintConfig,
        text: &str,
        bold: bool,
        italic: bool,
    ) -> Rc<Shaped> {
        let graphics = graphics_char(text).is_some();
        let face = if graphics {
            4
        } else {
            u8::from(bold) | (u8::from(italic) << 1)
        };
        self.clock += 1;
        let key = TextKey {
            text: text.into(),
            face,
        };
        if let Some(cached) = self.lines.get_mut(&key) {
            cached.used = self.clock;
            return cached.shaped.clone();
        }
        let font = self.fonts.entry(face).or_insert_with(|| {
            if graphics {
                // Graphics have a stable design plane independent of the
                // chosen text font and its bold/italic glyph variations.
                system_mono(cfg.size as f64)
            } else {
                make_font(cfg, bold, italic)
            }
        });
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
            // Enable the font's standard ligatures. Grid origins and span widths
            // remain Ghostty-owned; monospaced coding ligatures span their cells.
            CFMutableAttributedString::set_attribute(
                Some(&string),
                range,
                Some(kCTLigatureAttributeName),
                Some(CFNumber::new_i32(1).as_ref()),
            );
            CTLine::with_attributed_string(&string)
        };
        // A single primary-font baseline keeps fallback glyphs on the same row.
        let baseline = unsafe { (font.ascent() - font.descent()) / 2.0 };
        let graphics_path = graphics_char(text).and_then(|_| graphics_path(&line));
        // Measure both advance and ink once. Fallback fonts can supply a
        // full-width glyph even when Ghostty assigns a single terminal cell.
        let bounds = unsafe {
            line.bounds_with_options(objc2_core_text::CTLineBoundsOptions::UseGlyphPathBounds)
        };
        let advance = unsafe {
            line.typographic_bounds(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        let fallback = unsafe {
            let primary = font.post_script_name().to_string();
            let runs = line.glyph_runs();
            (0..runs.count()).any(|index| {
                let run = &*runs.value_at_index(index).cast::<CTRun>();
                let attrs = run.attributes();
                let actual = attrs.value((kCTFontAttributeName as *const CFString).cast());
                !actual.is_null()
                    && (&*actual.cast::<CTFont>()).post_script_name().to_string() != primary
            })
        };
        let horizontal = (bounds.origin.x, bounds.origin.x + bounds.size.width);
        let shaped = Rc::new(Shaped {
            line,
            baseline,
            horizontal,
            advance,
            fallback,
            graphics_path,
        });
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
}

fn graphics_char(text: &str) -> Option<char> {
    let mut chars = text.chars();
    let ch = chars.next()?;
    (chars.next().is_none() && matches!(ch, '\u{2500}'..='\u{259f}')).then_some(ch)
}

/// Extract the actual shaped font outlines, including CoreText fallback and
/// glyph positions. This reuses font-provided geometry instead of maintaining
/// a second table of block, corner, dash and line character shapes.
fn graphics_path(line: &CTLine) -> Option<CFRetained<CGPath>> {
    let combined = CGMutablePath::new();
    // SAFETY: CTLine supplies retained run storage. Each run's attributes own
    // the CTFont used to shape it; buffers have exactly the queried glyph count.
    unsafe {
        let runs = line.glyph_runs();
        for i in 0..runs.count() {
            let run = &*runs.value_at_index(i).cast::<CTRun>();
            let attributes = run.attributes();
            let font = attributes.value((kCTFontAttributeName as *const CFString).cast());
            if font.is_null() {
                return None;
            }
            let font = &*font.cast::<CTFont>();
            let count = usize::try_from(run.glyph_count()).ok()?;
            if count == 0 {
                continue;
            }
            let mut glyphs = vec![0u16; count];
            let mut positions = vec![CGPoint::new(0.0, 0.0); count];
            let range = CFRange::new(0, count as _);
            run.glyphs(range, NonNull::new(glyphs.as_mut_ptr())?);
            run.positions(range, NonNull::new(positions.as_mut_ptr())?);
            for (glyph, position) in glyphs.into_iter().zip(positions) {
                let path = font.path_for_glyph(glyph, std::ptr::null())?;
                let transform = CGAffineTransform {
                    a: 1.0,
                    b: 0.0,
                    c: 0.0,
                    d: 1.0,
                    tx: position.x,
                    ty: position.y,
                };
                CGMutablePath::add_path(Some(&combined), &transform, Some(&path));
            }
        }
    }
    CGPath::new_copy(Some(&combined))
}

fn make_font(cfg: &PaintConfig, bold: bool, italic: bool) -> CFRetained<CTFont> {
    let size = cfg.size as f64;
    // Match NSH's family-list interpretation, retaining the chosen font once.
    let mut fonts: Vec<_> = nsh_core::font_families(&cfg.family)
        .filter_map(|family| match family {
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
        .collect();
    if fonts.is_empty() {
        fonts.push(system_mono(size));
    }
    let mut traits = CTFontSymbolicTraits::empty();
    if bold {
        traits |= CTFontSymbolicTraits::TraitBold;
    }
    if italic {
        traits |= CTFontSymbolicTraits::TraitItalic;
    }
    // Apply the same face request to the primary and fallback fonts.
    let mut fonts = fonts.into_iter().map(|font| unsafe {
        font.copy_with_symbolic_traits(0.0, std::ptr::null(), traits, traits)
            .unwrap_or(font)
    });
    let primary = fonts.next().expect("a primary font");
    let descriptors: Vec<_> = fonts
        .map(|font| unsafe { font.font_descriptor() })
        .collect();
    if descriptors.is_empty() {
        return primary;
    }
    let cascade = CFArray::from_retained_objects(&descriptors);
    let attrs = CFDictionary::<CFString, CFType>::from_slices(
        &[unsafe { kCTFontCascadeListAttribute }],
        &[cascade.as_ref()],
    );
    // Use CoreText's cascade just as NSH's text measurement and field editor do.
    unsafe {
        primary.copy_with_attributes(
            0.0,
            std::ptr::null(),
            Some(&CTFontDescriptor::with_attributes(attrs.as_opaque())),
        )
    }
}
fn system_mono(size: f64) -> CFRetained<CTFont> {
    let font = NSFont::monospacedSystemFontOfSize_weight(size, unsafe { NSFontWeightRegular });
    // SAFETY: NSFont is toll-free bridged to CTFont; transfer its +1 reference.
    unsafe { CFRetained::from_raw(NonNull::new(Retained::into_raw(font).cast()).unwrap()) }
}

impl Shaped {
    /// Fit an overwide fallback grapheme to its allocated columns while
    /// retaining aspect ratio, grid clipping and the primary font baseline.
    pub(super) fn fit(&self, width: f64) -> (f64, f64) {
        // Ordinary ink overhang is not a font-width mismatch. Only adapt
        // fallback faces whose advance exceeds the terminal allocation.
        if !self.fallback || self.advance <= width + 0.01 {
            return (1.0, 0.0);
        }
        let (left, right) = self.horizontal;
        if !left.is_finite() || !right.is_finite() || right <= left || width <= 0.0 {
            return (1.0, 0.0);
        }
        if left >= -0.01 && right <= width + 0.01 {
            return (1.0, 0.0);
        }
        let scale = (width / (right - left)).min(1.0);
        (scale, -left * scale)
    }
}
