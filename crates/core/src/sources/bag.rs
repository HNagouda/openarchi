//! BAG, the building register: one footprint per pand, with year built and use.
//!
//! BAG rather than the BGT pand layer, because BAG carries bouwjaar and
//! gebruiksdoel, and its footprints are the ones AHN heights are computed for.

use geo::Rect;

use crate::scene::{Attributes, Credit, Feature, Layer};
use crate::sources::pdok::{self, Record};
use crate::Result;

const SERVICE: &str = "PDOK BAG";
const BASE_URL: &str = "https://api.pdok.nl/kadaster/bag/ogc/v2";
const COLLECTION: &str = "pand";

/// BAG statuses of a building that stands on the site today, in one place.
/// Left out: "Pand gesloopt", "Niet gerealiseerd pand", "Pand ten onrechte
/// opgevoerd", "Bouwvergunning verleend" and "Bouw gestart", which is not yet
/// a building.
const STANDING: [&str; 5] = [
    "Pand in gebruik",
    "Pand in gebruik (niet ingemeten)",
    "Pand buiten gebruik",
    "Verbouwing pand",
    "Sloopvergunning verleend",
];

pub fn credit() -> Credit {
    Credit {
        name: "Basisregistratie Adressen en Gebouwen (BAG), via PDOK".to_string(),
        license: "Public Domain Mark 1.0".to_string(),
        url: "https://www.pdok.nl/introductie/-/article/basisregistratie-adressen-en-gebouwen-ba-1"
            .to_string(),
    }
}

/// Every standing building touching `bbox` (RD New), still in RD New.
pub fn fetch(bbox: Rect<f64>) -> Result<Vec<Feature>> {
    let records = pdok::fetch_collection(SERVICE, BASE_URL, COLLECTION, bbox, None)?;
    let buildings = records
        .into_iter()
        .filter(is_standing)
        .map(to_building)
        .collect();

    Ok(buildings)
}

fn is_standing(record: &Record) -> bool {
    let status = pdok::text(record, "status").unwrap_or_default();
    STANDING.contains(&status.as_str())
}

fn to_building(record: Record) -> Feature {
    Feature {
        layer: Layer::Building,
        attributes: Attributes {
            source_id: pdok::text(&record, "identificatie"),
            kind: pdok::text(&record, "status"),
            year_built: pdok::integer(&record, "bouwjaar")
                .and_then(|year| i32::try_from(year).ok()),
            use_class: pdok::text(&record, "gebruiksdoel").map(|uses| join_uses(&uses)),
            ..Attributes::default()
        },
        geometry: record.geometry,
    }
}

/// "winkelfunctie,woonfunctie" -> "winkelfunctie, woonfunctie".
fn join_uses(uses: &str) -> String {
    uses.split(',')
        .map(str::trim)
        .filter(|single| !single.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{Geometry, Point};
    use serde_json::json;

    fn record(properties: serde_json::Value) -> Record {
        Record {
            id: None,
            properties: properties.as_object().unwrap().clone(),
            geometry: Geometry::Point(Point::new(0.0, 0.0)),
        }
    }

    #[test]
    fn mixed_use_is_joined_with_a_comma_and_a_space() {
        assert_eq!(
            join_uses("winkelfunctie,woonfunctie"),
            "winkelfunctie, woonfunctie"
        );
        assert_eq!(join_uses("woonfunctie"), "woonfunctie");
    }

    #[test]
    fn demolished_and_unbuilt_buildings_are_left_out() {
        assert!(is_standing(&record(json!({"status": "Pand in gebruik"}))));
        assert!(!is_standing(&record(json!({"status": "Pand gesloopt"}))));
        assert!(!is_standing(&record(
            json!({"status": "Niet gerealiseerd pand"})
        )));
        assert!(!is_standing(&record(json!({}))));
    }

    #[test]
    fn a_building_carries_year_and_use() {
        let building = to_building(record(json!({
            "identificatie": "0363100012345678",
            "status": "Pand in gebruik",
            "bouwjaar": 1638,
            "gebruiksdoel": "bijeenkomstfunctie,woonfunctie"
        })));
        assert_eq!(building.layer, Layer::Building);
        assert_eq!(building.attributes.year_built, Some(1638));
        assert_eq!(
            building.attributes.use_class.as_deref(),
            Some("bijeenkomstfunctie, woonfunctie")
        );
    }
}
