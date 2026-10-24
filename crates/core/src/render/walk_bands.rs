//! Walk bands: the areas within 5, 10 and 15 minutes on foot, as layers of one
//! translucent colour.
//!
//! Each band is drawn at the same light opacity, largest first. Because the
//! bands nest, the colour builds up towards the centre on its own, so the
//! nearest band is the strongest without a ramp that might clash with the
//! palette. Each band's edge is labelled with its minutes at its northern tip.

use std::fmt::Write;

use geo::{BoundingRect, Geometry};

use super::drawing::Canvas;
use super::geometry::{decimal, mm, paper_shapes, PaperShapes};
use super::legend::{Row, Swatch};
use super::style::ResolvedLayer;
use super::svg::layer_attributes;
use super::typeset::{Kind, Typesetter};
use crate::scene::Feature;

/// Each band's own opacity when the theme gives none below 1.
const DEFAULT_BAND_OPACITY: f64 = 0.14;
const EDGE_OPACITY: f64 = 0.75;
/// The halo behind the minute labels, in millimetres.
const LABEL_HALO_MM: f64 = 0.55;
/// How far below a band's northern tip its label sits, in label heights.
const LABEL_DROP: f64 = 1.6;

pub struct Bands<'a> {
    pub resolved: &'a ResolvedLayer,
    /// The bands to draw, each with `walk_min` set.
    pub features: &'a [&'a Feature],
    pub canvas: &'a Canvas<'a>,
    pub paper: &'a str,
    pub typesetter: &'a Typesetter,
    pub has_labels: bool,
}

/// Draws the bands and returns their legend rows, nearest band first.
pub fn write(out: &mut String, bands: &Bands) -> Vec<Row> {
    let style = &bands.resolved.style;
    let Some(fill) = style.fill.clone().or_else(|| style.stroke.clone()) else {
        return Vec::new();
    };
    let opacity = if style.opacity < 1.0 {
        style.opacity
    } else {
        DEFAULT_BAND_OPACITY
    };
    let mut ordered: Vec<&Feature> = bands.features.to_vec();
    ordered.sort_by(|a, b| minutes(b).total_cmp(&minutes(a)));
    if ordered.is_empty() {
        return Vec::new();
    }

    let canvas = bands.canvas;
    let stroke = style.stroke.clone().unwrap_or_else(|| fill.clone());
    let _ = write!(
        out,
        r#"<g {} clip-path="url(#{})" fill="{fill}" fill-rule="evenodd" stroke="{stroke}" stroke-width="{}" stroke-opacity="{}" stroke-linejoin="round">"#,
        layer_attributes(
            &format!("{}layer-walk-band", canvas.scope),
            bands.resolved.layer.label()
        ),
        canvas.clip_id,
        mm(style.stroke_mm),
        decimal(EDGE_OPACITY)
    );
    for band in &ordered {
        let mut shapes = PaperShapes::default();
        paper_shapes(&band.geometry, &canvas.projection, &mut shapes);
        let _ = write!(
            out,
            r#"<path d="{}" fill-opacity="{}"/>"#,
            shapes.areas.concat(),
            decimal(opacity)
        );
    }
    if bands.has_labels {
        write_labels(out, bands, &ordered);
    }
    out.push_str("</g>\n");

    // Nearest first: the innermost band has every band's colour on it.
    let count = ordered.len();
    let mut rows = Vec::with_capacity(count);
    let mut previous: Option<f64> = None;
    for (index, band) in ordered.iter().rev().enumerate() {
        let minutes = minutes(band);
        let layers = (count - index) as i32;
        let label = match previous {
            None => format!("Within {minutes:.0} minutes on foot"),
            Some(previous) => format!("{previous:.0} to {minutes:.0} minutes"),
        };
        rows.push(Row {
            label,
            swatch: Swatch::Area {
                fill: fill.clone(),
                stroke: Some(stroke.clone()),
                stroke_mm: style.stroke_mm,
                opacity: 1.0 - (1.0 - opacity).powi(layers),
            },
        });
        previous = Some(minutes);
    }

    rows
}

/// "5 min" just inside each band's northern tip, in ink over a paper halo.
fn write_labels(out: &mut String, bands: &Bands, ordered: &[&Feature]) {
    let typesetter = bands.typesetter;
    let kind = Kind::Small;
    let _ = write!(
        out,
        r#"<g stroke="{}" stroke-width="{}" stroke-opacity="1" paint-order="stroke" fill-opacity="1">"#,
        bands.paper,
        mm(LABEL_HALO_MM)
    );
    for band in ordered {
        let Some(tip) = northern_tip(&band.geometry) else {
            continue;
        };
        let (x, y) = bands.canvas.projection.to_paper(tip.0, tip.1);
        let baseline = y + LABEL_DROP * typesetter.cap_height(kind);
        let text = format!("{:.0} min", minutes(band));
        typesetter.write(out, x, baseline, &text, kind, Some("middle"));
    }
    out.push_str("</g>");
}

/// The most northern point of a band, in ground metres.
fn northern_tip(geometry: &Geometry<f64>) -> Option<(f64, f64)> {
    let Geometry::MultiPolygon(polygons) = geometry else {
        return None;
    };
    let top = geometry.bounding_rect()?.max().y;
    polygons
        .0
        .iter()
        .flat_map(|polygon| polygon.exterior().0.iter())
        .find(|coord| coord.y >= top)
        .map(|coord| (coord.x, coord.y))
}

fn minutes(band: &Feature) -> f64 {
    band.attributes.walk_min.unwrap_or_default()
}
