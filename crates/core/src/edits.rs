//! Changes the user makes to the fetched site: trees and buildings taken
//! away, trees moved, and new trees planted.
//!
//! An edit names a tree or building by its source id (a BGT or BAG id in the
//! Netherlands) rather than by its place in the scene, so it survives fetching
//! the site again. `edited` applies the edits, and every sheet, the 3D view,
//! the computed layers and the exports draw from the scene it returns. The
//! buildings and trees a model stands on are edits too, marked as under the
//! model, so placing the model again can pick them afresh.
//
// #! NOTE[edits, ratios] : Ratios and the site analysis count the site as fetched, without the edits, as they do without the model
// #! NOTE[edits, canopy] : The RIVM tree canopy raster (woods, sky view, radiation) is not edited when a single tree is removed, moved or planted
// #! NOTE[edits, context] : Only the site is edited; the neighbourhood around it is drawn as fetched

mod apply;
mod editable;
#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

pub use apply::{edited, standing_trees, StandingTree};
pub use editable::{editable_at, edited_things, Editable};

use crate::pin::is_on_site;
use crate::scene::{Layer, Scene};
use crate::{Error, Result};

/// Planted trees are named "planted-1", "planted-2" and so on.
pub const PLANTED_PREFIX: &str = "planted-";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SiteEdits {
    /// Trees and buildings taken away.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed: Vec<Removed>,
    /// Trees standing somewhere other than where the survey has them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub moved: Vec<TreeAt>,
    /// Trees the user planted, named with `PLANTED_PREFIX`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub planted: Vec<TreeAt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Removed {
    /// The source id of a tree or building, or a planted tree's id.
    pub id: String,
    /// Taken away because the model stands on it, not by hand. Placing the
    /// model again picks these afresh, and removing the model brings them back.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_under_model: bool,
}

/// A tree at a point in the scene's projected coordinates (RD New metres in
/// the Netherlands), like pins and the model's placement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TreeAt {
    pub id: String,
    pub x: f64,
    pub y: f64,
}

impl SiteEdits {
    pub fn is_empty(&self) -> bool {
        self.removed.is_empty() && self.moved.is_empty() && self.planted.is_empty()
    }

    pub fn is_removed(&self, id: &str) -> bool {
        self.removed.iter().any(|removed| removed.id == id)
    }

    /// The edits with `id` taken away by hand. A planted tree is simply gone.
    pub fn with_removed(&self, id: &str) -> SiteEdits {
        let mut edits = self.clone();
        if id.starts_with(PLANTED_PREFIX) {
            edits.planted.retain(|tree| tree.id != id);
        } else if !edits.is_removed(id) {
            edits.removed.push(Removed {
                id: id.to_string(),
                is_under_model: false,
            });
        }
        edits
    }

    /// The edits with `id` back as the survey has it: no longer removed, and a moved tree back in its place.
    pub fn with_restored(&self, id: &str) -> SiteEdits {
        let mut edits = self.clone();
        edits.removed.retain(|removed| removed.id != id);
        edits.moved.retain(|tree| tree.id != id);
        edits
    }

    /// The edits with tree `id` standing at `x`, `y`.
    pub fn with_moved(&self, id: &str, x: f64, y: f64) -> SiteEdits {
        let mut edits = self.clone();
        let moved = TreeAt {
            id: id.to_string(),
            x,
            y,
        };
        let trees = if id.starts_with(PLANTED_PREFIX) {
            &mut edits.planted
        } else {
            &mut edits.moved
        };
        match trees.iter_mut().find(|tree| tree.id == id) {
            Some(tree) => *tree = moved,
            None => trees.push(moved),
        }
        edits
    }

    /// The edits with a new tree at `x`, `y`, and the new tree's id.
    pub fn with_planted(&self, x: f64, y: f64) -> (SiteEdits, String) {
        let id = self.next_planted_id();
        let mut edits = self.clone();
        edits.planted.push(TreeAt {
            id: id.clone(),
            x,
            y,
        });
        (edits, id)
    }

    /// The first "planted-N" id no planted tree uses yet.
    fn next_planted_id(&self) -> String {
        let mut number = self.planted.len() + 1;
        loop {
            let id = format!("{PLANTED_PREFIX}{number}");
            if self.planted.iter().all(|tree| tree.id != id) {
                return id;
            }
            number += 1;
        }
    }

    /// The edits with what the model stands on picked afresh: `ids` taken
    /// away under the model, and whatever the model left behind back. Things
    /// removed by hand stay removed.
    pub fn with_model_picks(&self, ids: &[String]) -> SiteEdits {
        let mut edits = self.without_model_picks();
        for id in ids {
            if !edits.is_removed(id) {
                edits.removed.push(Removed {
                    id: id.clone(),
                    is_under_model: true,
                });
            }
        }
        edits
    }

    /// The edits with `id`, something the model stands on, taken away under
    /// the model or kept, as its box in the Model section was ticked.
    /// Unticking keeps it whoever took it away; a moved tree stays moved.
    pub fn with_ticked(&self, id: &str, is_ticked: bool) -> SiteEdits {
        let mut edits = self.clone();
        if !is_ticked {
            edits.removed.retain(|removed| removed.id != id);
        } else if !edits.is_removed(id) {
            edits.removed.push(Removed {
                id: id.to_string(),
                is_under_model: true,
            });
        }
        edits
    }

    /// The edits with everything the model took away back, for when the model goes.
    pub fn without_model_picks(&self) -> SiteEdits {
        let mut edits = self.clone();
        edits.removed.retain(|removed| !removed.is_under_model);
        edits
    }

    /// The edits that still name something in `scene`, for a site fetched
    /// again: a tree or building that is gone takes its edits with it, and a
    /// planted tree outside the new outline goes too.
    pub fn kept_in(&self, scene: &Scene) -> SiteEdits {
        let planted: Vec<TreeAt> = self
            .planted
            .iter()
            .filter(|tree| is_on_site(scene, tree.x, tree.y))
            .cloned()
            .collect();
        let is_kept =
            |id: &str| surveyed(scene, id).is_some() || planted.iter().any(|tree| tree.id == id);

        SiteEdits {
            removed: self
                .removed
                .iter()
                .filter(|removed| is_kept(&removed.id))
                .cloned()
                .collect(),
            moved: self
                .moved
                .iter()
                .filter(|tree| surveyed(scene, &tree.id) == Some(Layer::Tree))
                .cloned()
                .collect(),
            planted,
        }
    }

    /// Refuses a tree that is not a number or stands outside the site, so a saved project never holds one.
    pub fn check(&self, scene: &Scene) -> Result<()> {
        for tree in self.moved.iter().chain(&self.planted) {
            if !tree.x.is_finite() || !tree.y.is_finite() {
                return Err(Error::Invalid(format!(
                    "a tree's place must be numbers, got {tree:?}"
                )));
            }
            if !is_on_site(scene, tree.x, tree.y) {
                return Err(Error::Invalid(format!(
                    "trees stand inside the site's outline; ({}, {}) is outside it",
                    tree.x, tree.y
                )));
            }
        }
        Ok(())
    }
}

/// The layer of the tree or building in `scene` with source id `id`, `None`
/// when the site has none. Only trees and buildings can be edited.
pub fn surveyed(scene: &Scene, id: &str) -> Option<Layer> {
    scene
        .features
        .iter()
        .find(|feature| feature.attributes.source_id.as_deref() == Some(id))
        .map(|feature| feature.layer)
        .filter(|layer| matches!(layer, Layer::Tree | Layer::Building))
}
