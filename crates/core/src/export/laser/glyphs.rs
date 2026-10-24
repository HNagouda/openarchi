//! A tiny single-stroke font for the ids a laser engraves on each piece.
//!
//! Laser software handles text badly: it needs the font installed, or it fills
//! the letters as a slow raster engrave. Ids drawn as plain lines score in one
//! quick pass, look the same in every program, and need no font at all. The
//! ids only ever use digits, "B" (building), "T" (tile of the base plate),
//! "·" and "/", so that is all the font holds.

use geo::{BoundingRect, Contains, Coord, InteriorPoint, Polygon, Rect};

/// Label heights tried inside a footprint, largest first.
const LABEL_MAX_MM: f64 = 3.0;
const LABEL_STEP_MM: f64 = 0.25;
/// The smallest id worth engraving: readable up close, about a 9 pt letter.
pub const LABEL_MIN_MM: f64 = 1.0;
/// Glyphs are drawn on a grid this many units tall, y pointing down.
const GLYPH_UNITS_TALL: f64 = 6.0;
/// Most glyphs are this many units wide.
const GLYPH_UNITS_WIDE: f64 = 4.0;
/// The middle dot is narrow, so "B12·3/5" stays short.
const DOT_UNITS_WIDE: f64 = 1.0;
/// Space between two glyphs.
const SPACING_UNITS: f64 = 1.5;

type Stroke = &'static [(f64, f64)];

/// The strokes of one character on the glyph grid, and its width in units.
/// Digits follow a seven-segment display, which stays readable at 1 mm.
fn glyph(character: char) -> (&'static [Stroke], f64) {
    const WIDE: f64 = GLYPH_UNITS_WIDE;
    match character {
        '0' => (
            &[&[(0.0, 0.0), (4.0, 0.0), (4.0, 6.0), (0.0, 6.0), (0.0, 0.0)]],
            WIDE,
        ),
        '1' => (&[&[(1.0, 1.0), (2.5, 0.0), (2.5, 6.0)]], WIDE),
        '2' => (
            &[&[
                (0.0, 0.0),
                (4.0, 0.0),
                (4.0, 3.0),
                (0.0, 3.0),
                (0.0, 6.0),
                (4.0, 6.0),
            ]],
            WIDE,
        ),
        '3' => (
            &[
                &[(0.0, 0.0), (4.0, 0.0), (4.0, 6.0), (0.0, 6.0)],
                &[(0.0, 3.0), (4.0, 3.0)],
            ],
            WIDE,
        ),
        '4' => (
            &[
                &[(0.0, 0.0), (0.0, 3.0), (4.0, 3.0)],
                &[(4.0, 0.0), (4.0, 6.0)],
            ],
            WIDE,
        ),
        '5' => (
            &[&[
                (4.0, 0.0),
                (0.0, 0.0),
                (0.0, 3.0),
                (4.0, 3.0),
                (4.0, 6.0),
                (0.0, 6.0),
            ]],
            WIDE,
        ),
        '6' => (
            &[&[
                (4.0, 0.0),
                (0.0, 0.0),
                (0.0, 6.0),
                (4.0, 6.0),
                (4.0, 3.0),
                (0.0, 3.0),
            ]],
            WIDE,
        ),
        '7' => (&[&[(0.0, 0.0), (4.0, 0.0), (4.0, 6.0)]], WIDE),
        '8' => (
            &[
                &[(0.0, 0.0), (4.0, 0.0), (4.0, 6.0), (0.0, 6.0), (0.0, 0.0)],
                &[(0.0, 3.0), (4.0, 3.0)],
            ],
            WIDE,
        ),
        '9' => (
            &[&[
                (4.0, 3.0),
                (0.0, 3.0),
                (0.0, 0.0),
                (4.0, 0.0),
                (4.0, 6.0),
                (0.0, 6.0),
            ]],
            WIDE,
        ),
        'B' => (
            &[
                &[
                    (0.0, 0.0),
                    (0.0, 6.0),
                    (3.0, 6.0),
                    (4.0, 5.0),
                    (4.0, 4.0),
                    (3.0, 3.0),
                    (0.0, 3.0),
                ],
                &[(0.0, 0.0), (3.0, 0.0), (4.0, 1.0), (4.0, 2.0), (3.0, 3.0)],
            ],
            WIDE,
        ),
        'T' => (
            &[&[(0.0, 0.0), (4.0, 0.0)], &[(2.0, 0.0), (2.0, 6.0)]],
            WIDE,
        ),
        '/' => (&[&[(0.0, 6.0), (4.0, 0.0)]], WIDE),
        '·' => (
            &[&[(0.0, 2.5), (1.0, 2.5), (1.0, 3.5), (0.0, 3.5), (0.0, 2.5)]],
            DOT_UNITS_WIDE,
        ),
        // Ids are made in this module's callers, so any other character is a bug there.
        _ => panic!("the laser font has no glyph for {character:?}"),
    }
}

/// How wide `text` is when drawn `height_mm` tall.
pub fn text_width(text: &str, height_mm: f64) -> f64 {
    let mut units = 0.0;
    for (index, character) in text.chars().enumerate() {
        if index > 0 {
            units += SPACING_UNITS;
        }
        units += glyph(character).1;
    }

    units * height_mm / GLYPH_UNITS_TALL
}

/// `text` as open lines in millimetres, `height_mm` tall, its top-left corner at `left`, `top`.
pub fn strokes(text: &str, left: f64, top: f64, height_mm: f64) -> Vec<Vec<(f64, f64)>> {
    let unit = height_mm / GLYPH_UNITS_TALL;
    let mut lines = Vec::new();
    let mut cursor = left;
    for character in text.chars() {
        let (glyph_strokes, width) = glyph(character);
        for stroke in glyph_strokes {
            let line = stroke
                .iter()
                .map(|&(x, y)| (cursor + x * unit, top + y * unit))
                .collect();
            lines.push(line);
        }
        cursor += (width + SPACING_UNITS) * unit;
    }

    lines
}

/// Where `text` fits inside `polygon`, and how tall: its centre and height.
/// The bounding box's centre is tried first, then a point surely inside.
pub fn fitted_label(polygon: &Polygon<f64>, text: &str) -> Option<((f64, f64), f64)> {
    let mut centres = Vec::new();
    if let Some(rect) = polygon.bounding_rect() {
        centres.push((rect.center().x, rect.center().y));
    }
    if let Some(point) = polygon.interior_point() {
        centres.push((point.x(), point.y()));
    }

    let mut height = LABEL_MAX_MM;
    while height >= LABEL_MIN_MM {
        let width = text_width(text, height);
        for &(x, y) in &centres {
            let label = Rect::new(
                Coord {
                    x: x - width / 2.0,
                    y: y - height / 2.0,
                },
                Coord {
                    x: x + width / 2.0,
                    y: y + height / 2.0,
                },
            );
            if polygon.contains(&label.to_polygon()) {
                return Some(((x, y), height));
            }
        }
        height -= LABEL_STEP_MM;
    }

    None
}

/// `text` as strokes, `height` tall, centred on (`x`, `y`).
pub fn centred_label(text: &str, x: f64, y: f64, height: f64) -> Vec<Vec<(f64, f64)>> {
    let width = text_width(text, height);
    strokes(text, x - width / 2.0, y - height / 2.0, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_character_an_id_uses_has_a_glyph() {
        for character in "0123456789BT/·".chars() {
            assert!(!glyph(character).0.is_empty(), "{character}");
        }
    }

    #[test]
    fn text_is_as_tall_as_asked_and_starts_where_asked() {
        let lines = strokes("B12·3/5", 10.0, 20.0, 2.0);
        let points: Vec<(f64, f64)> = lines.into_iter().flatten().collect();
        let top = points.iter().map(|p| p.1).fold(f64::MAX, f64::min);
        let bottom = points.iter().map(|p| p.1).fold(f64::MIN, f64::max);
        let left = points.iter().map(|p| p.0).fold(f64::MAX, f64::min);
        let right = points.iter().map(|p| p.0).fold(f64::MIN, f64::max);
        assert_eq!((top, bottom, left), (20.0, 22.0, 10.0));
        assert!((right - 10.0 - text_width("B12·3/5", 2.0)).abs() < 1e-9);
    }

    #[test]
    fn a_label_inside_its_piece_stays_inside() {
        let ((x, y), height) = fitted_label(&square_mm(20.0), "B12·5/5").unwrap();
        assert_eq!((x, y, height), (10.0, 10.0, LABEL_MAX_MM));
    }

    fn square_mm(side: f64) -> Polygon<f64> {
        Polygon::new(
            vec![(0.0, 0.0), (side, 0.0), (side, side), (0.0, side)].into(),
            vec![],
        )
    }
}
