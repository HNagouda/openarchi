//! Design notes: short sentences under each section, written from fixed rules.
//!
//! Every rule reads numbers the section already shows and says what they mean
//! for a design. Every threshold is a named constant with where it comes from;
//! the ones that are rules of thumb say so. Nothing here is generated.

use super::climate::{Climate, COMFORTABLE_BAND, COMFORT_EDGES_C, SEASONS, WINTER_HALF};
use super::figure::percent;
use super::rain::Rain;
use super::summary::{SiteSummary, FACILITY_WALK_MIN, TRANSIT_WALK_MIN};

/// Closed urban blocks have a ground space index of about 0.4 to 0.6, open
/// row housing and slabs 0.15 to 0.3 (Berghauser Pont and Haupt, "Spacematrix", 2010).
const DENSE_COVERAGE: f64 = 0.4;
const OPEN_COVERAGE: f64 = 0.15;
/// The 3-30-300 guideline asks for 30 % tree canopy in every neighbourhood
/// (Konijnendijk, J. For. Res. 34, 2023). Ground green is not canopy, so the
/// note only says how far the ground alone is from it.
const CANOPY_TARGET: f64 = 0.3;
/// A rule of thumb used here: above half the ground paved, run-off and summer heat dominate.
const HIGH_PAVED: f64 = 0.5;
/// A rule of thumb used here: from a tenth of the site, water shapes views and summer evenings.
const NOTABLE_WATER: f64 = 0.1;
/// Before 1945 most Dutch towns were built as closed street fronts; a fabric
/// whose median building is older is read as historic.
const HISTORIC_YEAR: i32 = 1945;
/// A rule of thumb used here: a building twice the mean height reads as a landmark.
const LANDMARK_RATIO: f64 = 2.0;
/// WHO Environmental Noise Guidelines for the European Region (2018): road traffic below 53 dB Lden.
const WHO_ROAD_NOISE_DB: f64 = 53.0;
/// WHO Global Air Quality Guidelines (2021): annual mean NO2 at most 10 µg/m³.
const WHO_NO2_UG_M3: f64 = 10.0;

/// A month is a comfortable month when at least this share of its daytime
/// hours is in the comfortable band. Half is a rule of thumb used here.
const COMFORTABLE_MONTH_SHARE: f64 = 0.5;
/// Mean daily minimum below which nights freeze often enough to detail for frost.
const FROST_C: f64 = 0.0;
/// A rule of thumb used here: summer nights at or below this can still cool
/// a heavy building through open windows.
const NIGHT_COOLING_C: f64 = 16.0;
/// NEN 8100 (wind nuisance in the built environment) counts hours above
/// 5 m/s at pedestrian height; Beaufort 4 at 10 m, 5.5 m/s, is the band that
/// reaches it at street corners.
const NUISANCE_WIND_BAND: usize = 2;
/// A rule of thumb used here: a quarter of winter hours that windy is worth designing against.
const WINDY_SHARE: f64 = 0.25;
/// Earth's axial tilt, for the sun's noon height at the solstices.
const AXIAL_TILT_DEG: f64 = 23.44;
/// The building height the shadow examples use, about three storeys.
const EXAMPLE_HEIGHT_M: f64 = 10.0;
/// Monthly mean outdoor humidity from which walls and timber dry slowly.
/// IEA Annex 14 takes 80 % on a surface as the mould limit; damp outdoor air
/// keeps cold surfaces near it. A rule of thumb used here.
const DAMP_HUMIDITY_PCT: f64 = 85.0;

const DAYS_PER_YEAR: f64 = 365.25;

const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

pub fn site_notes(summary: &SiteSummary) -> Vec<String> {
    let mut notes = Vec::new();
    let coverage = percent(summary.coverage);
    if summary.coverage >= DENSE_COVERAGE {
        notes.push(format!("Buildings cover {coverage} of the site, a tight urban fabric. Outdoor space will be courtyards, roofs and the street, so each one has to work hard."));
    } else if summary.building_count > 0 && summary.coverage < OPEN_COVERAGE {
        notes.push(format!("Buildings cover only {coverage} of the site. They stand in open space rather than along streets, so a new building has to make its own edges."));
    }
    if let Some(year) = summary
        .median_year_built
        .filter(|year| *year < HISTORIC_YEAR)
    {
        notes.push(format!("Half the buildings were built in {year} or earlier. New work here is measured against an existing street front: its line, height and rhythm."));
    }
    if let (Some(mean), Some(tallest)) = (summary.mean_height_m, summary.tallest_m) {
        if tallest >= mean * LANDMARK_RATIO {
            notes.push(format!("The tallest building, {tallest:.0} m, is more than twice the mean height of {mean:.0} m. It is the landmark the rest of the site is read against."));
        }
    }
    if summary.paved_share >= HIGH_PAVED {
        notes.push(format!("{} of the ground is paved. Rain runs straight off and the stone holds summer heat into the night; unsealed ground, trees and green roofs help with both.", capitalised(&percent(summary.paved_share))));
    }
    if summary.green_share < CANOPY_TARGET {
        notes.push(format!("Green covers {} of the ground. The 3-30-300 guideline asks for 30 % tree canopy per neighbourhood, so trees and green roofs will have to make up the gap.", percent(summary.green_share)));
    }
    if summary.water_share >= NOTABLE_WATER {
        notes.push(format!("Water is {} of the site. It cools summer evenings and opens long views; turn living rooms and outdoor space towards it.", percent(summary.water_share)));
    }
    surroundings_notes(summary, &mut notes);

    notes
}

fn surroundings_notes(summary: &SiteSummary, notes: &mut Vec<String>) {
    let surroundings = &summary.surroundings;
    if surroundings.transit_stops_within_walk == Some(0) {
        notes.push(format!("No transit stop is within a {TRANSIT_WALK_MIN} minute walk. Expect more car and bike trips, and room to park both."));
    }
    let missing = &surroundings.facility_groups_missing;
    if surroundings.facilities_within_walk.is_some() && !missing.is_empty() {
        let names: Vec<String> = missing.iter().map(|name| name.to_lowercase()).collect();
        notes.push(format!("Within a {FACILITY_WALK_MIN} minute walk there is no {}. A project with ground floor space could bring it here.", list(&names)));
    }
    if let Some(noise) = surroundings
        .noise_lden_db
        .filter(|db| *db > WHO_ROAD_NOISE_DB)
    {
        notes.push(format!("Road noise reaches {noise:.0} dB Lden, above the 53 dB the WHO recommends. Put bedrooms and outdoor space on the quiet side of the building."));
    }
    if let Some(no2) = surroundings
        .no2_ug_m3
        .filter(|value| *value > WHO_NO2_UG_M3)
    {
        notes.push(format!("Nitrogen dioxide averages {no2:.0} µg/m³, above the WHO guideline of 10. Take fresh air in away from the busiest road."));
    }
    if let Some(depth) = surroundings.flood_depth_m.filter(|depth| *depth > 0.0) {
        notes.push(format!("In a flood the water here can reach {depth:.1} m. Keep entrances, installations and an escape route above that line."));
    }
}

pub fn temperature_notes(climate: &Climate) -> Vec<String> {
    let mut notes = Vec::new();
    let warmest = climate
        .months
        .iter()
        .max_by(|a, b| a.mean_daily_max_c.total_cmp(&b.mean_daily_max_c));
    let coldest = climate
        .months
        .iter()
        .min_by(|a, b| a.mean_daily_min_c.total_cmp(&b.mean_daily_min_c));
    if let (Some(warmest), Some(coldest)) = (warmest, coldest) {
        notes.push(format!(
            "{} is the warmest month, with days around {:.0} °C and nights around {:.0} °C. {} nights average {:.0} °C.",
            month_name(warmest.month),
            warmest.mean_daily_max_c,
            warmest.mean_daily_min_c,
            month_name(coldest.month),
            coldest.mean_daily_min_c
        ));
        if warmest.mean_daily_min_c <= NIGHT_COOLING_C {
            notes.push("Summer nights stay cool enough to flush a building with outdoor air. Heavy floors or walls that can store the cool, and windows that can stay open safely at night, keep rooms comfortable without cooling.".to_string());
        }
    }

    let frost: Vec<u32> = climate
        .months
        .iter()
        .filter(|month| month.mean_daily_min_c < FROST_C)
        .map(|month| month.month)
        .collect();
    if !frost.is_empty() {
        notes.push(format!("Nights freeze on average {}. Detail outdoor stairs, ramps and paving for frost and ice.", during(&frost)));
    }

    notes
}

pub fn comfort_notes(climate: &Climate) -> Vec<String> {
    let mut notes = Vec::new();
    let comfortable: Vec<u32> = climate
        .months
        .iter()
        .filter(|month| comfortable_share(&month.comfort_hours) >= COMFORTABLE_MONTH_SHARE)
        .map(|month| month.month)
        .collect();
    if comfortable.is_empty() {
        notes.push("No month is comfortable for more than half its daytime hours by air temperature alone. Outdoor space has to make its own climate: sun, shelter from the wind, somewhere dry.".to_string());
    } else {
        notes.push(format!("Air temperature is comfortable for most daytime hours {}. Outside that, a sunny spot out of the wind feels several degrees warmer than the chart says.", during(&comfortable)));
    }

    let total: u32 = climate
        .months
        .iter()
        .flat_map(|month| month.comfort_hours.iter())
        .sum();
    let hot: u32 = climate
        .months
        .iter()
        .map(|month| {
            month.comfort_hours[COMFORTABLE_BAND + 1..]
                .iter()
                .sum::<u32>()
        })
        .sum();
    if hot > 0 && total > 0 {
        notes.push(format!("About {hot} daytime hours a year are above {:.0} °C. Shade south and west glass from outside, and give outdoor space a shaded corner.", COMFORT_EDGES_C[2]));
    }

    notes
}

pub fn wind_notes(climate: &Climate) -> Vec<String> {
    let mut notes = Vec::new();
    for (rose, (season, ..)) in climate.wind.iter().zip(SEASONS) {
        let Some(prevailing) = rose
            .sectors
            .iter()
            .max_by(|a, b| a.total().total_cmp(&b.total()))
        else {
            continue;
        };
        let windy: f64 = rose
            .sectors
            .iter()
            .map(|sector| sector.shares[NUISANCE_WIND_BAND..].iter().sum::<f64>())
            .sum();
        match season {
            "Winter" => {
                notes.push(format!("In winter the wind comes most often from the {} ({} of hours). Keep entrances, balconies and terraces off that side, or shelter them behind a building, wall or hedge.", prevailing.from, percent(prevailing.total())));
                if windy >= WINDY_SHARE {
                    notes.push(format!("{} of winter hours bring a moderate breeze or more at 10 m. Street level is calmer, but tall flat faces turned to the {} pull that wind down to the pavement; a podium or setback breaks it.", capitalised(&percent(windy)), prevailing.from));
                }
            }
            "Summer" => notes.push(format!("In summer the wind comes most often from the {}. Openings on that side and on the opposite face let a breeze cross the rooms.", prevailing.from)),
            _ => {}
        }
    }

    notes
}

pub fn sun_notes(climate: &Climate) -> Vec<String> {
    let latitude = climate.location.lat;
    let winter_noon_deg = 90.0 - latitude - AXIAL_TILT_DEG;
    let equinox_noon_deg = 90.0 - latitude;
    let shadow = |altitude_deg: f64| EXAMPLE_HEIGHT_M / altitude_deg.to_radians().tan();
    let mut notes = vec![format!(
        "At midwinter noon the sun stands {winter_noon_deg:.0}° above the horizon, so a {EXAMPLE_HEIGHT_M:.0} m building throws a {:.0} m shadow. In March and September it stands {equinox_noon_deg:.0}° and the shadow is {:.0} m.",
        shadow(winter_noon_deg),
        shadow(equinox_noon_deg)
    )];

    let winter: f64 = climate.sun.iter().map(|sector| sector.winter_kwh_m2).sum();
    let summer: f64 = climate.sun.iter().map(|sector| sector.summer_kwh_m2).sum();
    if winter + summer > 0.0 {
        notes.push(format!("October to March brings only {} of the year's direct sun, almost all of it from the south-east to the south-west and low in the sky. Keep that side open to win it.", percent(winter / (winter + summer))));
    }
    let global: f64 = climate.months.iter().map(|month| month.global_kwh_m2).sum();
    notes.push(format!(
        "A flat roof receives about {} kWh of sun per square metre a year.",
        crate::ratios::grouped(global)
    ));

    notes
}

pub fn humidity_notes(climate: &Climate) -> Vec<String> {
    let damp: Vec<u32> = climate
        .months
        .iter()
        .filter(|month| month.humidity_pct >= DAMP_HUMIDITY_PCT)
        .map(|month| month.month)
        .collect();
    if damp.is_empty() {
        return Vec::new();
    }

    vec![format!("The air is damp {}, at {:.0} % or more on average. Ventilate well, avoid cold bridges where condensation forms, and let outside timber and render dry out.", during(&damp), DAMP_HUMIDITY_PCT)]
}

pub fn rain_notes(rain: &Rain) -> Vec<String> {
    let mut notes = vec![format!(
        "About {:.0} mm of rain falls a year, on {:.0} days with 1 mm or more: about one day in {:.0}. Covered routes from the street to the door and to the bike parking get used.",
        rain.annual_mm,
        rain.annual_rain_days,
        DAYS_PER_YEAR / rain.annual_rain_days.max(1.0)
    )];
    let wettest = rain
        .months
        .iter()
        .max_by(|a, b| a.mean_mm.total_cmp(&b.mean_mm));
    let driest = rain
        .months
        .iter()
        .min_by(|a, b| a.mean_mm.total_cmp(&b.mean_mm));
    if let (Some(wettest), Some(driest)) = (wettest, driest) {
        notes.push(format!(
            "{} is the wettest month ({:.0} mm) and {} the driest ({:.0} mm). Monthly totals hide cloudbursts, which are what gutters, drains and water storage are sized for.",
            month_name(wettest.month),
            wettest.mean_mm,
            month_name(driest.month),
            driest.mean_mm
        ));
    }

    notes
}

/// The winter half of the year, for the radiation rose's legend.
pub fn winter_half_text() -> String {
    format!(
        "{} to {}",
        &month_name(WINTER_HALF[0])[..3],
        &month_name(WINTER_HALF[WINTER_HALF.len() - 1])[..3]
    )
}

fn comfortable_share(hours: &[u32]) -> f64 {
    let total: u32 = hours.iter().sum();
    if total == 0 {
        return 0.0;
    }
    hours[COMFORTABLE_BAND] as f64 / total as f64
}

fn month_name(month: u32) -> &'static str {
    MONTH_NAMES[(month as usize + 11) % 12]
}

/// [5, 6, 7, 8, 9] -> "May to September", [12, 1, 2] -> "December to
/// February", [1, 4] -> "January and April".
fn month_span(months: &[u32]) -> String {
    let is_run = months.windows(2).all(|pair| pair[1] == pair[0] % 12 + 1);
    // A run across the new year comes in calendar order: [1, 2, 12].
    let wraps = !is_run && months.len() > 2 && months.contains(&12) && months.contains(&1);
    let ordered: Vec<u32> = if wraps {
        let start = months
            .iter()
            .position(|&month| month > months[0] + 1)
            .unwrap_or(0);
        months[start..]
            .iter()
            .chain(&months[..start])
            .copied()
            .collect()
    } else {
        months.to_vec()
    };
    let is_ordered_run = ordered.windows(2).all(|pair| pair[1] == pair[0] % 12 + 1);

    let names: Vec<&str> = ordered.iter().map(|&month| month_name(month)).collect();
    match names.as_slice() {
        [] => String::new(),
        [only] => only.to_string(),
        [first, .., last] if is_ordered_run && names.len() > 2 => format!("{first} to {last}"),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// ["a", "b", "c"] -> "a, b or c".
fn list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [only] => only.clone(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}

/// A run of months as "from May to September", anything else as "in January and April".
fn during(months: &[u32]) -> String {
    let span = month_span(months);
    if span.contains(" to ") {
        format!("from {span}")
    } else {
        format!("in {span}")
    }
}

fn capitalised(text: &str) -> String {
    let mut characters = text.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_runs_read_as_a_span_and_gaps_as_a_list() {
        assert_eq!(month_span(&[5, 6, 7, 8, 9]), "May to September");
        assert_eq!(month_span(&[1, 2, 12]), "December to February");
        assert_eq!(month_span(&[1, 4]), "January and April");
        assert_eq!(month_span(&[1, 2]), "January and February");
        assert_eq!(month_span(&[7]), "July");
    }

    #[test]
    fn the_winter_half_runs_from_october_to_march() {
        assert_eq!(winter_half_text(), "Oct to Mar");
    }
}
