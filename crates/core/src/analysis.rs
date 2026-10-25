//! Analysis: a report on where a site is, what is on it, and its climate,
//! with charts drawn in the current theme and design notes under each part.
//!
//! The report comes in four parts that load on their own, because they cost
//! very different amounts of time. Location and site come from the scene and
//! two quick PDOK lookups; climate (PVGIS) and rain (KNMI) each wait on a
//! service on first use and then on the tile cache. Every part becomes one or
//! more sections, and a section holds everything one block of the report
//! shows: facts, figures as SVG, notes, and the credits for its data.
//!
//! The same sections feed the inspector, the SVG and PDF export, and the
//! assistant's `get_analysis`, so all three always say the same thing.

mod a4;
mod a4_blocks;
mod charts;
mod climate;
mod comfort;
mod comfort_chart;
mod figure;
mod location;
mod locator;
mod notes;
mod page;
mod radiant;
mod rain;
mod roses;
mod sections;
mod source_lines;
mod summary;
mod utci;

use std::sync::Arc;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::crs::rd_to_wgs84;
use crate::memo::{self, PartKey};
use crate::ratios::{self, Ratios};
use crate::scene::{Credit, Scene};
use crate::sources::cache::TileCache;
use crate::sources::grid::Tile;
use crate::sources::{knmi, pvgis};
use crate::theme::Theme;
use crate::Result;

pub use a4::report_pages;
pub use climate::Climate;
pub use figure::Figure;
// Shared with report blocks, so they draw in the same colours and type.
pub(crate) use figure::{text, Paint};
pub use location::Location;
pub(crate) use page::paragraph;
pub use page::{report_svg, section_svg};
pub use rain::Rain;
pub use summary::{SiteSummary, Surroundings};

/// The parts of the report, each loaded on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Part {
    Location,
    Site,
    Climate,
    Rain,
}

/// Every part, in report order, in one place.
pub const PARTS: [Part; 4] = [Part::Location, Part::Site, Part::Climate, Part::Rain];

impl Part {
    pub fn title(self) -> &'static str {
        match self {
            Part::Location => "Location",
            Part::Site => "Site",
            Part::Climate => "Climate",
            Part::Rain => "Rain",
        }
    }
}

/// One block of the report.
#[derive(Debug, Clone, Serialize)]
pub struct Section {
    /// Stable, kebab-case: "location", "wind".
    pub id: String,
    pub title: String,
    pub facts: Vec<Fact>,
    pub figures: Vec<Figure>,
    /// How the figures were made, in one or two plain sentences.
    pub caption: Option<String>,
    pub notes: Vec<String>,
    /// Where the numbers come from, in one line: "PVGIS typical year 2005–2023, 52.38° N 4.88° E".
    pub source: String,
    pub credits: Vec<Credit>,
    /// What went wrong without stopping the section.
    pub warnings: Vec<String>,
}

/// One labelled value, already written out: "Coverage", "38 %".
#[derive(Debug, Clone, Serialize)]
pub struct Fact {
    pub label: String,
    pub value: String,
}

/// Everything the assistant gets: the numbers and the notes, no drawings.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub location: Location,
    pub summary: SiteSummary,
    pub climate: Option<Climate>,
    pub rain: Option<Rain>,
    /// Notes per section id, in report order.
    pub notes: Vec<SectionNotes>,
    pub warnings: Vec<String>,
    pub credits: Vec<Credit>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SectionNotes {
    pub section: String,
    /// Where the section's numbers come from, the same line the inspector shows under its title.
    pub source: String,
    pub notes: Vec<String>,
}

/// The sections of one part, drawn in `theme`. `sheet_area` is the scene cut
/// to a sheet's sub-selection, when the sheet has one, and only the site part
/// follows it. Location, climate and rain stay on the whole site, so their
/// lookups and caches do not change with every area drawn.
pub fn part(
    part: Part,
    scene: &Scene,
    sheet_area: Option<&Scene>,
    theme: &Theme,
    cache: &TileCache,
) -> Result<Vec<Section>> {
    match part {
        Part::Location => {
            let location = location::locate(scene, cache);
            Ok(vec![sections::location(&location, theme)?])
        }
        Part::Site => {
            let (ratios, summary) = site_numbers(scene, sheet_area, theme)?;
            let credits = sheet_area.unwrap_or(scene).credits.clone();
            Ok(vec![sections::site(&summary, &ratios, credits, theme)?])
        }
        Part::Climate => {
            let (climate, warnings) = climate(scene, cache)?;
            sections::climate(&climate, warnings, theme)
        }
        Part::Rain => {
            let (rain, warnings) = rain(scene, cache, last_full_year())?;
            Ok(vec![sections::rain(&rain, warnings, theme)?])
        }
    }
}

/// `part` as `part` draws it for the whole site, kept for the next block or
/// page that asks for it in the same palette (see `memo.rs`). The site part
/// is drawn afresh each time, since it follows every edit to the scene.
pub fn kept_part(
    part: Part,
    scene: &Scene,
    theme: &Theme,
    cache: &TileCache,
) -> Result<Arc<Vec<Section>>> {
    if part == Part::Site {
        return self::part(part, scene, None, theme, cache).map(Arc::new);
    }
    let key = PartKey {
        part,
        origin: scene.origin,
        boundary: scene.boundary.clone(),
        theme: theme.clone(),
        cache: cache.dir().to_path_buf(),
        last_year: last_full_year(),
    };
    memo::ANALYSIS_PARTS.kept_or_try(key, || self::part(part, scene, None, theme, cache))
}

/// `part` as the inspector and the analysis export show it: the site part
/// for the sheet's own area when it has one, every other part for the whole
/// site and kept between calls, so the climate files are not read again.
pub fn sheet_part(
    part: Part,
    scene: &Scene,
    sheet_area: Option<&Scene>,
    theme: &Theme,
    cache: &TileCache,
) -> Result<Vec<Section>> {
    if part == Part::Site {
        return self::part(part, scene, sheet_area, theme, cache);
    }
    kept_part(part, scene, theme, cache).map(|kept| kept.as_ref().clone())
}

/// The numbers and notes of every part. A part whose service fails is left
/// out with a warning; location and site always arrive. The site summary
/// follows `sheet_area` when there is one, as in `part`.
pub fn report(
    scene: &Scene,
    sheet_area: Option<&Scene>,
    theme: &Theme,
    cache: &TileCache,
) -> Result<Report> {
    let location = location::locate(scene, cache);
    let (_, summary) = site_numbers(scene, sheet_area, theme)?;
    let mut warnings = location.warnings.clone();
    let mut credits = location::credits();
    let mut notes = vec![
        SectionNotes {
            section: "location".to_string(),
            source: source_lines::location(),
            notes: Vec::new(),
        },
        SectionNotes {
            section: "site".to_string(),
            source: source_lines::site(&summary),
            notes: notes::site_notes(&summary),
        },
    ];

    let climate = match climate(scene, cache) {
        Ok((climate, climate_warnings)) => {
            warnings.extend(climate_warnings);
            credits.push(pvgis::credit());
            notes.extend(sections::climate_notes(&climate));
            Some(climate)
        }
        Err(error) => {
            warnings.push(format!("The climate could not be fetched: {error}"));
            None
        }
    };
    let rain = match rain(scene, cache, last_full_year()) {
        Ok((rain, rain_warnings)) => {
            warnings.extend(rain_warnings);
            credits.push(knmi::credit());
            notes.push(SectionNotes {
                section: "rain".to_string(),
                source: sections::rain_source(&rain),
                notes: notes::rain_notes(&rain),
            });
            Some(rain)
        }
        Err(error) => {
            warnings.push(format!("The rain could not be fetched: {error}"));
            None
        }
    };

    Ok(Report {
        location,
        summary,
        climate,
        rain,
        notes,
        warnings,
        credits,
    })
}

/// The ratios and the summary of the site, or of `sheet_area` when a sheet has one.
fn site_numbers(
    scene: &Scene,
    sheet_area: Option<&Scene>,
    theme: &Theme,
) -> Result<(Ratios, SiteSummary)> {
    let site = sheet_area.unwrap_or(scene);
    let ratios = ratios::compute(site, theme)?;
    let summary = summary::summarise(site, &ratios, sheet_area.is_some());
    Ok((ratios, summary))
}

/// The last year that has ended, from the clock: 2025 during 2026.
pub fn last_full_year() -> i32 {
    let now = humantime::format_rfc3339_seconds(SystemTime::now()).to_string();
    let this_year: i32 = now
        .get(0..4)
        .and_then(|year| year.parse().ok())
        .unwrap_or(2025);
    this_year - 1
}

fn climate(scene: &Scene, cache: &TileCache) -> Result<(Climate, Vec<String>)> {
    let cell = pvgis::cell_of(scene.origin);
    let (bytes, warning) = cache.read_or_fetch(pvgis::CACHE_DATASET, cell, || {
        let bytes = pvgis::fetch(cell)?;
        // Only an answer that parses is worth keeping for a month.
        pvgis::parse(&bytes)?;
        Ok(bytes)
    })?;
    let tmy = pvgis::parse(&bytes)?;

    Ok((climate::from_tmy(&tmy), warning.into_iter().collect()))
}

fn rain(scene: &Scene, cache: &TileCache, last_year: i32) -> Result<(Rain, Vec<String>)> {
    let centre = rd_to_wgs84(scene.origin.0, scene.origin.1);
    let (station, distance_km) = knmi::nearest(centre);
    let first_year = last_year - rain::YEARS + 1;
    let key = Tile {
        column: station.id as i64,
        row: last_year as i64,
    };
    let (bytes, warning) = cache.read_or_fetch(knmi::CACHE_DATASET, key, || {
        let bytes = knmi::fetch(station.id, first_year, last_year)?;
        // KNMI answers a bad query with an HTML page and status 200; never keep that.
        knmi::parse(&bytes)?;
        Ok(bytes)
    })?;
    let days = knmi::parse(&bytes)?;

    let rain = rain::from_days(station, distance_km, first_year, last_year, &days);
    Ok((rain, warning.into_iter().collect()))
}

#[cfg(test)]
mod tests;
