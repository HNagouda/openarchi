//! Shared reading of PDOK OGC API Features collections: paging and GeoJSON.
//!
//! Every PDOK vector service used here (BGT, BAG, BRK) speaks the same API, so
//! one function fetches any collection inside a bounding box. We ask for RD New
//! both for the box and for the answer, which spares any reprojection.
//!
//! Tiling and caching happen one level up, in `site/download.rs`, so this
//! file only knows how to read every page of one box.

use geo::{Geometry, Rect};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::sources::http;
use crate::{Error, Result};

const RD_NEW_URI: &str = "http://www.opengis.net/def/crs/EPSG/0/28992";
/// The largest page PDOK serves.
const PAGE_SIZE: &str = "1000";
/// A guard against a paging loop; 200 pages is 200 000 features, far beyond one tile.
const MAX_PAGES: usize = 200;

/// One feature as the service gave it, geometry in RD New. This is what the
/// tile cache stores, so a change in how we read a record needs no refetch.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// How a request narrows a collection beyond the box.
pub enum Filter<'a> {
    None,
    /// A CQL2 text filter, for example "eind_registratie IS NULL".
    Cql(&'a str),
    /// One property equal to one value, sent as a plain query parameter:
    /// ("jaarcode", "2024"). Prefer this to the same test in CQL: on the CBS
    /// grid `jaarcode=2024` answers in 0.6 s, while `filter=jaarcode=2024`
    /// took 20 to 30 s per tile and sometimes failed with 500 (2026-10-24).
    Property(&'a str, &'a str),
}

/// Every record of `collection` at `base_url` that touches `bbox`.
pub fn fetch_records(
    service: &str,
    base_url: &str,
    collection: &str,
    bbox: Rect<f64>,
    filter: Filter<'_>,
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
    match filter {
        Filter::None => {}
        Filter::Cql(text) => query.push(("filter", text)),
        Filter::Property(name, value) => query.push((name, value)),
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
