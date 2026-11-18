use super::*;

impl Renderer {
    pub(super) fn shape(
        &mut self,
        cfg: &PaintConfig,
        text: &str,
        bold: bool,
        italic: bool,
    ) -> Rc<Shaped> {
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
