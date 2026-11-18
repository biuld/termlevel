use super::*;

impl Renderer {
    pub(super) fn draw_builtin(
        &mut self,
        context: &CGContext,
        cell: CGRect,
        ch: char,
        nominal_width: f64,
        color: nsh_core::Hsla,
        alpha: f64,
    ) -> bool {
        let device = CGContext::convert_rect_to_device_space(Some(context), cell);
        let width = device.size.width.abs().round() as usize;
        let height = device.size.height.abs().round() as usize;
        if width == 0 || height == 0 {
            return false;
        }
        // Adjacent snapped cells can differ by a pixel. Keep Alacritty's line
        // weight based on the shared, unsnapped cell width across the row.
        let stroke_width = CGContext::convert_rect_to_device_space(
            Some(context),
            rect(0.0, 0.0, nominal_width, cell.size.height),
        )
        .size
        .width
        .abs()
        .round()
        .max(1.0) as usize;
        let key = (ch, width, height, stroke_width);
        if !self.builtin_masks.contains_key(&key) {
            let Some(glyph) = builtin_font::glyph(ch, width, height, stroke_width) else {
                return false;
            };
            let Some(mask) = image_for_glyph(&glyph) else {
                return false;
            };
            if self.builtin_masks.len() >= 1024 {
                self.builtin_masks.clear();
            }
            self.builtin_masks.insert(key, mask);
        }
        let mask = &self.builtin_masks[&key];
        let unit_y = cell.size.height / height as f64;
        let mask_rect = rect(
            cell.origin.x,
            cell.origin.y - (mask.top - height as i32) as f64 * unit_y,
            cell.size.width,
            mask.height as f64 * unit_y,
        );
        let c = Some(context);
        CGContext::save_g_state(c);
        CGContext::clip_to_rect(c, cell);
        // Alacritty's glyph rows are top-down; Quartz image masks use the
        // opposite vertical orientation in our flipped AppKit surface.
        CGContext::translate_ctm(c, 0.0, cell.origin.y * 2.0 + cell.size.height);
        CGContext::scale_ctm(c, 1.0, -1.0);
        CGContext::clip_to_mask(c, mask_rect, Some(&mask.image));
        set_color(context, color, alpha);
        CGContext::fill_rect(c, cell);
        CGContext::restore_g_state(c);
        true
    }
}

fn image_for_glyph(glyph: &builtin_font::RasterizedGlyph) -> Option<BuiltinMask> {
    let width = usize::try_from(glyph.width).ok()?;
    let height = usize::try_from(glyph.height).ok()?;
    let pixels: Vec<u8> = glyph
        .rgb()
        .as_chunks::<3>()
        .0
        .iter()
        .take(width * height)
        .flat_map(|pixel| [pixel[0], pixel[0], pixel[0], pixel[0]])
        .collect();
    if pixels.len() != width * height * 4 {
        return None;
    }
    let data = CFData::from_bytes(&pixels);
    let provider = CGDataProvider::with_cf_data(Some(&data))?;
    let space = CGColorSpace::new_device_rgb()?;
    let image = unsafe {
        CGImage::new(
            width,
            height,
            8,
            32,
            width * 4,
            Some(&space),
            CGBitmapInfo(
                CGImageAlphaInfo::PremultipliedLast.0 | CGImageByteOrderInfo::Order32Big.0,
            ),
            Some(&provider),
            std::ptr::null(),
            false,
            CGColorRenderingIntent::RenderingIntentDefault,
        )?
    };
    Some(BuiltinMask {
        image,
        top: glyph.top,
        height: glyph.height,
    })
}
