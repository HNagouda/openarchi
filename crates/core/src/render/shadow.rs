//! Shadows the buildings and trees cast on the ground for one sun position.
//!
//! A building is a prism, so its shadow is its footprint swept along the sun
//! direction: the footprint, the footprint moved by the shadow vector, and one
//! quadrilateral per wall joining the two. Every piece is written with the same
//! winding and filled nonzero, so the overlaps merge into one flat shadow
//! instead of darkening where pieces cross.
//!
//! The pieces go into opaque paths of a few hundred each, inside one
//! see-through group, which keeps the overlaps flat across paths too. One path
//! of 50 000 pieces, as an imported model casts, took Preview 24 s to draw and
//! left PDF pages blank; in paths of 200 it draws in a fraction of a second.

use std::f64::consts::TAU;
use std::fmt::Write;

use geo::{Geometry, LineString, Polygon};

use super::geometry::{decimal, push_paper_points, Projection};
use super::solid::{building_height, tree_size};
use crate::model::PlacedModel;
use crate::scene::{polygons, Feature, Layer};
use crate::sun::SunPosition;

/// Below this the sun grazes the ground and shadows run off the sheet.
pub const MIN_SUN_ALTITUDE_DEG: f64 = 2.0;
/// Corners on a tree's shadow ellipse.
const TREE_SHADOW_POINTS: usize = 12;
/// Buildings, trees or model faces per path element; see the module comment.
const PIECES_PER_PATH: usize = 200;

/// Shadow pieces gathered into path elements of `PIECES_PER_PATH` each. A
/// piece is everything one caster pushes, so a courtyard's hole stays in the
/// same path as its outline.
struct ShadowPaths {
    written: String,
    data: String,
    pieces: usize,
}

impl ShadowPaths {
    fn end_piece(&mut self) {
        self.pieces += 1;
        if self.pieces == PIECES_PER_PATH {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if !self.data.is_empty() {
            let _ = write!(self.written, r#"<path d="{}"/>"#, self.data);
            self.data.clear();
        }
        self.pieces = 0;
    }
}

/// The shadow of every building and tree as one see-through group of path
/// elements, or nothing when the sun is down.
pub fn write(
    out: &mut String,
    features: &[&Feature],
    model: Option<&PlacedModel>,
    sun: &SunPosition,
    projection: &Projection,
    opacity: f64,
) {
    let Some(per_metre) = shadow_per_metre(sun) else {
        return;
    };
    let mut paths = ShadowPaths {
        written: String::new(),
        data: String::new(),
        pieces: 0,
    };
    for feature in features {
        match (feature.layer, &feature.geometry) {
            (Layer::Building, geometry) => {
                let (height, _) = building_height(feature);
                let offset = (per_metre.0 * height, per_metre.1 * height);
                for polygon in polygons(geometry) {
                    push_prism_shadow(&mut paths.data, polygon, offset, projection);
                }
                paths.end_piece();
            }
            (Layer::Tree, Geometry::Point(point)) => {
                push_tree_shadow(
                    &mut paths.data,
                    (point.x(), point.y()),
                    per_metre,
                    sun,
                    projection,
                );
                paths.end_piece();
            }
            _ => {}
        }
    }
    if let Some(model) = model {
        push_model_shadow(&mut paths, model, per_metre, projection);
    }
    paths.flush();
    if paths.written.is_empty() {
        return;
    }

    let _ = write!(
        out,
        r#"<g fill-rule="nonzero" opacity="{}">{}</g>"#,
        decimal(opacity),
        paths.written
    );
}

/// Ground metres a shadow runs per metre of height, east and north, or `None` with the sun down.
pub fn shadow_per_metre(sun: &SunPosition) -> Option<(f64, f64)> {
    if sun.altitude_deg < MIN_SUN_ALTITUDE_DEG {
        return None;
    }
    let length = 1.0 / sun.altitude_deg.to_radians().tan();
    let azimuth = sun.azimuth_deg.to_radians();
    // Shadows fall away from the sun.
    Some((-azimuth.sin() * length, -azimuth.cos() * length))
}

fn push_prism_shadow(
    data: &mut String,
    polygon: &Polygon<f64>,
    offset: (f64, f64),
    projection: &Projection,
) {
    let rings: Vec<&LineString<f64>> = std::iter::once(polygon.exterior())
        .chain(polygon.interiors())
        .collect();
    for (index, ring) in rings.iter().enumerate() {
        let is_hole = index > 0;
        let points: Vec<(f64, f64)> = ring.0.iter().map(|c| (c.x, c.y)).collect();
        let moved: Vec<(f64, f64)> = points
            .iter()
            .map(|&(x, y)| (x + offset.0, y + offset.1))
            .collect();
        // Outer rings count up and holes count down, so a courtyard stays lit
        // where no wall shades it.
        push_ground_ring(data, &points, is_hole, projection);
        push_ground_ring(data, &moved, is_hole, projection);
        for pair in points.windows(2) {
            let quad = [
                pair[0],
                pair[1],
                (pair[1].0 + offset.0, pair[1].1 + offset.1),
                (pair[0].0 + offset.0, pair[0].1 + offset.1),
            ];
            push_ground_ring(data, &quad, false, projection);
        }
    }
}

/// Every face of the model moved along the sun onto the ground. Together they
/// cover the model's shadow, and they merge with the buildings' in the same
/// nonzero fill. A face's holes count down only inside its own outline.
fn push_model_shadow(
    paths: &mut ShadowPaths,
    model: &PlacedModel,
    per_metre: (f64, f64),
    projection: &Projection,
) {
    for face in &model.faces {
        for (index, ring) in face.rings.iter().enumerate() {
            let on_ground: Vec<(f64, f64)> = ring
                .iter()
                .map(|&[x, y, z]| (x + per_metre.0 * z, y + per_metre.1 * z))
                .collect();
            push_ground_ring(&mut paths.data, &on_ground, index > 0, projection);
        }
        paths.end_piece();
    }
}

fn push_tree_shadow(
    data: &mut String,
    base: (f64, f64),
    per_metre: (f64, f64),
    sun: &SunPosition,
    projection: &Projection,
) {
    let points = tree_shadow_ring(base, per_metre, sun);
    push_ground_ring(data, &points, false, projection);
}

/// An ellipse under the crown, stretched away from the sun, as ground points.
pub fn tree_shadow_ring(
    base: (f64, f64),
    per_metre: (f64, f64),
    sun: &SunPosition,
) -> Vec<(f64, f64)> {
    let (height, crown_radius) = tree_size();
    let crown_middle = height - crown_radius;
    let centre = (
        base.0 + per_metre.0 * crown_middle,
        base.1 + per_metre.1 * crown_middle,
    );
    let length = (per_metre.0.powi(2) + per_metre.1.powi(2)).sqrt();
    let along = if length > 0.0 {
        (per_metre.0 / length, per_metre.1 / length)
    } else {
        (0.0, 1.0)
    };
    let across = (-along.1, along.0);
    let stretch = 1.0 / sun.altitude_deg.to_radians().sin();
    (0..TREE_SHADOW_POINTS)
        .map(|index| {
            let angle = index as f64 / TREE_SHADOW_POINTS as f64 * TAU;
            let (a, b) = (
                angle.cos() * crown_radius * stretch,
                angle.sin() * crown_radius,
            );
            (
                centre.0 + along.0 * a + across.0 * b,
                centre.1 + along.1 * a + across.1 * b,
            )
        })
        .collect()
}

/// Projects a ground ring and writes it counterclockwise on the ground, or clockwise when `is_hole`.
fn push_ground_ring(
    data: &mut String,
    ring: &[(f64, f64)],
    is_hole: bool,
    projection: &Projection,
) {
    let is_counterclockwise = signed_area(ring) > 0.0;
    let mut paper: Vec<(f64, f64)> = ring
        .iter()
        .map(|&(x, y)| projection.to_paper(x, y))
        .collect();
    if is_counterclockwise == is_hole {
        paper.reverse();
    }
    push_paper_points(data, &paper, true);
}

/// Shoelace area: positive for counterclockwise rings on the ground (y north).
pub fn signed_area(ring: &[(f64, f64)]) -> f64 {
    let mut sum = 0.0;
    for index in 0..ring.len() {
        let (x1, y1) = ring[index];
        let (x2, y2) = ring[(index + 1) % ring.len()];
        sum += x1 * y2 - x2 * y1;
    }

    sum / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_noon_sun_in_the_south_casts_shadows_north() {
        let sun = SunPosition {
            azimuth_deg: 180.0,
            altitude_deg: 45.0,
        };
        let (east, north) = shadow_per_metre(&sun).unwrap();
        assert!(east.abs() < 1e-9);
        assert!((north - 1.0).abs() < 1e-9);
    }

    #[test]
    fn no_shadow_with_the_sun_below_the_horizon() {
        let sun = SunPosition {
            azimuth_deg: 0.0,
            altitude_deg: -5.0,
        };
        assert!(shadow_per_metre(&sun).is_none());
    }

    #[test]
    fn counterclockwise_square_has_positive_area() {
        let square = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        assert_eq!(signed_area(&square), 1.0);
    }
}
