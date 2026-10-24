//! Ratios drawn as one SVG page of charts, in the theme's paper, ink and type.
//!
//! Like the sheets, the page is sized in millimetres with one user unit per
//! millimetre, and each group is a named layer for Illustrator and Affinity.

use std::f64::consts::TAU;
use std::fmt::Write;

use serde::Deserialize;

use super::{Group, Ratios, Row, Unit};
use crate::render::{escape, layer_attributes, mix, Face};
use crate::theme::Theme;

const PAGE_WIDTH_MM: f64 = 140.0;
const MARGIN_MM: f64 = 12.0;
const CONTENT_WIDTH_MM: f64 = PAGE_WIDTH_MM - 2.0 * MARGIN_MM;

const TITLE_SIZE_MM: f64 = 6.0;
const SUBTITLE_SIZE_MM: f64 = 2.8;
const GROUP_TITLE_SIZE_MM: f64 = 3.4;
const TEXT_SIZE_MM: f64 = 2.8;

const SWATCH_MM: f64 = 2.6;
/// From the swatch's left edge to the label.
const LABEL_INDENT_MM: f64 = 4.4;
/// The share column ends this far left of the value column.
const SHARE_COLUMN_MM: f64 = 26.0;
const BAR_ROW_PITCH_MM: f64 = 7.6;
const BAR_HEIGHT_MM: f64 = 1.2;
const PIE_RADIUS_MM: f64 = 15.0;
const PIE_ROW_PITCH_MM: f64 = 5.0;
/// Paper-coloured gap between pie slices.
const SLICE_GAP_MM: f64 = 0.35;
const GROUP_GAP_MM: f64 = 9.0;
const HEADER_GAP_MM: f64 = 6.0;

/// How far the secondary text and the bar track move from ink towards paper.
const SECONDARY_SHARE: f64 = 0.45;
const TRACK_SHARE: f64 = 0.88;

const HECTARE_M2: f64 = 10_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartStyle {
    Bar,
    Pie,
}

/// A page with every group as bars or pies, headed by `title`.
pub fn chart_svg(ratios: &Ratios, theme: &Theme, style: ChartStyle, title: &str) -> String {
    let face = Face::for_family(&theme.typography.family);
    let paint = Paint {
        ink: theme.palette.ink.clone(),
        secondary: mix(&theme.palette.ink, &theme.palette.paper, SECONDARY_SHARE),
        track: mix(&theme.palette.ink, &theme.palette.paper, TRACK_SHARE),
        paper: theme.palette.paper.clone(),
        is_uppercase: theme.typography.uppercase_labels,
    };

    let mut body = String::new();
    let mut y = MARGIN_MM + TITLE_SIZE_MM;
    text(
        &mut body,
        MARGIN_MM,
        y,
        TITLE_SIZE_MM,
        600,
        &paint.ink,
        "start",
        title,
    );
    y += SUBTITLE_SIZE_MM + 2.5;
    text(
        &mut body,
        MARGIN_MM,
        y,
        SUBTITLE_SIZE_MM,
        400,
        &paint.secondary,
        "start",
        &site_line(ratios),
    );
    y += HEADER_GAP_MM;

    for group in &ratios.groups {
        y += GROUP_GAP_MM;
        let _ = writeln!(body, "<g {}>", layer_attributes(&group.id, &group.title));
        y = write_group_header(&mut body, group, y, &paint);
        y = match style {
            ChartStyle::Bar => write_bars(&mut body, group, y, &paint),
            ChartStyle::Pie => write_pie(&mut body, group, y, &paint),
        };
        body.push_str("</g>\n");
    }

    let height = y + MARGIN_MM;
    let mut out = String::with_capacity(body.len() + 512);
    let _ = writeln!(
        out,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" width="{PAGE_WIDTH_MM}mm" height="{height:.1}mm" viewBox="0 0 {PAGE_WIDTH_MM} {height:.1}" font-family="{}">"#,
        face.css_stack()
    );
    let _ = writeln!(
        out,
        r#"<rect id="paper" width="{PAGE_WIDTH_MM}" height="{height:.1}" fill="{}"/>"#,
        paint.paper
    );
    out.push_str(&body);
    out.push_str("</svg>\n");

    out
}

/// The theme colours every part of the page uses, resolved once.
struct Paint {
    ink: String,
    secondary: String,
    track: String,
    paper: String,
    is_uppercase: bool,
}

fn write_group_header(out: &mut String, group: &Group, y: f64, paint: &Paint) -> f64 {
    let title = if paint.is_uppercase {
        group.title.to_uppercase()
    } else {
        group.title.clone()
    };
    let right = MARGIN_MM + CONTENT_WIDTH_MM;
    text(
        out,
        MARGIN_MM,
        y,
        GROUP_TITLE_SIZE_MM,
        600,
        &paint.ink,
        "start",
        &title,
    );
    text(
        out,
        right,
        y,
        TEXT_SIZE_MM,
        400,
        &paint.secondary,
        "end",
        &total_text(group),
    );

    y + 3.0
}

fn write_bars(out: &mut String, group: &Group, mut y: f64, paint: &Paint) -> f64 {
    for row in &group.rows {
        y += BAR_ROW_PITCH_MM - BAR_HEIGHT_MM - 1.6;
        write_row_text(out, row, group.unit, MARGIN_MM, y, paint);
        let bar_y = y + 1.6;
        let _ = writeln!(
            out,
            r#"<rect x="{MARGIN_MM}" y="{bar_y:.2}" width="{CONTENT_WIDTH_MM}" height="{BAR_HEIGHT_MM}" fill="{}"/>"#,
            paint.track
        );
        let _ = writeln!(
            out,
            r#"<rect x="{MARGIN_MM}" y="{bar_y:.2}" width="{:.2}" height="{BAR_HEIGHT_MM}" fill="{}"/>"#,
            row.share * CONTENT_WIDTH_MM,
            row.fill
        );
        y = bar_y + BAR_HEIGHT_MM + 1.6;
    }

    y
}

fn write_pie(out: &mut String, group: &Group, y: f64, paint: &Paint) -> f64 {
    let top = y + 2.0;
    let centre = (MARGIN_MM + PIE_RADIUS_MM, top + PIE_RADIUS_MM);
    let mut start = 0.0;
    for row in &group.rows {
        let end = start + row.share * TAU;
        let _ = writeln!(
            out,
            r#"<path d="{}" fill="{}" stroke="{}" stroke-width="{SLICE_GAP_MM}"/>"#,
            slice_path(centre, PIE_RADIUS_MM, start, end),
            row.fill,
            paint.paper
        );
        start = end;
    }

    let legend_x = MARGIN_MM + 2.0 * PIE_RADIUS_MM + 6.0;
    let mut row_y = top + TEXT_SIZE_MM;
    for row in &group.rows {
        write_row_text(out, row, group.unit, legend_x, row_y, paint);
        row_y += PIE_ROW_PITCH_MM;
    }

    (top + 2.0 * PIE_RADIUS_MM).max(row_y - PIE_ROW_PITCH_MM + 1.0)
}

/// Swatch and label at `x`, share and value right-aligned at the page's right edge.
fn write_row_text(out: &mut String, row: &Row, unit: Unit, x: f64, y: f64, paint: &Paint) {
    let right = MARGIN_MM + CONTENT_WIDTH_MM;
    let _ = writeln!(
        out,
        r#"<rect x="{x:.2}" y="{:.2}" width="{SWATCH_MM}" height="{SWATCH_MM}" fill="{}" stroke="{}" stroke-width="0.1"/>"#,
        y - SWATCH_MM + 0.1,
        row.fill,
        paint.track
    );
    text(
        out,
        x + LABEL_INDENT_MM,
        y,
        TEXT_SIZE_MM,
        400,
        &paint.ink,
        "start",
        &row.label,
    );
    let share = format!("{:.1} %", row.share * 100.0);
    text(
        out,
        right - SHARE_COLUMN_MM,
        y,
        TEXT_SIZE_MM,
        600,
        &paint.ink,
        "end",
        &share,
    );
    text(
        out,
        right,
        y,
        TEXT_SIZE_MM,
        400,
        &paint.secondary,
        "end",
        &value_text(row.value, unit),
    );
}

/// A wedge from `start` to `end` radians, clockwise from twelve o'clock.
fn slice_path((cx, cy): (f64, f64), radius: f64, start: f64, end: f64) -> String {
    // A full circle has no wedge edges; an arc to its own start draws nothing.
    if end - start >= TAU - 1e-9 {
        return format!(
            "M{:.2} {cy:.2}a{radius} {radius} 0 1 0 {:.2} 0a{radius} {radius} 0 1 0 {:.2} 0Z",
            cx - radius,
            2.0 * radius,
            -2.0 * radius
        );
    }

    let point = |angle: f64| (cx + radius * angle.sin(), cy - radius * angle.cos());
    let (start_x, start_y) = point(start);
    let (end_x, end_y) = point(end);
    let is_large = if end - start > TAU / 2.0 { 1 } else { 0 };
    format!(
        "M{cx:.2} {cy:.2}L{start_x:.2} {start_y:.2}A{radius} {radius} 0 {is_large} 1 {end_x:.2} {end_y:.2}Z"
    )
}

#[allow(clippy::too_many_arguments)]
fn text(
    out: &mut String,
    x: f64,
    y: f64,
    size: f64,
    weight: u16,
    fill: &str,
    anchor: &str,
    content: &str,
) {
    let _ = writeln!(
        out,
        r#"<text x="{x:.2}" y="{y:.2}" font-size="{size}" font-weight="{weight}" fill="{fill}" text-anchor="{anchor}">{}</text>"#,
        escape(content)
    );
}

fn site_line(ratios: &Ratios) -> String {
    format!(
        "Site {} · {} buildings · coverage {:.0} % · floor area ratio {:.2}",
        area_text(ratios.site_area_m2),
        grouped(ratios.building_count as f64),
        ratios.coverage * 100.0,
        ratios.floor_area_ratio
    )
}

fn total_text(group: &Group) -> String {
    match group.unit {
        Unit::SquareMetres => area_text(group.total),
        Unit::Buildings => format!("{} buildings", grouped(group.total)),
        Unit::Parcels => format!("{} parcels", grouped(group.total)),
        Unit::Places => format!("{} places", grouped(group.total)),
        Unit::Stops => format!("{} stops", grouped(group.total)),
    }
}

fn value_text(value: f64, unit: Unit) -> String {
    match unit {
        Unit::SquareMetres => area_text(value),
        Unit::Buildings | Unit::Parcels | Unit::Places | Unit::Stops => grouped(value),
    }
}

/// 2430 -> "2,430 m²", 52 000 -> "5.2 ha", the way the app writes areas.
pub(crate) fn area_text(m2: f64) -> String {
    if m2 < HECTARE_M2 {
        format!("{} m²", grouped(m2))
    } else {
        format!("{:.1} ha", m2 / HECTARE_M2)
    }
}

/// 12345.6 -> "12,346".
pub(crate) fn grouped(value: f64) -> String {
    let digits = format!("{:.0}", value.round());
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_numbers_get_thousands_separators() {
        assert_eq!(grouped(12345.6), "12,346");
        assert_eq!(grouped(999.0), "999");
        assert_eq!(area_text(2430.0), "2,430 m²");
        assert_eq!(area_text(52_000.0), "5.2 ha");
    }

    #[test]
    fn a_quarter_slice_ends_at_three_o_clock() {
        let path = slice_path((0.0, 0.0), 10.0, 0.0, TAU / 4.0);
        // Starts at twelve o'clock, a small arc clockwise; the y may print as -0.00.
        assert!(path.starts_with("M0.00 0.00L0.00 -10.00"), "{path}");
        assert!(path.contains("A10 10 0 0 1 10.00 "), "{path}");
    }
}
