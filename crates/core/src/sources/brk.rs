//! BRK, the cadastral map: parcel outlines.

use geo::Rect;

use crate::scene::{Attributes, Credit, Feature, Layer};
use crate::sources::pdok::{self, Record};
use crate::Result;

const SERVICE: &str = "PDOK BRK";
const BASE_URL: &str = "https://api.pdok.nl/kadaster/brk-kadastrale-kaart/ogc/v1";
const COLLECTION: &str = "perceel";

pub fn credit() -> Credit {
    Credit {
        name: "Kadastrale Kaart (BRK), Kadaster, via PDOK".to_string(),
        // Unlike the other sources, CC BY: the credit must stay on the sheet.
        license: "CC BY 4.0".to_string(),
        url: "https://www.pdok.nl/introductie/-/article/kadastrale-kaart".to_string(),
    }
}

/// Every parcel record touching `bbox` (RD New), still in RD New.
pub fn fetch_tile(bbox: Rect<f64>) -> Result<Vec<Record>> {
    pdok::fetch_records(SERVICE, BASE_URL, COLLECTION, bbox, None)
}

pub fn to_features(records: Vec<Record>) -> Vec<Feature> {
    records.into_iter().map(to_parcel).collect()
}

fn to_parcel(record: Record) -> Feature {
    Feature {
        layer: Layer::Parcel,
        attributes: Attributes {
            source_id: designation(&record),
            ..Attributes::default()
        },
        geometry: record.geometry,
    }
}

/// The parcel's name as a notary writes it: "ASD04 K 1234".
fn designation(record: &Record) -> Option<String> {
    let municipality = pdok::text(record, "akr_kadastrale_gemeente_code_waarde")?;
    let section = pdok::text(record, "sectie")?;
    let number = pdok::text(record, "perceelnummer")?;
    Some(format!("{municipality} {section} {number}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{Geometry, Point};
    use serde_json::json;

    #[test]
    fn a_parcel_is_named_by_municipality_section_and_number() {
        let record = Record {
            id: None,
            properties: json!({"akr_kadastrale_gemeente_code_waarde": "ASD04", "sectie": "K", "perceelnummer": 1234})
                .as_object()
                .unwrap()
                .clone(),
            geometry: Geometry::Point(Point::new(0.0, 0.0)),
        };
        assert_eq!(designation(&record).as_deref(), Some("ASD04 K 1234"));
    }
}
