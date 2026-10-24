//! Businesses, facilities and transit stops as symbols that still read on a
//! dense city centre.
//!
//! Three things keep a few hundred places from turning into a cloud of dots.
//! Every symbol sits on a halo in the paper colour, so neighbours stay apart
//! and buildings do not swallow them. Places of one class that would touch on
//! paper merge into one larger symbol, its area growing with the count, so a
//! shopping street reads as a row of weights rather than a smear. And each
//! class has its own shape as well as its colour. Stops are also labelled,
//! most important first, leaving out any label that would overlap one placed
//! before it.

use std::fmt::Write;

use geo::Geometry;

use super::classify;
use super::drawing::{Canvas, Drawn};
use super::geometry::mm;
use super::style::ResolvedLayer;
use super::svg::{escape, layer_attributes};
use super::symbols::{self, Shape};
use super::typeset::{Kind, Typesetter};
use crate::diagram::Diagram;
use crate::scene::{Feature, Layer};

/// A symbol radius for themes that set none, in millimetres.
const DEFAULT_RADIUS_MM: f64 = 0.9;
/// The paper-coloured ring around every symbol.
const HALO_MM: f64 = 0.3;
/// Symbols of one class closer than this many radii merge into one.
const MERGE_RADII: f64 = 2.4;
/// A merged symbol stops growing at this many radii, so a busy street does
/// not grow one blot over its neighbours.
const MAX_GROWTH: f64 = 2.6;
/// The gap between a stop symbol and its label, in label heights.
const LABEL_GAP: f64 = 0.5;
/// The halo behind labels, in millimetres.
const LABEL_HALO_MM: f64 = 0.55;
/// Lines listed under a stop's name before the rest are cut.
const MAX_LABEL_LINES: usize = 8;
const LINE_SEPARATOR: &str = " · ";

pub struct PointLayer<'a> {
    pub resolved: &'a ResolvedLayer,
    pub features: &'a [&'a Feature],
    pub diagram: &'a Diagram,
    pub canvas: &'a Canvas<'a>,
    /// The paper colour, for halos.
    pub paper: &'a str,
    pub typesetter: &'a Typesetter,
    pub has_labels: bool,
}

/// What a point layer drew: the legend's `Drawn`, and every line that calls
/// at a drawn stop ("Tram 13"), for the legend to list.
pub struct DrawnPoints {
    pub drawn: Drawn,
    pub lines: Vec<String>,
    /// At least one symbol stands for several places.
    pub has_merged: bool,
}

/// One symbol on paper, standing for one or more places of one class.
struct Mark<'a> {
    class: usize,
    x: f64,
    y: f64,
    members: Vec<&'a Feature>,
}

pub fn write(out: &mut String, layer: &PointLayer) -> DrawnPoints {
    let resolved = layer.resolved;
    let style = &resolved.style;
    let classify = layer
        .diagram
        .classify
        .as_ref()
        .filter(|classify| classify.layer == resolved.layer);
    let classes = resolved.classes.as_ref().filter(|_| classify.is_some());
    let class_count = classes.map_or(1, |classes| classes.classes.len());
    let mut drawn = Drawn {
        has_features: !layer.features.is_empty(),
        classes_present: vec![false; classes.map_or(0, |classes| classes.classes.len())],
        has_unknown: false,
    };
    if layer.features.is_empty() {
        return DrawnPoints {
            drawn,
            lines: Vec::new(),
            has_merged: false,
        };
    }

    let radius = if style.radius_mm > 0.0 {
        style.radius_mm
    } else {
        DEFAULT_RADIUS_MM
    };
    let marks = merged_marks(layer, classify, class_count, radius);
    for mark in &marks {
        if let Some(present) = drawn.classes_present.get_mut(mark.class) {
            *present = true;
        }
    }

    let scope = layer.canvas.scope;
    let slug = super::drawing::layer_slug(resolved.layer);
    let stroke = style.stroke.as_deref().unwrap_or(layer.paper);
    let _ = write!(
        out,
        r#"<g {} clip-path="url(#{})" stroke-linejoin="round">"#,
        layer_attributes(&format!("{scope}layer-{slug}"), resolved.layer.label()),
        layer.canvas.clip_id
    );
    // Every halo first, so no halo cuts into a symbol drawn before it.
    let _ = write!(out, r#"<g fill="{}" stroke="none">"#, layer.paper);
    for mark in &marks {
        let size = mark_radius(mark, radius) + HALO_MM;
        out.push_str(&symbols::element(shape(mark), (mark.x, mark.y), size, ""));
    }
    out.push_str("</g>");
    for class in 0..class_count {
        let fill = match classes {
            Some(classes) => classes.classes[class].fill.clone(),
            None => style.fill.clone().unwrap_or_else(|| stroke.to_string()),
        };
        let label = classes.map_or(resolved.layer.label(), |classes| {
            classes.classes[class].label.as_str()
        });
        let _ = write!(
            out,
            r#"<g id="{scope}{slug}-class-{}" inkscape:label="{}" fill="{fill}" stroke="{stroke}" stroke-width="{}">"#,
            class + 1,
            escape(label),
            mm(style.stroke_mm)
        );
        for mark in marks.iter().filter(|mark| mark.class == class) {
            let size = mark_radius(mark, radius);
            out.push_str(&symbols::element(shape(mark), (mark.x, mark.y), size, ""));
        }
        out.push_str("</g>");
    }
    if layer.has_labels && resolved.layer == Layer::TransitStop {
        write_stop_labels(out, layer, &marks, radius);
    }
    out.push_str("</g>\n");

    let mut lines: Vec<String> = Vec::new();
    for mark in &marks {
        for member in &mark.members {
            for line in &member.attributes.lines {
                if !lines.contains(line) {
                    lines.push(line.clone());
                }
            }
        }
    }

    DrawnPoints {
        drawn,
        lines,
        has_merged: marks.iter().any(|mark| mark.members.len() > 1),
    }
}

/// One mark per place, then places of one class that would touch on paper
/// merged into one mark at their middle. Stops are never merged: each is a
/// named place that gets its own label.
fn merged_marks<'a>(
    layer: &PointLayer<'a>,
    classify: Option<&crate::diagram::Classify>,
    class_count: usize,
    radius: f64,
) -> Vec<Mark<'a>> {
    let merge_mm = if layer.resolved.layer == Layer::TransitStop {
        0.0
    } else {
        MERGE_RADII * radius
    };
    let mut marks: Vec<Mark> = Vec::new();
    for &feature in layer.features {
        let Geometry::Point(point) = feature.geometry else {
            continue;
        };
        let (x, y) = layer.canvas.projection.to_paper(point.x(), point.y());
        let class = match classify {
            Some(classify) => classify::class_of(feature, classify)
                .unwrap_or(class_count - 1)
                .min(class_count - 1),
            None => 0,
        };
        let near = marks.iter_mut().find(|mark| {
            mark.class == class && ((mark.x - x).powi(2) + (mark.y - y).powi(2)).sqrt() < merge_mm
        });
        match near {
            Some(mark) => {
                // The mark moves to the middle of its members, weighted by count.
                let count = mark.members.len() as f64;
                mark.x = (mark.x * count + x) / (count + 1.0);
                mark.y = (mark.y * count + y) / (count + 1.0);
                mark.members.push(feature);
            }
            None => marks.push(Mark {
                class,
                x,
                y,
                members: vec![feature],
            }),
        }
    }

    // Big marks first, so small ones are drawn on top and stay visible.
    marks.sort_by_key(|mark| std::cmp::Reverse(mark.members.len()));
    marks
}

/// The radius of a mark: its area grows with the places it stands for.
fn mark_radius(mark: &Mark, radius: f64) -> f64 {
    radius * (mark.members.len() as f64).sqrt().min(MAX_GROWTH)
}

fn shape(mark: &Mark) -> Shape {
    symbols::shape_for(mark.class)
}

/// Each stop's name, and under it the line numbers that call there. Stops
/// higher in the transit order and with more lines are placed first; a label
/// that would overlap a placed label or another stop is tried on the other
/// side, then left out.
fn write_stop_labels(out: &mut String, layer: &PointLayer, marks: &[Mark], radius: f64) {
    let typesetter = layer.typesetter;
    let kind = Kind::Small;
    let height = typesetter.cap_height(kind);
    let pitch = typesetter.line_pitch(kind);
    let mut order: Vec<&Mark> = marks.iter().collect();
    order.sort_by_key(|mark| (mark.class, std::cmp::Reverse(stop_lines(mark).len())));

    let symbol_boxes: Vec<[f64; 4]> = marks
        .iter()
        .map(|mark| {
            let reach = mark_radius(mark, radius) + HALO_MM;
            [
                mark.x - reach,
                mark.y - reach,
                mark.x + reach,
                mark.y + reach,
            ]
        })
        .collect();
    let mut placed: Vec<[f64; 4]> = Vec::new();
    let _ = write!(
        out,
        r#"<g id="{}stop-labels" stroke="{}" stroke-width="{}" stroke-linejoin="round" paint-order="stroke">"#,
        layer.canvas.scope,
        layer.paper,
        mm(LABEL_HALO_MM)
    );
    for mark in order {
        let Some(name) = mark
            .members
            .iter()
            .find_map(|member| member.attributes.name.clone())
        else {
            continue;
        };
        let numbers = line_numbers(&stop_lines(mark));
        let width = typesetter
            .width(&name, kind)
            .max(typesetter.width(&numbers, kind));
        let rows = if numbers.is_empty() { 1.0 } else { 2.0 };
        let block_height = height + (rows - 1.0) * pitch;
        let gap = mark_radius(mark, radius) + HALO_MM + LABEL_GAP * height;

        // Right of the symbol, then left of it.
        let sides = [(mark.x + gap, None), (mark.x - gap, Some("end"))];
        let top = mark.y - block_height / 2.0;
        let fits = sides.iter().find(|(x, anchor)| {
            let left = if anchor.is_some() { x - width } else { *x };
            let candidate = [left, top, left + width, top + block_height];
            !placed
                .iter()
                .chain(&symbol_boxes)
                .any(|other| overlaps(&candidate, other))
        });
        let Some(&(x, anchor)) = fits else {
            continue;
        };
        let left = if anchor.is_some() { x - width } else { x };
        placed.push([left, top, left + width, top + block_height]);
        typesetter.write(out, x, top + height, &name, kind, anchor);
        typesetter.write(out, x, top + height + pitch, &numbers, kind, anchor);
    }
    out.push_str("</g>");
}

fn stop_lines(mark: &Mark) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for member in &mark.members {
        for line in &member.attributes.lines {
            if !lines.contains(line) {
                lines.push(line.clone());
            }
        }
    }

    lines
}

/// "Tram 13", "Tram 17", "Bus 21" -> "13 · 17 · 21". The legend says which mode each is.
pub fn line_numbers(lines: &[String]) -> String {
    let mut numbers: Vec<&str> = lines
        .iter()
        .map(|line| {
            line.split_once(' ')
                .map_or(line.as_str(), |(_, number)| number)
        })
        .collect();
    if numbers.len() > MAX_LABEL_LINES {
        numbers.truncate(MAX_LABEL_LINES);
        numbers.push("\u{2026}");
    }

    numbers.join(LINE_SEPARATOR)
}

fn overlaps(a: &[f64; 4], b: &[f64; 4]) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_numbers_drop_the_mode_and_stop_after_a_few() {
        let lines: Vec<String> = ["Tram 13", "Tram 17", "Bus 21"]
            .iter()
            .map(|line| line.to_string())
            .collect();
        assert_eq!(line_numbers(&lines), "13 · 17 · 21");

        let many: Vec<String> = (1..=12).map(|number| format!("Bus {number}")).collect();
        assert!(line_numbers(&many).ends_with("8 · \u{2026}"));
    }

    #[test]
    fn boxes_that_only_touch_do_not_overlap() {
        assert!(!overlaps(&[0.0, 0.0, 1.0, 1.0], &[1.0, 0.0, 2.0, 1.0]));
        assert!(overlaps(&[0.0, 0.0, 1.0, 1.0], &[0.5, 0.5, 2.0, 2.0]));
    }
}
