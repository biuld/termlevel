use super::*;
use crate::{session_view::window::Window, terminal::GhosttyEmulator};
use nsh_base::{TerminalEmulator, TerminalSize};
use objc2_core_graphics::{CGBitmapContextCreate, CGBitmapContextGetData};

pub(super) fn config() -> PaintConfig {
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
fn erased_background_only_cells_fill_the_native_window() {
    let mut emulator = GhosttyEmulator::new(TerminalSize::new(4, 1), 0);
    emulator.feed(b"\x1b[48;2;55;65;75m\x1b[2K\x1b[0m");
    let snap = emulator
        .begin_window(&mut Window::default(), false, None)
        .unwrap()
        .finish();
    let scene = Scene {
        snap,
        config: config(),
        hits: Vec::new(),
        current: 0,
    };
    let context = bitmap();
    Renderer::default().draw(&context, &scene);
    for y in 0..20 {
        for x in 0..40 {
            assert_eq!(pixel(&context, x, y), [55, 65, 75, 255]);
        }
    }
}

#[test]
fn graphic_blocks_fill_cells_and_keep_half_block_geometry() {
    for family in ["Menlo", "Monaco", "monospace"] {
        for (text, halves) in [("████", false), ("▀▄▌▐", true)] {
            let mut emulator = GhosttyEmulator::new(TerminalSize::new(4, 1), 0);
            emulator.feed(text.as_bytes());
            let snap = emulator
                .begin_window(&mut Window::default(), false, None)
                .unwrap()
                .finish();
            let mut cfg = config();
            cfg.family = family.into();
            let scene = Scene {
                snap,
                config: cfg,
                hits: Vec::new(),
                current: 0,
            };
            let context = bitmap();
            let mut renderer = Renderer::default();
            renderer.draw(&context, &scene);
            for y in 0..20 {
                for x in 0..40 {
                    let filled = !halves
                        || match x / 10 {
                            0 => y < 10,
                            1 => y >= 10,
                            2 => x % 10 < 5,
                            _ => x % 10 >= 5,
                        };
                    assert_eq!(
                        pixel(&context, x, y)[3],
                        if filled { 255 } else { 0 },
                        "{family} {text} at {x},{y}"
                    );
                }
            }
        }
    }
}

#[test]
fn graphic_box_edges_connect_across_cells_and_rows() {
    for text in ["┌──┐\r\n└──┘", "┏━━┓\r\n┗━━┛", "╔══╗\r\n╚══╝"]
    {
        let mut emulator = GhosttyEmulator::new(TerminalSize::new(4, 2), 0);
        emulator.feed(text.as_bytes());
        let snap = emulator
            .begin_window(&mut Window::default(), false, None)
            .unwrap()
            .finish();
        let mut cfg = config();
        cfg.lh = 10.0;
        let scene = Scene {
            snap,
            config: cfg,
            hits: Vec::new(),
            current: 0,
        };
        let context = bitmap();
        Renderer::default().draw(&context, &scene);
        // Every column of the horizontal edge and every row of the
        // vertical edges must contain ink, including all cell joins.
        for x in 5..35 {
            assert!(
                (2..8).any(|y| pixel(&context, x, y)[3] == 255),
                "top {text} x={x}"
            );
            assert!(
                (12..18).any(|y| pixel(&context, x, y)[3] == 255),
                "bottom {text} x={x}"
            );
        }
        for y in 5..15 {
            assert!(
                (2..8).any(|x| pixel(&context, x, y)[3] == 255),
                "left {text} y={y}"
            );
            assert!(
                (32..38).any(|x| pixel(&context, x, y)[3] == 255),
                "right {text} y={y}"
            );
        }
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
