// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0-only

//! Fits a decoded phone frame into the camera's frame: turned upright, scaled to fit, and
//! centred between dark bars.

use crate::shared::{FRAME_BYTES, HEIGHT, WIDTH};

/// An NV12 picture as a decoder hands it over.
pub struct Picture<'a> {
    pub data: &'a [u8],
    pub width: usize,
    pub height: usize,
    /// Bytes from one row to the next.
    pub stride: usize,
    /// Rows in the brightness plane, which can be more than `height` with padding.
    pub plane_rows: usize,
}

const DARK: u8 = 16;
const NO_COLOUR: u8 = 128;

/// `quarter_turns` is how far to turn the picture clockwise to make it upright.
pub fn fit(picture: &Picture, quarter_turns: u32, frame: &mut [u8]) {
    assert_eq!(frame.len(), FRAME_BYTES);
    let (brightness, colour) = frame.split_at_mut(WIDTH * HEIGHT);
    brightness.fill(DARK);
    colour.fill(NO_COLOUR);

    let sideways = quarter_turns % 2 == 1;
    let (upright_width, upright_height) = if sideways {
        (picture.height, picture.width)
    } else {
        (picture.width, picture.height)
    };
    if upright_width == 0 || upright_height == 0 {
        return;
    }
    // Even sizes and offsets, so each colour sample covers whole pixels.
    let scale = f64::min(
        WIDTH as f64 / upright_width as f64,
        HEIGHT as f64 / upright_height as f64,
    );
    let width = ((upright_width as f64 * scale) as usize).min(WIDTH) & !1;
    let height = ((upright_height as f64 * scale) as usize).min(HEIGHT) & !1;
    let (left, top) = (((WIDTH - width) / 2) & !1, ((HEIGHT - height) / 2) & !1);

    // Where in the source a point of the upright picture comes from.
    let source = |u: usize, v: usize| match quarter_turns % 4 {
        1 => (v, picture.height - 1 - u),
        2 => (picture.width - 1 - u, picture.height - 1 - v),
        3 => (picture.width - 1 - v, u),
        _ => (u, v),
    };
    let upright = |x: usize, y: usize| (x * upright_width / width, y * upright_height / height);

    for y in 0..height {
        for x in 0..width {
            let (u, v) = upright(x, y);
            let (sx, sy) = source(u, v);
            brightness[(top + y) * WIDTH + left + x] = picture.data[sy * picture.stride + sx];
        }
    }
    let source_colour = &picture.data[picture.stride * picture.plane_rows..];
    for y in 0..height / 2 {
        for x in 0..width / 2 {
            let (u, v) = upright(x * 2, y * 2);
            let (sx, sy) = source(u, v);
            let from = sy / 2 * picture.stride + sx / 2 * 2;
            let to = (top / 2 + y) * WIDTH + left + x * 2;
            colour[to..to + 2].copy_from_slice(&source_colour[from..from + 2]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 4 by 2 picture whose brightness is 10 times the column plus the row, with no colour.
    fn picture() -> Vec<u8> {
        let mut data: Vec<u8> = (0..2)
            .flat_map(|row| (0..4).map(move |column| 10 * column + row))
            .collect();
        data.extend([NO_COLOUR; 4]);
        data
    }

    fn at(frame: &[u8], x: usize, y: usize) -> u8 {
        frame[y * WIDTH + x]
    }

    #[test]
    fn a_wide_picture_fills_the_width_between_bars_above_and_below() {
        let data = picture();
        let source = Picture {
            data: &data,
            width: 4,
            height: 2,
            stride: 4,
            plane_rows: 2,
        };
        let mut frame = vec![0; FRAME_BYTES];
        fit(&source, 0, &mut frame);

        // 4 by 2 scales to 1280 by 640, leaving 40 rows above and below.
        assert_eq!(at(&frame, 0, 39), DARK);
        assert_eq!(at(&frame, 0, 40), 0);
        assert_eq!(at(&frame, WIDTH - 1, 40), 30);
        assert_eq!(at(&frame, 0, HEIGHT - 41), 1);
        assert_eq!(at(&frame, 0, HEIGHT - 40), DARK);
    }

    #[test]
    fn a_sideways_picture_is_turned_upright_between_bars_left_and_right() {
        let data = picture();
        let source = Picture {
            data: &data,
            width: 4,
            height: 2,
            stride: 4,
            plane_rows: 2,
        };
        let mut frame = vec![0; FRAME_BYTES];
        fit(&source, 1, &mut frame);

        // Upright it's 2 by 4, scaled to 360 by 720 in the middle.
        let left = (WIDTH - 360) / 2;
        assert_eq!(at(&frame, left - 1, 0), DARK);
        // Turned clockwise, the bottom-left of the source comes to the top-left.
        assert_eq!(at(&frame, left, 0), 1);
        assert_eq!(at(&frame, left + 359, 0), 0);
        assert_eq!(at(&frame, left, HEIGHT - 1), 31);
    }
}
