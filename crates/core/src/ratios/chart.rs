//! Ratios drawn as one SVG page of charts, in the theme's paper, ink and type.
//!
//! Like the sheets, the page is sized in millimetres with one user unit per
//! millimetre, and each group is a named layer for Illustrator and Affinity.

use std::f64::consts::TAU;
use std::fmt::Write;

use serde::Deserialize;

use super::{group_chart, Group, Ratios, Unit};
use crate::analysis::{text, Paint};
use crate::render::layer_attributes;
use crate::theme::Theme;
use crate::Result;

const PAGE_WIDTH_MM: f64 = 140.0;
const MARGIN_MM: f64 = 12.0;
const CONTENT_WIDTH_MM: f64 = PAGE_WIDTH_MM - 2.0 * MARGIN_MM;

const TITLE_SIZE_MM: f64 = 6.0;
const SUBTITLE_SIZE_MM: f64 = 2.8;
const GROUP_GAP_MM: f64 = 9.0;
const HEADER_GAP_MM: f64 = 6.0;

const HECTARE_M2: f64 = 10_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartStyle {
    Bar,
    Pie,
}

/// A page with every group as bars or pies, headed by `title`. The groups
/// are drawn as the report's ratios blocks draw them.
pub fn chart_svg(ratios: &Ratios, theme: &Theme, style: ChartStyle, title: &str) -> Result<String> {
    let paint = Paint::new(theme)?;

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
        let _ = writeln!(
            body,
            r#"<g {} transform="translate({MARGIN_MM} {y:.2})">"#,
            layer_attributes(&group.id, &group.title)
        );
        let top = group_chart::header(&mut body, group, CONTENT_WIDTH_MM, &paint);
        y += match style {
            ChartStyle::Bar => group_chart::bars(&mut body, group, top, CONTENT_WIDTH_MM, &paint),
            ChartStyle::Pie => group_chart::pie(&mut body, group, top, CONTENT_WIDTH_MM, &paint),
        };
        body.push_str("</g>\n");
    }

    let height = y + MARGIN_MM;
    let mut out = String::with_capacity(body.len() + 512);
    let _ = writeln!(
        out,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" width="{PAGE_WIDTH_MM}mm" height="{height:.1}mm" viewBox="0 0 {PAGE_WIDTH_MM} {height:.1}" font-family="{}">"#,
        paint.face.css_stack()
    );
    let _ = writeln!(
        out,
        r#"<rect id="paper" width="{PAGE_WIDTH_MM}" height="{height:.1}" fill="{}"/>"#,
        paint.paper
    );
    out.push_str(&body);
    out.push_str("</svg>\n");

    Ok(out)
}

/// A wedge from `start` to `end` radians, clockwise from twelve o'clock.
pub(crate) fn slice_path((cx, cy): (f64, f64), radius: f64, start: f64, end: f64) -> String {
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

fn site_line(ratios: &Ratios) -> String {
    let mut line = format!(
        "Site {} · {} buildings · coverage {:.0} % · floor area ratio {:.2}",
        area_text(ratios.site_area_m2),
        grouped(ratios.building_count as f64),
        ratios.coverage * 100.0,
        ratios.floor_area_ratio
    );
    if let Some(ratio) = ratios.bag_floor_area_ratio {
        line.push_str(&format!(" ({ratio:.2} from BAG)"));
    }
    if let Some(dwellings) = ratios.dwellings {
        line.push_str(&format!(" · {} dwellings", grouped(f64::from(dwellings))));
    }

    line
}

pub(crate) fn total_text(group: &Group) -> String {
    match group.unit {
        Unit::SquareMetres => area_text(group.total),
        Unit::Buildings => format!("{} buildings", grouped(group.total)),
        Unit::Parcels => format!("{} parcels", grouped(group.total)),
        Unit::Places => format!("{} places", grouped(group.total)),
        Unit::Stops => format!("{} stops", grouped(group.total)),
    }
}

pub(crate) fn value_text(value: f64, unit: Unit) -> String {
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
