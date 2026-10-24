//! BAG verblijfsobjecten, the addressable units inside buildings, summed per
//! building: how many dwellings it holds, and its floor area.
//!
//! Each unit links to its building through `pand.href`, which ends in the same
//! feature id the building has in the `pand` collection. Units that BAG has
//! withdrawn, or that were never built, are left out.

use std::collections::HashMap;

use geo::Rect;
use serde_json::Value;

use crate::sources::pdok::{self, Filter, Record};
use crate::Result;

const SERVICE: &str = "PDOK BAG";
const BASE_URL: &str = "https://api.pdok.nl/kadaster/bag/ogc/v2";
const COLLECTION: &str = "verblijfsobject";
const BUILDING_LINK: &str = "pand.href";
const RESIDENTIAL_USE: &str = "woonfunctie";

/// Statuses of a unit that exists today, in one place. Left out:
/// "Verblijfsobject ingetrokken", "Niet gerealiseerd verblijfsobject",
/// "Verblijfsobject ten onrechte opgevoerd" and "Verblijfsobject gevormd",
/// which is a unit still being built.
const EXISTING: [&str; 4] = [
    "Verblijfsobject in gebruik",
    "Verblijfsobject in gebruik (niet ingemeten)",
    "Verblijfsobject buiten gebruik",
    "Verbouwing verblijfsobject",
];

/// What the units of one building add up to.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Totals {
    pub dwellings: u32,
    pub floor_area_m2: f64,
}

/// Every unit touching `bbox` (RD New), withdrawn ones included.
pub fn fetch_tile(bbox: Rect<f64>) -> Result<Vec<Record>> {
    pdok::fetch_records(SERVICE, BASE_URL, COLLECTION, bbox, Filter::None)
}

/// Totals per building feature id. A unit spread over several buildings
/// splits its floor area evenly and counts as a dwelling in the first.
pub fn totals_per_building(units: &[Record]) -> HashMap<String, Totals> {
    let mut totals: HashMap<String, Totals> = HashMap::new();
    for unit in units.iter().filter(|unit| exists(unit)) {
        let buildings = building_ids(unit);
        if buildings.is_empty() {
            continue;
        }
        let area = pdok::integer(unit, "oppervlakte").unwrap_or(0) as f64;
        let is_dwelling = pdok::text(unit, "gebruiksdoel").is_some_and(|uses| {
            uses.split(',')
                .any(|single| single.trim() == RESIDENTIAL_USE)
        });
        for (index, building) in buildings.iter().enumerate() {
            let entry = totals.entry(building.clone()).or_default();
            entry.floor_area_m2 += area / buildings.len() as f64;
            if is_dwelling && index == 0 {
                entry.dwellings += 1;
            }
        }
    }

    totals
}

fn exists(unit: &Record) -> bool {
    let status = pdok::text(unit, "status").unwrap_or_default();
    EXISTING.contains(&status.as_str())
}

/// ".../collections/pand/items/5348e207-..." -> "5348e207-...", for each linked building.
fn building_ids(unit: &Record) -> Vec<String> {
    let links = match unit.properties.get(BUILDING_LINK) {
        Some(Value::Array(links)) => links.clone(),
        Some(Value::String(link)) => vec![Value::String(link.clone())],
        _ => Vec::new(),
    };
    links
        .iter()
        .filter_map(Value::as_str)
        .filter_map(|link| link.rsplit('/').next())
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{Geometry, Point};
    use serde_json::json;

    fn unit(properties: serde_json::Value) -> Record {
        Record {
            id: None,
            properties: properties.as_object().unwrap().clone(),
            geometry: Geometry::Point(Point::new(0.0, 0.0)),
        }
    }

    #[test]
    fn dwellings_and_floor_area_add_up_per_building_without_withdrawn_units() {
        let link = json!(["https://api.pdok.nl/kadaster/bag/ogc/v2/collections/pand/items/abc"]);
        let units = [
            unit(
                json!({"status": "Verblijfsobject in gebruik", "gebruiksdoel": "woonfunctie", "oppervlakte": 80, "pand.href": link}),
            ),
            unit(
                json!({"status": "Verblijfsobject in gebruik", "gebruiksdoel": "winkelfunctie", "oppervlakte": 120, "pand.href": link}),
            ),
            unit(
                json!({"status": "Verblijfsobject ingetrokken", "gebruiksdoel": "woonfunctie", "oppervlakte": 60, "pand.href": link}),
            ),
        ];
        let totals = totals_per_building(&units);
        assert_eq!(
            totals["abc"],
            Totals {
                dwellings: 1,
                floor_area_m2: 200.0
            }
        );
    }
}
