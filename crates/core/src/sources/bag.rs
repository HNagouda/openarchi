//! BAG, the building register: one footprint per pand, with year built and use.
//!
//! BAG rather than the BGT pand layer, because BAG carries bouwjaar and
//! gebruiksdoel, and its footprints are the ones AHN heights are computed for.

use std::collections::HashMap;

use geo::Rect;

use crate::scene::{Attributes, Credit, Feature, Layer};
use crate::sources::bag_units::Totals;
use crate::sources::pdok::{self, Filter, Record};
use crate::Result;

const SERVICE: &str = "PDOK BAG";
const BASE_URL: &str = "https://api.pdok.nl/kadaster/bag/ogc/v2";
const COLLECTION: &str = "pand";

/// Placeholder years BAG uses when the real year is unknown (1005) or the
/// building is not finished (9999). Both become unknown, not a real year.
const PLACEHOLDER_YEARS: [i64; 2] = [1005, 9999];

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
        layers: Vec::new(),
    }
}

/// Every building record touching `bbox` (RD New), demolished ones included.
pub fn fetch_tile(bbox: Rect<f64>) -> Result<Vec<Record>> {
    pdok::fetch_records(SERVICE, BASE_URL, COLLECTION, bbox, Filter::None)
}

/// The standing buildings among `records`, still in RD New. With `units`,
/// from `bag_units`, each building also gets its dwellings and floor area,
/// zero when no unit links to it.
pub fn to_features(records: Vec<Record>, units: Option<&HashMap<String, Totals>>) -> Vec<Feature> {
    records
        .into_iter()
        .filter(is_standing)
        .map(|record| {
            let totals = units.map(|units| {
                record
                    .id
                    .as_ref()
                    .and_then(|id| units.get(id))
                    .copied()
                    .unwrap_or_default()
            });
            let mut building = to_building(record);
            building.attributes.dwellings = totals.map(|totals| totals.dwellings);
            building.attributes.floor_area_m2 = totals.map(|totals| totals.floor_area_m2);
            building
        })
        .collect()
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
                .filter(|year| !PLACEHOLDER_YEARS.contains(year))
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
    fn placeholder_years_become_unknown() {
        let building = to_building(record(json!({"bouwjaar": 1005})));
        assert_eq!(building.attributes.year_built, None);
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
