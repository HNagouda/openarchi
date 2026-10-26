//! The trees and buildings as the edit tool sees them: the one under a
//! pointer, and every one an edit names, with where it stood and whether it
//! is gone. Removed ones are included, so they can be found and restored.

use geo::{Contains, Distance, Euclidean, Geometry, Point, Polygon};
use serde::Serialize;

use super::{standing_trees, SiteEdits, StandingTree};
use crate::scene::{polygons, Feature, Layer, Scene};

/// A tree or building of the site, in the scene's projected coordinates (RD New metres in the Netherlands).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Editable {
    /// The source id, or a planted tree's id.
    pub id: String,
    /// `tree` or `building`.
    pub layer: Layer,
    /// The source's own kind, such as "boom".
    pub kind: Option<String>,
    pub year_built: Option<i32>,
    pub height_m: Option<f64>,
    /// Where a tree stands now.
    pub at: Option<[f64; 2]>,
    /// Where a moved tree stands in the survey.
    pub surveyed_at: Option<[f64; 2]>,
    /// A building's outline.
    pub outline: Vec<Vec<[f64; 2]>>,
    pub is_removed: bool,
    /// Taken away because the model stands on it.
    pub is_under_model: bool,
    pub is_planted: bool,
}

/// The tree within `reach_m` of RD New point (`x`, `y`), the nearest when
/// there are several, or else the building whose footprint holds it. Trees
/// win because they are small marks, hard to hit inside a large roof.
pub fn editable_at(
    scene: &Scene,
    edits: &SiteEdits,
    x: f64,
    y: f64,
    reach_m: f64,
) -> Option<Editable> {
    let point = Point::new(x - scene.origin.0, y - scene.origin.1);
    let nearest = standing_trees(scene, edits)
        .into_iter()
        .map(|tree| (Euclidean::distance(&tree.at, &point), tree))
        .filter(|(distance, _)| *distance <= reach_m)
        .min_by(|(a, _), (b, _)| a.total_cmp(b));
    if let Some((_, tree)) = nearest {
        return Some(tree_of(scene, edits, &tree));
    }

    let building = scene.features.iter().find(|feature| {
        feature.layer == Layer::Building
            && feature.attributes.source_id.is_some()
            && polygons(&feature.geometry)
                .iter()
                .any(|polygon| polygon.contains(&point))
    })?;
    Some(building_of(scene, edits, building))
}

/// Every tree and building an edit names, removed, moved or planted, in that
/// order. An edit whose tree or building the site no longer has is left out.
pub fn edited_things(scene: &Scene, edits: &SiteEdits) -> Vec<Editable> {
    let trees = standing_trees(scene, edits);
    let mut things = Vec::new();
    for removed in &edits.removed {
        if let Some(tree) = trees.iter().find(|tree| tree.id == removed.id) {
            things.push(tree_of(scene, edits, tree));
        } else if let Some(building) = building_with(scene, &removed.id) {
            things.push(building_of(scene, edits, building));
        }
    }
    let is_listed = |id: &str, things: &[Editable]| things.iter().any(|thing| thing.id == id);
    for tree in &trees {
        let is_edited = tree.is_planted || edits.moved.iter().any(|moved| moved.id == tree.id);
        if is_edited && !is_listed(&tree.id, &things) {
            things.push(tree_of(scene, edits, tree));
        }
    }

    things
}

fn tree_of(scene: &Scene, edits: &SiteEdits, tree: &StandingTree) -> Editable {
    let surveyed = scene.features.iter().find(|feature| {
        feature.layer == Layer::Tree && feature.attributes.source_id.as_deref() == Some(&tree.id)
    });
    let surveyed_at = surveyed
        .filter(|_| edits.moved.iter().any(|moved| moved.id == tree.id))
        .and_then(|feature| match feature.geometry {
            Geometry::Point(at) => Some(rd(scene, at)),
            _ => None,
        });
    Editable {
        id: tree.id.clone(),
        layer: Layer::Tree,
        kind: surveyed.and_then(|feature| feature.attributes.kind.clone()),
        year_built: None,
        height_m: None,
        at: Some(rd(scene, tree.at)),
        surveyed_at,
        outline: Vec::new(),
        is_removed: edits.is_removed(&tree.id),
        is_under_model: is_under_model(edits, &tree.id),
        is_planted: tree.is_planted,
    }
}

fn building_of(scene: &Scene, edits: &SiteEdits, building: &Feature) -> Editable {
    let id = building.attributes.source_id.clone().unwrap_or_default();
    // A building cut by a tile edge is several features with one id.
    let outline = scene
        .features
        .iter()
        .filter(|feature| {
            feature.layer == Layer::Building
                && feature.attributes.source_id == building.attributes.source_id
        })
        .flat_map(|feature| polygons(&feature.geometry))
        .map(|polygon| ring_in_rd(scene, polygon))
        .collect();
    Editable {
        is_removed: edits.is_removed(&id),
        is_under_model: is_under_model(edits, &id),
        id,
        layer: Layer::Building,
        kind: None,
        year_built: building.attributes.year_built,
        height_m: building.attributes.height_m,
        at: None,
        surveyed_at: None,
        outline,
        is_planted: false,
    }
}

fn building_with<'a>(scene: &'a Scene, id: &str) -> Option<&'a Feature> {
    scene.features.iter().find(|feature| {
        feature.layer == Layer::Building && feature.attributes.source_id.as_deref() == Some(id)
    })
}

fn is_under_model(edits: &SiteEdits, id: &str) -> bool {
    edits
        .removed
        .iter()
        .any(|removed| removed.id == id && removed.is_under_model)
}

fn rd(scene: &Scene, at: Point<f64>) -> [f64; 2] {
    [at.x() + scene.origin.0, at.y() + scene.origin.1]
}

fn ring_in_rd(scene: &Scene, polygon: &Polygon<f64>) -> Vec<[f64; 2]> {
    polygon
        .exterior()
        .points()
        .map(|point| rd(scene, point))
        .collect()
}
