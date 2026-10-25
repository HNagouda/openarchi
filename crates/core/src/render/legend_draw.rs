//! The legend drawn into its share of the furniture: the title, the note, then
//! the rows in as few columns as fit, each a swatch and its label.

use std::fmt::Write;

use super::geometry::{decimal, mm};
use super::highlights;
use super::layout::Rect;
use super::legend::{Legend, Row, Swatch};
use super::symbols;
use super::typeset::{Kind, Typesetter};

const SWATCH_WIDTH: f64 = 6.0;
const SWATCH_HEIGHT: f64 = 3.8;
const LABEL_GAP: f64 = 2.6;
const ROW_PITCH: f64 = 5.4;
const COLUMN_GAP: f64 = 5.0;
const TITLE_GAP: f64 = 4.2;
/// From the title's baseline to the note's, and the most lines a note may take.
const NOTE_GAP: f64 = 1.6;
const MAX_NOTE_LINES: usize = 2;
const MIN_SWATCH_STROKE_MM: f64 = 0.1;
const SWATCH_EDGE_OPACITY: f64 = 0.3;
/// A symbol's radius as a share of the swatch height, leaving room for its corners.
const SYMBOL_SHARE: f64 = 0.32;

/// Draws the legend inside `area`, in as few columns as fit, at most
/// `max_columns`. Rows that still do not fit are summed up in a last row.
pub fn write(
    out: &mut String,
    legend: &Legend,
    typesetter: &Typesetter,
    area: &Rect,
    max_columns: usize,
) {
    let unit = typesetter.unit;
    let note_lines = match &legend.note {
        Some(note) => {
            let mut lines = typesetter.wrap(note, Kind::Credit, area.width);
            lines.truncate(MAX_NOTE_LINES);
            lines
        }
        None => Vec::new(),
    };
    let note_pitch = typesetter.line_pitch(Kind::Credit);
    let note_height = if note_lines.is_empty() {
        0.0
    } else {
        NOTE_GAP * unit
            + typesetter.cap_height(Kind::Credit)
            + note_pitch * (note_lines.len() - 1) as f64
    };
    let header = typesetter.cap_height(Kind::Heading) + note_height + TITLE_GAP * unit;
    let rows_that_fit = (((area.height - header) / (ROW_PITCH * unit))
        .floor()
        .max(1.0)) as usize;
    let columns = legend
        .rows
        .len()
        .div_ceil(rows_that_fit)
        .clamp(1, max_columns.max(1));
    let capacity = rows_that_fit * columns;

    let mut rows = legend.rows.clone();
    if rows.len() > capacity {
        let hidden = rows.len() - capacity + 1;
        rows.truncate(capacity - 1);
        rows.push(Row {
            label: format!("and {hidden} more"),
            swatch: Swatch::Dot {
                fill: None,
                stroke: None,
                opacity: 0.0,
            },
        });
    }

    typesetter.write(
        out,
        area.x,
        area.y + typesetter.cap_height(Kind::Heading),
        &legend.title,
        Kind::Heading,
        None,
    );
    let first_note_baseline = area.y
        + typesetter.cap_height(Kind::Heading)
        + NOTE_GAP * unit
        + typesetter.cap_height(Kind::Credit);
    for (index, line) in note_lines.iter().enumerate() {
        typesetter.write(
            out,
            area.x,
            first_note_baseline + note_pitch * index as f64,
            line,
            Kind::Credit,
            None,
        );
    }
    let rows_per_column = rows.len().div_ceil(columns);
    let column_width = (area.width - COLUMN_GAP * unit * (columns - 1) as f64) / columns as f64;
    let label_width = column_width - (SWATCH_WIDTH + LABEL_GAP) * unit;
    for (index, row) in rows.iter().enumerate() {
        let column = index / rows_per_column;
        let row_in_column = index % rows_per_column;
        let x = area.x + column as f64 * (column_width + COLUMN_GAP * unit);
        let top = area.y + header + row_in_column as f64 * ROW_PITCH * unit;
        write_swatch(out, &row.swatch, x, top, unit, &typesetter.ink);
        let label = typesetter.fit(&row.label, Kind::Label, label_width);
        let baseline = top + (SWATCH_HEIGHT * unit + typesetter.cap_height(Kind::Label)) / 2.0;
        typesetter.write(
            out,
            x + (SWATCH_WIDTH + LABEL_GAP) * unit,
            baseline,
            &label,
            Kind::Label,
            None,
        );
    }
}

fn write_swatch(out: &mut String, swatch: &Swatch, x: f64, top: f64, unit: f64, ink: &str) {
    let width = SWATCH_WIDTH * unit;
    let height = SWATCH_HEIGHT * unit;
    match swatch {
        Swatch::Area {
            fill,
            stroke,
            stroke_mm,
            opacity,
        } => {
            // A swatch without its own edge gets a faint ink one, or a pale
            // fill like a road would vanish into the paper.
            let stroke_attributes = match stroke {
                Some(stroke) if *stroke != *fill => format!(
                    r#" stroke="{stroke}" stroke-width="{}""#,
                    mm(stroke_mm.max(MIN_SWATCH_STROKE_MM))
                ),
                _ => format!(
                    r#" stroke="{ink}" stroke-opacity="{}" stroke-width="{}""#,
                    decimal(SWATCH_EDGE_OPACITY),
                    mm(MIN_SWATCH_STROKE_MM)
                ),
            };
            let _ = write!(
                out,
                r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{fill}"{stroke_attributes} opacity="{}"/>"#,
                mm(x),
                mm(top),
                mm(width),
                mm(height),
                decimal(*opacity)
            );
        }
        Swatch::Line {
            stroke,
            stroke_mm,
            dash_mm,
            opacity,
        } => {
            let dashes = if dash_mm.is_empty() {
                String::new()
            } else {
                let lengths: Vec<String> = dash_mm.iter().map(|&length| mm(length)).collect();
                format!(r#" stroke-dasharray="{}""#, lengths.join(" "))
            };
            let middle = top + height / 2.0;
            let _ = write!(
                out,
                r#"<path d="M{} {}H{}" stroke="{stroke}" stroke-width="{}"{dashes} opacity="{}"/>"#,
                mm(x),
                mm(middle),
                mm(x + width),
                mm(stroke_mm.max(0.15)),
                decimal(*opacity)
            );
        }
        Swatch::Highlight {
            fill,
            stroke,
            outline,
        } => {
            highlights::write_swatch(out, (x, top), (width, height), fill, stroke, *outline);
        }
        Swatch::Dot {
            fill,
            stroke,
            opacity,
        } => {
            let stroke_attributes = match stroke {
                Some(stroke) => format!(
                    r#" stroke="{stroke}" stroke-width="{}""#,
                    mm(MIN_SWATCH_STROKE_MM * 1.5)
                ),
                None => String::new(),
            };
            let _ = write!(
                out,
                r#"<circle cx="{}" cy="{}" r="{}" fill="{}"{stroke_attributes} opacity="{}"/>"#,
                mm(x + width / 2.0),
                mm(top + height / 2.0),
                mm(height * 0.4),
                fill.as_deref().unwrap_or("none"),
                decimal(*opacity)
            );
        }
        Swatch::Symbol {
            shape,
            fill,
            stroke,
        } => {
            let stroke_attributes = match stroke {
                Some(stroke) => format!(
                    r#" stroke="{stroke}" stroke-width="{}" stroke-linejoin="round""#,
                    mm(MIN_SWATCH_STROKE_MM * 1.5)
                ),
                None => String::new(),
            };
            out.push_str(&symbols::element(
                *shape,
                (x + width / 2.0, top + height / 2.0),
                height * SYMBOL_SHARE,
                &format!(r#" fill="{fill}"{stroke_attributes}"#),
            ));
        }
    }
}
