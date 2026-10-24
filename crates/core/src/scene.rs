//! A fetched site: every feature in one projected, metric frame.
//!
//! Coordinates are metres in `crs`, minus `origin`, so numbers stay small and
//! y points north. Adding `origin` back gives real RD New coordinates, which is
//! what a DXF needs to land in the right place in a Dutch drawing set.

use std::collections::BTreeMap;

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
    /// The walkable neighbourhood, which reaches past the boundary. `None` for
    /// sites fetched before it existed, or when OpenStreetMap could not be reached.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub walk: Option<Walk>,
    /// The year the data of each dated layer describes, from the edition its
    /// source served when the site was fetched: `{"road_noise": 2022}`. Kept,
    /// so a saved project goes on naming its years after a newer edition
    /// appears. Scenes from before this field get the years then pinned in code.
    #[serde(default = "crate::site::years_before_editions_were_kept")]
    pub editions: BTreeMap<Layer, i32>,
    /// Buildings, roads, water and green around the site, outside the
    /// boundary, drawn muted so the site reads in its neighbourhood. Never
    /// counted in ratios, legends or analysis. Empty for scenes fetched before it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<Feature>,
}

impl Scene {
    /// The year of the data `layer` shows, when the scene knows it.
    pub fn year_of(&self, layer: Layer) -> Option<i32> {
        self.editions.get(&layer).copied()
    }
}

/// Everything within a 15 minute walk of the site centre, in the scene's local
/// frame. Unlike `Scene::features` it is not clipped to the boundary, because
/// a walk does not stop at the edge of the drawn area.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Walk {
    /// One area per band (5, 10 and 15 minutes), each holding the ones before it.
    pub bands: Vec<Feature>,
    /// The streets and paths the walk was computed on, for context outside the site.
    pub paths: Vec<Feature>,
    /// Business, facility and transit points within reach, each with its walk time.
    pub places: Vec<Feature>,
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
    /// A place's or a stop's own name: "Westermarkt".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The group a point is classified by, such as "food_drink" or "tram".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// The transit lines that call at a stop: "13", "17", "N80".
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<String>,
    /// Minutes on foot from the site centre; for a walk band, the band's limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub walk_min: Option<f64>,
    /// How many bikes or cars a parking place holds, where it is mapped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credit {
    pub name: String,
    pub license: String,
    pub url: String,
    /// The layers this source provides. A credit with layers is only printed on
    /// sheets that draw one of them; an empty list is printed on every sheet.
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
    /// Shops, food and drink, offices, hotels and services, as points.
    Business,
    /// Schools, culture, civic, health, leisure and parking, as points.
    Facility,
    /// Bus, tram, metro, train and ferry stops, as points.
    TransitStop,
    /// Streets and paths of the walk network, which reach past the site.
    WalkPath,
    /// Areas within 5, 10 and 15 minutes on foot.
    WalkBand,
}

impl Layer {
    /// Every layer, bottom to top in the default drawing order.
    pub const ALL: [Layer; 31] = [
        // Measured surfaces lie under the map, as the ground they describe.
        Layer::Population,
        Layer::Slope,
        Layer::Flood,
        Layer::Pm25,
        Layer::Pm10,
        Layer::No2,
        Layer::AllNoise,
        Layer::RoadNoise,
        Layer::WalkBand,
        Layer::WalkPath,
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
        Layer::Facility,
        Layer::Business,
        Layer::TransitStop,
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
            Layer::Business => "Businesses",
            Layer::Facility => "Facilities",
            Layer::TransitStop => "Transit stops",
            Layer::WalkPath => "Walking routes",
            Layer::WalkBand => "Walk times",
        }
    }
}
