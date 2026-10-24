//! What the environment of a site is like, in a handful of numbers: noise,
//! air, flood depth, slope, people and dwellings.
//!
//! The raster figures are measured when the site is fetched, over the full
//! rasters, because the bands drawn on the sheet keep only which band a place
//! falls in. People and dwellings are counted from the scene's features.

use geo::Area;
use serde::{Deserialize, Serialize};

use crate::scene::{Layer, Scene};
use crate::sources::cbs::CELL_AREA_M2;
use crate::sources::raster::Stats;

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
