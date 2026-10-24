//! One ratios group laid out at a block's width: as bars, a pie with its
//! legend, or a plain table. Colours are the rows' own, which already come
//! from the palette the ratios were computed in.

use std::f64::consts::TAU;
use std::fmt::Write;

use super::draw::Drawing;
use super::RatiosStyle;
use crate::analysis::{text, Paint};
use crate::ratios::{slice_path, total_text, value_text, Group, Row, Unit};

const TITLE_MM: f64 = 3.6;
const TEXT_MM: f64 = 2.8;
const HEADER_GAP_MM: f64 = 3.5;
const HAIRLINE_MM: f64 = 0.15;

const SWATCH_MM: f64 = 2.6;
/// From the swatch's left edge to the label.
const LABEL_INDENT_MM: f64 = 4.4;
/// The share column ends this far left of the value column.
const SHARE_COLUMN_MM: f64 = 24.0;

const BAR_ROW_MM: f64 = 7.6;
const BAR_HEIGHT_MM: f64 = 1.2;
const BAR_GAP_MM: f64 = 1.6;
const TABLE_ROW_MM: f64 = 5.4;

/// The pie takes this share of the width, within these limits, and the legend the rest.
const PIE_SHARE: f64 = 0.36;
const PIE_RADIUS_MIN_MM: f64 = 8.0;
const PIE_RADIUS_MAX_MM: f64 = 26.0;
const PIE_LEGEND_GAP_MM: f64 = 6.0;
const PIE_ROW_MM: f64 = 5.0;
/// Paper-coloured gap between pie slices.
const SLICE_GAP_MM: f64 = 0.35;

pub fn draw(group: &Group, style: RatiosStyle, width: f64, paint: &Paint) -> Drawing {
    let mut body = String::new();
    let top = header(&mut body, group, width, paint);
    let height = match style {
        RatiosStyle::Bars => bars(&mut body, group, top, width, paint),
        RatiosStyle::Pie => pie(&mut body, group, top, width, paint),
        RatiosStyle::Table => table(&mut body, group, top, width, paint),
    };

    Drawing {
        body,
        width,
        height,
    }
}

/// The group title on the left and its total on the right. Returns the y below.
fn header(out: &mut String, group: &Group, width: f64, paint: &Paint) -> f64 {
    text(
        out,
        0.0,
        TITLE_MM,
        TITLE_MM,
        600,
        &paint.ink,
        "start",
        &paint.label(&group.title),
    );
    text(
        out,
        width,
        TITLE_MM,
        TEXT_MM,
        400,
        &paint.secondary,
        "end",
        &total_text(group),
    );

    TITLE_MM + HEADER_GAP_MM
}

fn bars(out: &mut String, group: &Group, top: f64, width: f64, paint: &Paint) -> f64 {
    let mut y = top;
    for row in &group.rows {
        let baseline = y + TEXT_MM;
        row_text(out, row, group.unit, 0.0, baseline, width, paint);
        let bar_y = baseline + BAR_GAP_MM;
        rect(out, 0.0, bar_y, width, BAR_HEIGHT_MM, &paint.faint);
        rect(out, 0.0, bar_y, row.share * width, BAR_HEIGHT_MM, &row.fill);
        y += BAR_ROW_MM;
    }

    y
}

fn pie(out: &mut String, group: &Group, top: f64, width: f64, paint: &Paint) -> f64 {
    let radius = (width * PIE_SHARE / 2.0).clamp(PIE_RADIUS_MIN_MM, PIE_RADIUS_MAX_MM);
    let centre = (radius, top + radius);
    let mut start = 0.0;
    for row in &group.rows {
        let end = start + row.share * TAU;
        let _ = writeln!(
            out,
            r#"<path d="{}" fill="{}" stroke="{}" stroke-width="{SLICE_GAP_MM}"/>"#,
            slice_path(centre, radius, start, end),
            row.fill,
            paint.paper
        );
        start = end;
    }

    let legend_x = 2.0 * radius + PIE_LEGEND_GAP_MM;
    let mut baseline = top + TEXT_MM;
    for row in &group.rows {
        row_text(out, row, group.unit, legend_x, baseline, width, paint);
        baseline += PIE_ROW_MM;
    }

    (top + 2.0 * radius).max(baseline - PIE_ROW_MM + BAR_GAP_MM)
}

fn table(out: &mut String, group: &Group, top: f64, width: f64, paint: &Paint) -> f64 {
    let mut y = top;
    for row in &group.rows {
        row_text(out, row, group.unit, 0.0, y + TEXT_MM, width, paint);
        y += TABLE_ROW_MM;
        let rule_y = y - BAR_GAP_MM;
        let _ = writeln!(
            out,
            r#"<line x1="0" y1="{rule_y:.2}" x2="{width:.2}" y2="{rule_y:.2}" stroke="{}" stroke-width="{HAIRLINE_MM}"/>"#,
            paint.faint
        );
    }

    y
}

/// Swatch and label from `x`, share and value right-aligned at `right`.
fn row_text(
    out: &mut String,
    row: &Row,
    unit: Unit,
    x: f64,
    baseline: f64,
    right: f64,
    paint: &Paint,
) {
    let _ = writeln!(
        out,
        r#"<rect x="{x:.2}" y="{:.2}" width="{SWATCH_MM}" height="{SWATCH_MM}" fill="{}" stroke="{}" stroke-width="0.1"/>"#,
        baseline - SWATCH_MM + 0.1,
        row.fill,
        paint.faint
    );
    text(
        out,
        x + LABEL_INDENT_MM,
        baseline,
        TEXT_MM,
        400,
        &paint.ink,
        "start",
        &row.label,
    );
    let share = format!("{:.1} %", row.share * 100.0);
    text(
        out,
        right - SHARE_COLUMN_MM,
        baseline,
        TEXT_MM,
        600,
        &paint.ink,
        "end",
        &share,
    );
    text(
        out,
        right,
        baseline,
        TEXT_MM,
        400,
        &paint.secondary,
        "end",
        &value_text(row.value, unit),
    );
}

fn rect(out: &mut String, x: f64, y: f64, width: f64, height: f64, fill: &str) {
    let _ = writeln!(
        out,
        r#"<rect x="{x:.2}" y="{y:.2}" width="{:.2}" height="{height:.2}" fill="{fill}"/>"#,
        width.max(0.0)
    );
}
