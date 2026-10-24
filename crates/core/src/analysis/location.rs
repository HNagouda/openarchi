//! Where the site is: its address, municipality and province, and the
//! outlines the locator map draws.
//!
//! The country outline ships with the app, so the locator draws without a
//! network. The municipality and the address come from PDOK once per spot
//! and are kept in the tile cache, keyed by a 10 m cell around the site centre.

use std::sync::OnceLock;

use geo::{MapCoords, MultiPolygon, Polygon};
use serde::Serialize;

use super::cached;
use crate::area::LonLat;
use crate::crs::rd_to_wgs84;
use crate::geocode;
use crate::scene::{Credit, Scene};
use crate::sources::boundaries::{self, Municipality};
use crate::sources::cache::TileCache;
use crate::sources::grid::Tile;
use crate::{Error, Result};

/// Made by `examples/netherlands_outline.rs`, which says how; RD New metres.
const COUNTRY_GEOJSON: &str = include_str!("../../assets/places/netherlands.geojson");

/// Sites closer than this share their cached municipality and address.
const SPOT_M: f64 = 10.0;
const MUNICIPALITY_DATASET: &str = "pdok-gemeente-10m";
const ADDRESS_DATASET: &str = "pdok-address-10m";

#[derive(Debug, Clone, Serialize)]
pub struct Location {
    pub centre: LonLat,
    /// RD New metres.
    pub rd: (f64, f64),
    /// The registered address nearest the centre: "Westermarkt 68, Amsterdam".
    pub address: Option<String>,
    pub municipality: Option<String>,
    /// "GM0363".
    pub municipality_code: Option<String>,
    pub province: Option<String>,
    #[serde(skip)]
    pub municipality_outline: Option<MultiPolygon<f64>>,
    /// The drawn site in RD New.
    #[serde(skip)]
    pub site: Polygon<f64>,
    /// Lookups that failed; the rest of the location still stands.
    pub warnings: Vec<String>,
}

/// The location of the scene's site. Never fails: a lookup that does not
/// work leaves its field empty and adds a warning.
pub fn locate(scene: &Scene, cache: &TileCache) -> Location {
    let rd = scene.origin;
    let centre = rd_to_wgs84(rd.0, rd.1);
    let spot = Tile {
        column: (rd.0 / SPOT_M).floor() as i64,
        row: (rd.1 / SPOT_M).floor() as i64,
    };
    let mut warnings = Vec::new();

    let municipality = match municipality(cache, spot, rd) {
        Ok((found, cache_warning)) => {
            warnings.extend(cache_warning);
            found
        }
        Err(error) => {
            warnings.push(format!("The municipality could not be looked up: {error}"));
            None
        }
    };
    let address = match address(cache, spot, centre) {
        Ok((found, cache_warning)) => {
            warnings.extend(cache_warning);
            found
        }
        Err(error) => {
            warnings.push(format!("The address could not be looked up: {error}"));
            None
        }
    };

    Location {
        centre,
        rd,
        address,
        municipality: municipality.as_ref().map(|found| found.name.clone()),
        municipality_code: municipality.as_ref().map(|found| found.code.clone()),
        province: municipality.as_ref().map(|found| found.province.clone()),
        municipality_outline: municipality.map(|found| found.outline),
        site: scene.boundary.map_coords(|coord| geo::Coord {
            x: coord.x + rd.0,
            y: coord.y + rd.1,
        }),
        warnings,
    }
}

pub fn credits() -> Vec<Credit> {
    vec![boundaries::credit(), boundaries::country_credit()]
}

/// The Netherlands in RD New, parsed once.
pub fn country() -> &'static MultiPolygon<f64> {
    static COUNTRY: OnceLock<MultiPolygon<f64>> = OnceLock::new();
    COUNTRY.get_or_init(|| {
        parse_country(COUNTRY_GEOJSON).expect("the bundled Netherlands outline is valid GeoJSON")
    })
}

fn parse_country(text: &str) -> Result<MultiPolygon<f64>> {
    let invalid = |detail: String| Error::Invalid(format!("Netherlands outline: {detail}"));
    let feature: geojson::Feature = text
        .parse()
        .map_err(|error: geojson::Error| invalid(error.to_string()))?;
    let geometry = feature
        .geometry
        .ok_or_else(|| invalid("no geometry".to_string()))?;
    MultiPolygon::<f64>::try_from(geometry).map_err(|error| invalid(error.to_string()))
}

fn municipality(
    cache: &TileCache,
    spot: Tile,
    rd: (f64, f64),
) -> Result<(Option<Municipality>, Option<String>)> {
    let (bytes, warning) = cached(cache, MUNICIPALITY_DATASET, spot, || {
        let found = boundaries::fetch_municipality(rd)?;
        serde_json::to_vec(&found).map_err(|error| Error::Invalid(error.to_string()))
    })?;
    let found =
        serde_json::from_slice(&bytes).map_err(|error| Error::Invalid(error.to_string()))?;
    Ok((found, warning))
}

fn address(
    cache: &TileCache,
    spot: Tile,
    centre: LonLat,
) -> Result<(Option<String>, Option<String>)> {
    let (bytes, warning) = cached(cache, ADDRESS_DATASET, spot, || {
        let found = geocode::nearest_address(centre)?;
        serde_json::to_vec(&found).map_err(|error| Error::Invalid(error.to_string()))
    })?;
    let found =
        serde_json::from_slice(&bytes).map_err(|error| Error::Invalid(error.to_string()))?;
    Ok((found, warning))
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{Area, Contains, Point};

    #[test]
    fn the_bundled_outline_holds_amsterdam_and_not_the_north_sea() {
        let country = country();
        assert!(country.contains(&Point::new(120_733.0, 487_521.0)));
        assert!(!country.contains(&Point::new(50_000.0, 520_000.0)));
        // The land of the Netherlands is about 34,000 km².
        let square_km = country.unsigned_area() / 1e6;
        assert!((32_000.0..36_000.0).contains(&square_km), "{square_km}");
    }
}
