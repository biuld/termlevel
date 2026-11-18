use super::*;

#[test]
fn upstream_glyph_ranges_rasterize_at_supported_cell_sizes() {
    let chars = ('\u{2500}'..='\u{259f}')
        .chain('\u{1fb00}'..='\u{1fb3b}')
        .chain('\u{1fb82}'..='\u{1fb8b}')
        .chain('\u{e0b0}'..='\u{e0b3}');
    for ch in chars {
        assert!(supports(ch));
        for (width, height) in [(6, 18), (15, 22), (15, 42), (24, 64)] {
            if ('\u{e0b0}'..='\u{e0b3}').contains(&ch) && (height + 1) / 2 - 1 > width + 1 {
                assert!(glyph(ch, width, height, width).is_none(), "{ch:?}");
                continue;
            }
            let glyph = glyph(ch, width, height, width).unwrap();
            assert_eq!(glyph.width, width as i32, "{ch:?}");
            assert!(
                glyph.rgb().len() >= glyph.width as usize * glyph.height as usize * 3,
                "{ch:?}"
            );
        }
    }
    for ch in ['A', '\u{24ff}', '\u{25a0}', '\u{e0af}', '\u{e0b4}'] {
        assert!(!supports(ch));
        assert!(glyph(ch, 15, 42, 15).is_none());
    }
}

#[test]
fn shades_preserve_the_upstream_coverage_levels() {
    for (ch, expected) in [('░', 64), ('▒', 128), ('▓', 192), ('█', 255)] {
        let glyph = glyph(ch, 15, 42, 15).unwrap();
        assert!(glyph.rgb().iter().all(|&value| value == expected), "{ch}");
    }
}

#[test]
fn adjacent_fractional_cells_share_the_same_stroke_weight() {
    let thickness = |width| {
        let glyph = glyph('─', width, 30, 12).unwrap();
        glyph
            .rgb()
            .chunks_exact(width * 3)
            .filter(|row| row[(width / 2) * 3] != 0)
            .count()
    };
    assert_eq!(thickness(11), thickness(12));
    assert_eq!(thickness(11), 2);
}
