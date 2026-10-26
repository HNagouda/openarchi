//! One solid on paper: a building's walls that face the viewer, lit by the
//! sun, under its flat roof, or a tree's trunk and round crown.

use std::fmt::Write;

use geo::{LineString, Polygon, Simplify};

use super::color;
use super::geometry::{decimal, mm, push_paper_points, Projection};
use super::lighting::{wall_fill, Light};
use super::shadow::signed_area;
use super::simplify::SIMPLIFY_MM;
use super::solid::tree_size;

const TRUNK_MM: f64 = 0.14;

struct Wall {
    corners: [(f64, f64, f64); 4],
    /// Outward normal on the ground, unit length.
    normal: (f64, f64),
    depth: f64,
}

pub(super) fn write_building(
    out: &mut String,
    polygon: &Polygon<f64>,
    height: f64,
    roof: &str,
    light: &Light,
    projection: &Projection,
    fade: Option<(&str, f64)>,
) {
    // Footprint edges far shorter than the paper can show would each become a
    // wall, and a large site at a small scale has tens of thousands of them.
    let polygon = &polygon.simplify(&(SIMPLIFY_MM / projection.mm_per_m));
    let mut walls = Vec::new();
    collect_walls(polygon.exterior(), false, height, projection, &mut walls);
    for interior in polygon.interiors() {
        collect_walls(interior, true, height, projection, &mut walls);
    }
    walls.sort_by(|a, b| b.depth.total_cmp(&a.depth));

    let base = if roof.starts_with('#') {
        roof
    } else {
        "#808080"
    };
    for wall in &walls {
        let points: Vec<(f64, f64)> = wall
            .corners
            .iter()
            .map(|&(x, y, z)| projection.to_paper_at(x, y, z))
            .collect();
        let mut data = String::new();
        if push_paper_points(&mut data, &points, true) {
            let fill = faded(wall_fill(base, wall.normal, light), fade);
            let _ = write!(
                out,
                r#"<path d="{data}" fill="{fill}" stroke="{}"/>"#,
                light.edge(&fill)
            );
        }
    }

    let mut data = String::new();
    for ring in std::iter::once(polygon.exterior()).chain(polygon.interiors()) {
        let points: Vec<(f64, f64)> = ring
            .0
            .iter()
            .map(|c| projection.to_paper_at(c.x, c.y, height))
            .collect();
        push_paper_points(&mut data, &points, true);
    }
    if !data.is_empty() {
        let roof = if roof.starts_with('#') {
            faded(roof.to_string(), fade)
        } else {
            roof.to_string()
        };
        let _ = write!(
            out,
            r#"<path d="{data}" fill="{roof}" stroke="{}"/>"#,
            light.edge(&roof)
        );
    }
}

/// `colour` moved `share` of the way toward the fade colour, or as it is without one.
fn faded(colour: String, fade: Option<(&str, f64)>) -> String {
    match fade {
        Some((toward, share)) => color::mix(&colour, toward, share),
        None => colour,
    }
}

/// The walls along one ring that face the viewer.
fn collect_walls(
    ring: &LineString<f64>,
    is_hole: bool,
    height: f64,
    projection: &Projection,
    walls: &mut Vec<Wall>,
) {
    let points: Vec<(f64, f64)> = ring.0.iter().map(|c| (c.x, c.y)).collect();
    // For a counterclockwise outer ring the solid is on the left, so outward is to the right.
    let outward_sign = if (signed_area(&points) > 0.0) != is_hole {
        1.0
    } else {
        -1.0
    };
    for pair in points.windows(2) {
        let ((x1, y1), (x2, y2)) = (pair[0], pair[1]);
        let length = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
        if length < 1e-6 {
            continue;
        }
        let normal = (
            outward_sign * (y2 - y1) / length,
            -outward_sign * (x2 - x1) / length,
        );
        // Facing the viewer means the normal points to smaller v in the view frame.
        let (_, normal_v) = projection.turn(normal.0, normal.1);
        if normal_v >= 0.0 {
            continue;
        }
        let (_, depth) = projection.to_view((x1 + x2) / 2.0, (y1 + y2) / 2.0);
        walls.push(Wall {
            corners: [
                (x1, y1, 0.0),
                (x2, y2, 0.0),
                (x2, y2, height),
                (x1, y1, height),
            ],
            normal,
            depth,
        });
    }
}

pub(super) fn write_tree(
    out: &mut String,
    at: (f64, f64),
    fill: &str,
    opacity: f64,
    light: &Light,
    projection: &Projection,
) {
    let (height, radius) = tree_size();
    let (base_x, base_y) = projection.to_paper_at(at.0, at.1, 0.0);
    let (crown_x, crown_y) = projection.to_paper_at(at.0, at.1, height - radius);
    let trunk = color::mix(fill, &light.shade, 0.5);
    let crown_radius = radius * projection.mm_per_m;
    let _ = write!(
        out,
        r#"<path d="M{} {}V{}" stroke="{trunk}" stroke-width="{}"/><circle cx="{}" cy="{}" r="{}" fill="{fill}" stroke="{}" opacity="{}"/>"#,
        mm(base_x),
        mm(base_y),
        mm(crown_y),
        mm(TRUNK_MM),
        mm(crown_x),
        mm(crown_y),
        mm(crown_radius),
        light.edge(fill),
        decimal(opacity)
    );
}
