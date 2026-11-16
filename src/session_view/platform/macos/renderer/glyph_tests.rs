use super::*;
use crate::{session_view::window::Window, terminal::GhosttyEmulator};
use nsh_base::{TerminalEmulator, TerminalSize};

#[test]
fn fallback_graphemes_fit_their_ghostty_columns() {
    for family in [
        "Menlo",
        "Monaco",
        "Courier New",
        "Andale Mono",
        "ui-monospace",
    ] {
        for size in [11.0, 13.0, 18.0] {
            let mut cfg = super::tests::config();
            cfg.family = family.into();
            cfg.size = size;
            let mut renderer = Renderer::default();
            let zero = renderer.shape(&cfg, "0", false, false);
            cfg.cw = unsafe {
                zero.line.typographic_bounds(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            } as f32;
            let mut emulator = GhosttyEmulator::new(TerminalSize::new(20, 1), 0);
            emulator.feed("A①B②中③".as_bytes());
            let snap = emulator
                .begin_window(&mut Window::default(), false, None)
                .unwrap()
                .finish();
            assert_eq!(
                snap.rows[0]
                    .iter()
                    .map(|span| (span.col, span.cells))
                    .collect::<Vec<_>>()[..6],
                [(0, 1), (1, 1), (2, 1), (3, 1), (4, 2), (6, 1)]
            );
            for text in ["①", "②", "③", "中"] {
                for (bold, italic) in [(false, false), (true, false), (false, true)] {
                    let shaped = renderer.shape(&cfg, text, bold, italic);
                    let width = cfg.cw as f64 * if text == "中" { 2.0 } else { 1.0 };
                    let (scale, offset) = shaped.fit(width);
                    let (left, right) = shaped.horizontal;
                    assert!(scale > 0.0 && scale <= 1.0);
                    assert!(offset + left * scale >= -0.01, "{family} {text}: left");
                    assert!(
                        offset + right * scale <= width + 0.01,
                        "{family} {text}: right"
                    );
                    if text == "中" {
                        assert_eq!(scale, 1.0);
                    }
                }
            }
            for text in ["é", "e\u{301}", "f"] {
                let shaped = renderer.shape(&cfg, text, false, true);
                assert_eq!(
                    shaped.fit(cfg.cw as f64),
                    (1.0, 0.0),
                    "keep ordinary italic/combining text: {family} {text}"
                );
            }
            let ascii = renderer.shape(&cfg, "ABC", false, false);
            assert_eq!(ascii.fit(3.0 * cfg.cw as f64), (1.0, 0.0));
        }
    }
}
