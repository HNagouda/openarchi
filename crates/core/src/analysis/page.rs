//! Analysis sections as SVG pages for export: one section on its own page,
//! or the whole report as one long page, in the theme's paper, ink and type.
//!
//! Like the sheets and the ratios page, a page is sized in millimetres and
//! every section is a named layer for Illustrator, Inkscape and Affinity.

use std::fmt::Write;

use super::figure::{text, Paint, WIDTH_MM as FIGURE_WIDTH_MM};
use super::sections::unique_credits;
use super::Section;
use crate::render::{layer_attributes, wrap_text};
use crate::theme::Theme;
use crate::Result;

const PAGE_WIDTH_MM: f64 = 120.0;
const MARGIN_MM: f64 = 12.0;
const CONTENT_MM: f64 = PAGE_WIDTH_MM - 2.0 * MARGIN_MM;
/// Figures are drawn for the inspector's width and scaled up to the column.
const FIGURE_SCALE: f64 = CONTENT_MM / FIGURE_WIDTH_MM;

const TITLE_MM: f64 = 6.0;
const KICKER_MM: f64 = 2.8;
const SECTION_TITLE_MM: f64 = 4.2;
const FACT_LABEL_MM: f64 = 2.5;
const FACT_VALUE_MM: f64 = 3.3;
const FACT_ROW_MM: f64 = 9.0;
const FACT_COLUMNS: usize = 2;
const BODY_MM: f64 = 3.0;
const BODY_LINE_MM: f64 = 4.3;
const SMALL_MM: f64 = 2.5;
const SMALL_LINE_MM: f64 = 3.6;
const NOTE_INDENT_MM: f64 = 3.5;
const RULE_MM: f64 = 0.2;
const SECTION_GAP_MM: f64 = 12.0;
const BLOCK_GAP_MM: f64 = 4.0;

/// One section on its own page, headed with the project's name.
pub fn section_svg(section: &Section, theme: &Theme, project_name: &str) -> Result<String> {
    report_svg(std::slice::from_ref(section), &[], theme, project_name)
}

/// Every section on one long page, then a line for each part that could not
/// be fetched, then the data credits.
pub fn report_svg(
    sections: &[Section],
    failures: &[(String, String)],
    theme: &Theme,
    project_name: &str,
) -> Result<String> {
    let paint = Paint::new(theme)?;
    let mut body = String::new();
    let mut y = MARGIN_MM + KICKER_MM;
    let kicker = if sections.len() == 1 {
        format!("Site analysis · {}", sections[0].title)
    } else {
        "Site analysis".to_string()
    };
    text(
        &mut body,
        MARGIN_MM,
        y,
        KICKER_MM,
        400,
        &paint.secondary,
        "start",
        &paint.label(&kicker),
    );
    y += TITLE_MM + 1.5;
    text(
        &mut body,
        MARGIN_MM,
        y,
        TITLE_MM,
        paint.face.title_weight,
        &paint.ink,
        "start",
        project_name,
    );
    y += BLOCK_GAP_MM;

    for section in sections {
        y += SECTION_GAP_MM;
        let _ = writeln!(
            body,
            "<g {}>",
            layer_attributes(&section.id, &section.title)
        );
        y = write_section(&mut body, section, y, &paint);
        body.push_str("</g>\n");
    }
    for (title, error) in failures {
        y += SECTION_GAP_MM;
        y = write_heading(&mut body, title, y, &paint);
        y = paragraph(
            &mut body,
            &format!("Could not be fetched: {error}"),
            MARGIN_MM,
            y,
            CONTENT_MM,
            BODY_MM,
            BODY_LINE_MM,
            &paint.secondary,
            &paint,
        );
    }

    y += SECTION_GAP_MM;
    let credits: Vec<String> = unique_credits(sections)
        .iter()
        .map(|credit| format!("{} ({})", credit.name, credit.license))
        .collect();
    if !credits.is_empty() {
        let line = format!("Data: {}.", credits.join("; "));
        y = paragraph(
            &mut body,
            &line,
            MARGIN_MM,
            y,
            CONTENT_MM,
            SMALL_MM,
            SMALL_LINE_MM,
            &paint.secondary,
            &paint,
        );
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

fn write_section(out: &mut String, section: &Section, y: f64, paint: &Paint) -> f64 {
    let mut y = write_heading(out, &section.title, y, paint);
    // Where the numbers come from, right under the title, as in the inspector.
    y = paragraph(
        out,
        &section.source,
        MARGIN_MM,
        y,
        CONTENT_MM,
        SMALL_MM,
        SMALL_LINE_MM,
        &paint.secondary,
        paint,
    );
    y += BLOCK_GAP_MM / 2.0;

    for (index, fact) in section.facts.iter().enumerate() {
        let column = index % FACT_COLUMNS;
        let row = index / FACT_COLUMNS;
        let x = MARGIN_MM + column as f64 * CONTENT_MM / FACT_COLUMNS as f64;
        let top = y + row as f64 * FACT_ROW_MM;
        text(
            out,
            x,
            top + FACT_LABEL_MM,
            FACT_LABEL_MM,
            400,
            &paint.secondary,
            "start",
            &fact.label,
        );
        text(
            out,
            x,
            top + FACT_LABEL_MM + FACT_VALUE_MM + 0.8,
            FACT_VALUE_MM,
            600,
            &paint.ink,
            "start",
            &fact.value,
        );
    }
    y += section.facts.len().div_ceil(FACT_COLUMNS) as f64 * FACT_ROW_MM;

    for figure in &section.figures {
        y += BLOCK_GAP_MM;
        let _ = writeln!(
            out,
            r#"<g transform="translate({MARGIN_MM} {y:.2}) scale({FIGURE_SCALE:.4})">"#
        );
        out.push_str(&figure.body);
        out.push_str("</g>\n");
        y += figure.height_mm * FIGURE_SCALE;
    }
    if let Some(caption) = &section.caption {
        y += BLOCK_GAP_MM / 2.0;
        y = paragraph(
            out,
            caption,
            MARGIN_MM,
            y,
            CONTENT_MM,
            SMALL_MM,
            SMALL_LINE_MM,
            &paint.secondary,
            paint,
        );
    }

    for note in &section.notes {
        y += BLOCK_GAP_MM / 2.0;
        text(
            out,
            MARGIN_MM,
            y + BODY_MM,
            BODY_MM,
            600,
            &paint.accent,
            "start",
            "—",
        );
        y = paragraph(
            out,
            note,
            MARGIN_MM + NOTE_INDENT_MM,
            y,
            CONTENT_MM - NOTE_INDENT_MM,
            BODY_MM,
            BODY_LINE_MM,
            &paint.ink,
            paint,
        );
    }
    for warning in &section.warnings {
        y += BLOCK_GAP_MM / 2.0;
        y = paragraph(
            out,
            warning,
            MARGIN_MM,
            y,
            CONTENT_MM,
            SMALL_MM,
            SMALL_LINE_MM,
            &paint.secondary,
            paint,
        );
    }

    y
}

fn write_heading(out: &mut String, title: &str, y: f64, paint: &Paint) -> f64 {
    text(
        out,
        MARGIN_MM,
        y,
        SECTION_TITLE_MM,
        600,
        &paint.ink,
        "start",
        &paint.label(title),
    );
    let rule_y = y + 2.2;
    let _ = writeln!(
        out,
        r#"<line x1="{MARGIN_MM}" y1="{rule_y:.2}" x2="{:.2}" y2="{rule_y:.2}" stroke="{}" stroke-width="{RULE_MM}"/>"#,
        MARGIN_MM + CONTENT_MM,
        paint.faint
    );

    rule_y + BLOCK_GAP_MM
}

/// Wrapped text from `y` (the top of the first line). Returns the y below it.
#[allow(clippy::too_many_arguments)]
pub(super) fn paragraph(
    out: &mut String,
    content: &str,
    x: f64,
    y: f64,
    width: f64,
    size: f64,
    pitch: f64,
    fill: &str,
    paint: &Paint,
) -> f64 {
    let lines = wrap_text(content, &paint.face, 400, false, size, 0.0, width);
    let mut baseline = y + size;
    for line in &lines {
        text(out, x, baseline, size, 400, fill, "start", line);
        baseline += pitch;
    }

    baseline - pitch + size * 0.4
}
