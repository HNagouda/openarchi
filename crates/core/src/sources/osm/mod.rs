//! Businesses, facilities, transit stops and the walk network from
//! OpenStreetMap, through the Overpass API.
//!
//! A site sends one combined query: the points, the stops with the route
//! relations that list them, and every `highway` way within a walk of the
//! site. Overpass is a volunteer-run server that is often busy, so one query
//! per site rather than per tile or per diagram is what its usage policy asks.
//!
//! The answer is cached like a tile, but per site: the query is centred on the
//! middle of the grid tile the site centre falls in, and reaches far enough
//! that any site centred anywhere in that tile is covered. So redrawing a site
//! or nudging it reuses the cached answer, without splitting one query into
//! the hundred tiles a 1.5 km radius would need.

mod stops;
pub mod tags;

use std::collections::BTreeMap;
use std::time::Duration;

use geo::{Geometry, Point};
use serde::Deserialize;

use self::stops::{Route, StopNode};
use self::tags::{Tags, AMENITIES, LEISURE, PARKING_AMENITY, ROUTES, TOURISM};
use crate::area::LonLat;
use crate::crs::{rd_to_wgs84, wgs84_to_rd};
use crate::scene::{Attributes, Credit, Feature, Layer};
use crate::sources::grid::{Tile, TILE_M};
use crate::sources::http;
use crate::walk::REACH_M;
use crate::{Error, Result};

pub use self::stops::line_order;
pub use self::tags::{BUSINESS_GROUPS, FACILITY_GROUPS, TRANSIT_GROUPS};

const SERVICE: &str = "OpenStreetMap Overpass";
const URL: &str = "https://overpass-api.de/api/interpreter";
/// What Overpass may spend on the query. A 1.5 km radius around the Westerkerk,
/// the densest case, took 45 s on a quiet afternoon.
const SERVER_TIMEOUT_S: u32 = 180;
/// The client waits a little longer than the server, so a slow answer is not cut off.
const CLIENT_TIMEOUT: Duration = Duration::from_secs(200);
/// Pauses before each retry when Overpass says it is busy (429, 502, 503,
/// 504). Its usage policy asks clients to back off rather than retry at once.
const BACKOFF: [Duration; 2] = [Duration::from_secs(10), Duration::from_secs(30)];
/// From a tile's centre to its corner, so a site centred anywhere in the tile
/// is still covered by a query around the tile's centre.
const TILE_HALF_DIAGONAL_M: f64 = TILE_M * std::f64::consts::FRAC_1_SQRT_2;

/// Every layer OpenStreetMap provides, in one place, for the credit.
pub const LAYERS: [Layer; 5] = [
    Layer::Business,
    Layer::Facility,
    Layer::TransitStop,
    Layer::WalkPath,
    Layer::WalkBand,
];

/// The ODbL asks for this credit on every produced work, such as a printed diagram.
pub fn credit() -> Credit {
    Credit {
        name: "© OpenStreetMap contributors".to_string(),
        license: "ODbL 1.0".to_string(),
        url: "https://www.openstreetmap.org/copyright".to_string(),
        layers: LAYERS.to_vec(),
    }
}

/// How far a site's query reaches around the centre of `tile`, rounded up to
/// whole tiles so that sites of similar size share one cached answer.
pub fn radius_for(site_radius_m: f64) -> u32 {
    let needed = REACH_M.max(site_radius_m) + TILE_HALF_DIAGONAL_M;
    ((needed / TILE_M).ceil() * TILE_M) as u32
}

/// What one site needs from OpenStreetMap, in RD New.
#[derive(Debug, Default)]
pub struct Osm {
    /// Businesses and facilities as points, with `group`, `kind` and `name`.
    pub places: Vec<Feature>,
    /// One point per transit stop, with `group`, `name` and `lines`.
    pub stops: Vec<Feature>,
    pub ways: Vec<Way>,
}

/// One walkable way: its node ids, which tell where ways meet, and its shape.
#[derive(Debug, Clone)]
pub struct Way {
    pub nodes: Vec<i64>,
    pub points: Vec<(f64, f64)>,
    /// Worth drawing as context; pavements and crossings are walked but not drawn.
    pub is_drawn: bool,
}

/// The raw Overpass answer for the query around `tile`. It is checked before
/// it is returned, so a timed-out query never ends up in the cache.
pub fn fetch(tile: Tile, radius_m: u32) -> Result<Vec<u8>> {
    let middle = tile.rect().center();
    let query = query(rd_to_wgs84(middle.x, middle.y), radius_m);
    let body = http::post_form(SERVICE, URL, &[("data", &query)], CLIENT_TIMEOUT, &BACKOFF)?;
    parse(&body)?;
    Ok(body)
}

/// The combined query: places, stops, the routes through the stops, and ways.
fn query(centre: LonLat, radius_m: u32) -> String {
    let around = format!("(around:{radius_m},{:.6},{:.6})", centre.lat, centre.lon);
    let amenities: Vec<&str> = AMENITIES
        .iter()
        .map(|(value, _, _)| *value)
        .chain([PARKING_AMENITY])
        .collect();
    let tourism: Vec<&str> = TOURISM.iter().map(|(value, _, _)| *value).collect();
    let one_of = |values: &[&str]| format!("^({})$", values.join("|"));

    format!(
        "[out:json][timeout:{SERVER_TIMEOUT_S}];\n\
         (\n\
         nwr[\"shop\"]{around};\n\
         nwr[\"office\"]{around};\n\
         nwr[\"craft\"]{around};\n\
         nwr[\"amenity\"~\"{amenity}\"]{around};\n\
         nwr[\"tourism\"~\"{tourism}\"]{around};\n\
         nwr[\"leisure\"~\"{leisure}\"]{around};\n\
         )->.places;\n\
         (\n\
         node[\"public_transport\"~\"^(platform|stop_position|station)$\"]{around};\n\
         node[\"highway\"=\"bus_stop\"]{around};\n\
         node[\"railway\"~\"^(tram_stop|station|halt)$\"]{around};\n\
         )->.stops;\n\
         .places out center tags;\n\
         .stops out body;\n\
         rel(bn.stops)[\"type\"=\"route\"][\"route\"~\"{routes}\"];\n\
         out body;\n\
         way[\"highway\"]{around};\n\
         out geom qt;\n",
        amenity = one_of(&amenities),
        tourism = one_of(&tourism),
        leisure = one_of(&LEISURE),
        routes = one_of(&ROUTES),
    )
}

/// An Overpass JSON answer, split into places, stops and ways in RD New.
pub fn parse(body: &[u8]) -> Result<Osm> {
    let invalid = |detail: String| Error::Network {
        service: SERVICE.to_string(),
        detail,
    };
    let answer: Answer = serde_json::from_slice(body)
        .map_err(|error| invalid(format!("the answer is not Overpass JSON: {error}")))?;
    // A query that runs out of time still answers 200, with what it found so far and a remark.
    if let Some(remark) = answer.remark.filter(|remark| remark.contains("error")) {
        return Err(invalid(remark));
    }

    let mut osm = Osm::default();
    let mut stop_nodes = Vec::new();
    let mut routes = Vec::new();
    for element in answer.elements {
        match element.kind.as_str() {
            "relation" if !element.members.is_empty() => routes.extend(route(&element)),
            "way" if !element.geometry.is_empty() => osm.ways.extend(way(&element)),
            "node" if tags::is_stop(&element.tags) => {
                if let Some(position) = element.position() {
                    stop_nodes.push(StopNode {
                        id: element.id,
                        position,
                        tags: element.tags,
                    });
                }
            }
            _ => osm.places.extend(place(&element)),
        }
    }
    osm.stops = stops::merge(&stop_nodes, &routes);

    Ok(osm)
}

fn place(element: &Element) -> Option<Feature> {
    let (layer, group) = tags::place_group(&element.tags)?;
    let (x, y) = element.position()?;
    Some(Feature {
        layer,
        geometry: Geometry::Point(Point::new(x, y)),
        attributes: Attributes {
            source_id: Some(format!("osm/{}/{}", element.kind, element.id)),
            kind: tags::place_kind(&element.tags),
            name: element.tags.get("name").cloned(),
            group: Some(group.to_string()),
            ..Attributes::default()
        },
    })
}

fn route(element: &Element) -> Option<Route> {
    let mode = tags::route_mode(element.tags.get("route")?)?;
    Some(Route {
        mode,
        line: element.tags.get("ref").cloned().unwrap_or_default(),
        nodes: element
            .members
            .iter()
            .filter(|member| member.kind == "node")
            .map(|member| member.id)
            .collect(),
    })
}

fn way(element: &Element) -> Option<Way> {
    if !tags::is_walkable(&element.tags) {
        return None;
    }
    let points: Vec<(f64, f64)> = element
        .geometry
        .iter()
        .flatten()
        .map(|point| point.to_rd())
        .collect();
    // A way with a node outside the answer has a gap in its geometry, and
    // its node ids no longer line up with its points.
    let is_whole = points.len() == element.nodes.len() && points.len() >= 2;
    is_whole.then(|| Way {
        nodes: element.nodes.clone(),
        points,
        is_drawn: tags::is_drawn(&element.tags),
    })
}

/// The parts of an Overpass JSON answer openarchi reads.
#[derive(Deserialize)]
struct Answer {
    elements: Vec<Element>,
    #[serde(default)]
    remark: Option<String>,
}

#[derive(Deserialize)]
struct Element {
    /// "node", "way" or "relation".
    #[serde(rename = "type")]
    kind: String,
    id: i64,
    lat: Option<f64>,
    lon: Option<f64>,
    /// The middle of a way or relation, from `out center`.
    center: Option<LatLon>,
    #[serde(default)]
    nodes: Vec<i64>,
    /// A way's points, from `out geom`. A point outside the query is null.
    #[serde(default)]
    geometry: Vec<Option<LatLon>>,
    #[serde(default)]
    members: Vec<Member>,
    #[serde(default)]
    tags: Tags,
}

#[derive(Deserialize)]
struct LatLon {
    lat: f64,
    lon: f64,
}

#[derive(Deserialize)]
struct Member {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "ref")]
    id: i64,
}

impl Element {
    /// A node's own position or a way's centre, in RD New.
    fn position(&self) -> Option<(f64, f64)> {
        match (self.lat, self.lon, &self.center) {
            (Some(lat), Some(lon), _) => Some(LatLon { lat, lon }.to_rd()),
            (_, _, Some(center)) => Some(center.to_rd()),
            _ => None,
        }
    }
}

impl LatLon {
    fn to_rd(&self) -> (f64, f64) {
        wgs84_to_rd(LonLat {
            lon: self.lon,
            lat: self.lat,
        })
    }
}

/// Places of one kind counted by group, in the order of `groups`, for the
/// ratios and the walk counts.
pub fn count_by_group<'a>(
    features: impl Iterator<Item = &'a Feature>,
    groups: &[(&str, &str)],
) -> Vec<usize> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for feature in features {
        if let Some(group) = feature.attributes.group.as_deref() {
            *counts.entry(group).or_default() += 1;
        }
    }
    groups
        .iter()
        .map(|(group, _)| counts.get(group).copied().unwrap_or(0))
        .collect()
}

#[cfg(test)]
mod tests;
