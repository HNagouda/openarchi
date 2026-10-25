//! The parts every drawing of an analysis section is made of: its heading,
//! its facts in a grid, its design notes and its smaller lines of text.
//!
//! The export page, the A4 pages and the report blocks lay a section out at
//! their own widths, but they draw these parts the same way, from the sizes
//! here, so the three cannot drift apart.

use std::fmt::Write;

use super::figure::{text, Paint};
use super::Fact;
use crate::render::{text_width, wrap_text};

const TITLE_MM: f64 = 4.2;
const TITLE_RULE_GAP_MM: f64 = 2.2;
const HAIRLINE_MM: f64 = 0.2;
/// The room between the parts of a section; half of it between lines of one part.
pub(super) const GAP_MM: f64 = 4.0;

const FACT_LABEL_MM: f64 = 2.5;
const FACT_VALUE_MM: f64 = 3.6;
/// From the label's top to the value's baseline, past the value's ascent.
const FACT_VALUE_DROP_MM: f64 = 0.9;
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
/// How far a paragraph's last line reaches below its baseline, as a share of its size.
const DESCENT: f64 = 0.4;

/// The section title from `y` with a hairline under it. Returns the y below.
pub(crate) fn heading(
    out: &mut String,
    title: &str,
    x: f64,
    y: f64,
    width: f64,
    paint: &Paint,
) -> f64 {
    text(
        out,
        x,
        y + TITLE_MM,
        TITLE_MM,
        600,
        &paint.ink,
        "start",
        &paint.label(title),
    );
    let rule_y = y + TITLE_MM + TITLE_RULE_GAP_MM;
    let _ = writeln!(
        out,
        r#"<line x1="{x:.2}" y1="{rule_y:.2}" x2="{:.2}" y2="{rule_y:.2}" stroke="{}" stroke-width="{HAIRLINE_MM}"/>"#,
        x + width,
        paint.faint
    );

    rule_y + GAP_MM
}

/// The facts from `y` in as many columns as `width` holds, up to four; a long
/// value takes as many columns as it needs and moves to the next row when
/// this one is full. Returns the y below the last row.
pub(crate) fn fact_grid(
    out: &mut String,
    facts: &[Fact],
    x: f64,
    y: f64,
    width: f64,
    paint: &Paint,
) -> f64 {
    let columns = ((width / FACT_COLUMN_MIN_MM).floor() as usize).clamp(1, FACT_COLUMNS_MAX);
    let column_width = width / columns as f64;

    let mut column = 0;
    let mut row = 0;
    for fact in facts {
        let needed = text_width(&fact.value, &paint.face, 600, false, FACT_VALUE_MM, 0.0).max(
            text_width(&fact.label, &paint.face, 400, false, FACT_LABEL_MM, 0.0),
        ) + FACT_GAP_MM;
        let span = ((needed / column_width).ceil() as usize).clamp(1, columns);
        if column + span > columns {
            column = 0;
            row += 1;
        }
        let left = x + column as f64 * column_width;
        let top = y + row as f64 * FACT_ROW_MM;
        text(
            out,
            left,
            top + FACT_LABEL_MM,
            FACT_LABEL_MM,
            400,
            &paint.secondary,
            "start",
            &fact.label,
        );
        text(
            out,
            left,
            top + FACT_LABEL_MM + FACT_VALUE_MM + FACT_VALUE_DROP_MM,
            FACT_VALUE_MM,
            600,
            &paint.ink,
            "start",
            &fact.value,
        );
        column += span;
    }
    let rows = if facts.is_empty() { 0 } else { row + 1 };

    y + rows as f64 * FACT_ROW_MM
}

/// The design notes from `y`, each after a dash in the accent colour, with a
/// little room between them. Returns the y below the last.
pub(crate) fn note_list(
    out: &mut String,
    notes: &[String],
    x: f64,
    y: f64,
    width: f64,
    paint: &Paint,
) -> f64 {
    let mut y = y;
    for (index, note) in notes.iter().enumerate() {
        if index > 0 {
            y += GAP_MM / 2.0;
        }
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
        let note_width = (width - NOTE_INDENT_MM).max(BODY_MM);
        y = paragraph(
            out,
            note,
            x + NOTE_INDENT_MM,
            y,
            note_width,
            BODY_MM,
            BODY_LINE_MM,
            &paint.ink,
            paint,
        );
    }

    y
}

/// Body text in ink from `y`, wrapped to `width`. Returns the y below it.
pub(crate) fn body(
    out: &mut String,
    content: &str,
    x: f64,
    y: f64,
    width: f64,
    paint: &Paint,
) -> f64 {
    paragraph(
        out,
        content,
        x,
        y,
        width,
        BODY_MM,
        BODY_LINE_MM,
        &paint.ink,
        paint,
    )
}

/// A source line, caption or warning: small and secondary. Returns the y below it.
pub(crate) fn small(
    out: &mut String,
    content: &str,
    x: f64,
    y: f64,
    width: f64,
    paint: &Paint,
) -> f64 {
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
}

/// A name in bold body type, such as a data source's. Returns the y below it.
pub(crate) fn bold_line(out: &mut String, content: &str, x: f64, y: f64, paint: &Paint) -> f64 {
    text(
        out,
        x,
        y + BODY_MM,
        BODY_MM,
        600,
        &paint.ink,
        "start",
        content,
    );
    y + BODY_LINE_MM
}

/// Wrapped text from `y` (the top of the first line). Returns the y below it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paragraph(
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

    baseline - pitch + size * DESCENT
}
