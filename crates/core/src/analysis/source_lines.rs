//! The line under the location and site headings that says where their facts
//! come from. Climate and rain write theirs next to their data in `sections.rs`,
//! because their lines name the weather years and the station.

use super::summary::SiteSummary;
use crate::attribution::{dated, short_name};
use crate::sources::{ahn, bag, bgt, boundaries, cbs, liwo, osm, rivm};

/// The address service has no credit of its own: PDOK serves it alongside BAG.
const ADDRESS_SERVICE: &str = "the PDOK Locatieserver";

pub fn location() -> String {
    format!(
        "Nearest registered address from {ADDRESS_SERVICE}; municipality and province from {}, the country outline from {}.",
        short_name(&boundaries::credit()),
        short_name(&boundaries::country_credit())
    )
}

/// Every source behind a site fact that is there, with its year when the scene knows it.
pub fn site(summary: &SiteSummary) -> String {
    let environment = &summary.surroundings.environment;
    let mut parts = vec![format!(
        "Buildings from {}, ground from {}, heights from {}",
        short_name(&bag::credit()),
        short_name(&bgt::credit()),
        short_name(&ahn::credit())
    )];
    if environment.road_noise_mean_db.is_some() {
        parts.push(format!(
            "road noise from {}",
            dated(&rivm::credit(), environment.noise_year)
        ));
    }
    if environment.no2_mean_ug_m3.is_some() {
        parts.push(format!(
            "NO₂ from {}",
            dated(&rivm::credit(), environment.air_year)
        ));
    }
    if environment.flood_max_depth_m.is_some() {
        parts.push(format!(
            "flood depth from {}",
            dated(&liwo::credit(), environment.flood_year)
        ));
    }
    if environment.residents.is_some() {
        parts.push(format!(
            "residents from {}",
            dated(&cbs::credit(), environment.population_year)
        ));
    }
    let has_walk = summary.surroundings.facilities_within_walk.is_some()
        || summary.surroundings.transit_stops_within_walk.is_some();
    if has_walk {
        parts.push(format!(
            "places and stops from {}",
            short_name(&osm::credit())
        ));
    }

    format!("{}.", parts.join(", "))
}
