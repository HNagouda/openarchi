//! The site in a few numbers, from data already in the scene.
//!
//! Area, coverage, use and the ground shares come from `ratios::compute`, so
//! the summary and the Ratios tab can never disagree. Only what ratios does
//! not work out (mean and tallest height, median year) is computed here.

use serde::Serialize;

use crate::ratios::{Group, Ratios};
use crate::scene::{Layer, Scene};
use crate::walk::{self, GroupCount};

/// The ratios groups and rows the summary reads. These are the labels
/// `ratios.rs` gives them; a test fails if they change there.
const LAND_USE_GROUP: &str = "land_use";
const BUILDING_USE_GROUP: &str = "building_use";
const GREEN_ROWS: [&str; 1] = ["Green"];
const WATER_ROWS: [&str; 1] = ["Water"];
const PAVED_ROWS: [&str; 2] = ["Streets and paths", "Paved open space"];
const UNKNOWN_ROW: &str = "Unknown";

#[derive(Debug, Clone, Serialize)]
pub struct SiteSummary {
    pub site_area_m2: f64,
    pub building_count: usize,
    /// Building footprint over site area, 0 to 1.
    pub coverage: f64,
    pub floor_area_ratio: f64,
    /// Over buildings with a measured height.
    pub mean_height_m: Option<f64>,
    pub tallest_m: Option<f64>,
    pub median_year_built: Option<i32>,
    /// The most common building use and its share of all buildings.
    pub dominant_use: Option<UseShare>,
    /// Shares of the mapped ground, 0 to 1.
    pub green_share: f64,
    pub water_share: f64,
    /// Streets, paths and paved squares.
    pub paved_share: f64,
    pub surroundings: Surroundings,
}

#[derive(Debug, Clone, Serialize)]
pub struct UseShare {
    pub label: String,
    pub share: f64,
}

/// Numbers about the site's surroundings that other sources provide. Each
/// stays `None` until its source is in the scene, and the summary and the
/// design notes simply leave it out.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Surroundings {
    /// Road traffic noise, Lden in dB, mean over the site (RIVM).
    pub noise_lden_db: Option<f64>,
    /// Annual mean NO2 in µg/m³, mean over the site (RIVM).
    pub no2_ug_m3: Option<f64>,
    /// Maximum water depth in metres in the most likely flood scenario that reaches the site (LIWO).
    pub flood_depth_m: Option<f64>,
    /// Residents in the CBS 100 m cells the site touches.
    pub population: Option<u32>,
    /// Dwellings in the same cells.
    pub dwellings: Option<u32>,
    /// Facilities (schools, health, culture, sport) within a `FACILITY_WALK_MIN` walk (OpenStreetMap).
    pub facilities_within_walk: Option<usize>,
    /// Facility groups with nothing within that walk: "Education", "Health".
    pub facility_groups_missing: Vec<String>,
    /// Shops, food and drink and other businesses within the same walk.
    pub businesses_within_walk: Option<usize>,
    /// Transit stops within a `TRANSIT_WALK_MIN` walk.
    pub transit_stops_within_walk: Option<usize>,
}

/// The walk the facility counts are for: the 15 minute city (Moreno et al.,
/// "Introducing the 15-Minute City", Smart Cities 4, 2021).
pub const FACILITY_WALK_MIN: u32 = 15;
/// About 400 m, the walk to a stop that transit planning commonly takes as
/// close (Walker, "Human Transit", 2011). One of the walk's own bands.
pub const TRANSIT_WALK_MIN: u32 = 5;

pub fn summarise(scene: &Scene, ratios: &Ratios) -> SiteSummary {
    let heights: Vec<f64> = buildings(scene)
        .filter_map(|building| building.attributes.height_m)
        .collect();
    let mut years: Vec<i32> = buildings(scene)
        .filter_map(|building| building.attributes.year_built)
        .collect();
    years.sort_unstable();

    SiteSummary {
        site_area_m2: ratios.site_area_m2,
        building_count: ratios.building_count,
        coverage: ratios.coverage,
        floor_area_ratio: ratios.floor_area_ratio,
        mean_height_m: (!heights.is_empty())
            .then(|| heights.iter().sum::<f64>() / heights.len() as f64),
        tallest_m: heights.iter().copied().reduce(f64::max),
        median_year_built: years.get(years.len() / 2).copied(),
        dominant_use: dominant_use(ratios),
        green_share: land_share(ratios, &GREEN_ROWS),
        water_share: land_share(ratios, &WATER_ROWS),
        paved_share: land_share(ratios, &PAVED_ROWS),
        surroundings: surroundings(scene),
    }
}

/// The land use group, for the ground cover figure.
pub fn land_use(ratios: &Ratios) -> Option<&Group> {
    group(ratios, LAND_USE_GROUP)
}

// #! TODO[analysis, integration] : Fill noise, NO2, flood, population and dwellings from the environment layers once they are merged
// The environment branch should give `environment::site_numbers(scene) -> EnvironmentNumbers`
// with noise_lden_db, no2_ug_m3, flood_depth_m, population and dwellings as Option fields.
fn surroundings(scene: &Scene) -> Surroundings {
    let Some(neighbourhood) = walk::neighbourhood(scene) else {
        return Surroundings::default();
    };
    let band = |minutes: u32| {
        neighbourhood
            .bands
            .iter()
            .find(|band| band.minutes == minutes)
    };
    let total = |groups: &[GroupCount]| groups.iter().map(|group| group.count).sum::<usize>();
    let facilities = band(FACILITY_WALK_MIN);

    Surroundings {
        facilities_within_walk: facilities.map(|band| total(&band.facilities)),
        facility_groups_missing: facilities
            .map(|band| {
                band.facilities
                    .iter()
                    .filter(|group| group.count == 0)
                    .map(|group| group.label.clone())
                    .collect()
            })
            .unwrap_or_default(),
        businesses_within_walk: facilities.map(|band| total(&band.businesses)),
        transit_stops_within_walk: band(TRANSIT_WALK_MIN).map(|band| total(&band.transit_stops)),
        ..Surroundings::default()
    }
}

fn buildings(scene: &Scene) -> impl Iterator<Item = &crate::scene::Feature> {
    scene
        .features
        .iter()
        .filter(|feature| feature.layer == Layer::Building)
}

fn group<'a>(ratios: &'a Ratios, id: &str) -> Option<&'a Group> {
    ratios.groups.iter().find(|group| group.id == id)
}

fn land_share(ratios: &Ratios, labels: &[&str]) -> f64 {
    let Some(land_use) = group(ratios, LAND_USE_GROUP) else {
        return 0.0;
    };
    land_use
        .rows
        .iter()
        .filter(|row| labels.contains(&row.label.as_str()))
        .map(|row| row.share)
        .sum()
}

fn dominant_use(ratios: &Ratios) -> Option<UseShare> {
    let uses = group(ratios, BUILDING_USE_GROUP)?;
    uses.rows
        .iter()
        .filter(|row| row.label != UNKNOWN_ROW)
        .max_by(|a, b| a.value.total_cmp(&b.value))
        .map(|row| UseShare {
            label: row.label.clone(),
            share: row.share,
        })
}
