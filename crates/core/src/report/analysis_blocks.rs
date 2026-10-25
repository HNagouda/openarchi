//! An analysis section's facts or design notes, laid out at a block's width.

// #! TODO[analysis, report] : One fact grid and heading for analysis/page.rs, analysis/a4_blocks.rs and this file
// The three draw the same section with their own sizes, and the heading and fact gap have drifted apart.

use std::fmt::Write;

use super::draw::Drawing;
use crate::analysis::{paragraph, text, Paint, Section};
use crate::render::text_width;

const TITLE_MM: f64 = 4.2;
const TITLE_RULE_GAP_MM: f64 = 2.2;
const HAIRLINE_MM: f64 = 0.2;
const GAP_MM: f64 = 3.5;

const FACT_LABEL_MM: f64 = 2.5;
const FACT_VALUE_MM: f64 = 3.6;
const FACT_ROW_MM: f64 = 10.0;
/// A fact column is never narrower than this, and there are at most four.
const FACT_COLUMN_MIN_MM: f64 = 38.0;
const FACT_COLUMNS_MAX: usize = 4;
/// The least room between one fact and the next.
const FACT_GAP_MM: f64 = 5.0;

const BODY_MM: f64 = 3.0;
const BODY_LINE_MM: f64 = 4.3;
const SMALL_MM: f64 = 2.5;
const SMALL_LINE_MM: f64 = 3.5;
const NOTE_INDENT_MM: f64 = 4.5;

/// The section title over its facts in a grid; a long value takes as many
/// columns as it needs and moves to the next row when this one is full.
pub fn facts(section: &Section, width: f64, paint: &Paint) -> Drawing {
    let mut body = String::new();
    let top = heading(&mut body, &section.title, width, paint);
    let columns = ((width / FACT_COLUMN_MIN_MM).floor() as usize).clamp(1, FACT_COLUMNS_MAX);
    let column_width = width / columns as f64;

    let mut column = 0;
    let mut row = 0;
    for fact in &section.facts {
        let needed = text_width(&fact.value, &paint.face, 600, false, FACT_VALUE_MM, 0.0).max(
            text_width(&fact.label, &paint.face, 400, false, FACT_LABEL_MM, 0.0),
        ) + FACT_GAP_MM;
        let span = ((needed / column_width).ceil() as usize).clamp(1, columns);
        if column + span > columns {
            column = 0;
            row += 1;
        }
        let x = column as f64 * column_width;
        let y = top + row as f64 * FACT_ROW_MM;
        text(
            &mut body,
            x,
            y + FACT_LABEL_MM,
            FACT_LABEL_MM,
            400,
            &paint.secondary,
            "start",
            &fact.label,
        );
        let value_y = y + FACT_LABEL_MM + FACT_VALUE_MM + 0.9;
        text(
            &mut body,
            x,
            value_y,
            FACT_VALUE_MM,
            600,
            &paint.ink,
            "start",
            &fact.value,
        );
        column += span;
    }
    let rows = if section.facts.is_empty() { 0 } else { row + 1 };

    Drawing {
        body,
        width,
        height: top + rows as f64 * FACT_ROW_MM,
    }
}

/// The section title over its design notes, any warnings, and the line
/// saying where its numbers come from.
pub fn notes(section: &Section, width: f64, paint: &Paint) -> Drawing {
    let mut body = String::new();
    let mut y = heading(&mut body, &section.title, width, paint);
    if section.notes.is_empty() {
        y = paragraph(
            &mut body,
            "No design notes for this section.",
            0.0,
            y,
            width,
            BODY_MM,
            BODY_LINE_MM,
            &paint.secondary,
            paint,
        );
    }
    for (index, note) in section.notes.iter().enumerate() {
        if index > 0 {
            y += GAP_MM / 2.0;
        }
        text(
            &mut body,
            0.0,
            y + BODY_MM,
            BODY_MM,
            600,
            &paint.accent,
            "start",
            "—",
        );
        let note_width = (width - NOTE_INDENT_MM).max(BODY_MM);
        y = paragraph(
            &mut body,
            note,
            NOTE_INDENT_MM,
            y,
            note_width,
            BODY_MM,
            BODY_LINE_MM,
            &paint.ink,
            paint,
        );
    }
    for warning in &section.warnings {
        y += GAP_MM / 2.0;
        y = paragraph(
            &mut body,
            warning,
            0.0,
            y,
            width,
            SMALL_MM,
            SMALL_LINE_MM,
            &paint.secondary,
            paint,
        );
    }
    y += GAP_MM / 2.0;
    y = paragraph(
        &mut body,
        &section.source,
        0.0,
        y,
        width,
        SMALL_MM,
        SMALL_LINE_MM,
        &paint.secondary,
        paint,
    );

    Drawing {
        body,
        width,
        height: y,
    }
}

/// The section title and a hairline under it. Returns the y below.
fn heading(out: &mut String, title: &str, width: f64, paint: &Paint) -> f64 {
    text(
        out,
        0.0,
        TITLE_MM,
        TITLE_MM,
        600,
        &paint.ink,
        "start",
        &paint.label(title),
    );
    let rule_y = TITLE_MM + TITLE_RULE_GAP_MM;
    let _ = writeln!(
        out,
        r#"<line x1="0" y1="{rule_y:.2}" x2="{width:.2}" y2="{rule_y:.2}" stroke="{}" stroke-width="{HAIRLINE_MM}"/>"#,
        paint.faint
    );

    rule_y + GAP_MM
}
