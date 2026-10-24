//! One laser bed as SVG, in the colours most laser shops expect.
//!
//! The page is the bed, sized in millimetres with a viewBox of the same
//! numbers, so one unit is one millimetre in every program. There are no
//! transforms and every coordinate is absolute, because some laser software
//! drops or misreads them. Each path carries its own colour and width rather
//! than inheriting them from its group, for the same reason. Score lines come
//! first, then cuts, holes before outlines, so nothing is scored on a piece
//! that has already fallen free.

use std::fmt::Write;

use super::Part;
use crate::render::escape;

/// Pure red: the colour every laser shop reads as "cut through".
pub const CUT_COLOUR: &str = "#FF0000";
/// Pure blue: the colour every laser shop reads as "score" or vector engrave.
pub const ENGRAVE_COLOUR: &str = "#0000FF";
/// A hairline: lasers treat it as a vector path, and most refuse thicker lines as vectors.
pub const HAIRLINE_MM: f64 = 0.01;

/// `sheet` holds (index into `parts`, x, y) for each piece on this bed.
pub fn write(parts: &[Part], sheet: &[(usize, f64, f64)], bed: (f64, f64), title: &str) -> String {
    let (width, height) = bed;
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = writeln!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" version=\"1.1\" width=\"{width}mm\" height=\"{height}mm\" viewBox=\"0 0 {width} {height}\">"
    );
    let _ = writeln!(out, "<title>{}</title>", escape(title));

    out.push_str("<g id=\"engrave\">\n");
    for &(index, x, y) in sheet {
        let mut data = String::new();
        for line in &parts[index].engrave {
            push_line(&mut data, line, (x, y), false);
        }
        push_path(&mut out, &data, ENGRAVE_COLOUR);
    }
    out.push_str("</g>\n<g id=\"cut\">\n");
    for &(index, x, y) in sheet {
        let mut data = String::new();
        for ring in &parts[index].cut {
            push_line(&mut data, ring, (x, y), true);
        }
        push_path(&mut out, &data, CUT_COLOUR);
    }
    out.push_str("</g>\n</svg>\n");

    out
}

fn push_path(out: &mut String, data: &str, colour: &str) {
    if data.is_empty() {
        return;
    }
    let _ = writeln!(
        out,
        "<path fill=\"none\" stroke=\"{colour}\" stroke-width=\"{HAIRLINE_MM}\" d=\"{data}\"/>"
    );
}

/// "M x y L x y ..." in bed millimetres, closed with Z when `is_closed`.
fn push_line(data: &mut String, points: &[(f64, f64)], offset: (f64, f64), is_closed: bool) {
    // A closed ring repeats its first point; Z closes it instead.
    let points = if is_closed && points.len() > 1 && points.first() == points.last() {
        &points[..points.len() - 1]
    } else {
        points
    };
    if points.len() < 2 {
        return;
    }

    for (index, &(x, y)) in points.iter().enumerate() {
        let command = if index == 0 { 'M' } else { 'L' };
        let _ = write!(data, "{command}{:.2} {:.2}", x + offset.0, y + offset.1);
    }
    if is_closed {
        data.push('Z');
    }
}
