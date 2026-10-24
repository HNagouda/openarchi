//! The blocks the A4 pages are dealt from: one per section, one for the
//! parts that could not be fetched, and one for the data credits.

use std::fmt::Write;

use super::a4::{CONTENT_MM, HAIRLINE_MM};
use super::figure::{text, Paint, WIDTH_MM as FIGURE_WIDTH_MM};
use super::page::paragraph;
use super::sections::unique_credits;
use super::Section;
use crate::render::{layer_attributes, text_width};

/// Figures in a left column, text in a right one.
const FIGURE_COLUMN_MM: f64 = 92.0;
const COLUMN_GAP_MM: f64 = 8.0;
const TEXT_COLUMN_MM: f64 = CONTENT_MM - FIGURE_COLUMN_MM - COLUMN_GAP_MM;
/// Figures are drawn for the inspector's width and scaled up to the column.
const FIGURE_SCALE: f64 = FIGURE_COLUMN_MM / FIGURE_WIDTH_MM;

const SECTION_TITLE_MM: f64 = 5.0;
const FACT_LABEL_MM: f64 = 2.5;
const FACT_VALUE_MM: f64 = 3.6;
const FACT_ROW_MM: f64 = 10.0;
const FACT_COLUMNS: usize = 4;
/// The least room between one fact and the next.
const FACT_GAP_MM: f64 = 6.0;
const BODY_MM: f64 = 3.0;
const BODY_LINE_MM: f64 = 4.3;
const SMALL_MM: f64 = 2.5;
const SMALL_LINE_MM: f64 = 3.5;
const NOTE_INDENT_MM: f64 = 4.5;
const BLOCK_GAP_MM: f64 = 4.0;

/// One drawn block, with its top at y = 0 and the content's left edge at x = 0.
pub struct Block {
    pub body: String,
    pub height: f64,
}

pub fn section_block(section: &Section, paint: &Paint) -> Block {
    let mut body = String::new();
    let _ = writeln!(
        body,
        "<g {}>",
        layer_attributes(&section.id, &section.title)
    );
    let mut y = heading(&mut body, &section.title, paint);
    // Where the numbers come from, right under the title, as in the inspector.
    y = paragraph(
        &mut body,
        &section.source,
        0.0,
        y,
        CONTENT_MM,
        SMALL_MM,
        SMALL_LINE_MM,
        &paint.secondary,
        paint,
    );
    y += BLOCK_GAP_MM / 2.0;
    y = facts(&mut body, section, y, paint);

    let mut left = y;
    for figure in &section.figures {
        let _ = writeln!(
            body,
            r#"<g transform="translate(0 {left:.2}) scale({FIGURE_SCALE:.4})">"#
        );
        body.push_str(&figure.body);
        body.push_str("</g>\n");
        left += figure.height_mm * FIGURE_SCALE;
    }
    let (x, width) = if section.figures.is_empty() {
        (0.0, CONTENT_MM)
    } else {
        (FIGURE_COLUMN_MM + COLUMN_GAP_MM, TEXT_COLUMN_MM)
    };
    let right = text_column(&mut body, section, x, y, width, paint);
    body.push_str("</g>\n");

    Block {
        body,
        height: left.max(right),
    }
}

/// The section title and a rule under it. Returns the y below.
pub(super) fn heading(out: &mut String, title: &str, paint: &Paint) -> f64 {
    text(
        out,
        0.0,
        SECTION_TITLE_MM,
        SECTION_TITLE_MM,
        600,
        &paint.ink,
        "start",
        &paint.label(title),
    );
    let rule_y = SECTION_TITLE_MM + 2.4;
    let _ = writeln!(
        out,
        r#"<line x1="0" y1="{rule_y:.2}" x2="{CONTENT_MM}" y2="{rule_y:.2}" stroke="{}" stroke-width="{HAIRLINE_MM}"/>"#,
        paint.faint
    );

    rule_y + BLOCK_GAP_MM
}

/// The facts in a grid of `FACT_COLUMNS`; a long value takes as many columns
/// as it needs, and moves to the next row when the row has no room left.
fn facts(out: &mut String, section: &Section, y: f64, paint: &Paint) -> f64 {
    let column_width = CONTENT_MM / FACT_COLUMNS as f64;
    let mut column = 0;
    let mut row = 0;
    for fact in &section.facts {
        let width = text_width(&fact.value, &paint.face, 600, false, FACT_VALUE_MM, 0.0).max(
            text_width(&fact.label, &paint.face, 400, false, FACT_LABEL_MM, 0.0),
        ) + FACT_GAP_MM;
        let span = ((width / column_width).ceil() as usize).clamp(1, FACT_COLUMNS);
        if column + span > FACT_COLUMNS {
            column = 0;
            row += 1;
        }
        let x = column as f64 * column_width;
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
            top + FACT_LABEL_MM + FACT_VALUE_MM + 0.9,
            FACT_VALUE_MM,
            600,
            &paint.ink,
            "start",
            &fact.value,
        );
        column += span;
    }
    let rows = if section.facts.is_empty() { 0 } else { row + 1 };

    y + rows as f64 * FACT_ROW_MM + BLOCK_GAP_MM / 2.0
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
    let small = |out: &mut String, content: &str, y: f64| {
        paragraph(
            out,
            content,
            x,
            y,
            width,
            SMALL_MM,
            SMALL_LINE_MM,
            &paint.secondary,
            paint,
        )
    };
    let mut y = y;
    if let Some(caption) = &section.caption {
        y = small(out, caption, y);
    }
    for note in &section.notes {
        y += BLOCK_GAP_MM / 2.0;
        text(
            out,
            x,
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
            x + NOTE_INDENT_MM,
            y,
            width - NOTE_INDENT_MM,
            BODY_MM,
            BODY_LINE_MM,
            &paint.ink,
            paint,
        );
    }
    for warning in &section.warnings {
        y += BLOCK_GAP_MM / 2.0;
        y = small(out, warning, y);
    }

    y
}

/// The parts that could not be fetched, so a reader knows what is missing.
pub fn failures_block(failures: &[(String, String)], paint: &Paint) -> Block {
    let mut body = String::new();
    let mut y = heading(&mut body, "Not included", paint);
    for (title, error) in failures {
        let line = format!("{title} could not be fetched: {error}");
        y = paragraph(
            &mut body,
            &line,
            0.0,
            y,
            CONTENT_MM,
            BODY_MM,
            BODY_LINE_MM,
            &paint.ink,
            paint,
        );
        y += BLOCK_GAP_MM / 2.0;
    }

    Block { body, height: y }
}

/// Every source the sections used, with its licence and where to find it.
pub fn credits_block(sections: &[Section], paint: &Paint) -> Block {
    let mut body = String::new();
    let mut y = heading(&mut body, "Data credits", paint);
    for credit in unique_credits(sections) {
        text(
            &mut body,
            0.0,
            y + BODY_MM,
            BODY_MM,
            600,
            &paint.ink,
            "start",
            &credit.name,
        );
        y += BODY_LINE_MM;
        let detail = format!("{}. {}", credit.license, credit.url);
        y = paragraph(
            &mut body,
            &detail,
            0.0,
            y,
            CONTENT_MM,
            SMALL_MM,
            SMALL_LINE_MM,
            &paint.secondary,
            paint,
        );
        y += BLOCK_GAP_MM / 2.0;
    }

    Block { body, height: y }
}
