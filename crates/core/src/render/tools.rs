//! The north arrow and the scale bar.

use std::fmt::Write;

use super::furniture::Furniture;
use super::geometry::{decimal, mm};
use super::typeset::{Kind, Typesetter};
use crate::sheet::{Paper, Sheet};

// Sizes at A3, in millimetres; multiplied by the layout unit.
const ARROW_WIDTH: f64 = 5.2;
const ARROW_HEIGHT: f64 = 9.5;
/// How far the notch in the arrow's base reaches up, as a share of its height.
const ARROW_NOTCH_SHARE: f64 = 0.28;
const ARROW_LABEL_GAP: f64 = 1.6;
const ARROW_TO_BAR: f64 = 9.0;
const BAR_HEIGHT: f64 = 1.1;
const BAR_SEGMENTS: usize = 4;
const BAR_STROKE_MM: f64 = 0.18;
const BAR_LABEL_GAP: f64 = 1.6;
/// The height the arrow and bar take, for the blocks stacked around them.
pub const TOOLS_HEIGHT: f64 = 14.0;
/// Ground lengths a scale bar may span: round numbers a reader can halve in
/// their head. The shortest serve a sheet of one plot at 1:50.
const NICE_LENGTHS_M: [f64; 17] = [
    1.0, 2.0, 2.5, 5.0, 10.0, 20.0, 25.0, 50.0, 100.0, 200.0, 250.0, 500.0, 1000.0, 2000.0, 2500.0,
    5000.0, 10000.0,
];

/// North arrow at the left, scale bar to its right, both from `top`.
pub fn write_tools(
    out: &mut String,
    furniture: &Furniture,
    typesetter: &Typesetter,
    x: f64,
    top: f64,
    width: f64,
) {
    let unit = typesetter.unit;
    let ink = &furniture.palette.ink;
    let mut bar_x = x;
    if furniture.sheet.show_north_arrow {
        write_north_arrow(out, typesetter, x, top, ink, furniture.north_deg);
        bar_x = x + ARROW_WIDTH * unit + ARROW_TO_BAR * unit;
    }
    if furniture.sheet.show_scale_bar {
        // Leave room for the half-width of the last number and its unit.
        let max_length = x + width - bar_x - 8.0 * unit;
        write_scale_bar(out, furniture, typesetter, bar_x, top, max_length);
    }
}

/// A split needle: the west half solid, the east half outlined, "N" above.
/// In a turned view the needle turns with the drawing to keep pointing north.
fn write_north_arrow(
    out: &mut String,
    typesetter: &Typesetter,
    x: f64,
    top: f64,
    ink: &str,
    north_deg: f64,
) {
    let unit = typesetter.unit;
    let half = ARROW_WIDTH * unit / 2.0;
    let centre_x = x + half;
    let cap_height = typesetter.cap_height(Kind::Heading);
    let label_baseline = top + cap_height;
    let tip_y = label_baseline + ARROW_LABEL_GAP * unit;
    let base_y = tip_y + ARROW_HEIGHT * unit;
    let notch_y = base_y - ARROW_HEIGHT * unit * ARROW_NOTCH_SHARE;
    let middle_y = (tip_y + base_y) / 2.0;

    // The "N" swings round the needle's middle with it and stays upright.
    let angle = north_deg.to_radians();
    let reach = middle_y - (label_baseline - cap_height / 2.0);
    typesetter.write(
        out,
        centre_x + reach * angle.sin(),
        middle_y - reach * angle.cos() + cap_height / 2.0,
        "N",
        Kind::Heading,
        Some("middle"),
    );
    let turn = if north_deg.abs() < 0.01 {
        String::new()
    } else {
        format!(
            r#" transform="rotate({} {} {})""#,
            decimal(north_deg),
            mm(centre_x),
            mm(middle_y)
        )
    };
    let _ = write!(
        out,
        r#"<g id="north-arrow" stroke="{ink}" stroke-width="{w}" stroke-linejoin="miter"{turn}><path d="M{cx} {tip}L{left} {base}L{cx} {notch}z" fill="{ink}"/><path d="M{cx} {tip}L{right} {base}L{cx} {notch}z" fill="none"/></g>"#,
        w = mm(BAR_STROKE_MM * unit.min(1.4)),
        cx = mm(centre_x),
        tip = mm(tip_y),
        left = mm(centre_x - half),
        right = mm(centre_x + half),
        base = mm(base_y),
        notch = mm(notch_y),
    );
}

/// Alternating blocks with "0", the midpoint and the full length underneath,
/// and the scale note above.
fn write_scale_bar(
    out: &mut String,
    furniture: &Furniture,
    typesetter: &Typesetter,
    x: f64,
    top: f64,
    max_length_mm: f64,
) {
    let unit = typesetter.unit;
    let ink = &furniture.palette.ink;
    let length_m = nice_length(max_length_mm / furniture.mm_per_m);
    let length_mm = length_m * furniture.mm_per_m;

    let note_baseline = top + typesetter.cap_height(Kind::Small);
    typesetter.write(
        out,
        x,
        note_baseline,
        &scale_note(furniture.sheet, furniture.is_foreshortened),
        Kind::Small,
        None,
    );

    let bar_top = note_baseline + 2.4 * unit;
    let bar_height = BAR_HEIGHT * unit;
    let segment = length_mm / BAR_SEGMENTS as f64;
    let _ = write!(
        out,
        r#"<g id="scale-bar" stroke="{ink}" stroke-width="{}">"#,
        mm(BAR_STROKE_MM * unit.min(1.4))
    );
    for index in 0..BAR_SEGMENTS {
        let fill = if index % 2 == 0 {
            ink.as_str()
        } else {
            furniture.palette.paper.as_str()
        };
        let _ = write!(
            out,
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{fill}"/>"#,
            mm(x + segment * index as f64),
            mm(bar_top),
            mm(segment),
            mm(bar_height)
        );
    }
    out.push_str("</g>");

    let label_baseline =
        bar_top + bar_height + BAR_LABEL_GAP * unit + typesetter.cap_height(Kind::Small);
    typesetter.write(out, x, label_baseline, "0", Kind::Small, Some("middle"));
    typesetter.write(
        out,
        x + length_mm / 2.0,
        label_baseline,
        &metres(length_m / 2.0),
        Kind::Small,
        Some("middle"),
    );
    let end_label = metres(length_m);
    typesetter.write(
        out,
        x + length_mm,
        label_baseline,
        &end_label,
        Kind::Small,
        Some("middle"),
    );
    let unit_x = x + length_mm + typesetter.width(&end_label, Kind::Small) / 2.0 + 0.8 * unit;
    typesetter.write(out, unit_x, label_baseline, "m", Kind::Small, None);
}

/// The longest round length that fits, or the shortest one if none does.
fn nice_length(max_m: f64) -> f64 {
    NICE_LENGTHS_M
        .iter()
        .copied()
        .filter(|&length| length <= max_m)
        .fold(NICE_LENGTHS_M[0], f64::max)
}

/// 50.0 -> "50", 12.5 -> "12.5".
fn metres(value: f64) -> String {
    decimal(value)
}

/// "Scale 1:2500 on A3", or "Scale 1:2000 left to right, on A3" in a tilted
/// view, where the bar only holds for lengths running across the paper.
fn scale_note(sheet: &Sheet, is_foreshortened: bool) -> String {
    let paper = match sheet.paper {
        Paper::A4 => "A4",
        Paper::A3 => "A3",
        Paper::A2 => "A2",
        Paper::A1 => "A1",
        Paper::A0 => "A0",
        Paper::Square => "300 × 300 mm",
    };

    if is_foreshortened {
        return format!("Scale 1:{} left to right, on {paper}", sheet.scale);
    }

    format!("Scale 1:{} on {paper}", sheet.scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nice_length_picks_the_longest_round_length_that_fits() {
        assert_eq!(nice_length(180.0), 100.0);
        assert_eq!(nice_length(260.0), 250.0);
        assert_eq!(nice_length(4.0), 2.5);
        assert_eq!(nice_length(0.5), 1.0);
    }

    #[test]
    fn a_tilted_view_says_its_scale_holds_left_to_right() {
        let sheet = Sheet {
            scale: 2000,
            paper: Paper::A3,
            ..Sheet::default()
        };
        assert_eq!(scale_note(&sheet, false), "Scale 1:2000 on A3");
        assert_eq!(
            scale_note(&sheet, true),
            "Scale 1:2000 left to right, on A3"
        );
    }
}
