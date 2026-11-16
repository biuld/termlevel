use super::*;
use objc2_core_foundation::CFURL;
use objc2_core_text::{CTFontManagerRegisterFontsForURL, CTFontManagerScope};
use std::sync::Once;

fn register() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/fonts");
        for path in [
            "jetbrains-mono/JetBrainsMono-Regular.ttf",
            "jetbrains-mono/JetBrainsMono-Bold.ttf",
            "jetbrains-mono/JetBrainsMono-Italic.ttf",
            "jetbrains-mono/JetBrainsMono-BoldItalic.ttf",
            "sarasa-term-sc/SarasaTermSC-Regular.ttf",
            "sarasa-term-sc/SarasaTermSC-Bold.ttf",
        ] {
            let url = CFURL::from_file_path(root.join(path)).unwrap();
            let mut error = std::ptr::null_mut();
            // Process scope: the test does not install fonts on the user's system.
            let ok = unsafe {
                CTFontManagerRegisterFontsForURL(&url, CTFontManagerScope::Process, &mut error)
            };
            if !ok {
                let error = unsafe {
                    CFRetained::from_raw(NonNull::new(error).expect("font registration error"))
                };
                assert_eq!(error.code(), 105, "{path}: {error:?}");
            }
        }
    });
}

fn font_names(line: &CTLine) -> Vec<String> {
    // The retained line owns the run array and each run's font attributes.
    unsafe {
        let runs = line.glyph_runs();
        (0..runs.count())
            .map(|index| {
                let run = &*runs.value_at_index(index).cast::<CTRun>();
                let attrs = run.attributes();
                let font = attrs.value((kCTFontAttributeName as *const CFString).cast());
                assert!(!font.is_null());
                (&*font.cast::<CTFont>()).post_script_name().to_string()
            })
            .collect()
    }
}

#[test]
fn bundled_fonts_resolve_primary_faces_and_cjk_symbol_fallback() {
    register();
    let mut cfg = super::tests::config();
    cfg.family = "JetBrains Mono, Sarasa Term SC, ui-monospace".into();
    for (bold, italic, face) in [
        (false, false, "Regular"),
        (true, false, "Bold"),
        (false, true, "Italic"),
        (true, true, "BoldItalic"),
    ] {
        let mut renderer = Renderer::default();
        let zero = renderer.shape(&cfg, "0", bold, italic);
        assert_eq!(font_names(&zero.line), [format!("JetBrainsMono-{face}")]);
        let cw = zero.advance;
        for text in ["①", "②", "③", "中", "文", "，"] {
            let shaped = renderer.shape(&cfg, text, bold, italic);
            assert!(
                font_names(&shaped.line)
                    .iter()
                    .all(|name| name.starts_with("Sarasa-Term-SC-")),
                "{text}: {:?}",
                font_names(&shaped.line)
            );
            let width = cw
                * if matches!(text, "①" | "②" | "③") {
                    1.0
                } else {
                    2.0
                };
            assert!(shaped.advance <= width + 0.01);
            assert_eq!(shaped.fit(width), (1.0, 0.0), "no fitting needed: {text}");
        }
    }
}

fn glyph_ids(line: &CTLine) -> Vec<u16> {
    unsafe {
        let runs = line.glyph_runs();
        let mut result = Vec::new();
        for index in 0..runs.count() {
            let run = &*runs.value_at_index(index).cast::<CTRun>();
            let mut glyphs = vec![0u16; run.glyph_count() as usize];
            run.glyphs(
                CFRange::new(0, 0),
                NonNull::new(glyphs.as_mut_ptr()).unwrap(),
            );
            result.extend(glyphs);
        }
        result
    }
}

#[test]
fn bundled_coding_ligatures_shape_without_changing_grid_advance() {
    register();
    let mut cfg = super::tests::config();
    cfg.family = "JetBrains Mono, Sarasa Term SC, ui-monospace".into();
    for (bold, italic) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut renderer = Renderer::default();
        let cw = renderer.shape(&cfg, "0", bold, italic).advance;
        for text in ["->", "!=", "=>", "==", "===", "<=", ">="] {
            let shaped = renderer.shape(&cfg, text, bold, italic);
            let separate: Vec<_> = text
                .chars()
                .flat_map(|ch| glyph_ids(&renderer.shape(&cfg, &ch.to_string(), bold, italic).line))
                .collect();
            assert_ne!(
                glyph_ids(&shaped.line),
                separate,
                "ligature is inactive: {text}"
            );
            assert!(
                (shaped.advance - cw * text.len() as f64).abs() < 0.01,
                "ligature changed column width: {text}"
            );
            let with_suffix = renderer.shape(&cfg, &format!("{text}X"), bold, italic);
            assert!((with_suffix.advance - cw * (text.len() + 1) as f64).abs() < 0.01);
        }
    }
}
