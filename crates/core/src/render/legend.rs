//! The legend, built from what the diagram actually drew.
//!
//! A classified diagram lists the classes that occur, then "Unknown" when any
//! feature had no value. Other diagrams list their visible layers top to
//! bottom. Muted layers share one "Context" row and figure layers one row,
//! because they share one colour and listing them apart would suggest a
//! difference the drawing does not show.

use super::drawing::{pattern_id, Drawn};
use super::style::{ResolvedLayer, Role};
use super::symbols::{self, Shape};
use crate::diagram::Diagram;
use crate::highlight::Outline;
use crate::scene::Layer;
use crate::theme::LayerStyle;

const DEFAULT_TITLE: &str = "Legend";
const CONTEXT_LABEL: &str = "Context";
pub const SITE_LABEL: &str = "Site boundary";
/// For a row shared by many layers, which in practice are the open surfaces.
const OPEN_GROUND_LABEL: &str = "Open ground";
/// Rows merged from more layers than this are labelled by what they share, not by name.
const MAX_JOINED_LABELS: usize = 3;
const SITE_STROKE_MM: f64 = 0.5;

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
    /// A highlight: its tint, edged in its outline, solid, dashed or dotted.
    Highlight {
        fill: String,
        stroke: String,
        outline: Outline,
    },
    /// A place or stop symbol in its class's shape.
    Symbol {
        shape: Shape,
        fill: String,
        stroke: Option<String>,
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
                let classes_present = classes.classes.iter().zip(&drawn.classes_present);
                for (index, (class, &is_present)) in classes_present.enumerate() {
                    if !is_present {
                        continue;
                    }
                    let swatch = if is_point_layer(resolved.layer) {
                        Swatch::Symbol {
                            shape: symbols::shape_for(index),
                            fill: class.fill.clone(),
                            stroke: resolved.style.stroke.clone(),
                        }
                    } else {
                        with_fill(&swatch, &class.fill)
                    };
                    class_rows.push(Row {
                        label: class.label.clone(),
                        swatch,
                    });
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

/// Layers drawn as class-shaped symbols by `points`.
pub fn is_point_layer(layer: Layer) -> bool {
    matches!(
        layer,
        Layer::Business | Layer::Facility | Layer::TransitStop
    )
}

fn swatch_for(layer: Layer, style: &LayerStyle) -> Swatch {
    let fill = if style.pattern.is_some() {
        Some(format!("url(#{})", pattern_id("", layer)))
    } else {
        style.fill.clone()
    };
    let visible_stroke = style.stroke.clone().filter(|_| style.stroke_mm > 0.0);
    if layer == Layer::Tree || is_point_layer(layer) {
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
        Swatch::Highlight { .. } => swatch.clone(),
        Swatch::Symbol { shape, stroke, .. } => Swatch::Symbol {
            shape: *shape,
            fill: fill.to_string(),
            stroke: stroke.clone(),
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
