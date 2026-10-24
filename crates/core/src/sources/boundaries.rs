//! Administrative boundaries from PDOK: the municipality a site lies in, and
//! the outline of the Netherlands that `examples/netherlands_outline.rs`
//! simplifies into the bundled asset.
//!
//! The country comes from CBS rather than from Kadaster's `landgebied`.
//! `landgebied` is the legal territory, Wadden Sea and IJsselmeer included,
//! which on a locator map reads as an unfamiliar blob. The CBS regions follow
//! the coast, so the map looks like the country people know.

use geo::{BooleanOps, Contains, Coord, Geometry, MultiPolygon, Point, Rect, Simplify};
use serde::{Deserialize, Serialize};

use crate::scene::Credit;
use crate::sources::pdok::{self, Filter, Record};
use crate::{Error, Result};

const SERVICE: &str = "PDOK bestuurlijke gebieden";
const BASE_URL: &str = "https://api.pdok.nl/kadaster/bestuurlijkegebieden/ogc/v1";
const MUNICIPALITY_COLLECTION: &str = "gemeentegebied";

const COUNTRY_SERVICE: &str = "PDOK CBS gebiedsindelingen";
const COUNTRY_BASE_URL: &str = "https://api.pdok.nl/cbs/gebiedsindelingen/ogc/v1";
/// The four NUTS 1 regions, generalised by CBS, which together make the land of the country.
const COUNTRY_COLLECTION: &str = "landsdeel_gegeneraliseerd";

/// Half the side of the box asked around the site centre. Any box finds the
/// municipality the point lies in; a small one keeps neighbours out of the answer.
const PROBE_HALF_SIDE_M: f64 = 5.0;

/// The municipality outline is 135 KB for Eindhoven at full detail. On a
/// 40 mm locator map a 20 m wiggle is far below a pen width.
const MUNICIPALITY_TOLERANCE_M: f64 = 20.0;

pub fn credit() -> Credit {
    Credit {
        name: "Bestuurlijke gebieden, Kadaster, via PDOK".to_string(),
        license: "CC BY 4.0".to_string(),
        url: "https://www.pdok.nl/introductie/-/article/bestuurlijke-gebieden".to_string(),
        layers: Vec::new(),
    }
}

/// For the bundled country outline.
pub fn country_credit() -> Credit {
    Credit {
        name: "CBS gebiedsindelingen, via PDOK".to_string(),
        license: "CC BY 4.0".to_string(),
        url: "https://www.pdok.nl/introductie/-/article/cbs-gebiedsindelingen".to_string(),
        layers: Vec::new(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Municipality {
    /// "Amsterdam".
    pub name: String,
    /// "GM0363", the CBS code.
    pub code: String,
    /// "Noord-Holland".
    pub province: String,
    /// In RD New, simplified for a locator map.
    pub outline: MultiPolygon<f64>,
}

/// The municipality containing `point` (RD New), or `None` at sea or abroad.
pub fn fetch_municipality(point: (f64, f64)) -> Result<Option<Municipality>> {
    let probe = Rect::new(
        Coord {
            x: point.0 - PROBE_HALF_SIDE_M,
            y: point.1 - PROBE_HALF_SIDE_M,
        },
        Coord {
            x: point.0 + PROBE_HALF_SIDE_M,
            y: point.1 + PROBE_HALF_SIDE_M,
        },
    );
    let records = pdok::fetch_records(
        SERVICE,
        BASE_URL,
        MUNICIPALITY_COLLECTION,
        probe,
        Filter::None,
    )?;
    let centre = Point::new(point.0, point.1);

    // A box on a border touches two municipalities; the point lies in one.
    let found = records
        .iter()
        .filter_map(to_municipality)
        .find(|municipality| municipality.outline.contains(&centre));
    Ok(found.map(|municipality| Municipality {
        outline: municipality.outline.simplify(&MUNICIPALITY_TOLERANCE_M),
        ..municipality
    }))
}

/// The land of the country in `year`'s regions, joined into one outline.
/// Only the outline example calls this.
pub fn fetch_country(year: i32) -> Result<MultiPolygon<f64>> {
    // The Dutch RD grid lies entirely inside this box.
    let netherlands = Rect::new(
        Coord {
            x: 0.0,
            y: 300_000.0,
        },
        Coord {
            x: 300_000.0,
            y: 650_000.0,
        },
    );
    // Every year since 2016 is in the collection, so ask for one.
    let year = year.to_string();
    let records = pdok::fetch_records(
        COUNTRY_SERVICE,
        COUNTRY_BASE_URL,
        COUNTRY_COLLECTION,
        netherlands,
        Filter::Property("jaarcode", &year),
    )?;
    let regions: Vec<MultiPolygon<f64>> = records
        .into_iter()
        .filter_map(|record| as_multi_polygon(record.geometry))
        .collect();
    let Some((first, rest)) = regions.split_first() else {
        return Err(Error::Data {
            service: COUNTRY_SERVICE.to_string(),
            detail: format!("{COUNTRY_COLLECTION} has no regions for {year}"),
        });
    };

    // Joined, so the borders between the regions do not show as lines.
    let country = rest
        .iter()
        .fold(first.clone(), |joined, region| joined.union(region));
    Ok(country)
}

fn to_municipality(record: &Record) -> Option<Municipality> {
    Some(Municipality {
        name: pdok::text(record, "naam")?,
        code: pdok::text(record, "identificatie")?,
        province: pdok::text(record, "ligt_in_provincie_naam").unwrap_or_default(),
        outline: as_multi_polygon(record.geometry.clone())?,
    })
}

fn as_multi_polygon(geometry: Geometry<f64>) -> Option<MultiPolygon<f64>> {
    match geometry {
        Geometry::Polygon(polygon) => Some(MultiPolygon::new(vec![polygon])),
        Geometry::MultiPolygon(polygons) => Some(polygons),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::polygon;
    use serde_json::json;

    #[test]
    fn a_record_becomes_a_municipality_with_its_province() {
        let record = Record {
            id: Some("3bec289b".to_string()),
            properties: json!({
                "code": "0363",
                "identificatie": "GM0363",
                "ligt_in_provincie_naam": "Noord-Holland",
                "naam": "Amsterdam"
            })
            .as_object()
            .unwrap()
            .clone(),
            geometry: Geometry::Polygon(polygon![
                (x: 0.0, y: 0.0), (x: 10.0, y: 0.0), (x: 10.0, y: 10.0), (x: 0.0, y: 10.0)
            ]),
        };
        let municipality = to_municipality(&record).unwrap();
        assert_eq!(municipality.name, "Amsterdam");
        assert_eq!(municipality.code, "GM0363");
        assert_eq!(municipality.province, "Noord-Holland");
        assert!(municipality.outline.contains(&Point::new(5.0, 5.0)));
    }
}
