//! Buildings and trees standing up in a tilted view.
//!
//! Each building is its footprint extruded to its height: the walls that face
//! the viewer, then a flat roof. Roofs keep the diagram's fill, so a building
//! age or use diagram reads on the roofs. Walls are lit by the sun of the
//! look's date and time, see `lighting.rs`. Trees are a trunk and a round
//! crown. The neighbours around the site stand among them in one muted roof
//! colour, sorted with them, so a neighbour in front hides what is behind it.

use std::fmt::Write;

use geo::{Geometry, LineString, Polygon};

use super::classify;
use super::color;
use super::drawing::{Canvas, Drawn};
use super::geometry::{decimal, mm, push_paper_points, Projection};
use super::lighting::{light, wall_fill, Light};
use super::order::{back_to_front, Footprint};
use super::shadow::{polygons, signed_area};
use super::style::ResolvedLayer;
use super::svg::layer_attributes;
use crate::diagram::Diagram;
use crate::scene::Feature;
use crate::sun::SunPosition;
use crate::theme::Theme;

/// Used when a building has no measured height: three storeys, a common Dutch
/// terraced house. Such buildings are greyed so nobody reads the guess as data.
pub const DEFAULT_HEIGHT_M: f64 = 9.0;
/// Trees carry no height in the data. A mature street tree in a Dutch town.
pub const TREE_HEIGHT_M: f64 = 10.0;
const TREE_CROWN_RADIUS_M: f64 = 3.0;
/// How far a building of unknown height moves towards the unknown colour.
const ESTIMATED_GREY: f64 = 0.65;
/// Edges between faces: a light hairline in the paper colour.
const EDGE_MM: f64 = 0.06;
const EDGE_OPACITY: f64 = 0.7;
const TRUNK_MM: f64 = 0.14;
/// Neighbours' walls stay this close to their roof, so the site's own
/// massing is the one that models in the light.
const NEIGHBOUR_LIGHT: f64 = 0.35;

/// Height in metres, and whether it is the default guess.
pub fn building_height(feature: &Feature) -> (f64, bool) {
    match feature.attributes.height_m {
        Some(height) if height > 0.0 => (height, false),
        _ => (DEFAULT_HEIGHT_M, true),
    }
}

/// Height and crown radius of a tree, in metres.
pub fn tree_size() -> (f64, f64) {
    (TREE_HEIGHT_M, TREE_CROWN_RADIUS_M)
}

/// What a solid layer would put in the legend, worked out without drawing it.
pub fn drawn(resolved: &ResolvedLayer, features: &[&Feature], diagram: &Diagram) -> Drawn {
    let class_count = resolved.classes.as_ref().map_or(0, |c| c.classes.len());
    let mut drawn = Drawn {
        has_features: !features.is_empty(),
        classes_present: vec![false; class_count],
        has_unknown: false,
    };
    let classify = diagram
        .classify
        .as_ref()
        .filter(|classify| classify.layer == resolved.layer && resolved.classes.is_some());
    if let Some(classify) = classify {
        for feature in features {
            match classify::class_of(feature, classify) {
                Some(index) if index < class_count => drawn.classes_present[index] = true,
                _ => drawn.has_unknown = true,
            }
        }
    }

    drawn
}

/// The buildings around the site, in one roof colour, each clipped to the drawing zone.
pub struct Neighbours<'a> {
    pub features: Vec<&'a Feature>,
    pub roof: String,
    pub clip_id: &'a str,
}

enum Kind<'a> {
    Building {
        polygon: &'a Polygon<f64>,
        roof: String,
        /// A neighbour is clipped to the drawing zone; the site's own buildings stand free.
        clip_id: Option<&'a str>,
    },
    Tree {
        at: (f64, f64),
    },
}

struct Solid<'a> {
    kind: Kind<'a>,
    height: f64,
}

/// Every building and tree, and every neighbour, back to front, as one group.
#[allow(clippy::too_many_arguments)]
pub fn write(
    out: &mut String,
    buildings: Option<(&ResolvedLayer, &[&Feature])>,
    trees: Option<(&ResolvedLayer, &[&Feature])>,
    neighbours: Option<&Neighbours>,
    diagram: &Diagram,
    theme: &Theme,
    canvas: &Canvas,
    sun: &SunPosition,
) {
    let projection = &canvas.projection;
    let scope = canvas.scope;
    let mut solids = Vec::new();
    if let Some((resolved, features)) = buildings {
        for feature in features {
            let (height, is_estimated) = building_height(feature);
            let mut roof = roof_fill(resolved, feature, diagram, theme);
            if is_estimated {
                roof = color::mix(&roof, &theme.palette.unknown, ESTIMATED_GREY);
            }
            for polygon in polygons(&feature.geometry) {
                solids.push(Solid {
                    kind: Kind::Building {
                        polygon,
                        roof: roof.clone(),
                        clip_id: None,
                    },
                    height,
                });
            }
        }
    }
    if let Some(neighbours) = neighbours {
        for feature in &neighbours.features {
            for polygon in polygons(&feature.geometry) {
                solids.push(Solid {
                    kind: Kind::Building {
                        polygon,
                        roof: neighbours.roof.clone(),
                        clip_id: Some(neighbours.clip_id),
                    },
                    height: building_height(feature).0,
                });
            }
        }
    }
    if let Some((_, features)) = trees {
        for feature in features {
            if let Geometry::Point(point) = feature.geometry {
                solids.push(Solid {
                    kind: Kind::Tree {
                        at: (point.x(), point.y()),
                    },
                    height: TREE_HEIGHT_M,
                });
            }
        }
    }
    if solids.is_empty() {
        return;
    }

    let footprints: Vec<Footprint> = solids
        .iter()
        .map(|solid| footprint(solid, projection))
        .collect();
    let order = back_to_front(&footprints, projection.squash, projection.rise);
    let light = light(sun, theme);
    let neighbour_light = light.softened(NEIGHBOUR_LIGHT);
    let tree_style = trees.map(|(resolved, _)| &resolved.style);

    let _ = write!(
        out,
        r#"<g {} fill-rule="evenodd" stroke="{}" stroke-width="{}" stroke-opacity="{}" stroke-linejoin="round">"#,
        layer_attributes(&format!("{scope}solids"), "Buildings and trees"),
        theme.palette.paper,
        mm(EDGE_MM),
        decimal(EDGE_OPACITY)
    );
    for index in order {
        let solid = &solids[index];
        match &solid.kind {
            Kind::Building {
                polygon,
                roof,
                clip_id,
            } => match clip_id {
                Some(clip_id) => {
                    let _ = write!(out, r#"<g clip-path="url(#{clip_id})">"#);
                    write_building(
                        out,
                        polygon,
                        solid.height,
                        roof,
                        &neighbour_light,
                        projection,
                    );
                    out.push_str("</g>");
                }
                None => write_building(out, polygon, solid.height, roof, &light, projection),
            },
            Kind::Tree { at } => {
                let fill = tree_style
                    .and_then(|style| style.fill.clone())
                    .unwrap_or_else(|| theme.palette.green.clone());
                let opacity = tree_style.map_or(1.0, |style| style.opacity);
                write_tree(out, *at, &fill, opacity, &light, projection);
            }
        }
    }
    out.push_str("</g>\n");
}

/// The roof colour: the feature's class, or the layer's own fill.
fn roof_fill(
    resolved: &ResolvedLayer,
    feature: &Feature,
    diagram: &Diagram,
    theme: &Theme,
) -> String {
    let classify = diagram
        .classify
        .as_ref()
        .filter(|classify| classify.layer == resolved.layer);
    if let (Some(classes), Some(classify)) = (&resolved.classes, classify) {
        return match classify::class_of(feature, classify) {
            Some(index) if index < classes.classes.len() => classes.classes[index].fill.clone(),
            _ => classes.unknown_fill.clone(),
        };
    }

    resolved
        .style
        .fill
        .clone()
        .unwrap_or_else(|| theme.palette.figure.clone())
}

fn footprint(solid: &Solid, projection: &Projection) -> Footprint {
    let ring: Vec<(f64, f64)> = match &solid.kind {
        Kind::Building { polygon, .. } => polygon
            .exterior()
            .0
            .iter()
            .map(|c| projection.to_view(c.x, c.y))
            .collect(),
        Kind::Tree { at } => {
            let (u, v) = projection.to_view(at.0, at.1);
            let r = TREE_CROWN_RADIUS_M;
            vec![
                (u - r, v - r),
                (u + r, v - r),
                (u + r, v + r),
                (u - r, v + r),
                (u - r, v - r),
            ]
        }
    };

    Footprint::new(ring, solid.height)
}

struct Wall {
    corners: [(f64, f64, f64); 4],
    /// Outward normal on the ground, unit length.
    normal: (f64, f64),
    depth: f64,
}

fn write_building(
    out: &mut String,
    polygon: &Polygon<f64>,
    height: f64,
    roof: &str,
    light: &Light,
    projection: &Projection,
) {
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
            let _ = write!(
                out,
                r#"<path d="{data}" fill="{}"/>"#,
                wall_fill(base, wall.normal, light)
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
        let _ = write!(out, r#"<path d="{data}" fill="{roof}"/>"#);
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

fn write_tree(
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
        r#"<path d="M{} {}V{}" stroke="{trunk}" stroke-width="{}" stroke-opacity="1"/><circle cx="{}" cy="{}" r="{}" fill="{fill}" opacity="{}"/>"#,
        mm(base_x),
        mm(base_y),
        mm(crown_y),
        mm(TRUNK_MM),
        mm(crown_x),
        mm(crown_y),
        mm(crown_radius),
        decimal(opacity)
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_height_falls_back_to_the_default_and_says_so() {
        let feature = Feature {
            layer: crate::scene::Layer::Building,
            geometry: geo::Point::new(0.0, 0.0).into(),
            attributes: Default::default(),
        };
        assert_eq!(building_height(&feature), (DEFAULT_HEIGHT_M, true));
    }
}
