//! The site with the user's edits applied: the one scene every sheet, the 3D
//! view, the computed layers and the exports draw from.

use std::borrow::Cow;

use geo::{Geometry, Point};

use super::{SiteEdits, TreeAt};
use crate::scene::{Attributes, Feature, Layer, Scene};

/// `scene` with the removed trees and buildings left out, the moved trees
/// where they now stand and the planted trees added. Without edits it is the
/// scene itself, so a sheet without edits draws exactly as before.
pub fn edited<'a>(scene: &'a Scene, edits: &SiteEdits) -> Cow<'a, Scene> {
    if edits.is_empty() {
        return Cow::Borrowed(scene);
    }

    let mut edited = scene.clone();
    edited
        .features
        .retain(|feature| !is_editable(feature) || !edits.is_removed(id_of(feature)));
    for feature in &mut edited.features {
        if feature.layer != Layer::Tree {
            continue;
        }
        if let Some(tree) = edits.moved.iter().find(|tree| tree.id == id_of(feature)) {
            feature.geometry = Geometry::Point(local(scene, tree));
        }
    }
    let planted = edits
        .planted
        .iter()
        .filter(|tree| !edits.is_removed(&tree.id))
        .map(|tree| planted_tree(scene, tree));
    edited.features.extend(planted);

    Cow::Owned(edited)
}

/// A tree as it stands after the edits, removed or not.
#[derive(Debug, Clone, PartialEq)]
pub struct StandingTree {
    pub id: String,
    /// In the scene's local frame.
    pub at: Point<f64>,
    pub is_planted: bool,
}

/// Every tree of the site with a source id, moved where the edits moved it,
/// and every planted tree. Removed trees are in it too, so a model placed
/// over one can still list it.
pub fn standing_trees(scene: &Scene, edits: &SiteEdits) -> Vec<StandingTree> {
    let mut trees = Vec::new();
    for feature in &scene.features {
        let (Layer::Tree, Geometry::Point(at), Some(id)) = (
            feature.layer,
            &feature.geometry,
            &feature.attributes.source_id,
        ) else {
            continue;
        };
        let at = match edits.moved.iter().find(|tree| &tree.id == id) {
            Some(tree) => local(scene, tree),
            None => *at,
        };
        trees.push(StandingTree {
            id: id.clone(),
            at,
            is_planted: false,
        });
    }
    for tree in &edits.planted {
        trees.push(StandingTree {
            id: tree.id.clone(),
            at: local(scene, tree),
            is_planted: true,
        });
    }

    trees
}

fn is_editable(feature: &Feature) -> bool {
    matches!(feature.layer, Layer::Tree | Layer::Building)
}

fn id_of(feature: &Feature) -> &str {
    feature.attributes.source_id.as_deref().unwrap_or("")
}

fn local(scene: &Scene, tree: &TreeAt) -> Point<f64> {
    Point::new(tree.x - scene.origin.0, tree.y - scene.origin.1)
}

fn planted_tree(scene: &Scene, tree: &TreeAt) -> Feature {
    Feature {
        layer: Layer::Tree,
        geometry: Geometry::Point(local(scene, tree)),
        attributes: Attributes {
            source_id: Some(tree.id.clone()),
            ..Attributes::default()
        },
    }
}
