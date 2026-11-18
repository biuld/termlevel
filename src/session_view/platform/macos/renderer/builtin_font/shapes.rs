// Adapted from Alacritty builtin_font.rs, Apache-2.0; see NOTICE.md.
use super::*;

pub(super) fn draw(
    character: char,
    canvas: &mut Canvas,
    width: usize,
    height: usize,
    stroke_size: usize,
) -> bool {
    match character {
        // Arcs: '╭', '╮', '╯', '╰'.
        '\u{256d}' | '\u{256e}' | '\u{256f}' | '\u{2570}' => {
            canvas.draw_rounded_corner(stroke_size);

            // Mirror `X` axis.
            if character == '\u{256d}' || character == '\u{2570}' {
                let center = canvas.x_center() as usize;

                let extra_offset = usize::from(stroke_size % 2 != width % 2);

                let buffer = canvas.buffer_mut();
                for y in 1..height {
                    let left = (y - 1) * width;
                    let right = y * width - 1;
                    if extra_offset != 0 {
                        buffer[right] = buffer[left];
                    }
                    for offset in 0..center {
                        buffer.swap(left + offset, right - offset - extra_offset);
                    }
                }
            }
            // Mirror `Y` axis.
            if character == '\u{256d}' || character == '\u{256e}' {
                let center = canvas.y_center() as usize;

                let extra_offset = usize::from(stroke_size % 2 != height % 2);

                let buffer = canvas.buffer_mut();
                if extra_offset != 0 {
                    let bottom_row = (height - 1) * width;
                    for index in 0..width {
                        buffer[bottom_row + index] = buffer[index];
                    }
                }
                for offset in 1..=center {
                    let top_row = (offset - 1) * width;
                    let bottom_row = (height - offset - extra_offset) * width;
                    for index in 0..width {
                        buffer.swap(top_row + index, bottom_row + index);
                    }
                }
            }
        }
        // Parts of full block: '▀', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '▔', '▉', '▊', '▋', '▌',
        // '▍', '▎', '▏', '▐', '▕', '🮂', '🮃', '🮄', '🮅', '🮆', '🮇', '🮈', '🮉', '🮊', '🮋'.
        '\u{2580}'..='\u{2587}'
        | '\u{2589}'..='\u{2590}'
        | '\u{2594}'
        | '\u{2595}'
        | '\u{1fb82}'..='\u{1fb8b}' => {
            let width = width as f32;
            let height = height as f32;
            let mut rect_width = match character {
                '\u{2589}' | '\u{1fb8b}' => width * 7. / 8.,
                '\u{258a}' | '\u{1fb8a}' => width * 6. / 8.,
                '\u{258b}' | '\u{1fb89}' => width * 5. / 8.,
                '\u{258c}' => width * 4. / 8.,
                '\u{258d}' | '\u{1fb88}' => width * 3. / 8.,
                '\u{258e}' | '\u{1fb87}' => width * 2. / 8.,
                '\u{258f}' => width * 1. / 8.,
                '\u{2590}' => width * 4. / 8.,
                '\u{2595}' => width * 1. / 8.,
                _ => width,
            };

            let (mut rect_height, mut y) = match character {
                '\u{2580}' => (height * 4. / 8., height * 8. / 8.),
                '\u{2581}' => (height * 1. / 8., height * 1. / 8.),
                '\u{2582}' => (height * 2. / 8., height * 2. / 8.),
                '\u{2583}' => (height * 3. / 8., height * 3. / 8.),
                '\u{2584}' => (height * 4. / 8., height * 4. / 8.),
                '\u{2585}' => (height * 5. / 8., height * 5. / 8.),
                '\u{2586}' => (height * 6. / 8., height * 6. / 8.),
                '\u{2587}' => (height * 7. / 8., height * 7. / 8.),
                '\u{2594}' => (height * 1. / 8., height * 8. / 8.),
                '\u{1fb82}' => (height * 2. / 8., height * 8. / 8.),
                '\u{1fb83}' => (height * 3. / 8., height * 8. / 8.),
                '\u{1fb84}' => (height * 5. / 8., height * 8. / 8.),
                '\u{1fb85}' => (height * 6. / 8., height * 8. / 8.),
                '\u{1fb86}' => (height * 7. / 8., height * 8. / 8.),
                _ => (height, height),
            };

            // Fix `y` coordinates.
            y = (height - y).round();

            // Ensure that resulted glyph will be visible and also round sizes instead of straight
            // flooring them.
            rect_width = rect_width.round().max(1.);
            rect_height = rect_height.round().max(1.);

            let x = match character {
                '\u{2590}' => canvas.x_center(),
                '\u{2595}' | '\u{1fb87}'..='\u{1fb8b}' => width - rect_width,
                _ => 0.,
            };

            canvas.draw_rect(x, y, rect_width, rect_height, COLOR_FILL);
        }
        // Shades: '░', '▒', '▓', '█'.
        '\u{2588}' | '\u{2591}' | '\u{2592}' | '\u{2593}' => {
            let color = match character {
                '\u{2588}' => COLOR_FILL,
                '\u{2591}' => COLOR_FILL_ALPHA_STEP_3,
                '\u{2592}' => COLOR_FILL_ALPHA_STEP_2,
                '\u{2593}' => COLOR_FILL_ALPHA_STEP_1,
                _ => unreachable!(),
            };
            canvas.fill(color);
        }
        // Quadrants: '▖', '▗', '▘', '▙', '▚', '▛', '▜', '▝', '▞', '▟'.
        '\u{2596}'..='\u{259F}' => {
            let x_center = canvas.x_center().round().max(1.);
            let y_center = canvas.y_center().round().max(1.);

            let (w_second, h_second) = match character {
                '\u{2598}' | '\u{2599}' | '\u{259a}' | '\u{259b}' | '\u{259c}' => {
                    (x_center, y_center)
                }
                _ => (0., 0.),
            };
            let (w_first, h_first) = match character {
                '\u{259b}' | '\u{259c}' | '\u{259d}' | '\u{259e}' | '\u{259f}' => {
                    (x_center, y_center)
                }
                _ => (0., 0.),
            };
            let (w_third, h_third) = match character {
                '\u{2596}' | '\u{2599}' | '\u{259b}' | '\u{259e}' | '\u{259f}' => {
                    (x_center, y_center)
                }
                _ => (0., 0.),
            };
            let (w_fourth, h_fourth) = match character {
                '\u{2597}' | '\u{2599}' | '\u{259a}' | '\u{259c}' | '\u{259f}' => {
                    (x_center, y_center)
                }
                _ => (0., 0.),
            };

            // Second quadrant.
            canvas.draw_rect(0., 0., w_second, h_second, COLOR_FILL);
            // First quadrant.
            canvas.draw_rect(x_center, 0., w_first, h_first, COLOR_FILL);
            // Third quadrant.
            canvas.draw_rect(0., y_center, w_third, h_third, COLOR_FILL);
            // Fourth quadrant.
            canvas.draw_rect(x_center, y_center, w_fourth, h_fourth, COLOR_FILL);
        }
        // Sextants: '🬀', '🬁', '🬂', '🬃', '🬄', '🬅', '🬆', '🬇', '🬈', '🬉', '🬊', '🬋', '🬌', '🬍', '🬎',
        // '🬏', '🬐', '🬑', '🬒', '🬓', '🬔', '🬕', '🬖', '🬗', '🬘', '🬙', '🬚', '🬛', '🬜', '🬝', '🬞', '🬟',
        // '🬠', '🬡', '🬢', '🬣', '🬤', '🬥', '🬦', '🬧', '🬨', '🬩', '🬪', '🬫', '🬬', '🬭', '🬮', '🬯', '🬰',
        // '🬱', '🬲', '🬳', '🬴', '🬵', '🬶', '🬷', '🬸', '🬹', '🬺', '🬻'.
        '\u{1fb00}'..='\u{1fb3b}' => {
            let x_center = canvas.x_center().round().max(1.);
            let y_third = (height as f32 / 3.).round().max(1.);
            let y_last_third = height as f32 - 2. * y_third;

            let (w_top_left, h_top_left) = match character {
                '\u{1fb00}' | '\u{1fb02}' | '\u{1fb04}' | '\u{1fb06}' | '\u{1fb08}'
                | '\u{1fb0a}' | '\u{1fb0c}' | '\u{1fb0e}' | '\u{1fb10}' | '\u{1fb12}'
                | '\u{1fb15}' | '\u{1fb17}' | '\u{1fb19}' | '\u{1fb1b}' | '\u{1fb1d}'
                | '\u{1fb1f}' | '\u{1fb21}' | '\u{1fb23}' | '\u{1fb25}' | '\u{1fb27}'
                | '\u{1fb28}' | '\u{1fb2a}' | '\u{1fb2c}' | '\u{1fb2e}' | '\u{1fb30}'
                | '\u{1fb32}' | '\u{1fb34}' | '\u{1fb36}' | '\u{1fb38}' | '\u{1fb3a}' => {
                    (x_center, y_third)
                }
                _ => (0., 0.),
            };
            let (w_top_right, h_top_right) = match character {
                '\u{1fb01}' | '\u{1fb02}' | '\u{1fb05}' | '\u{1fb06}' | '\u{1fb09}'
                | '\u{1fb0a}' | '\u{1fb0d}' | '\u{1fb0e}' | '\u{1fb11}' | '\u{1fb12}'
                | '\u{1fb14}' | '\u{1fb15}' | '\u{1fb18}' | '\u{1fb19}' | '\u{1fb1c}'
                | '\u{1fb1d}' | '\u{1fb20}' | '\u{1fb21}' | '\u{1fb24}' | '\u{1fb25}'
                | '\u{1fb28}' | '\u{1fb2b}' | '\u{1fb2c}' | '\u{1fb2f}' | '\u{1fb30}'
                | '\u{1fb33}' | '\u{1fb34}' | '\u{1fb37}' | '\u{1fb38}' | '\u{1fb3b}' => {
                    (x_center, y_third)
                }
                _ => (0., 0.),
            };
            let (w_mid_left, h_mid_left) = match character {
                '\u{1fb03}' | '\u{1fb04}' | '\u{1fb05}' | '\u{1fb06}' | '\u{1fb0b}'
                | '\u{1fb0c}' | '\u{1fb0d}' | '\u{1fb0e}' | '\u{1fb13}' | '\u{1fb14}'
                | '\u{1fb15}' | '\u{1fb1a}' | '\u{1fb1b}' | '\u{1fb1c}' | '\u{1fb1d}'
                | '\u{1fb22}' | '\u{1fb23}' | '\u{1fb24}' | '\u{1fb25}' | '\u{1fb29}'
                | '\u{1fb2a}' | '\u{1fb2b}' | '\u{1fb2c}' | '\u{1fb31}' | '\u{1fb32}'
                | '\u{1fb33}' | '\u{1fb34}' | '\u{1fb39}' | '\u{1fb3a}' | '\u{1fb3b}' => {
                    (x_center, y_third)
                }
                _ => (0., 0.),
            };
            let (w_mid_right, h_mid_right) = match character {
                '\u{1fb07}' | '\u{1fb08}' | '\u{1fb09}' | '\u{1fb0a}' | '\u{1fb0b}'
                | '\u{1fb0c}' | '\u{1fb0d}' | '\u{1fb0e}' | '\u{1fb16}' | '\u{1fb17}'
                | '\u{1fb18}' | '\u{1fb19}' | '\u{1fb1a}' | '\u{1fb1b}' | '\u{1fb1c}'
                | '\u{1fb1d}' | '\u{1fb26}' | '\u{1fb27}' | '\u{1fb28}' | '\u{1fb29}'
                | '\u{1fb2a}' | '\u{1fb2b}' | '\u{1fb2c}' | '\u{1fb35}' | '\u{1fb36}'
                | '\u{1fb37}' | '\u{1fb38}' | '\u{1fb39}' | '\u{1fb3a}' | '\u{1fb3b}' => {
                    (x_center, y_third)
                }
                _ => (0., 0.),
            };
            let (w_bottom_left, h_bottom_left) = match character {
                '\u{1fb0f}' | '\u{1fb10}' | '\u{1fb11}' | '\u{1fb12}' | '\u{1fb13}'
                | '\u{1fb14}' | '\u{1fb15}' | '\u{1fb16}' | '\u{1fb17}' | '\u{1fb18}'
                | '\u{1fb19}' | '\u{1fb1a}' | '\u{1fb1b}' | '\u{1fb1c}' | '\u{1fb1d}'
                | '\u{1fb2d}' | '\u{1fb2e}' | '\u{1fb2f}' | '\u{1fb30}' | '\u{1fb31}'
                | '\u{1fb32}' | '\u{1fb33}' | '\u{1fb34}' | '\u{1fb35}' | '\u{1fb36}'
                | '\u{1fb37}' | '\u{1fb38}' | '\u{1fb39}' | '\u{1fb3a}' | '\u{1fb3b}' => {
                    (x_center, y_last_third)
                }
                _ => (0., 0.),
            };
            let (w_bottom_right, h_bottom_right) = match character {
                '\u{1fb1e}' | '\u{1fb1f}' | '\u{1fb20}' | '\u{1fb21}' | '\u{1fb22}'
                | '\u{1fb23}' | '\u{1fb24}' | '\u{1fb25}' | '\u{1fb26}' | '\u{1fb27}'
                | '\u{1fb28}' | '\u{1fb29}' | '\u{1fb2a}' | '\u{1fb2b}' | '\u{1fb2c}'
                | '\u{1fb2d}' | '\u{1fb2e}' | '\u{1fb2f}' | '\u{1fb30}' | '\u{1fb31}'
                | '\u{1fb32}' | '\u{1fb33}' | '\u{1fb34}' | '\u{1fb35}' | '\u{1fb36}'
                | '\u{1fb37}' | '\u{1fb38}' | '\u{1fb39}' | '\u{1fb3a}' | '\u{1fb3b}' => {
                    (x_center, y_last_third)
                }
                _ => (0., 0.),
            };

            canvas.draw_rect(0., 0., w_top_left, h_top_left, COLOR_FILL);
            canvas.draw_rect(x_center, 0., w_top_right, h_top_right, COLOR_FILL);
            canvas.draw_rect(0., y_third, w_mid_left, h_mid_left, COLOR_FILL);
            canvas.draw_rect(x_center, y_third, w_mid_right, h_mid_right, COLOR_FILL);
            canvas.draw_rect(0., y_third * 2., w_bottom_left, h_bottom_left, COLOR_FILL);
            canvas.draw_rect(
                x_center,
                y_third * 2.,
                w_bottom_right,
                h_bottom_right,
                COLOR_FILL,
            );
        }
        _ => return false,
    }
    true
}
