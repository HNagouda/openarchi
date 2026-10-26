//! The imported model on a sheet: its footprint in plan, its faces standing
//! up in a tilted view, and its row in the legend. It draws in the palette's
//! accent, so a proposal reads as the subject of every sheet in the series.
//! Its shadow is cast with the buildings' in `shadow.rs`.
//
// #! NOTE[3d, owner] : The model draws in the accent; a palette role of its own would need every built-in and saved palette to gain one

use std::fmt::Write;

use geo::{ConvexHull, MultiPoint, Point};

use super::drawing::Canvas;
use super::geometry::{mm, push_paper_points, Projection};
use super::legend::{Row, Swatch};
use super::lighting::{face_fill, Light};
use super::order::Footprint;
use super::style::ResolvedLayer;
use super::svg::layer_attributes;
use super::RenderInput;
use crate::model::{self, PlacedModel};
use crate::scene::Layer;
use crate::theme::Theme;

pub const LEGEND_LABEL: &str = "Proposal";
/// The footprint's outline in plan, a little heavier than a building's edge.
const OUTLINE_MM: f64 = 0.18;
/// A face seen this close to edge-on is skipped: it would draw as a sliver.
const EDGE_ON: f64 = 1e-6;

/// The model on the scene of `input`, when the map draws buildings: only a
/// diagram with buildings draws the model among them.
pub fn placed(input: &RenderInput, layers: &[ResolvedLayer]) -> Option<PlacedModel> {
    let draws_buildings = layers
        .iter()
        .any(|resolved| resolved.layer == Layer::Building);
    input
        .model
        .filter(|_| draws_buildings)
        .map(|model| model::placed(model, input.scene))
}

/// The footprint in plan, over the buildings around it.
pub fn write_plan(out: &mut String, model: &PlacedModel, canvas: &Canvas, theme: &Theme) {
    let mut data = String::new();
    for polygon in &model.footprint {
        for ring in std::iter::once(polygon.exterior()).chain(polygon.interiors()) {
            let points: Vec<(f64, f64)> = ring
                .0
                .iter()
                .map(|c| canvas.projection.to_paper(c.x, c.y))
                .collect();
            push_paper_points(&mut data, &points, true);
        }
    }
    if data.is_empty() {
        return;
    }

    let _ = writeln!(
        out,
        r#"<g {} clip-path="url(#{})"><path d="{data}" fill="{}" fill-rule="evenodd" stroke="{}" stroke-width="{}" stroke-linejoin="round"/></g>"#,
        layer_attributes(&format!("{}proposal", canvas.scope), LEGEND_LABEL),
        canvas.clip_id,
        theme.palette.accent,
        theme.palette.ink,
        mm(OUTLINE_MM)
    );
}

/// The faces that look toward the viewer, furthest first, each lit by the
/// sun. Written into the solids group, which gives every face its edge.
pub fn write_solid(
    out: &mut String,
    model: &PlacedModel,
    accent: &str,
    light: &Light,
    projection: &Projection,
) {
    let mut seen: Vec<(f64, usize)> = Vec::new();
    for (index, face) in model.faces.iter().enumerate() {
        let [east, north, up] = face.normal;
        let (_, toward) = projection.turn(east, north);
        // The viewer looks along +v and down, so a face shows when its normal
        // points back along -v or up more than it points away.
        let facing = -toward * projection.rise + up * projection.squash;
        if facing <= EDGE_ON {
            continue;
        }
        seen.push((depth(&face.rings[0], projection), index));
    }
    seen.sort_by(|a, b| b.0.total_cmp(&a.0));

    for (_, index) in seen {
        let face = &model.faces[index];
        let mut data = String::new();
        for ring in &face.rings {
            let points: Vec<(f64, f64)> = ring
                .iter()
                .map(|&[x, y, z]| projection.to_paper_at(x, y, z))
                .collect();
            push_paper_points(&mut data, &points, true);
        }
        if !data.is_empty() {
            let fill = face_fill(accent, face.normal, light);
            let _ = write!(
                out,
                r#"<path d="{data}" fill="{fill}" stroke="{}"/>"#,
                light.edge(&fill)
            );
        }
    }
}

/// How far into the picture a ring's middle lies, larger further away.
fn depth(ring: &[[f64; 3]], projection: &Projection) -> f64 {
    let count = ring.len().max(1) as f64;
    let (mut v, mut z) = (0.0, 0.0);
    for &[x, y, height] in ring {
        v += projection.to_view(x, y).1;
        z += height;
    }
    (v * projection.rise - z * projection.squash) / count
}

/// The model as the back-to-front ordering sees it: the hull of its footprint.
pub fn footprint(model: &PlacedModel, projection: &Projection) -> Footprint {
    let points: MultiPoint<f64> = model
        .footprint
        .iter()
        .flat_map(|polygon| polygon.exterior().0.iter())
        .map(|c| Point::from(projection.to_view(c.x, c.y)))
        .collect();
    let ring = points
        .convex_hull()
        .exterior()
        .0
        .iter()
        .map(|c| (c.x, c.y))
        .collect();
    Footprint::new(ring, model.height)
}

pub fn legend_row(theme: &Theme) -> Row {
    Row {
        label: LEGEND_LABEL.to_string(),
        swatch: Swatch::Area {
            fill: theme.palette.accent.clone(),
            stroke: Some(theme.palette.ink.clone()),
            stroke_mm: OUTLINE_MM,
            opacity: 1.0,
        },
    }
}
