//! CBS statistics per 100 m square: residents, households and dwellings.
//!
//! Every year since 2000 sits in the same collection, so each request names
//! its year. The newest year does not have every figure yet (WOZ values come
//! later and read -99995), so each figure is taken from the newest year that
//! has it. Counts are rounded to 5, and a figure CBS keeps secret because too
//! few people live in the square reads -99997.

use std::collections::BTreeMap;

use geo::Rect;

use crate::scene::{Attributes, Credit, Feature, Layer};
use crate::sources::pdok::{self, Record};
use crate::Result;

const SERVICE: &str = "PDOK CBS";
const BASE_URL: &str = "https://api.pdok.nl/cbs/vierkantstatistieken100m/ogc/v1";
const COLLECTION: &str = "vierkant100";
/// The square's own code, the same in every year: "E1204N4877".
const CELL_CODE: &str = "crs28992res100m";
const RESIDENTS: &str = "aantal_inwoners";
const HOUSEHOLDS: &str = "aantal_part_huishoudens";
const DWELLINGS: &str = "aantal_woningen";
/// CBS's marker for "not yet available"; the next older year is used instead.
const NOT_YET_AVAILABLE: i64 = -99995;

// #! TODO[population] : Add 2025 in front once PDOK serves it; 2025 returned nothing on 2026-10-24
/// Years asked for, newest first, in one place. The first is the one legends quote.
pub const YEARS: [i32; 2] = [2024, 2023];
pub const CELL_AREA_M2: f64 = 100.0 * 100.0;

pub fn credit() -> Credit {
    Credit {
        name: "CBS Kaart van 100 meter bij 100 meter met statistieken, via PDOK".to_string(),
        license: "CC BY 4.0".to_string(),
        url: "https://www.cbs.nl/nl-nl/dossier/nederland-regionaal/geografische-data/kaart-van-100-meter-bij-100-meter-met-statistieken".to_string(),
        layers: vec![Layer::Population],
    }
}

/// Every square of `year` touching `bbox` (RD New).
pub fn fetch_tile(year: i32, bbox: Rect<f64>) -> Result<Vec<Record>> {
    let filter = format!("jaarcode={year}");
    pdok::fetch_records(SERVICE, BASE_URL, COLLECTION, bbox, Some(&filter))
}

/// One feature per square, each figure from the newest year that has it.
/// `years` holds the records of each year in `YEARS`, newest first.
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
    fn a_secret_figure_becomes_no_data() {
        let secret = square(json!({"crs28992res100m": "E2", "aantal_inwoners": -99997}));
        let features = to_features(vec![vec![secret], Vec::new()]);
        assert_eq!(features[0].attributes.value, None);
    }
}
