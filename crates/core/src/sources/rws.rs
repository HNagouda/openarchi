//! Rijkswaterstaat's water data for sections: the measured bed of the large
//! waters it manages, and the target levels it publishes along fairways.
//!
//! Neither covers every water. The bed survey covers rivers, canals and lakes
//! in Rijkswaterstaat's care, not city canals or ponds. Target levels exist
//! only where a weir or lock holds the water at one level: the Amsterdam
//! canals at the Amstelland level, the Maas-Waal canal behind the Grave weir.
//! A free-flowing river like the Waal has no target level, because its level
//! follows the river's discharge. What a section draws without them is in
//! `render/section_water.rs`; what was searched and not used is in
//! `docs/data-sources.md`.

use std::time::Duration;

use geo::{Geometry, LineString, Rect};

use crate::scene::{Credit, WaterLevel};
use crate::sources::edition::ASK_TIMEOUT;
use crate::sources::grid::Tile;
use crate::sources::raster::Grid;
use crate::sources::{http, wcs};
use crate::{Error, Result};

const SERVICE: &str = "Rijkswaterstaat";

/// "Bathymetrie Nederland - binnenwateren 1 mtr": bed heights in metres above
/// NAP, 1 m cells, one coverage per monthly edition. Cells nobody surveyed are nodata.
const BED_URL: &str =
    "https://geo.rijkswaterstaat.nl/services/ogc/gdr/bodemhoogte_1mtr_historie/ows";
/// Every edition's id is this followed by its month: "…_202602".
const BED_PREFIX: &str = "bodemhoogte_1mtr_historie__bodemhoogte_1mtr_";
/// The newest edition when this was written, asked for when the list cannot be read.
pub const KNOWN_BED_ID: &str = "bodemhoogte_1mtr_historie__bodemhoogte_1mtr_202602";

/// Target levels ("streefpeil") of the fairway network, as lines along the
/// fairway with the level in metres above NAP.
const LEVELS_URL: &str =
    "https://geo.rijkswaterstaat.nl/services/ogc/gdr/fis_vnds/ogc/features/v1/collections/streefpeil/items";
const RD_NEW_URI: &str = "http://www.opengis.net/def/crs/EPSG/0/28992";
/// A neighbourhood holds a few dozen fairway stretches; this is far more than any has.
const LEVELS_LIMIT: &str = "1000";
/// Asked once per fetch, so a slow answer may wait longer than an edition list.
const LEVELS_TIMEOUT: Duration = Duration::from_secs(30);

/// Every bed edition the service lists, for picking the newest.
pub fn list_bed_ids() -> Result<Vec<String>> {
    wcs::list_coverage_ids(SERVICE, BED_URL, ASK_TIMEOUT)
}

/// The newest bed edition in `ids`. The month in the id sorts as text.
pub fn newest_bed(ids: &[String]) -> Option<String> {
    ids.iter()
        .filter(|id| id.starts_with(BED_PREFIX))
        .max()
        .cloned()
}

/// The surveyed bed in one tile, RD New, NaN where it was not surveyed.
pub fn fetch_bed(id: &str, tile: Tile) -> Result<Grid> {
    let coverage = wcs::Coverage {
        service: SERVICE,
        url: BED_URL,
        id,
        axes: ("X", "Y"),
        scale_factor: None,
    };
    wcs::fetch(&coverage, tile.rect())
}

/// Every target level whose fairway reaches into `rect` (RD New), in RD New.
pub fn fetch_levels(rect: Rect<f64>) -> Result<Vec<WaterLevel>> {
    let bbox = format!(
        "{},{},{},{}",
        rect.min().x,
        rect.min().y,
        rect.max().x,
        rect.max().y
    );
    let query = [
        ("f", "json"),
        ("limit", LEVELS_LIMIT),
        ("bbox", bbox.as_str()),
        ("bbox-crs", RD_NEW_URI),
        ("crs", RD_NEW_URI),
    ];
    let body = http::get_once_within(SERVICE, LEVELS_URL, &query, LEVELS_TIMEOUT)?;
    levels_from(&body)
}

/// The levels in a GeoJSON answer. A fairway without a number is left out.
pub fn levels_from(body: &[u8]) -> Result<Vec<WaterLevel>> {
    let collection: geojson::FeatureCollection =
        serde_json::from_slice(body).map_err(|error| Error::Data {
            service: SERVICE.to_string(),
            detail: format!("the target levels are not GeoJSON: {error}"),
        })?;

    let mut levels = Vec::new();
    for feature in collection.features {
        let Some(level_m) = feature.property("value").and_then(|value| value.as_f64()) else {
            continue;
        };
        let name = feature
            .property("name")
            .and_then(|name| name.as_str())
            .unwrap_or_default()
            .to_string();
        let Some(geometry) = feature.geometry else {
            continue;
        };
        let lines = match Geometry::<f64>::try_from(geometry) {
            Ok(Geometry::LineString(line)) => vec![line],
            Ok(Geometry::MultiLineString(lines)) => lines.0,
            _ => continue,
        };
        for line in lines {
            levels.push(WaterLevel {
                name: name.clone(),
                level_m,
                line,
            });
        }
    }

    Ok(levels)
}

pub fn credit() -> Credit {
    Credit {
        name: "Rijkswaterstaat bed survey and target levels".to_string(),
        license: "CC0 1.0".to_string(),
        url: "https://geo.rijkswaterstaat.nl".to_string(),
        layers: Vec::new(),
    }
}

/// A line kept in the scene's frame.
pub fn localised(line: LineString<f64>, origin: (f64, f64)) -> LineString<f64> {
    line.0
        .into_iter()
        .map(|coord| (coord.x - origin.0, coord.y - origin.1))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_newest_bed_edition_is_the_latest_month() {
        let ids = [
            format!("{BED_PREFIX}202512"),
            format!("{BED_PREFIX}202602"),
            "something_else_209901".to_string(),
            format!("{BED_PREFIX}202601"),
        ];
        assert_eq!(newest_bed(&ids), Some(format!("{BED_PREFIX}202602")));
    }

    #[test]
    fn a_target_level_reads_with_its_name_and_line() {
        let body = br#"{"type": "FeatureCollection", "features": [
            {"type": "Feature", "properties": {"name": "Amstellands boezempeil", "value": -0.4},
             "geometry": {"type": "LineString", "coordinates": [[120500, 487500], [120600, 487550]]}},
            {"type": "Feature", "properties": {"name": "No number", "value": null},
             "geometry": {"type": "LineString", "coordinates": [[0, 0], [1, 1]]}}
        ]}"#;
        let levels = levels_from(body).unwrap();
        assert_eq!(levels.len(), 1);
        assert_eq!(levels[0].name, "Amstellands boezempeil");
        assert_eq!(levels[0].level_m, -0.4);
        assert_eq!(levels[0].line.0.len(), 2);
    }

    #[test]
    #[ignore = "asks Rijkswaterstaat"]
    fn the_waal_at_nijmegen_has_a_surveyed_bed() {
        let ids = list_bed_ids().unwrap();
        let id = newest_bed(&ids).unwrap();
        // Tile 752, 1716 is X 188000 to 188250, Y 429000 to 429250.
        let grid = fetch_bed(
            &id,
            Tile {
                column: 752,
                row: 1716,
            },
        )
        .unwrap();
        assert!(grid.values.iter().any(|value| value.is_finite()));
    }

    #[test]
    #[ignore = "asks Rijkswaterstaat"]
    fn the_amsterdam_canals_have_a_target_level() {
        let rect = Rect::new((120400.0, 487300.0), (121100.0, 487800.0));
        let levels = fetch_levels(rect).unwrap();
        assert!(levels.iter().any(|level| level.level_m == -0.4));
    }
}
