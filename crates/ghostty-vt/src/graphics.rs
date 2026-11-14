use super::{Error, Result, check, handle, ptr, sys};
use std::{
    collections::HashMap,
    io::Cursor,
    sync::{Arc, Once},
};
const MAX_PIXELS: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pixels {
    pub width: u32,
    pub height: u32,
    pub rgba: Arc<[u8]>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagePlacement {
    pub id: u32,
    pub generation: u64,
    pub layer: u32,
    pub z: i32,
    pub column: i32,
    pub row: i32,
    pub columns: u32,
    pub rows: u32,
    pub offset_x: u32,
    pub offset_y: u32,
    pub width: u32,
    pub height: u32,
    pub source_x: u32,
    pub source_y: u32,
    pub source_width: u32,
    pub source_height: u32,
    pub pixels: Arc<Pixels>,
}
handle!(
    Images,
    sys::GhosttyRenderStateImageIterator,
    sys::ghostty_render_state_image_iterator_new,
    sys::ghostty_render_state_image_iterator_free
);

pub(super) fn initialize() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        // SAFETY: every safe Terminal constructor crosses this Once before any
        // terminal can use PNG decoding. This process-global hook never changes.
        let code = unsafe {
            sys::ghostty_sys_set(
                sys::GHOSTTY_SYS_OPT_DECODE_PNG,
                decode_png as *const () as *const std::ffi::c_void,
            )
        };
        assert_eq!(code, sys::GHOSTTY_SUCCESS, "install PNG decoder");
    });
}
unsafe extern "C" fn decode_png(
    _: *mut std::ffi::c_void,
    allocator: *const sys::GhosttyAllocator,
    data: *const u8,
    len: usize,
    out: *mut sys::GhosttySysImage,
) -> bool {
    std::panic::catch_unwind(|| -> Option<()> {
        if len > MAX_PIXELS || out.is_null() {
            return None;
        }
        let bytes = if len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(data, len) }
        };
        let mut decoder = png::Decoder::new(Cursor::new(bytes));
        decoder.set_limits(png::Limits { bytes: MAX_PIXELS });
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().ok()?;
        let count = reader.output_buffer_size()?;
        if count > MAX_PIXELS {
            return None;
        }
        let mut decoded = vec![0; count];
        let info = reader.next_frame(&mut decoded).ok()?;
        let count = (info.width as usize)
            .checked_mul(info.height as usize)?
            .checked_mul(4)?;
        if count == 0 || count > MAX_PIXELS {
            return None;
        }
        let mut rgba = Vec::with_capacity(count);
        let channels = info.color_type.samples();
        for pixel in decoded[..info.buffer_size()].chunks_exact(channels) {
            match info.color_type {
                png::ColorType::Rgba => rgba.extend_from_slice(pixel),
                png::ColorType::Rgb => rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]),
                png::ColorType::Grayscale => {
                    rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], 255])
                }
                png::ColorType::GrayscaleAlpha => {
                    rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]])
                }
                _ => return None,
            }
        }
        if rgba.len() != count {
            return None;
        }
        // SAFETY: use precisely the allocator supplied by Ghostty; the library
        // takes ownership of this allocation and frees it with the same one.
        unsafe {
            let native = sys::ghostty_alloc(allocator, count);
            if native.is_null() {
                return None;
            }
            ptr::copy_nonoverlapping(rgba.as_ptr(), native, count);
            *out = sys::GhosttySysImage {
                width: info.width,
                height: info.height,
                data: native,
                data_len: count,
            };
        }
        Some(())
    })
    .ok()
    .flatten()
    .is_some()
}

// These helpers are private and unsafe: callers must match each key's exact
// documented output type. No generic key/value getter is exposed to safe users.
unsafe fn placement<T: Default>(
    iter: sys::GhosttyRenderStateImageIterator,
    key: sys::GhosttyRenderStateImageData,
) -> Result<T> {
    let mut value = T::default();
    check(unsafe {
        sys::ghostty_render_state_image_get(iter, key, (&mut value as *mut T).cast())
    })?;
    Ok(value)
}
unsafe fn image_field<T: Default>(
    image: sys::GhosttyKittyGraphicsImage,
    key: sys::GhosttyKittyGraphicsImageData,
) -> Result<T> {
    let mut value = T::default();
    check(unsafe {
        sys::ghostty_kitty_graphics_image_get(image, key, (&mut value as *mut T).cast())
    })?;
    Ok(value)
}
pub(super) unsafe fn read_images(
    state: sys::GhosttyRenderState,
    terminal: sys::GhosttyTerminal,
    cache: &mut HashMap<(u32, u64), Arc<Pixels>>,
) -> Result<Vec<ImagePlacement>> {
    let mut graphics: sys::GhosttyKittyGraphics = ptr::null_mut();
    let code = unsafe {
        sys::ghostty_terminal_get(
            terminal,
            sys::GHOSTTY_TERMINAL_DATA_KITTY_GRAPHICS,
            (&mut graphics as *mut sys::GhosttyKittyGraphics).cast(),
        )
    };
    if code == sys::GHOSTTY_UNSUPPORTED || code == sys::GHOSTTY_NO_VALUE {
        return Ok(Vec::new());
    }
    check(code)?;
    if graphics.is_null() {
        return Ok(Vec::new());
    }
    let mut iter = Images::new()?;
    check(unsafe {
        sys::ghostty_render_state_get(
            state,
            sys::GHOSTTY_RENDER_STATE_DATA_IMAGE_ITERATOR,
            (&mut iter.0 as *mut sys::GhosttyRenderStateImageIterator).cast(),
        )
    })?;
    let mut output = Vec::new();
    let mut live = HashMap::new();
    while unsafe { sys::ghostty_render_state_image_iterator_next(iter.0) } {
        // SAFETY: iterator is positioned and neither terminal nor render state
        // changes until every borrowed pixel buffer has been copied.
        unsafe {
            let protocol: u32 = placement(iter.0, sys::GHOSTTY_RENDER_STATE_IMAGE_DATA_PROTOCOL)?;
            if protocol != sys::GHOSTTY_IMAGE_PROTOCOL_KITTY {
                continue;
            }
            let id: u32 = placement(iter.0, sys::GHOSTTY_RENDER_STATE_IMAGE_DATA_IMAGE_ID)?;
            let generation: u64 = placement(
                iter.0,
                sys::GHOSTTY_RENDER_STATE_IMAGE_DATA_IMAGE_GENERATION,
            )?;
            let pixels = if let Some(value) = cache.get(&(id, generation)) {
                value.clone()
            } else {
                let image = sys::ghostty_kitty_graphics_image(graphics, id);
                if image.is_null() {
                    continue;
                }
                let width: u32 = image_field(image, sys::GHOSTTY_KITTY_GRAPHICS_IMAGE_DATA_WIDTH)?;
                let height: u32 =
                    image_field(image, sys::GHOSTTY_KITTY_GRAPHICS_IMAGE_DATA_HEIGHT)?;
                let format: u32 =
                    image_field(image, sys::GHOSTTY_KITTY_GRAPHICS_IMAGE_DATA_FORMAT)?;
                let pixels: sys::GhosttyString =
                    match image_field(image, sys::GHOSTTY_KITTY_GRAPHICS_IMAGE_DATA_PIXELS) {
                        Ok(v) => v,
                        Err(Error(sys::GHOSTTY_NO_VALUE)) => continue,
                        Err(e) => return Err(e),
                    };
                let length = (width as usize)
                    .checked_mul(height as usize)
                    .and_then(|n| n.checked_mul(4))
                    .ok_or(Error(sys::GHOSTTY_LIMIT_EXCEEDED))?;
                if length == 0 || length > MAX_PIXELS {
                    continue;
                }
                let source = if pixels.len == 0 {
                    &[]
                } else {
                    std::slice::from_raw_parts(pixels.ptr, pixels.len)
                };
                let channels = match format {
                    sys::GHOSTTY_KITTY_IMAGE_FORMAT_RGB => 3,
                    sys::GHOSTTY_KITTY_IMAGE_FORMAT_RGBA => 4,
                    sys::GHOSTTY_KITTY_IMAGE_FORMAT_GRAY_ALPHA => 2,
                    sys::GHOSTTY_KITTY_IMAGE_FORMAT_GRAY => 1,
                    _ => continue,
                };
                if source.len() != length / 4 * channels {
                    return Err(Error(sys::GHOSTTY_INVALID_VALUE));
                }
                let mut rgba = Vec::with_capacity(length);
                for p in source.chunks_exact(channels) {
                    match channels {
                        4 => rgba.extend_from_slice(p),
                        3 => rgba.extend_from_slice(&[p[0], p[1], p[2], 255]),
                        2 => rgba.extend_from_slice(&[p[0], p[0], p[0], p[1]]),
                        _ => rgba.extend_from_slice(&[p[0], p[0], p[0], 255]),
                    }
                }
                Arc::new(Pixels {
                    width,
                    height,
                    rgba: rgba.into(),
                })
            };
            live.insert((id, generation), pixels.clone());
            output.push(ImagePlacement {
                id,
                generation,
                pixels,
                layer: placement(iter.0, sys::GHOSTTY_RENDER_STATE_IMAGE_DATA_LAYER)?,
                z: placement(iter.0, sys::GHOSTTY_RENDER_STATE_IMAGE_DATA_Z)?,
                column: placement(iter.0, sys::GHOSTTY_RENDER_STATE_IMAGE_DATA_VIEWPORT_COL)?,
                row: placement(iter.0, sys::GHOSTTY_RENDER_STATE_IMAGE_DATA_VIEWPORT_ROW)?,
                columns: placement(iter.0, sys::GHOSTTY_RENDER_STATE_IMAGE_DATA_GRID_COLS)?,
                rows: placement(iter.0, sys::GHOSTTY_RENDER_STATE_IMAGE_DATA_GRID_ROWS)?,
                offset_x: placement(iter.0, sys::GHOSTTY_RENDER_STATE_IMAGE_DATA_CELL_OFFSET_X)?,
                offset_y: placement(iter.0, sys::GHOSTTY_RENDER_STATE_IMAGE_DATA_CELL_OFFSET_Y)?,
                width: placement(iter.0, sys::GHOSTTY_RENDER_STATE_IMAGE_DATA_PIXEL_WIDTH)?,
                height: placement(iter.0, sys::GHOSTTY_RENDER_STATE_IMAGE_DATA_PIXEL_HEIGHT)?,
                source_x: placement(iter.0, sys::GHOSTTY_RENDER_STATE_IMAGE_DATA_SOURCE_X)?,
                source_y: placement(iter.0, sys::GHOSTTY_RENDER_STATE_IMAGE_DATA_SOURCE_Y)?,
                source_width: placement(iter.0, sys::GHOSTTY_RENDER_STATE_IMAGE_DATA_SOURCE_WIDTH)?,
                source_height: placement(
                    iter.0,
                    sys::GHOSTTY_RENDER_STATE_IMAGE_DATA_SOURCE_HEIGHT,
                )?,
            });
        }
    }
    *cache = live;
    output.sort_by_key(|image| (image.layer, image.z));
    Ok(output)
}

impl super::Terminal {
    /// Advance Kitty animations on a monotonic host clock.
    /// `Some(next_ms)` means an animation is playing and `next_ms` is when
    /// the next frame is due. The host should redraw, then wait until then.
    pub fn tick_images(&mut self, now_ms: u64) -> Result<Option<u64>> {
        let mut graphics: sys::GhosttyKittyGraphics = ptr::null_mut();
        let code = unsafe {
            sys::ghostty_terminal_get(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_DATA_KITTY_GRAPHICS,
                (&mut graphics as *mut sys::GhosttyKittyGraphics).cast(),
            )
        };
        if code == sys::GHOSTTY_UNSUPPORTED || code == sys::GHOSTTY_NO_VALUE || graphics.is_null() {
            return Ok(None);
        }
        check(code)?;
        let mut next = 0;
        let code =
            unsafe { sys::ghostty_kitty_graphics_animation_tick(graphics, now_ms, &mut next) };
        if code == sys::GHOSTTY_NO_VALUE || code == sys::GHOSTTY_UNSUPPORTED {
            Ok(None)
        } else {
            check(code)?;
            Ok(Some(next))
        }
    }
}
