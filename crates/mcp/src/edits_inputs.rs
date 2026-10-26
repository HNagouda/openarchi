//! What the site edit tools accept, with the schema an assistant sees.

use schemars::JsonSchema;
use serde::Deserialize;

use crate::inputs::LonLatInput;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PickOnSite {
    /// The project id. Its site must be fetched.
    pub id: String,
    /// The point to look at, in longitude and latitude.
    pub at: Option<LonLatInput>,
    /// The same point in RD New metres, `[x, y]`, instead of `at`.
    pub rd: Option<[f64; 2]>,
    /// How far from the point a tree may stand and still be found, in metres.
    /// Defaults to 3, a tree's crown as every view draws it.
    pub reach_m: Option<f64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SiteThing {
    /// The project id.
    pub id: String,
    /// The id of a tree or building, as pick_on_site or list_site_edits
    /// return it: a BGT id for a tree, a BAG id for a building, or
    /// "planted-N" for a planted tree.
    pub thing: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveTree {
    /// The project id.
    pub id: String,
    /// The tree's id, as pick_on_site returns it.
    pub tree: String,
    /// Where it should stand, in longitude and latitude, inside the site.
    pub at: Option<LonLatInput>,
    /// The same point in RD New metres, `[x, y]`, instead of `at`.
    pub rd: Option<[f64; 2]>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PlantTree {
    /// The project id.
    pub id: String,
    /// Where the new tree stands, in longitude and latitude, inside the site.
    pub at: Option<LonLatInput>,
    /// The same point in RD New metres, `[x, y]`, instead of `at`.
    pub rd: Option<[f64; 2]>,
}
