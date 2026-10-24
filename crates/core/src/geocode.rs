//! Address and place search.
//!
//! Uses PDOK Locatieserver, the national address and place index. It only knows
//! the Netherlands, which matches the only country we have map data for.

use serde::{Deserialize, Serialize};

use crate::area::LonLat;
use crate::sources::http;
use crate::{Error, Result};

const SERVICE: &str = "PDOK Locatieserver";
const SEARCH_URL: &str = "https://api.pdok.nl/bzk/locatieserver/search/v3_1/free";
/// Only the fields we read, which keeps the answer small.
const FIELDS: &str = "weergavenaam,type,centroide_ll";
/// Locatieserver serves at most 100 rows per request.
const MAX_ROWS: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Place {
    /// "Westermarkt 20, 1016DK Amsterdam".
    pub label: String,
    /// "adres", "weg", "woonplaats" and so on, as the service names it.
    pub kind: String,
    pub location: LonLat,
}

#[derive(Deserialize)]
struct Answer {
    response: Response,
}

#[derive(Deserialize)]
struct Response {
    docs: Vec<Document>,
}

#[derive(Deserialize)]
struct Document {
    weergavenaam: String,
    #[serde(rename = "type")]
    kind: String,
    centroide_ll: String,
}

/// Up to `limit` places matching `query`, best first.
pub fn search(query: &str, limit: usize) -> Result<Vec<Place>> {
    let query = query.trim();
    if query.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    let rows = limit.min(MAX_ROWS).to_string();
    let body = http::get(
        SERVICE,
        SEARCH_URL,
        &[("q", query), ("rows", rows.as_str()), ("fl", FIELDS)],
    )?;
    let answer: Answer = serde_json::from_slice(&body).map_err(|error| Error::Data {
        service: SERVICE.to_string(),
        detail: error.to_string(),
    })?;

    answer
        .response
        .docs
        .into_iter()
        .map(|document| {
            Ok(Place {
                location: parse_point(&document.centroide_ll)?,
                label: document.weergavenaam,
                kind: document.kind,
            })
        })
        .collect()
}

/// "POINT(4.88431719 52.37493637)" -> lon 4.88431719, lat 52.37493637.
fn parse_point(text: &str) -> Result<LonLat> {
    let bad_point = || Error::Data {
        service: SERVICE.to_string(),
        detail: format!("expected a point like 'POINT(lon lat)', got {text:?}"),
    };

    let inner = text
        .trim()
        .strip_prefix("POINT(")
        .and_then(|rest| rest.strip_suffix(')'))
        .ok_or_else(bad_point)?;
    let mut numbers = inner.split_whitespace().map(str::parse::<f64>);
    match (numbers.next(), numbers.next(), numbers.next()) {
        (Some(Ok(lon)), Some(Ok(lat)), None) => Ok(LonLat { lon, lat }),
        _ => Err(bad_point()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_well_known_text_point_becomes_lon_and_lat() {
        let point = parse_point("POINT(4.88431719 52.37493637)").unwrap();
        assert_eq!(
            point,
            LonLat {
                lon: 4.884_317_19,
                lat: 52.374_936_37
            }
        );
    }

    #[test]
    fn a_malformed_point_is_a_data_error() {
        assert!(matches!(
            parse_point("POINT(4.88)"),
            Err(Error::Data { .. })
        ));
        assert!(matches!(
            parse_point("LINESTRING(1 2, 3 4)"),
            Err(Error::Data { .. })
        ));
    }

    #[test]
    fn an_empty_query_finds_nothing_without_asking_the_service() {
        assert!(search("   ", 5).unwrap().is_empty());
    }
}
