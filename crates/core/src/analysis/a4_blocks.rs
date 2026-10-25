//! The blocks the A4 pages are dealt from: one per section, one for the
//! parts that could not be fetched, and one for the data credits.

use std::fmt::Write;

use super::a4::CONTENT_MM;
use super::figure::{Paint, WIDTH_MM as FIGURE_WIDTH_MM};
use super::section_parts::{body, bold_line, fact_grid, heading, note_list, small, GAP_MM};
use super::sections::unique_credits;
use super::Section;
use crate::render::layer_attributes;

/// Figures in a left column, text in a right one.
const FIGURE_COLUMN_MM: f64 = 92.0;
const COLUMN_GAP_MM: f64 = 8.0;
const TEXT_COLUMN_MM: f64 = CONTENT_MM - FIGURE_COLUMN_MM - COLUMN_GAP_MM;
/// Figures are drawn for the inspector's width and scaled up to the column.
const FIGURE_SCALE: f64 = FIGURE_COLUMN_MM / FIGURE_WIDTH_MM;

/// One drawn block, with its top at y = 0 and the content's left edge at x = 0.
pub struct Block {
    pub body: String,
    pub height: f64,
}

pub fn section_block(section: &Section, paint: &Paint) -> Block {
    let mut out = String::new();
    let _ = writeln!(out, "<g {}>", layer_attributes(&section.id, &section.title));
    let mut y = heading(&mut out, &section.title, 0.0, 0.0, CONTENT_MM, paint);
    // Where the numbers come from, right under the title, as in the inspector.
    y = small(&mut out, &section.source, 0.0, y, CONTENT_MM, paint);
    y += GAP_MM / 2.0;
    y = fact_grid(&mut out, &section.facts, 0.0, y, CONTENT_MM, paint);
    y += GAP_MM / 2.0;

    let mut left = y;
    for figure in &section.figures {
        let _ = writeln!(
            out,
            r#"<g transform="translate(0 {left:.2}) scale({FIGURE_SCALE:.4})">"#
        );
        out.push_str(&figure.body);
        out.push_str("</g>\n");
        left += figure.height_mm * FIGURE_SCALE;
    }
    let (x, width) = if section.figures.is_empty() {
        (0.0, CONTENT_MM)
    } else {
        (FIGURE_COLUMN_MM + COLUMN_GAP_MM, TEXT_COLUMN_MM)
    };
    let right = text_column(&mut out, section, x, y, width, paint);
    out.push_str("</g>\n");

    Block {
        body: out,
        height: left.max(right),
    }
}

/// Caption, notes and warnings, top to bottom. Returns the y below.
fn text_column(
    out: &mut String,
    section: &Section,
    x: f64,
    y: f64,
    width: f64,
    paint: &Paint,
) -> f64 {
    let mut y = y;
    if let Some(caption) = &section.caption {
        y = small(out, caption, x, y, width, paint);
    }
    if !section.notes.is_empty() {
        y += GAP_MM / 2.0;
        y = note_list(out, &section.notes, x, y, width, paint);
    }
    for warning in &section.warnings {
        y += GAP_MM / 2.0;
        y = small(out, warning, x, y, width, paint);
    }

    y
}

/// The parts that could not be fetched, so a reader knows what is missing.
pub fn failures_block(failures: &[(String, String)], paint: &Paint) -> Block {
    let mut out = String::new();
    let mut y = heading(&mut out, "Not included", 0.0, 0.0, CONTENT_MM, paint);
    for (title, error) in failures {
        let line = format!("{title} could not be fetched: {error}");
        y = body(&mut out, &line, 0.0, y, CONTENT_MM, paint);
        y += GAP_MM / 2.0;
    }

    Block {
        body: out,
        height: y,
    }
}

/// Every source the sections used, with its licence and where to find it.
pub fn credits_block(sections: &[Section], paint: &Paint) -> Block {
    let mut out = String::new();
    let mut y = heading(&mut out, "Data credits", 0.0, 0.0, CONTENT_MM, paint);
    for credit in unique_credits(sections) {
        y = bold_line(&mut out, &credit.name, 0.0, y, paint);
        let detail = format!("{}. {}", credit.license, credit.url);
        y = small(&mut out, &detail, 0.0, y, CONTENT_MM, paint);
        y += GAP_MM / 2.0;
    }

    Block {
        body: out,
        height: y,
    }
}
