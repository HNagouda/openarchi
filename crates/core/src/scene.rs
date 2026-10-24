//! A fetched site: every feature in one projected, metric frame.
//!
//! Coordinates are metres in `crs`, minus `origin`, so numbers stay small and
//! y points north. Adding `origin` back gives real RD New coordinates, which is
//! what a DXF needs to land in the right place in a Dutch drawing set.

use geo::{Geometry, Polygon};
use serde::{Deserialize, Serialize};

use crate::area::Area;
use crate::environment::Measured;

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
    /// Noise, air, flood and slope inside the site, from the full rasters
    /// before they were turned into bands. Empty in scenes fetched before 0.2.
    #[serde(default)]
    pub measured: Measured,
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
    /// The lower edge of a band (dB, µg/m³, metres of water, percent slope),
    /// or the residents of a CBS statistics cell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// BAG dwellings (units with a residential use) in a building, or the
    /// dwellings CBS counts in a statistics cell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dwellings: Option<u32>,
    /// The BAG floor area of every unit in a building, in m². BAG records
    /// usable floor area (NEN 2580), which is smaller than gross floor area.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor_area_m2: Option<f64>,
    /// Private households in a CBS statistics cell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub households: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credit {
    pub name: String,
    pub license: String,
    pub url: String,
    /// The layers this source draws. The sheet credits it only when the
    /// diagram shows one of them; empty means always.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<Layer>,
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
    /// Road traffic noise bands, Lden.
    RoadNoise,
    /// Noise bands from every source together, Lden.
    AllNoise,
    /// Annual mean nitrogen dioxide bands.
    No2,
    /// Annual mean particulate matter bands, PM10 and PM2.5.
    Pm10,
    Pm25,
    /// Maximum flood depth bands.
    Flood,
    /// Terrain slope bands.
    Slope,
    /// CBS 100 m statistics cells, with residents per cell.
    Population,
}

impl Layer {
    /// Every layer, bottom to top in the default drawing order.
    pub const ALL: [Layer; 26] = [
        // Measured surfaces lie under the map, as the ground they describe.
        Layer::Population,
        Layer::Slope,
        Layer::Flood,
        Layer::Pm25,
        Layer::Pm10,
        Layer::No2,
        Layer::AllNoise,
        Layer::RoadNoise,
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
            Layer::RoadNoise => "Road traffic noise",
            Layer::AllNoise => "Noise, all sources",
            Layer::No2 => "Nitrogen dioxide",
            Layer::Pm10 => "PM10",
            Layer::Pm25 => "PM2.5",
            Layer::Flood => "Flood depth",
            Layer::Slope => "Slope",
            Layer::Population => "Population",
        }
    }
}
