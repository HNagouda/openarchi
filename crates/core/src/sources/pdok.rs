//! Shared reading of PDOK OGC API Features collections: paging and GeoJSON.
//!
//! Every PDOK vector service used here (BGT, BAG, BRK) speaks the same API, so
//! one function fetches any collection inside a bounding box. We ask for RD New
//! both for the box and for the answer, which spares any reprojection.
//!
//! PDOK takes a few seconds per page of 1000 features and pages through a
//! cursor, one page after the other. So a large box is split into tiles that
//! page side by side, and features on a tile edge are kept once.

use std::collections::HashSet;
use std::thread;

use geo::{Coord, Geometry, Rect};
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::sources::http;
use crate::{Error, Result};

const RD_NEW_URI: &str = "http://www.opengis.net/def/crs/EPSG/0/28992";
/// The largest page PDOK serves.
const PAGE_SIZE: &str = "1000";
/// Side of the tiles a large query is split into. A 1 km radius site becomes
/// four tiles, which brought one BGT collection from about 60 s to about 11 s.
const TILE_M: f64 = 1000.0;
/// A guard against a paging loop; 200 pages is 200 000 features, far beyond a 4 km² site.
const MAX_PAGES: usize = 200;

/// One feature as the service gave it, geometry in RD New.
#[derive(Debug, Clone)]
pub struct Record {
    /// The service's feature id, unique within the collection.
    pub id: Option<String>,
    pub properties: Map<String, Value>,
    pub geometry: Geometry<f64>,
}

#[derive(Deserialize)]
struct Page {
    features: Vec<geojson::Feature>,
    #[serde(default)]
    links: Vec<Link>,
}

#[derive(Deserialize)]
struct Link {
    rel: String,
    href: String,
}

/// Every record of `collection` at `base_url` that touches `bbox`. `filter` is
/// an optional CQL2 text filter, for example "eind_registratie IS NULL".
pub fn fetch_collection(
    service: &str,
    base_url: &str,
    collection: &str,
    bbox: Rect<f64>,
    filter: Option<&str>,
) -> Result<Vec<Record>> {
    let per_tile: Vec<Result<Vec<Record>>> = thread::scope(|scope| {
        let handles: Vec<_> = tiles(bbox)
            .into_iter()
            .map(|tile| {
                scope.spawn(move || fetch_tile(service, base_url, collection, tile, filter))
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("PDOK tile thread panicked"))
            .collect()
    });

    let mut seen = HashSet::new();
    let mut records = Vec::new();
    for tile in per_tile {
        for record in tile? {
            let is_new = match &record.id {
                Some(id) => seen.insert(id.clone()),
                None => true,
            };
            if is_new {
                records.push(record);
            }
        }
    }

    Ok(records)
}

/// `bbox` cut into a grid of equal tiles no wider or taller than `TILE_M`.
fn tiles(bbox: Rect<f64>) -> Vec<Rect<f64>> {
    let columns = (bbox.width() / TILE_M).ceil().max(1.0) as usize;
    let rows = (bbox.height() / TILE_M).ceil().max(1.0) as usize;
    let tile_width = bbox.width() / columns as f64;
    let tile_height = bbox.height() / rows as f64;

    let mut tiles = Vec::with_capacity(columns * rows);
    for row in 0..rows {
        for column in 0..columns {
            let min = Coord {
                x: bbox.min().x + column as f64 * tile_width,
                y: bbox.min().y + row as f64 * tile_height,
            };
            let max = Coord {
                x: min.x + tile_width,
                y: min.y + tile_height,
            };
            tiles.push(Rect::new(min, max));
        }
    }

    tiles
}

fn fetch_tile(
    service: &str,
    base_url: &str,
    collection: &str,
    bbox: Rect<f64>,
    filter: Option<&str>,
) -> Result<Vec<Record>> {
    let first_url = format!("{base_url}/collections/{collection}/items");
    let bbox_text = format!(
        "{},{},{},{}",
        bbox.min().x,
        bbox.min().y,
        bbox.max().x,
        bbox.max().y
    );
    let mut query = vec![
        ("f", "json"),
        ("limit", PAGE_SIZE),
        ("bbox", bbox_text.as_str()),
        ("bbox-crs", RD_NEW_URI),
        ("crs", RD_NEW_URI),
    ];
    if let Some(filter) = filter {
        query.push(("filter", filter));
    }

    let mut records = Vec::new();
    let mut page = read_page(service, &first_url, &query)?;
    for _ in 0..MAX_PAGES {
        records.extend(page.features.into_iter().filter_map(to_record));
        // The next link already carries every query parameter, plus a cursor.
        let next = page.links.into_iter().find(|link| link.rel == "next");
        match next {
            Some(link) => page = read_page(service, &link.href, &[])?,
            None => return Ok(records),
        }
    }

    Err(Error::Data {
        service: service.to_string(),
        detail: format!("{collection} kept paging past {MAX_PAGES} pages"),
    })
}

fn read_page(service: &str, url: &str, query: &[(&str, &str)]) -> Result<Page> {
    let body = http::get(service, url, query)?;
    serde_json::from_slice(&body).map_err(|error| Error::Data {
        service: service.to_string(),
        detail: format!("{url}: {error}"),
    })
}

// Features without a geometry, or with one geo cannot hold, are of no use to a map.
fn to_record(feature: geojson::Feature) -> Option<Record> {
    let geometry = Geometry::<f64>::try_from(feature.geometry?).ok()?;
    let id = feature.id.map(|id| match id {
        geojson::feature::Id::String(text) => text,
        geojson::feature::Id::Number(number) => number.to_string(),
    });
    Some(Record {
        id,
        properties: feature.properties.unwrap_or_default(),
        geometry,
    })
}

/// A text property, `None` when missing, null or empty.
pub fn text(record: &Record, key: &str) -> Option<String> {
    match record.properties.get(key) {
        Some(Value::String(value)) if !value.is_empty() => Some(value.clone()),
        Some(Value::Number(value)) => Some(value.to_string()),
        _ => None,
    }
}

/// An integer property, also when the service sends it as text.
pub fn integer(record: &Record, key: &str) -> Option<i64> {
    match record.properties.get(key) {
        Some(Value::Number(value)) => value.as_i64(),
        Some(Value::String(value)) => value.trim().parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_small_box_is_one_tile_and_a_large_one_is_split_evenly() {
        let small = Rect::new(Coord { x: 0.0, y: 0.0 }, Coord { x: 600.0, y: 600.0 });
        assert_eq!(tiles(small), vec![small]);

        let large = Rect::new(
            Coord { x: 0.0, y: 0.0 },
            Coord {
                x: 2200.0,
                y: 1000.0,
            },
        );
        let split = tiles(large);
        assert_eq!(split.len(), 3);
        assert!(split
            .iter()
            .all(|tile| (tile.width() - 2200.0 / 3.0).abs() < 1e-9));
        assert_eq!(split[2].max(), large.max());
    }

    #[test]
    fn a_page_parses_into_records_and_finds_the_next_link() {
        let body = r#"{
            "type": "FeatureCollection",
            "links": [{"rel": "next", "href": "https://example.org/next"}],
            "features": [
                {"type": "Feature", "properties": {"functie": "voetpad", "bouwjaar": "1920"},
                 "geometry": {"type": "Point", "coordinates": [120000.5, 487000.25]}},
                {"type": "Feature", "properties": {}, "geometry": null}
            ]
        }"#;

        let page: Page = serde_json::from_str(body).unwrap();
        assert_eq!(page.links[0].href, "https://example.org/next");

        let records: Vec<Record> = page.features.into_iter().filter_map(to_record).collect();
        assert_eq!(records.len(), 1);
        assert_eq!(text(&records[0], "functie").as_deref(), Some("voetpad"));
        assert_eq!(integer(&records[0], "bouwjaar"), Some(1920));
        assert_eq!(text(&records[0], "missing"), None);
    }
}
