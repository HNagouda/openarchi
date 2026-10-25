//! Analysis sections as SVG pages for export: one section on its own page,
//! or the whole report as one long page, in the theme's paper, ink and type.
//!
//! Like the sheets and the ratios page, a page is sized in millimetres and
//! every section is a named layer for Illustrator, Inkscape and Affinity.

use std::fmt::Write;

use super::figure::{text, Paint, WIDTH_MM as FIGURE_WIDTH_MM};
use super::section_parts::{fact_grid, heading, note_list, small, GAP_MM};
use super::sections::unique_credits;
use super::Section;
use crate::render::layer_attributes;
use crate::theme::Theme;
use crate::Result;

const PAGE_WIDTH_MM: f64 = 120.0;
const MARGIN_MM: f64 = 12.0;
const CONTENT_MM: f64 = PAGE_WIDTH_MM - 2.0 * MARGIN_MM;
/// Figures are drawn for the inspector's width and scaled up to the column.
const FIGURE_SCALE: f64 = CONTENT_MM / FIGURE_WIDTH_MM;

const TITLE_MM: f64 = 6.0;
const KICKER_MM: f64 = 2.8;
/// From the last line of one section to the top of the next one's title.
const SECTION_GAP_MM: f64 = 8.0;

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
    y += GAP_MM;

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
        y = heading(&mut body, title, MARGIN_MM, y, CONTENT_MM, &paint);
        let line = format!("Could not be fetched: {error}");
        y = small(&mut body, &line, MARGIN_MM, y, CONTENT_MM, &paint);
    }

    y += SECTION_GAP_MM;
    let credits: Vec<String> = unique_credits(sections)
        .iter()
        .map(|credit| format!("{} ({})", credit.name, credit.license))
        .collect();
    if !credits.is_empty() {
        let line = format!("Data: {}.", credits.join("; "));
        y = small(&mut body, &line, MARGIN_MM, y, CONTENT_MM, &paint);
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
    let mut y = heading(out, &section.title, MARGIN_MM, y, CONTENT_MM, paint);
    // Where the numbers come from, right under the title, as in the inspector.
    y = small(out, &section.source, MARGIN_MM, y, CONTENT_MM, paint);
    y += GAP_MM / 2.0;
    y = fact_grid(out, &section.facts, MARGIN_MM, y, CONTENT_MM, paint);

    for figure in &section.figures {
        y += GAP_MM;
        let _ = writeln!(
            out,
            r#"<g transform="translate({MARGIN_MM} {y:.2}) scale({FIGURE_SCALE:.4})">"#
        );
        out.push_str(&figure.body);
        out.push_str("</g>\n");
        y += figure.height_mm * FIGURE_SCALE;
    }
    if let Some(caption) = &section.caption {
        y += GAP_MM / 2.0;
        y = small(out, caption, MARGIN_MM, y, CONTENT_MM, paint);
    }
    if !section.notes.is_empty() {
        y += GAP_MM / 2.0;
        y = note_list(out, &section.notes, MARGIN_MM, y, CONTENT_MM, paint);
    }
    for warning in &section.warnings {
        y += GAP_MM / 2.0;
        y = small(out, warning, MARGIN_MM, y, CONTENT_MM, paint);
    }

    y
}
