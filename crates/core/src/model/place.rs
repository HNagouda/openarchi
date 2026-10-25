//! Where a model stands on the site: its faces and footprint moved into the
//! scene's frame, the ground under it, and the buildings it takes the place of.

use geo::{Area, BooleanOps, BoundingRect, Coord, Intersects, MapCoords, MultiPolygon, Polygon};

use serde::Serialize;

use super::faces::Face;
use super::{Model, Placement};
use crate::scene::{Layer, Scene};

/// The ground under a model is the median terrain in this band around its
/// footprint, the rule `sources/heights.rs` uses for a building's ground: the
/// terrain model has no values under buildings.
const GROUND_RING_M: f64 = 2.0;

/// An existing building counts as replaced when the model covers more than
/// this share of its footprint, so a model that only touches a neighbour's
/// wall leaves the neighbour standing.
// #! NOTE[3d, rule] : More than half of a building's footprint under the model replaces it; try real plots before changing the share
pub const REPLACED_SHARE: f64 = 0.5;

/// A model in the scene's frame, ready to draw.
#[derive(Debug, Clone)]
pub struct PlacedModel {
    /// Metres in the scene's local frame, z above the ground the buildings stand on.
    pub faces: Vec<Face>,
    pub footprint: Vec<Polygon<f64>>,
    /// The top of the model above that ground.
    pub height: f64,
}

/// `model` moved, turned and lifted onto the site of `scene`.
pub fn placed(model: &Model, scene: &Scene) -> PlacedModel {
    let shape = model.shape();
    let lift = lift_m(model, scene);
    let turn = Turn::of(&model.placement, scene);
    let faces = shape
        .faces
        .iter()
        .map(|face| Face {
            normal: turn.direction(face.normal),
            rings: face
                .rings
                .iter()
                .map(|ring| ring.iter().map(|&p| turn.point(p, lift)).collect())
                .collect(),
        })
        .collect();

    PlacedModel {
        faces,
        footprint: footprint_on_site(model, scene),
        height: shape.height + lift,
    }
}

/// Every triangle of the model in the scene's local frame, three corners each,
/// for the 3D view: the mesh as the file drew it, not the merged faces.
pub fn triangles_on_site(model: &Model, scene: &Scene) -> Vec<[f64; 3]> {
    let lift = lift_m(model, scene);
    let turn = Turn::of(&model.placement, scene);
    (0..model.mesh.triangles.len())
        .flat_map(|index| model.mesh.corners(index))
        .map(|corner| turn.point(corner, lift))
        .collect()
}

/// The model's footprint in the scene's local frame.
pub fn footprint_on_site(model: &Model, scene: &Scene) -> Vec<Polygon<f64>> {
    let turn = Turn::of(&model.placement, scene);
    model
        .shape()
        .footprint
        .iter()
        .map(|polygon| {
            polygon.map_coords(|c| {
                let [x, y, _] = turn.point([c.x, c.y, 0.0], 0.0);
                Coord { x, y }
            })
        })
        .collect()
}

/// The terrain under the model in metres above NAP, `None` for a scene without terrain.
pub fn ground_m(model: &Model, scene: &Scene) -> Option<f64> {
    let terrain = scene.terrain.as_ref()?;
    let footprint = MultiPolygon::new(footprint_on_site(model, scene));
    let rect = footprint.bounding_rect()?;
    let mut heights = Vec::new();
    let mut y = rect.min().y - GROUND_RING_M;
    while y <= rect.max().y + GROUND_RING_M {
        let mut x = rect.min().x - GROUND_RING_M;
        while x <= rect.max().x + GROUND_RING_M {
            if let Some(height) = terrain.value_at((x, y)) {
                heights.push(height);
            }
            x += terrain.cell_m;
        }
        y += terrain.cell_m;
    }
    if heights.is_empty() {
        return None;
    }
    heights.sort_by(f32::total_cmp);

    Some(f64::from(heights[heights.len() / 2]))
}

/// How far the model's base stands above the ground the buildings are drawn
/// on: nothing when it stands on the terrain, or its NAP level less the ground.
/// Without terrain the ground is unknown and the model stands on it.
fn lift_m(model: &Model, scene: &Scene) -> f64 {
    match (model.placement.base_m, ground_m(model, scene)) {
        (Some(base_m), Some(ground_m)) => base_m - ground_m,
        _ => 0.0,
    }
}

/// An existing building the model's footprint reaches over.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Covered {
    /// The building's source id, a BAG pand id in the Netherlands.
    pub id: String,
    /// How much of the building's footprint the model covers, 0 to 1.
    pub share: f64,
    pub year_built: Option<i32>,
    pub height_m: Option<f64>,
}

/// The source ids of the buildings in `scene` that the model covers more
/// than `REPLACED_SHARE` of, in the scene's order.
pub fn buildings_under(model: &Model, scene: &Scene) -> Vec<String> {
    covered_buildings(model, scene)
        .into_iter()
        .filter(|covered| covered.share > REPLACED_SHARE)
        .map(|covered| covered.id)
        .collect()
}

/// Every building with a source id that the model's footprint reaches over, in the scene's order.
pub fn covered_buildings(model: &Model, scene: &Scene) -> Vec<Covered> {
    let footprint = MultiPolygon::new(footprint_on_site(model, scene));
    let Some(reach) = footprint.bounding_rect() else {
        return Vec::new();
    };
    let mut covered: Vec<Covered> = Vec::new();
    for feature in &scene.features {
        let Some(id) = &feature.attributes.source_id else {
            continue;
        };
        if feature.layer != Layer::Building || !feature.geometry.intersects(&reach) {
            continue;
        }
        let building = match &feature.geometry {
            geo::Geometry::Polygon(polygon) => MultiPolygon::new(vec![polygon.clone()]),
            geo::Geometry::MultiPolygon(polygons) => polygons.clone(),
            _ => continue,
        };
        let area = building.unsigned_area();
        let share = building.intersection(&footprint).unsigned_area() / area;
        let is_new = !covered.iter().any(|seen| &seen.id == id);
        if area > 0.0 && share > 0.0 && is_new {
            covered.push(Covered {
                id: id.clone(),
                share,
                year_built: feature.attributes.year_built,
                height_m: feature.attributes.height_m,
            });
        }
    }

    covered
}

/// The placement as arithmetic: turn clockwise, then move into the scene's frame.
struct Turn {
    cos: f64,
    sin: f64,
    centre: (f64, f64),
}

impl Turn {
    fn of(placement: &Placement, scene: &Scene) -> Turn {
        let angle = placement.rotation_deg.to_radians();
        Turn {
            cos: angle.cos(),
            sin: angle.sin(),
            centre: (placement.x - scene.origin.0, placement.y - scene.origin.1),
        }
    }

    fn direction(&self, [x, y, z]: [f64; 3]) -> [f64; 3] {
        [x * self.cos + y * self.sin, -x * self.sin + y * self.cos, z]
    }

    fn point(&self, p: [f64; 3], lift: f64) -> [f64; 3] {
        let [x, y, z] = self.direction(p);
        [x + self.centre.0, y + self.centre.1, z + lift]
    }
}
