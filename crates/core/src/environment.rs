//! What the environment of a site is like, in a handful of numbers: noise,
//! air, flood depth, slope, people and dwellings.
//!
//! The raster figures are measured when the site is fetched, over the full
//! rasters, because the bands drawn on the sheet keep only which band a place
//! falls in. The rasters are kept, cut to the site's box, so a sheet area can
//! be measured again over its own cells. People and dwellings are counted
//! from the scene's features.

use std::collections::BTreeMap;

use geo::{Area, Polygon};
use serde::{Deserialize, Serialize};

use crate::scene::{Layer, Scene};
use crate::sources::cbs::CELL_AREA_M2;
use crate::sources::raster::{self, Grid, Stats};

/// Every layer with a figure in `Measured`, in one place.
const MEASURED_LAYERS: [Layer; 10] = [
    Layer::RoadNoise,
    Layer::AllNoise,
    Layer::No2,
    Layer::Pm10,
    Layer::Pm25,
    Layer::Flood,
    Layer::Slope,
    Layer::TreeCanopy,
    Layer::VegetationIndex,
    Layer::HeatIsland,
];

/// Raster figures over the cells inside the site. `None` when the source
/// failed or had no cells there. For flood depth only flooded cells have a
/// value, so its `coverage` is the share of the site that can flood.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Measured {
    pub road_noise_db: Option<Stats>,
    pub all_noise_db: Option<Stats>,
    pub no2_ug_m3: Option<Stats>,
    pub pm10_ug_m3: Option<Stats>,
    pub pm25_ug_m3: Option<Stats>,
    pub flood_depth_m: Option<Stats>,
    pub slope_percent: Option<Stats>,
    /// Tree height in metres; its `coverage` is the share of the site under
    /// tree canopy. `None` in scenes fetched before trees were read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree_height_m: Option<Stats>,
    /// The approximate vegetation index, -1 to 1, see `sources/luchtfoto.rs`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vegetation_index: Option<Stats>,
    /// The urban heat island effect in °C.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heat_island_c: Option<Stats>,
    /// The raster each figure was measured on, cut to the site's box, so a
    /// sheet area can be measured again: `{"road_noise": grid}`. The tree
    /// heights reach further, for the trees around the site that block sight,
    /// sky and sun (`forest.rs`, `site::greenery::TREE_REACH_M`). Only for
    /// figures that found a value. Slope is never kept, see `site::environment`.
    /// Empty in scenes fetched before sheet areas measured their own figures.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub grids: BTreeMap<Layer, Grid>,
    /// On a sheet area, the figures that still describe the whole site
    /// because no raster was kept to measure them again. Empty for a whole site.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub whole_site: Vec<Layer>,
}

impl Measured {
    /// The figure measured on `layer`'s raster, `None` for a layer without one.
    pub fn get(&self, layer: Layer) -> Option<Stats> {
        match layer {
            Layer::RoadNoise => self.road_noise_db,
            Layer::AllNoise => self.all_noise_db,
            Layer::No2 => self.no2_ug_m3,
            Layer::Pm10 => self.pm10_ug_m3,
            Layer::Pm25 => self.pm25_ug_m3,
            Layer::Flood => self.flood_depth_m,
            Layer::Slope => self.slope_percent,
            Layer::TreeCanopy => self.tree_height_m,
            Layer::VegetationIndex => self.vegetation_index,
            Layer::HeatIsland => self.heat_island_c,
            _ => None,
        }
    }

    /// Keeps `stats` in the field the summary reads `layer` from.
    pub fn set(&mut self, layer: Layer, stats: Option<Stats>) {
        match layer {
            Layer::RoadNoise => self.road_noise_db = stats,
            Layer::AllNoise => self.all_noise_db = stats,
            Layer::No2 => self.no2_ug_m3 = stats,
            Layer::Pm10 => self.pm10_ug_m3 = stats,
            Layer::Pm25 => self.pm25_ug_m3 = stats,
            Layer::Flood => self.flood_depth_m = stats,
            Layer::Slope => self.slope_percent = stats,
            Layer::TreeCanopy => self.tree_height_m = stats,
            Layer::VegetationIndex => self.vegetation_index = stats,
            Layer::HeatIsland => self.heat_island_c = stats,
            _ => {}
        }
    }
}

/// `measured` again over `area`, a sheet area inside the site, from the kept
/// rasters. A figure without a kept raster stays as it was and is listed in
/// `whole_site`. One that found no value over the whole site finds none in a
/// part of it either, so it stays `None` and is exact.
pub fn measured_within(measured: &Measured, area: &Polygon<f64>) -> Measured {
    let mut framed = measured.clone();
    for layer in MEASURED_LAYERS {
        match measured.grids.get(&layer) {
            Some(grid) => framed.set(layer, raster::stats_within(grid, area)),
            None if measured.get(layer).is_some() => framed.whole_site.push(layer),
            None => {}
        }
    }

    framed
}

/// One site in numbers, for a summary page. Every figure is `None` when its
/// source is missing from the scene.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Summary {
    pub road_noise_mean_db: Option<f64>,
    pub road_noise_max_db: Option<f64>,
    pub all_noise_mean_db: Option<f64>,
    /// The year each figure describes, from the edition the scene was fetched
    /// in; `None` when the scene does not say.
    pub noise_year: Option<i32>,
    pub no2_mean_ug_m3: Option<f64>,
    pub pm10_mean_ug_m3: Option<f64>,
    pub pm25_mean_ug_m3: Option<f64>,
    pub air_year: Option<i32>,
    /// The deepest water in the extremely-small-chance flood scenario, 0 when the site stays dry.
    pub flood_max_depth_m: Option<f64>,
    /// 0 to 1: the share of the site that water can reach in that scenario.
    pub flood_share: Option<f64>,
    pub flood_year: Option<i32>,
    pub slope_max_percent: Option<f64>,
    /// Residents, with each CBS cell counted by the share of it inside the site.
    pub residents: Option<f64>,
    pub households: Option<f64>,
    /// CBS cells kept secret because too few people live there.
    pub suppressed_cells: usize,
    pub population_year: Option<i32>,
    /// BAG dwellings in the buildings on the site.
    pub dwellings: Option<u32>,
    /// BAG floor area of every unit on the site, in m².
    pub floor_area_m2: Option<f64>,
    /// 0 to 1: the share of the site under tree canopy, from RIVM's tree map.
    pub tree_canopy_share: Option<f64>,
    pub tree_year: Option<i32>,
    /// The mean approximate vegetation index (NDVI) over the site, -1 to 1.
    pub vegetation_index_mean: Option<f64>,
    pub vegetation_year: Option<i32>,
    /// The mean urban heat island effect over the site in °C, summer air
    /// temperature above the countryside around.
    pub heat_island_mean_c: Option<f64>,
    pub heat_island_year: Option<i32>,
    /// On a sheet area, the figures above that describe the whole site
    /// instead: `["slope"]`. See `Measured::whole_site`.
    pub whole_site: Vec<Layer>,
}

/// The summary of `scene`.
pub fn summarise(scene: &Scene) -> Summary {
    let measured = &scene.measured;
    let mean = |stats: &Option<Stats>| stats.map(|stats| stats.mean);
    let population = population_in(scene);
    // A scene that fetched LIWO but met no water has no stats; it is dry, not unknown.
    let has_flood_data = scene
        .credits
        .iter()
        .any(|credit| credit.layers.contains(&Layer::Flood));
    let (flood_max_depth_m, flood_share) = match measured.flood_depth_m {
        Some(stats) => (Some(stats.max), Some(stats.coverage)),
        None if has_flood_data => (Some(0.0), Some(0.0)),
        None => (None, None),
    };

    Summary {
        road_noise_mean_db: mean(&measured.road_noise_db),
        road_noise_max_db: measured.road_noise_db.map(|stats| stats.max),
        all_noise_mean_db: mean(&measured.all_noise_db),
        noise_year: scene.year_of(Layer::RoadNoise),
        no2_mean_ug_m3: mean(&measured.no2_ug_m3),
        pm10_mean_ug_m3: mean(&measured.pm10_ug_m3),
        pm25_mean_ug_m3: mean(&measured.pm25_ug_m3),
        air_year: scene.year_of(Layer::No2),
        flood_max_depth_m,
        flood_share,
        flood_year: scene.year_of(Layer::Flood),
        slope_max_percent: measured.slope_percent.map(|stats| stats.max),
        residents: population.residents,
        households: population.households,
        suppressed_cells: population.suppressed_cells,
        population_year: scene.year_of(Layer::Population),
        dwellings: dwellings_in(scene),
        floor_area_m2: floor_area_in(scene),
        tree_canopy_share: tree_canopy_share(scene),
        tree_year: scene.year_of(Layer::TreeCanopy),
        vegetation_index_mean: mean(&measured.vegetation_index),
        vegetation_year: scene.year_of(Layer::VegetationIndex),
        heat_island_mean_c: mean(&measured.heat_island_c),
        heat_island_year: scene.year_of(Layer::HeatIsland),
        whole_site: measured.whole_site.clone(),
    }
}

/// The share of the site under trees. A scene that read the tree map but met
/// no tree has no figure; it has no canopy, which is not unknown.
fn tree_canopy_share(scene: &Scene) -> Option<f64> {
    let has_tree_data = scene
        .credits
        .iter()
        .any(|credit| credit.layers.contains(&Layer::TreeCanopy));
    match scene.measured.tree_height_m {
        Some(stats) => Some(stats.coverage),
        None if has_tree_data => Some(0.0),
        None => None,
    }
}

/// People in the site from the CBS cells, each weighted by its share inside the site.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Population {
    pub residents: Option<f64>,
    pub households: Option<f64>,
    pub suppressed_cells: usize,
}

pub fn population_in(scene: &Scene) -> Population {
    let mut population = Population::default();
    for cell in scene
        .features
        .iter()
        .filter(|feature| feature.layer == Layer::Population)
    {
        let share = cell.geometry.unsigned_area() / CELL_AREA_M2;
        match cell.attributes.value {
            Some(residents) => {
                population.residents =
                    Some(population.residents.unwrap_or(0.0) + residents * share);
            }
            None => population.suppressed_cells += 1,
        }
        if let Some(households) = cell.attributes.households {
            population.households = Some(population.households.unwrap_or(0.0) + households * share);
        }
    }

    population
}

/// `None` when no building knows its dwellings, which means BAG units were not fetched.
fn dwellings_in(scene: &Scene) -> Option<u32> {
    let mut total = None;
    for building in buildings(scene) {
        if let Some(dwellings) = building.attributes.dwellings {
            total = Some(total.unwrap_or(0) + dwellings);
        }
    }

    total
}

fn floor_area_in(scene: &Scene) -> Option<f64> {
    let mut total = None;
    for building in buildings(scene) {
        if let Some(area) = building.attributes.floor_area_m2 {
            total = Some(total.unwrap_or(0.0) + area);
        }
    }

    total
}

fn buildings(scene: &Scene) -> impl Iterator<Item = &crate::scene::Feature> {
    scene
        .features
        .iter()
        .filter(|feature| feature.layer == Layer::Building)
}
