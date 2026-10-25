//! Turns the analysis data into sections: facts written out, figures drawn
//! in the theme, notes, and the credits for what the section shows.

mod climate_facts;

use self::climate_facts::{
    comfort_facts, humidity_facts, sun_facts, temperature_facts, wind_facts,
};
use super::charts;
use super::climate::{Climate, DAY_END_HOUR, DAY_START_HOUR};
use super::comfort::{Exposure, METHOD as COMFORT_METHOD};
use super::comfort_chart;
use super::figure::{percent, Paint};
use super::location::{self, Location};
use super::locator::locator;
use super::notes;
use super::rain::{Rain, RAIN_DAY_MM};
use super::roses;
use super::source_lines;
use super::summary::{land_use, SiteSummary, FACILITY_WALK_MIN, TRANSIT_WALK_MIN};
use super::{Fact, Section, SectionNotes};
use crate::ratios::{area_text, grouped, Ratios};
use crate::scene::Credit;
use crate::sources::{knmi, pvgis};
use crate::theme::Theme;
use crate::Result;

pub fn location(location: &Location, theme: &Theme) -> Result<Section> {
    let paint = Paint::new(theme)?;
    let unknown = || "Not found".to_string();
    let mut facts = vec![fact(
        "Address",
        location.address.clone().unwrap_or_else(unknown),
    )];
    facts.push(fact(
        "Municipality",
        location.municipality.clone().unwrap_or_else(unknown),
    ));
    facts.push(fact(
        "Province",
        location.province.clone().unwrap_or_else(unknown),
    ));
    facts.push(fact(
        "Coordinates",
        format!(
            "{:.4}° N, {:.4}° E",
            location.centre.lat, location.centre.lon
        ),
    ));
    facts.push(fact(
        "RD New",
        format!("{}, {}", grouped(location.rd.0), grouped(location.rd.1)),
    ));

    Ok(Section {
        id: "location".to_string(),
        title: "Location".to_string(),
        facts,
        figures: vec![locator(location, &paint)],
        caption: None,
        notes: Vec::new(),
        source: source_lines::location(),
        credits: location::credits(),
        warnings: location.warnings.clone(),
    })
}

pub fn site(
    summary: &SiteSummary,
    ratios: &Ratios,
    credits: Vec<Credit>,
    theme: &Theme,
) -> Result<Section> {
    let paint = Paint::new(theme)?;
    let area_label = source_lines::area_label(summary);
    let mut facts = vec![
        fact(area_label, area_text(summary.site_area_m2)),
        fact("Buildings", grouped(summary.building_count as f64)),
        fact("Coverage", percent(summary.coverage)),
        fact(
            "Floor area ratio",
            format!("{:.2}", summary.floor_area_ratio),
        ),
    ];
    if let Some(mean) = summary.mean_height_m {
        facts.push(fact("Mean height", format!("{mean:.1} m")));
    }
    if let Some(tallest) = summary.tallest_m {
        facts.push(fact("Tallest", format!("{tallest:.1} m")));
    }
    if let Some(year) = summary.median_year_built {
        facts.push(fact("Median year built", year.to_string()));
    }
    if let Some(dominant) = &summary.dominant_use {
        facts.push(fact(
            "Most common use",
            format!("{}, {}", dominant.label, percent(dominant.share)),
        ));
    }
    facts.push(fact("Green", percent(summary.green_share)));
    facts.push(fact("Water", percent(summary.water_share)));
    facts.push(fact("Paved", percent(summary.paved_share)));
    surroundings_facts(summary, &mut facts);

    let figures = land_use(ratios)
        .map(|group| charts::ground(group, &paint))
        .into_iter()
        .collect();
    Ok(Section {
        id: "site".to_string(),
        title: "Site".to_string(),
        facts,
        figures,
        caption: Some(source_lines::site_caption(summary)),
        notes: notes::site_notes(summary),
        source: source_lines::site(summary),
        credits,
        warnings: Vec::new(),
    })
}

fn surroundings_facts(summary: &SiteSummary, facts: &mut Vec<Fact>) {
    let surroundings = &summary.surroundings;
    let environment = &surroundings.environment;
    if let Some(noise) = environment.road_noise_mean_db {
        let label = with_year("Road noise", environment.noise_year);
        facts.push(fact(&label, format!("{noise:.0} dB Lden")));
    }
    if let Some(no2) = environment.no2_mean_ug_m3 {
        let label = with_year("NO₂", environment.air_year);
        facts.push(fact(&label, format!("{no2:.0} µg/m³")));
    }
    if let Some(depth) = environment.flood_max_depth_m {
        let share = environment.flood_share.unwrap_or(0.0);
        let value = if depth > 0.0 {
            format!("{depth:.1} m, {} of site", percent(share))
        } else {
            "Stays dry".to_string()
        };
        facts.push(fact("Worst-case flood", value));
    }
    if let Some(slope) = environment.slope_max_percent {
        facts.push(fact("Steepest slope", format!("{slope:.0} %")));
    }
    if let Some(canopy) = environment.tree_canopy_share {
        let label = with_year("Tree canopy", environment.tree_year);
        facts.push(fact(&label, percent(canopy)));
    }
    if let Some(heat) = environment.heat_island_mean_c {
        facts.push(fact("Heat island", format!("{heat:.1} °C warmer")));
    }
    if let Some(residents) = environment.residents {
        let label = with_year("Residents", environment.population_year);
        facts.push(fact(&label, grouped(residents)));
    }
    if let Some(dwellings) = environment.dwellings {
        facts.push(fact("Dwellings", grouped(dwellings as f64)));
    }
    if let Some(facilities) = surroundings.facilities_within_walk {
        let label = format!("Facilities, {FACILITY_WALK_MIN} min walk");
        facts.push(fact(&label, grouped(facilities as f64)));
    }
    if let Some(businesses) = surroundings.businesses_within_walk {
        let label = format!("Shops and food, {FACILITY_WALK_MIN} min walk");
        facts.push(fact(&label, grouped(businesses as f64)));
    }
    if let Some(stops) = surroundings.transit_stops_within_walk {
        let label = format!("Transit stops, {TRANSIT_WALK_MIN} min walk");
        facts.push(fact(&label, grouped(stops as f64)));
    }
}

/// Temperature, outdoor comfort, wind, sun and humidity.
pub fn climate(climate: &Climate, warnings: Vec<String>, theme: &Theme) -> Result<Vec<Section>> {
    let paint = Paint::new(theme)?;
    let source = climate_source(climate);
    let section = |id: &str, title: &str, facts, figures, caption: String, notes| Section {
        id: id.to_string(),
        title: title.to_string(),
        facts,
        figures,
        caption: Some(caption),
        notes,
        source: source.clone(),
        credits: vec![pvgis::credit()],
        warnings: Vec::new(),
    };

    let mut sections = vec![
        section("temperature", "Temperature", temperature_facts(climate), vec![charts::temperature(climate, &paint)], "Bars run from the mean daily low to the mean daily high, with the monthly mean across them and the mean high above.".to_string(), notes::temperature_notes(climate)),
        section("comfort", "Outdoor comfort", comfort_facts(climate), vec![comfort_chart::comfort(climate, Exposure::Sun, &paint), comfort_chart::comfort(climate, Exposure::Shade, &paint)], format!("Daytime hours, {DAY_START_HOUR}:00 to {DAY_END_HOUR}:00, in the ten UTCI stress categories, for a person in the sun and in the shade. {COMFORT_METHOD}. Shade blocks the direct sun only."), notes::comfort_notes(climate)),
        section("wind", "Wind", wind_facts(climate), vec![roses::wind(climate, &paint)], "Where the wind comes from, by season, as a share of all hours, at 10 m above open ground. Street-level wind is weaker but gustier around tall buildings.".to_string(), notes::wind_notes(climate)),
        section("sun", "Sun", sun_facts(climate), vec![charts::radiation(climate, &paint), roses::sun(climate, &paint)], "Monthly sun on a flat surface (global) and on a surface facing the sun (direct), then the direct sun by the compass direction it comes from. Terrain is taken into account, neighbouring buildings are not.".to_string(), notes::sun_notes(climate)),
        section("humidity", "Humidity", humidity_facts(climate), vec![charts::humidity(climate, &paint)], "Mean relative humidity of the air per month.".to_string(), notes::humidity_notes(climate)),
    ];
    if let Some(first) = sections.first_mut() {
        first.warnings = warnings;
    }
    if climate.comfort_outside_range_hours > 0 {
        if let Some(comfort) = sections.iter_mut().find(|section| section.id == "comfort") {
            comfort.warnings.push(format!("{} daytime hours lie outside the temperatures the UTCI polynomial was fitted for (air −50 to 50 °C, radiant temperature 30 K below to 70 K above the air) and are less certain.", climate.comfort_outside_range_hours));
        }
    }

    Ok(sections)
}

/// The climate notes per section id, for the assistant.
pub fn climate_notes(climate: &Climate) -> Vec<SectionNotes> {
    [
        ("temperature", notes::temperature_notes(climate)),
        ("comfort", notes::comfort_notes(climate)),
        ("wind", notes::wind_notes(climate)),
        ("sun", notes::sun_notes(climate)),
        ("humidity", notes::humidity_notes(climate)),
    ]
    .into_iter()
    .map(|(section, notes)| SectionNotes {
        section: section.to_string(),
        source: climate_source(climate),
        notes,
    })
    .collect()
}

pub fn rain(rain: &Rain, warnings: Vec<String>, theme: &Theme) -> Result<Section> {
    let paint = Paint::new(theme)?;
    let facts = vec![
        fact(
            "Station",
            format!("{}, {:.0} km away", rain.station.name, rain.distance_km),
        ),
        fact("Period", format!("{}–{}", rain.first_year, rain.last_year)),
        fact("Per year", format!("{:.0} mm", rain.annual_mm)),
        fact("Rain days", format!("{:.0} a year", rain.annual_rain_days)),
    ];

    Ok(Section {
        id: "rain".to_string(),
        title: "Rain".to_string(),
        facts,
        figures: vec![charts::rain(rain, &paint)],
        caption: Some(format!(
            "Mean monthly rain at KNMI station {} ({:.0} km from the site) over {}–{}, months with a missing day left out. A rain day has {RAIN_DAY_MM:.0} mm or more.",
            rain.station.name, rain.distance_km, rain.first_year, rain.last_year
        )),
        notes: notes::rain_notes(rain),
        source: rain_source(rain),
        credits: vec![knmi::credit()],
        warnings,
    })
}

pub fn climate_source(climate: &Climate) -> String {
    format!(
        "PVGIS typical year from {}–{} weather, for {:.2}° N, {:.2}° E.",
        climate.first_year, climate.last_year, climate.location.lat, climate.location.lon
    )
}

pub fn rain_source(rain: &Rain) -> String {
    format!(
        "KNMI daily data, station {} {}.",
        rain.station.id, rain.station.name
    )
}

fn fact(label: &str, value: String) -> Fact {
    Fact {
        label: label.to_string(),
        value,
    }
}

/// "Road noise, 2022", or just "Road noise" when the scene does not say.
fn with_year(label: &str, year: Option<i32>) -> String {
    match year {
        Some(year) => format!("{label}, {year}"),
        None => label.to_string(),
    }
}

/// The credits of every section, each once, for a page's footer.
pub fn unique_credits(sections: &[Section]) -> Vec<Credit> {
    let mut credits: Vec<Credit> = Vec::new();
    for credit in sections.iter().flat_map(|section| &section.credits) {
        if !credits.iter().any(|known| known.name == credit.name) {
            credits.push(credit.clone());
        }
    }

    credits
}
