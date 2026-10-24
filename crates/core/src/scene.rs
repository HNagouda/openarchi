//! A fetched site: every feature in one projected, metric frame.
//!
//! Coordinates are metres in `crs`, minus `origin`, so numbers stay small and
//! y points north. Adding `origin` back gives real RD New coordinates, which is
//! what a DXF needs to land in the right place in a Dutch drawing set.

use geo::{Geometry, Polygon};
use serde::{Deserialize, Serialize};

use crate::area::Area;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub area: Area,
    /// "EPSG:28992" (RD New) for the Netherlands.
    pub crs: String,
    /// Projected coordinates of the site centre. Every feature is relative to it.
    pub origin: (f64, f64),
    /// The drawn area in the local frame. Features are already clipped to it.
    pub boundary: Polygon<f64>,
    pub features: Vec<Feature>,
    pub credits: Vec<Credit>,
    /// RFC 3339 timestamp of the fetch.
    pub fetched_at: String,
    /// Sources that failed. The scene is still usable without them.
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub layer: Layer,
    pub geometry: Geometry<f64>,
    pub attributes: Attributes,
}

/// What we know about a feature beyond its shape. Unknown stays `None`, so a
/// diagram can show it as "unknown" instead of guessing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Attributes {
    /// Identifier in the source, for example a BAG pand id.
    pub source_id: Option<String>,
    /// The source's own subtype, untranslated: "voetpad", "groenvoorziening".
    pub kind: Option<String>,
    pub year_built: Option<i32>,
    /// BAG gebruiksdoel, for example "woonfunctie". Mixed use is joined with ", ".
    pub use_class: Option<String>,
    /// Roof height above the surrounding ground.
    pub height_m: Option<f64>,
    /// Ground level above NAP.
    pub ground_m: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credit {
    pub name: String,
    pub license: String,
    pub url: String,
}

/// Every layer a scene can hold, in one place. Themes and diagrams refer to
/// these names, so renaming one breaks saved themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    Building,
    /// Carriageways, driveways and bus lanes.
    Road,
    Footpath,
    Cycleway,
    Parking,
    /// Track beds drawn as areas.
    RailArea,
    /// Track centre lines.
    RailTrack,
    Water,
    /// Banks and shores along water.
    WaterBank,
    Green,
    /// Verges and planted strips along roads.
    Verge,
    /// Private yards and gardens without a planted classification.
    Yard,
    Paved,
    Unpaved,
    Bridge,
    /// Single trees, as points.
    Tree,
    /// Cadastral parcels, as outlines.
    Parcel,
    /// Terrain contour lines.
    Contour,
}

impl Layer {
    /// Every layer, bottom to top in the default drawing order.
    pub const ALL: [Layer; 18] = [
        Layer::Unpaved,
        Layer::Paved,
        Layer::Yard,
        Layer::Green,
        Layer::Verge,
        Layer::WaterBank,
        Layer::Water,
        Layer::RailArea,
        Layer::Road,
        Layer::Parking,
        Layer::Cycleway,
        Layer::Footpath,
        Layer::Bridge,
        Layer::RailTrack,
        Layer::Contour,
        Layer::Parcel,
        Layer::Tree,
        Layer::Building,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Layer::Building => "Buildings",
            Layer::Road => "Roads",
            Layer::Footpath => "Footpaths",
            Layer::Cycleway => "Cycleways",
            Layer::Parking => "Parking",
            Layer::RailArea => "Rail beds",
            Layer::RailTrack => "Tracks",
            Layer::Water => "Water",
            Layer::WaterBank => "Banks",
            Layer::Green => "Green",
            Layer::Verge => "Verges",
            Layer::Yard => "Yards",
            Layer::Paved => "Paved",
            Layer::Unpaved => "Unpaved",
            Layer::Bridge => "Bridges",
            Layer::Tree => "Trees",
            Layer::Parcel => "Parcels",
            Layer::Contour => "Contours",
        }
    }
}
