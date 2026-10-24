//! The line under the location and site headings that says where their facts
//! come from. Climate and rain write theirs next to their data in `sections.rs`,
//! because their lines name the weather years and the station. The site's
//! caption and area label live here too, since they change with the source
//! line when a sheet's sub-selection is summed up instead of the whole site.

use super::summary::SiteSummary;
use crate::attribution::{dated, short_name};
use crate::scene::Layer;
use crate::sources::{ahn, bag, bgt, boundaries, cbs, liwo, osm, rivm};

/// The address service has no credit of its own: PDOK serves it alongside BAG.
const ADDRESS_SERVICE: &str = "the PDOK Locatieserver";

/// Opens the site line on a sheet with a sub-selection, so the reader knows
/// which figures moved with the area and which did not.
const SHEET_AREA_SCOPE: &str =
    "Of this sheet's area only; location, climate and rain are of the whole site.";

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

    let mut line = String::new();
    if summary.is_sheet_area {
        line.push_str(SHEET_AREA_SCOPE);
        line.push(' ');
    }
    line.push_str(&format!("{}.", parts.join(", ")));
    line.push_str(&whole_site_figures(&environment.whole_site));
    line
}

/// The site figure's caption: what its ground cover is of.
pub fn site_caption(summary: &SiteSummary) -> String {
    let place = if summary.is_sheet_area {
        "this sheet's area"
    } else {
        "the site"
    };
    format!("Ground cover inside {place}, as in the Ratios tab. Paved is streets, paths and paved squares.")
}

/// "Sheet area" on a sheet with a sub-selection, "Site area" otherwise.
pub fn area_label(summary: &SiteSummary) -> &'static str {
    if summary.is_sheet_area {
        "Sheet area"
    } else {
        "Site area"
    }
}

/// On a sheet area, which figures still describe the whole site, and why.
fn whole_site_figures(whole_site: &[Layer]) -> String {
    let mut text = String::new();
    if whole_site.iter().any(|layer| *layer != Layer::Slope) {
        text.push_str(" Noise, air and flood figures are of the whole site, because it was fetched before sheet areas measured their own; fetch it again to measure this area.");
    }
    if whole_site.contains(&Layer::Slope) {
        text.push_str(" The steepest slope is of the whole site.");
    }

    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_sheet_area_names_only_the_slope_as_whole_site() {
        let line = whole_site_figures(&[Layer::Slope]);
        assert!(line.contains("slope"), "{line}");
        assert!(!line.contains("fetch it again"), "{line}");
    }

    #[test]
    fn an_old_sheet_area_says_to_fetch_again() {
        let line = whole_site_figures(&[Layer::RoadNoise, Layer::Slope]);
        assert!(line.contains("fetch it again"), "{line}");
        assert!(whole_site_figures(&[]).is_empty());
    }
}
