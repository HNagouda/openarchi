//! CBS statistics per 100 m square: residents, households and dwellings.
//!
//! Every year since 2000 sits in the same collection, so each request names
//! its year. The newest year is found when a site is fetched, by asking for
//! one square of each recent year. It does not have every figure yet (WOZ
//! values come later and read -99995), so the year before it is fetched too
//! and each figure is taken from the newest year that has it. Counts are
//! rounded to 5, and a figure CBS keeps secret because too few people live in
//! the square reads -99997.

use std::collections::BTreeMap;

use geo::Rect;
use serde::Deserialize;

use crate::scene::{Attributes, Credit, Feature, Layer};
use crate::sources::edition::{self, ASK_TIMEOUT};
use crate::sources::http;
use crate::sources::pdok::{self, Filter, Record};
use crate::{Error, Result};

const SERVICE: &str = "PDOK CBS";
const BASE_URL: &str = "https://api.pdok.nl/cbs/vierkantstatistieken100m/ogc/v1";
const COLLECTION: &str = "vierkant100";
const YEAR_PROPERTY: &str = "jaarcode";
/// CBS adds a year in the summer after it (2024 arrived in June 2025), so the
/// current year never has squares and the newest is one or two years back.
/// Four years back is far past any delay CBS has had.
const YEARS_TO_TRY: i32 = 4;
/// The square's own code, the same in every year: "E1204N4877".
const CELL_CODE: &str = "crs28992res100m";
const RESIDENTS: &str = "aantal_inwoners";
const HOUSEHOLDS: &str = "aantal_part_huishoudens";
const DWELLINGS: &str = "aantal_woningen";
/// CBS's marker for "not yet available"; the next older year is used instead.
const NOT_YET_AVAILABLE: i64 = -99995;

/// The newest year on 2026-10-24, used when the service cannot be asked.
pub const KNOWN_YEAR: i32 = 2024;
pub const CELL_AREA_M2: f64 = 100.0 * 100.0;

/// One page of squares; only its length is read.
#[derive(Deserialize)]
struct Probe {
    #[serde(rename = "numberReturned")]
    number_returned: usize,
}

pub fn credit() -> Credit {
    Credit {
        name: "CBS Kaart van 100 meter bij 100 meter met statistieken, via PDOK".to_string(),
        license: "CC BY 4.0".to_string(),
        url: "https://www.cbs.nl/nl-nl/dossier/nederland-regionaal/geografische-data/kaart-van-100-meter-bij-100-meter-met-statistieken".to_string(),
        layers: vec![Layer::Population],
    }
}

/// The years fetched for a site whose newest year is `newest`, newest first.
/// The first is the one legends quote.
pub fn years(newest: i32) -> [i32; 2] {
    [newest, newest - 1]
}

/// The newest year with squares, asking one square of each year back from
/// last year. About 50 ms a question.
pub fn latest_year() -> Result<i32> {
    let this_year = edition::this_year().ok_or_else(|| Error::Data {
        service: SERVICE.to_string(),
        detail: "the clock gives no year".to_string(),
    })?;
    newest_year_with_squares(this_year, ask_has_squares)
}

/// The first year back from `this_year - 1` for which `has_squares` says yes.
fn newest_year_with_squares(
    this_year: i32,
    has_squares: impl Fn(i32) -> Result<bool>,
) -> Result<i32> {
    for year in (this_year - YEARS_TO_TRY..this_year).rev() {
        if has_squares(year)? {
            return Ok(year);
        }
    }

    Err(Error::Data {
        service: SERVICE.to_string(),
        detail: format!("no squares for any year from {} back", this_year - 1),
    })
}

fn ask_has_squares(year: i32) -> Result<bool> {
    let url = format!("{BASE_URL}/collections/{COLLECTION}/items");
    let year = year.to_string();
    let query = [
        ("f", "json"),
        ("limit", "1"),
        (YEAR_PROPERTY, year.as_str()),
    ];
    let body = http::get_once_within(SERVICE, &url, &query, ASK_TIMEOUT)?;
    has_squares(&body)
}

fn has_squares(body: &[u8]) -> Result<bool> {
    let probe: Probe = serde_json::from_slice(body).map_err(|error| Error::Data {
        service: SERVICE.to_string(),
        detail: error.to_string(),
    })?;
    Ok(probe.number_returned > 0)
}

/// Every square of `year` touching `bbox` (RD New).
pub fn fetch_tile(year: i32, bbox: Rect<f64>) -> Result<Vec<Record>> {
    let year = year.to_string();
    pdok::fetch_records(
        SERVICE,
        BASE_URL,
        COLLECTION,
        bbox,
        Filter::Property(YEAR_PROPERTY, &year),
    )
}

/// One feature per square, each figure from the newest year that has it.
/// `years` holds the records of each year from `years`, newest first.
pub fn to_features(years: Vec<Vec<Record>>) -> Vec<Feature> {
    let mut squares: BTreeMap<String, Vec<Record>> = BTreeMap::new();
    for records in years {
        for record in records {
            if let Some(code) = pdok::text(&record, CELL_CODE) {
                squares.entry(code).or_default().push(record);
            }
        }
    }

    squares
        .into_values()
        .map(|records| {
            let residents = newest(&records, RESIDENTS);
            Feature {
                layer: Layer::Population,
                attributes: Attributes {
                    source_id: pdok::text(&records[0], CELL_CODE),
                    value: residents.map(|count| count as f64),
                    households: newest(&records, HOUSEHOLDS).map(|count| count as f64),
                    dwellings: newest(&records, DWELLINGS)
                        .and_then(|count| u32::try_from(count).ok()),
                    ..Attributes::default()
                },
                geometry: records[0].geometry.clone(),
            }
        })
        .collect()
}

/// The figure from the newest record that has one. Secret and missing
/// figures are `None`, so the sheet can show them as no data.
fn newest(records: &[Record], key: &str) -> Option<i64> {
    let value = records
        .iter()
        .filter_map(|record| pdok::integer(record, key))
        .find(|&value| value != NOT_YET_AVAILABLE)?;
    (value >= 0).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{Geometry, Point};
    use serde_json::json;

    fn square(properties: serde_json::Value) -> Record {
        Record {
            id: None,
            properties: properties.as_object().unwrap().clone(),
            geometry: Geometry::Point(Point::new(0.0, 0.0)),
        }
    }

    #[test]
    fn a_figure_not_yet_available_comes_from_the_year_before() {
        let newer = square(
            json!({"crs28992res100m": "E1", "aantal_inwoners": 135, "aantal_woningen": -99995}),
        );
        let older =
            square(json!({"crs28992res100m": "E1", "aantal_inwoners": 140, "aantal_woningen": 85}));
        let features = to_features(vec![vec![newer], vec![older]]);
        assert_eq!(features.len(), 1);
        assert_eq!(features[0].attributes.value, Some(135.0));
        assert_eq!(features[0].attributes.dwellings, Some(85));
    }

    #[test]
    fn the_newest_year_is_the_first_one_back_that_has_squares() {
        // Recorded on 2026-10-24: 2025 had no squares yet, 2024 had.
        let none = include_bytes!("../../tests/fixtures/cbs-jaarcode-2025.json");
        let one = include_bytes!("../../tests/fixtures/cbs-jaarcode-2024.json");
        let answer = |year| match year {
            2025 => has_squares(none),
            2024 => has_squares(one),
            _ => panic!("asked for {year}, after a year that had squares"),
        };

        assert_eq!(newest_year_with_squares(2026, answer).unwrap(), 2024);
    }

    #[test]
    fn no_squares_in_any_recent_year_is_an_error() {
        assert!(newest_year_with_squares(2026, |_| Ok(false)).is_err());
    }

    #[test]
    fn a_secret_figure_becomes_no_data() {
        let secret = square(json!({"crs28992res100m": "E2", "aantal_inwoners": -99997}));
        let features = to_features(vec![vec![secret], Vec::new()]);
        assert_eq!(features[0].attributes.value, None);
    }
}
