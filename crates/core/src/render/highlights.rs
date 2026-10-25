//! The project's highlights, drawn on the ground of a map: a light tint in
//! the palette's accent, an accent outline, solid or dashed, and a label.
//!
//! They are drawn inside `map::draw`, right after the site outline, so in an
//! axonometric they lie on the ground under the buildings, and in an explode
//! view on the bottom plate, which holds the site's surroundings. Labels go
//! in a second pass after the buildings, so a tall block never hides one.
//! "Dim the rest" lays a paper-coloured veil over the frame with the
//! highlights cut out of it, so the eye goes to them. In an axonometric the
//! veil comes after the buildings too, so they fade with the ground, and
//! its hole is the highlight raised to its tallest building, so whatever
//! stands inside keeps its full colour.
//!
//! Each shape carries its id in `data-highlight`, so the app can tell which
//! highlight a click lands in on the very SVG it shows.

use std::fmt::Write;

use geo::{ConvexHull, Intersects, MultiPoint, Point, Polygon};

use super::color::mix;
use super::drawing::Canvas;
use super::geometry::{decimal, mm, push_paper_points};
use super::legend::{Row, Swatch};
use super::solid::building_height;
use super::svg::{escape, layer_attributes};
use super::typeset::{Kind, Typesetter};
use crate::highlight::{Highlight, Outline};
use crate::scene::{Layer, Scene};
use crate::theme::Palette;

/// The tint inside a highlight: enough to mark it, little enough that the
/// diagram's own colours still read through.
const TINT_OPACITY: f64 = 0.16;
/// The outline, a little heavier than the site outline so the two never blur.
const OUTLINE_MM: f64 = 0.6;
const DASH_MM: f64 = 1.8;
const GAP_MM: f64 = 1.1;
/// How strongly "Dim the rest" fades what lies outside: a little, not a blackout.
const DIM_OPACITY: f64 = 0.45;
const LABEL_HALO_MM: f64 = 0.6;
/// Room between a highlight's top edge and its label's baseline, at A3.
const LABEL_GAP_MM: f64 = 1.2;
/// Far enough out, in paper millimetres, that the veil covers any frame before its clip cuts it.
const VEIL_REACH_MM: f64 = 5000.0;

pub struct HighlightLayer<'a> {
    /// Only the highlights this map's scene reaches; see `shown`.
    pub highlights: Vec<&'a Highlight>,
    pub scene: &'a Scene,
    /// Buildings stand up on this map, so the veil goes over them; see `write_over`.
    pub has_volume: bool,
    pub canvas: &'a Canvas<'a>,
    pub palette: &'a Palette,
    pub typesetter: &'a Typesetter,
}

/// The highlights that touch the area the sheet draws. A sub-selection
/// sheet leaves out the ones that lie elsewhere, legend row and all.
pub fn shown<'a>(highlights: &'a [Highlight], scene: &Scene) -> Vec<&'a Highlight> {
    highlights
        .iter()
        .filter(|highlight| {
            highlight
                .shape
                .polygon_from(scene.origin)
                .intersects(&scene.boundary)
        })
        .collect()
}

/// The veil on a flat map, then each tint and outline. Called on the ground, under the buildings.
pub fn write_shapes(out: &mut String, layer: &HighlightLayer) {
    if layer.highlights.is_empty() {
        return;
    }
    let palette = layer.palette;
    let scope = layer.canvas.scope;
    let _ = write!(
        out,
        r#"<g {} clip-path="url(#{})">"#,
        layer_attributes(&format!("{scope}highlights"), "Highlights"),
        layer.canvas.clip_id
    );
    if !layer.has_volume {
        write_veil(out, layer);
    }
    for highlight in &layer.highlights {
        let dash = match highlight.outline {
            Outline::Solid => String::new(),
            Outline::Dashed => format!(r#" stroke-dasharray="{} {}""#, mm(DASH_MM), mm(GAP_MM)),
        };
        let _ = write!(
            out,
            r#"<path data-highlight="{}" d="{}" fill="{}" fill-opacity="{}" stroke="{}" stroke-width="{}" stroke-linejoin="round"{dash}/>"#,
            escape(&highlight.id),
            paper_path(highlight, layer),
            palette.accent,
            decimal(TINT_OPACITY),
            palette.accent,
            mm(OUTLINE_MM)
        );
    }
    out.push_str("</g>\n");
}

/// Paper over everything in the frame except the highlights that dim the
/// rest. On a flat map a huge rectangle, cut down by the frame's clip; with
/// volume the site swept up to its tallest roof, so the buildings standing
/// past the ground's edge fade too.
fn write_veil(out: &mut String, layer: &HighlightLayer) {
    let holes: Vec<String> = layer
        .highlights
        .iter()
        .filter(|highlight| highlight.dims_rest)
        .map(|highlight| veil_hole(highlight, layer))
        .collect();
    if holes.is_empty() {
        return;
    }
    let outer = if layer.has_volume {
        let site = layer
            .scene
            .boundary
            .exterior()
            .points()
            .map(|point| point.x_y());
        swept_outline(site, tallest(layer.scene, None), layer)
    } else {
        let reach = mm(VEIL_REACH_MM);
        format!("M-{reach} -{reach}H{reach}V{reach}H-{reach}Z")
    };
    let _ = write!(
        out,
        r#"<path d="{outer}{}" fill="{}" fill-opacity="{}" fill-rule="evenodd"/>"#,
        holes.join(""),
        layer.palette.paper,
        decimal(DIM_OPACITY)
    );
}

/// The veil of a map with volume, then the labels. Called after the buildings.
pub fn write_over(out: &mut String, layer: &HighlightLayer) {
    if layer.has_volume {
        write_veil(out, layer);
    }
    write_labels(out, layer);
}

/// Where the veil leaves a gap: the shape on a flat map; with volume, the
/// shape swept up to its tallest building, so what stands inside keeps its colour.
fn veil_hole(highlight: &Highlight, layer: &HighlightLayer) -> String {
    if !layer.has_volume {
        return paper_path(highlight, layer);
    }
    let (origin_x, origin_y) = layer.scene.origin;
    let ground = highlight
        .shape
        .ring()
        .into_iter()
        .map(|(x, y)| (x - origin_x, y - origin_y));
    let shape = highlight.shape.polygon_from(layer.scene.origin);
    swept_outline(ground, tallest(layer.scene, Some(&shape)), layer)
}

/// The paper outline of a ground ring, in the scene's local metres, raised
/// `rise_m` into a prism. The convex hull is a little generous for a concave
/// ring, which only leaves a sliver more or less faded.
fn swept_outline(
    ground: impl Iterator<Item = (f64, f64)>,
    rise_m: f64,
    layer: &HighlightLayer,
) -> String {
    let projection = layer.canvas.projection;
    let mut swept: Vec<Point<f64>> = Vec::new();
    for (x, y) in ground {
        swept.push(projection.to_paper_at(x, y, 0.0).into());
        swept.push(projection.to_paper_at(x, y, rise_m).into());
    }
    let hull = MultiPoint::from(swept).convex_hull();
    let corners: Vec<(f64, f64)> = hull.exterior().points().map(|point| point.x_y()).collect();
    let mut data = String::new();
    push_paper_points(&mut data, &corners, true);
    data
}

/// The height of the tallest building in the scene, or of those that reach
/// into `within`, in metres; 0 when there are none.
fn tallest(scene: &Scene, within: Option<&Polygon<f64>>) -> f64 {
    scene
        .features
        .iter()
        .filter(|feature| feature.layer == Layer::Building)
        .filter(|feature| within.is_none_or(|shape| feature.geometry.intersects(shape)))
        .map(|feature| building_height(feature).0)
        .fold(0.0, f64::max)
}

/// Each label in ink on a paper halo, centred just above its highlight, where
/// it names the shape without covering it, and where a house drawn inside a
/// plot keeps its label apart from the plot's.
fn write_labels(out: &mut String, layer: &HighlightLayer) {
    let typesetter = layer.typesetter;
    let captions: Vec<(String, (f64, f64))> = layer
        .highlights
        .iter()
        .map(|highlight| (highlight.caption(), label_anchor(highlight, layer)))
        .filter(|(caption, _)| !caption.is_empty())
        .collect();
    if captions.is_empty() {
        return;
    }

    let _ = write!(
        out,
        r#"<g {} font-family="{}" stroke="{}" stroke-width="{}" stroke-linejoin="round" paint-order="stroke">"#,
        layer_attributes(
            &format!("{}highlight-labels", layer.canvas.scope),
            "Highlight labels"
        ),
        escape(typesetter.face.css_stack()),
        layer.palette.paper,
        mm(LABEL_HALO_MM * typesetter.unit)
    );
    for (caption, (x, baseline)) in captions {
        typesetter.write(out, x, baseline, &caption, Kind::PinLabel, Some("middle"));
    }
    out.push_str("</g>\n");
}

/// One legend row per highlight, by its label; highlights sharing a label share a row.
pub fn legend_rows(layer: &HighlightLayer) -> Vec<Row> {
    let palette = layer.palette;
    let mut rows: Vec<Row> = Vec::new();
    for highlight in &layer.highlights {
        let label = highlight.legend_label();
        if rows.iter().any(|row| row.label == label) {
            continue;
        }
        rows.push(Row {
            label: label.to_string(),
            swatch: Swatch::Area {
                // The tint as it shows over paper, since a swatch has no diagram under it.
                fill: mix(&palette.paper, &palette.accent, TINT_OPACITY),
                stroke: Some(palette.accent.clone()),
                stroke_mm: OUTLINE_MM,
                opacity: 1.0,
            },
        });
    }

    rows
}

fn paper_points(highlight: &Highlight, layer: &HighlightLayer) -> Vec<(f64, f64)> {
    let (origin_x, origin_y) = layer.scene.origin;
    highlight
        .shape
        .ring()
        .into_iter()
        .map(|(x, y)| layer.canvas.projection.to_paper(x - origin_x, y - origin_y))
        .collect()
}

fn paper_path(highlight: &Highlight, layer: &HighlightLayer) -> String {
    let mut data = String::new();
    push_paper_points(&mut data, &paper_points(highlight, layer), true);
    data
}

/// Where a label's baseline centre goes: over the middle of the shape's
/// paper width, a little above its highest point.
fn label_anchor(highlight: &Highlight, layer: &HighlightLayer) -> (f64, f64) {
    let points = paper_points(highlight, layer);
    let left = points
        .iter()
        .map(|point| point.0)
        .fold(f64::INFINITY, f64::min);
    let right = points
        .iter()
        .map(|point| point.0)
        .fold(f64::NEG_INFINITY, f64::max);
    let top = points
        .iter()
        .map(|point| point.1)
        .fold(f64::INFINITY, f64::min);
    let gap = LABEL_GAP_MM * layer.typesetter.unit;
    ((left + right) / 2.0, top - gap)
}
