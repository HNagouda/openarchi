//! Buildings and trees standing up in a tilted view.
//!
//! Each building is its footprint extruded to its height: the walls that face
//! the viewer, then a flat roof. Roofs keep the diagram's fill, so a building
//! age or use diagram reads on the roofs. Walls are lit by the sun of the
//! look's date and time, see `lighting.rs`. Trees are a trunk and a round
//! crown. The neighbours around the site stand among them in one muted roof
//! colour, sorted with them, so a neighbour in front hides what is behind it.

use std::fmt::Write;

use geo::{Geometry, Polygon};

use super::classify;
use super::color;
use super::drawing::{Canvas, Drawn};
use super::geometry::{decimal, mm, Projection};
use super::lighting::light;
use super::order::{back_to_front, Footprint};
use super::proposal;
use super::solid_draw::{write_building, write_tree};
use super::style::ResolvedLayer;
use super::svg::layer_attributes;
use crate::diagram::Diagram;
use crate::floors::STOREY_HEIGHT_M;
use crate::model::PlacedModel;
use crate::scene::{polygons, Feature};
use crate::sun::SunPosition;
use crate::theme::Theme;

/// Used when a building has no measured height: three storeys, a common Dutch
/// terraced house. Such buildings are greyed so nobody reads the guess as data.
pub const DEFAULT_HEIGHT_M: f64 = 3.0 * STOREY_HEIGHT_M;
/// Trees carry no height in the data. A mature street tree in a Dutch town.
pub const TREE_HEIGHT_M: f64 = 10.0;
const TREE_CROWN_RADIUS_M: f64 = 3.0;
/// How far a building of unknown height moves towards the unknown colour.
pub(crate) const ESTIMATED_GREY: f64 = 0.65;
/// Edges between faces: a light hairline in the paper colour.
const EDGE_MM: f64 = 0.06;
const EDGE_OPACITY: f64 = 0.7;
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

/// The buildings around the site, in one roof colour, clipped to the drawing zone.
pub struct Neighbours<'a> {
    pub features: Vec<&'a Feature>,
    pub roof: String,
    pub clip_id: &'a str,
    /// The sheet's `context_opacity`, 0 to 1: below 1 the neighbours fade toward the paper.
    pub opacity: f64,
}

enum Kind<'a> {
    Building {
        polygon: &'a Polygon<f64>,
        roof: String,
        /// A neighbour is clipped to the drawing zone and may be faded;
        /// the site's own buildings stand free: `(clip id, opacity)`.
        neighbour: Option<(&'a str, f64)>,
    },
    Tree {
        at: (f64, f64),
    },
    /// The imported model, sorted among the buildings as one solid.
    Model(&'a PlacedModel),
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
    model: Option<&PlacedModel>,
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
                        neighbour: None,
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
                        neighbour: Some((neighbours.clip_id, neighbours.opacity)),
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
    if let Some(model) = model {
        solids.push(Solid {
            kind: Kind::Model(model),
            height: model.height,
        });
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
    // Neighbours next to each other in drawing order share one clip group. A
    // group per neighbour, let alone a see-through one, made a tilted sheet
    // with a few thousand of them paint twice as slowly.
    let mut is_clip_open = false;
    for index in order {
        let solid = &solids[index];
        let neighbour = match &solid.kind {
            Kind::Building { neighbour, .. } => *neighbour,
            Kind::Tree { .. } | Kind::Model(_) => None,
        };
        match (neighbour, is_clip_open) {
            (Some((clip_id, _)), false) => {
                let _ = write!(out, r#"<g clip-path="url(#{clip_id})">"#);
                is_clip_open = true;
            }
            (None, true) => {
                out.push_str("</g>");
                is_clip_open = false;
            }
            _ => {}
        }

        match &solid.kind {
            Kind::Building {
                polygon,
                roof,
                neighbour,
            } => match neighbour {
                Some((_, opacity)) => {
                    // Faded by colour rather than by an opacity group, so a
                    // neighbour stays solid and its walls never show through its roof.
                    let fade =
                        (*opacity < 1.0).then(|| (theme.palette.paper.as_str(), 1.0 - opacity));
                    write_building(
                        out,
                        polygon,
                        solid.height,
                        roof,
                        &neighbour_light,
                        projection,
                        fade,
                    );
                }
                None => write_building(out, polygon, solid.height, roof, &light, projection, None),
            },
            Kind::Tree { at } => {
                let fill = tree_style
                    .and_then(|style| style.fill.clone())
                    .unwrap_or_else(|| theme.palette.green.clone());
                let opacity = tree_style.map_or(1.0, |style| style.opacity);
                write_tree(out, *at, &fill, opacity, &light, projection);
            }
            Kind::Model(model) => {
                proposal::write_solid(out, model, &theme.palette.accent, &light, projection);
            }
        }
    }
    if is_clip_open {
        out.push_str("</g>");
    }
    out.push_str("</g>\n");
}

/// The roof colour: the feature's class, or the layer's own fill.
pub(crate) fn roof_fill(
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
        Kind::Model(model) => return proposal::footprint(model, projection),
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
