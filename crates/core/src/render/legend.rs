//! The legend, built from what the diagram actually drew.
//!
//! A classified diagram lists the classes that occur, then "Unknown" when any
//! feature had no value. Other diagrams list their visible layers top to
//! bottom. Muted layers share one "Context" row and figure layers one row,
//! because they share one colour and listing them apart would suggest a
//! difference the drawing does not show.

use std::fmt::Write;

use super::drawing::{pattern_id, Drawn};
use super::geometry::{decimal, mm};
use super::layout::Rect;
use super::style::{ResolvedLayer, Role};
use super::typeset::{Kind, Typesetter};
use crate::diagram::Diagram;
use crate::scene::Layer;
use crate::theme::LayerStyle;

const DEFAULT_TITLE: &str = "Legend";
const CONTEXT_LABEL: &str = "Context";
pub const SITE_LABEL: &str = "Site boundary";
/// For a row shared by many layers, which in practice are the open surfaces.
const OPEN_GROUND_LABEL: &str = "Open ground";
/// Rows merged from more layers than this are labelled by what they share, not by name.
const MAX_JOINED_LABELS: usize = 3;

const SWATCH_WIDTH: f64 = 6.0;
const SWATCH_HEIGHT: f64 = 3.8;
const LABEL_GAP: f64 = 2.6;
const ROW_PITCH: f64 = 5.4;
const COLUMN_GAP: f64 = 5.0;
const TITLE_GAP: f64 = 4.2;
/// From the title's baseline to the note's, and the most lines a note may take.
const NOTE_GAP: f64 = 1.6;
const MAX_NOTE_LINES: usize = 2;
const SITE_STROKE_MM: f64 = 0.5;
const MIN_SWATCH_STROKE_MM: f64 = 0.1;
const SWATCH_EDGE_OPACITY: f64 = 0.3;

#[derive(Debug, Clone, PartialEq)]
pub enum Swatch {
    /// A filled rectangle: `fill` is a colour or a pattern url.
    Area {
        fill: String,
        stroke: Option<String>,
        stroke_mm: f64,
        opacity: f64,
    },
    Line {
        stroke: String,
        stroke_mm: f64,
        dash_mm: Vec<f64>,
        opacity: f64,
    },
    /// A point symbol; trees may be filled, outlined or both.
    Dot {
        fill: Option<String>,
        stroke: Option<String>,
        opacity: f64,
    },
}

#[derive(Debug, Clone)]
pub struct Row {
    pub label: String,
    pub swatch: Swatch,
}

#[derive(Debug, Clone)]
pub struct Legend {
    pub title: String,
    /// Unit, source and year under the title, for a classified diagram that has them.
    pub note: Option<String>,
    pub rows: Vec<Row>,
}

pub fn build(layers: &[ResolvedLayer], drawn: &[Drawn], diagram: &Diagram, accent: &str) -> Legend {
    let mut title = DEFAULT_TITLE.to_string();
    let mut note = None;
    let mut class_rows: Vec<Row> = Vec::new();
    let mut theme_rows: Vec<(Vec<Layer>, Swatch)> = Vec::new();
    let mut figure: Option<(Vec<Layer>, Swatch)> = None;
    let mut muted: Option<(Vec<Layer>, Swatch)> = None;

    // Top of the drawing first, the way a reader scans a legend.
    for (resolved, drawn) in layers.iter().zip(drawn).rev() {
        if !drawn.has_features {
            continue;
        }
        let swatch = swatch_for(resolved.layer, &resolved.style);
        match (resolved.role, &resolved.classes) {
            (Role::Classified, Some(classes)) => {
                title = classes.title.clone();
                note = classes.note.clone();
                for (class, &is_present) in classes.classes.iter().zip(&drawn.classes_present) {
                    if is_present {
                        class_rows.push(Row {
                            label: class.label.clone(),
                            swatch: with_fill(&swatch, &class.fill),
                        });
                    }
                }
                if drawn.has_unknown {
                    class_rows.push(Row {
                        label: classes.unknown_label.clone(),
                        swatch: with_fill(&swatch, &classes.unknown_fill),
                    });
                }
            }
            (Role::Muted, _) => muted
                .get_or_insert_with(|| (Vec::new(), swatch))
                .0
                .push(resolved.layer),
            (Role::Figure, _) => figure
                .get_or_insert_with(|| (Vec::new(), swatch))
                .0
                .push(resolved.layer),
            _ => match theme_rows
                .iter_mut()
                .find(|(_, existing)| *existing == swatch)
            {
                Some((same_style, _)) => same_style.push(resolved.layer),
                None => theme_rows.push((vec![resolved.layer], swatch)),
            },
        }
    }

    let mut rows = class_rows;
    if let Some((members, swatch)) = figure {
        let label = if members.len() == 1 {
            members[0].label().to_string()
        } else {
            diagram.name.clone()
        };
        rows.push(Row { label, swatch });
    }
    for (members, swatch) in theme_rows {
        rows.push(Row {
            label: joined_label(&members),
            swatch,
        });
    }
    if let Some((members, swatch)) = muted {
        let label = if members.len() == 1 {
            members[0].label().to_string()
        } else {
            CONTEXT_LABEL.to_string()
        };
        rows.push(Row { label, swatch });
    }
    rows.push(Row {
        label: SITE_LABEL.to_string(),
        swatch: Swatch::Line {
            stroke: accent.to_string(),
            stroke_mm: SITE_STROKE_MM,
            dash_mm: Vec::new(),
            opacity: 1.0,
        },
    });

    Legend { title, note, rows }
}

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

fn swatch_for(layer: Layer, style: &LayerStyle) -> Swatch {
    let fill = if style.pattern.is_some() {
        Some(format!("url(#{})", pattern_id("", layer)))
    } else {
        style.fill.clone()
    };
    let visible_stroke = style.stroke.clone().filter(|_| style.stroke_mm > 0.0);
    if layer == Layer::Tree {
        return Swatch::Dot {
            fill,
            stroke: visible_stroke,
            opacity: style.opacity,
        };
    }

    match fill {
        Some(fill) => Swatch::Area {
            fill,
            stroke: visible_stroke,
            stroke_mm: style.stroke_mm,
            opacity: style.opacity,
        },
        None => Swatch::Line {
            stroke: visible_stroke.unwrap_or_default(),
            stroke_mm: style.stroke_mm,
            dash_mm: style.dash_mm.clone(),
            opacity: style.opacity,
        },
    }
}

fn with_fill(swatch: &Swatch, fill: &str) -> Swatch {
    match swatch {
        Swatch::Area {
            stroke,
            stroke_mm,
            opacity,
            ..
        } => Swatch::Area {
            fill: fill.to_string(),
            stroke: stroke.clone(),
            stroke_mm: *stroke_mm,
            opacity: *opacity,
        },
        Swatch::Dot { opacity, .. } => Swatch::Dot {
            fill: Some(fill.to_string()),
            stroke: None,
            opacity: *opacity,
        },
        Swatch::Line { .. } => Swatch::Area {
            fill: fill.to_string(),
            stroke: None,
            stroke_mm: 0.0,
            opacity: 1.0,
        },
    }
}

fn joined_label(layers: &[Layer]) -> String {
    if layers.len() > MAX_JOINED_LABELS {
        return OPEN_GROUND_LABEL.to_string();
    }
    let labels: Vec<&str> = layers.iter().map(|layer| layer.label()).collect();

    labels.join(", ")
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
    }
}
