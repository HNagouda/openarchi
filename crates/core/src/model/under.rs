//! What a placed model stands on: the existing buildings its footprint
//! reaches over and the trees in its way. Placing the model takes away the
//! ones the rules below pick, as site edits marked under the model.

use geo::{Area, BooleanOps, BoundingRect, Distance, Euclidean, Intersects, MultiPolygon, Rect};
use serde::Serialize;

use super::{footprint_on_site, Model};
use crate::edits::{standing_trees, SiteEdits};
use crate::render::tree_size;
use crate::scene::{Layer, Scene};

/// An existing building counts as replaced when the model covers more than
/// this share of its footprint, so a model that only touches a neighbour's
/// wall leaves the neighbour standing.
// #! NOTE[3d, rule] : More than half of a building's footprint under the model replaces it; try real plots before changing the share
pub const REPLACED_SHARE: f64 = 0.5;

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

/// A tree the model stands on or into.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CoveredTree {
    /// The tree's source id, a BGT id in the Netherlands, or a planted tree's id.
    pub id: String,
    /// How far the trunk stands from the model's footprint, 0 inside it.
    pub distance_m: f64,
    pub is_planted: bool,
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

/// Every tree in the model's way, where the edits have it standing: a tree
/// whose trunk is inside the footprint, or whose crown reaches over it (its
/// trunk within the crown radius every view draws a tree with). A tree whose
/// crown only brushes a wall would really be pruned, but a student placing a
/// design wants the trees it stands in out of the way.
// #! NOTE[edits, rule] : A tree goes when its trunk is inside the model's footprint or within the 3 m crown radius of it
pub fn covered_trees(model: &Model, scene: &Scene, edits: &SiteEdits) -> Vec<CoveredTree> {
    let footprint = MultiPolygon::new(footprint_on_site(model, scene));
    let Some(rect) = footprint.bounding_rect() else {
        return Vec::new();
    };
    let (_, crown_m) = tree_size();
    let reach = Rect::new(
        (rect.min().x - crown_m, rect.min().y - crown_m),
        (rect.max().x + crown_m, rect.max().y + crown_m),
    );

    standing_trees(scene, edits)
        .into_iter()
        .filter(|tree| reach.intersects(&tree.at))
        .filter_map(|tree| {
            let distance_m = Euclidean::distance(&tree.at, &footprint);
            (distance_m <= crown_m).then_some(CoveredTree {
                id: tree.id,
                distance_m,
                is_planted: tree.is_planted,
            })
        })
        .collect()
}

/// The ids of what placing the model takes away: the buildings it covers
/// more than `REPLACED_SHARE` of and the trees in its way.
pub fn picked(model: &Model, scene: &Scene, edits: &SiteEdits) -> Vec<String> {
    let buildings = covered_buildings(model, scene)
        .into_iter()
        .filter(|covered| covered.share > REPLACED_SHARE)
        .map(|covered| covered.id);
    let trees = covered_trees(model, scene, edits)
        .into_iter()
        .map(|covered| covered.id);

    buildings.chain(trees).collect()
}
