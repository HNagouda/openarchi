//! BGT, the large-scale base map: roads, water, green, paving, bridges, tracks, trees.
//!
//! BGT surfaces tile the ground without gaps or overlaps, which is why a site
//! drawn from them looks clean. Buildings come from BAG instead, see `bag.rs`.

use geo::Rect;

use crate::scene::{Attributes, Credit, Feature, Layer};
use crate::sources::pdok::{self, Record};
use crate::Result;

const SERVICE: &str = "PDOK BGT";
const BASE_URL: &str = "https://api.pdok.nl/lv/bgt/ogc/v1";
/// BGT keeps history; records with an end of registration no longer exist.
const CURRENT_ONLY: &str = "eind_registratie IS NULL";

/// Every BGT collection we read, in one place. `layer_for` maps each of them.
pub const COLLECTIONS: [&str; 9] = [
    "wegdeel",
    "ondersteunendwegdeel",
    "waterdeel",
    "ondersteunendwaterdeel",
    "begroeidterreindeel",
    "onbegroeidterreindeel",
    "overbruggingsdeel",
    "spoor",
    "vegetatieobject_punt",
];

pub fn credit() -> Credit {
    Credit {
        name: "Basisregistratie Grootschalige Topografie (BGT), via PDOK".to_string(),
        license: "CC0 1.0".to_string(),
        url: "https://www.pdok.nl/introductie/-/article/basisregistratie-grootschalige-topografie-bgt-".to_string(),
        layers: Vec::new(),
    }
}

/// Every current record of `collection` touching `bbox` (RD New), still in RD New.
pub fn fetch_tile(collection: &str, bbox: Rect<f64>) -> Result<Vec<Record>> {
    pdok::fetch_records(SERVICE, BASE_URL, collection, bbox, Some(CURRENT_ONLY))
}

pub fn to_features(collection: &str, records: Vec<Record>) -> Vec<Feature> {
    records
        .into_iter()
        .map(|record| {
            let (layer, kind) = layer_for(collection, &record);
            Feature {
                layer,
                attributes: Attributes {
                    source_id: pdok::text(&record, "lokaal_id"),
                    kind,
                    ..Attributes::default()
                },
                geometry: record.geometry,
            }
        })
        .collect()
}

/// The layer for one BGT record, and its own subtype for `attributes.kind`.
/// Values we do not know go to the most neutral layer of their collection, so
/// a new BGT code still shows up instead of disappearing.
fn layer_for(collection: &str, record: &Record) -> (Layer, Option<String>) {
    let functie = pdok::text(record, "functie");
    let fysiek_voorkomen = pdok::text(record, "fysiek_voorkomen");
    match collection {
        "wegdeel" => (road_layer(functie.as_deref()), functie),
        "ondersteunendwegdeel" => (support_road_layer(fysiek_voorkomen.as_deref()), functie),
        "waterdeel" => (Layer::Water, pdok::text(record, "type")),
        "ondersteunendwaterdeel" => (Layer::WaterBank, pdok::text(record, "type")),
        "begroeidterreindeel" => (Layer::Green, fysiek_voorkomen),
        "onbegroeidterreindeel" => (
            bare_ground_layer(fysiek_voorkomen.as_deref()),
            fysiek_voorkomen,
        ),
        "overbruggingsdeel" => (Layer::Bridge, pdok::text(record, "type_overbruggingsdeel")),
        "spoor" => (Layer::RailTrack, functie),
        // Municipalities put the real kind ("boom") in plus_type; type is often "niet-bgt".
        "vegetatieobject_punt" => (
            Layer::Tree,
            pdok::text(record, "plus_type").or_else(|| pdok::text(record, "type")),
        ),
        _ => (Layer::Unpaved, None),
    }
}

// wegdeel functie. Unknown functions are drawn as plain paving.
fn road_layer(functie: Option<&str>) -> Layer {
    match functie.unwrap_or_default() {
        "rijbaan autosnelweg"
        | "rijbaan autoweg"
        | "rijbaan regionale weg"
        | "rijbaan lokale weg"
        | "inrit"
        | "OV-baan"
        | "overweg"
        | "woonerf"
        | "transitie"
        | "baan voor vliegverkeer" => Layer::Road,
        "voetpad" | "voetpad op trap" | "voetgangersgebied" | "ruiterpad" => Layer::Footpath,
        "fietspad" => Layer::Cycleway,
        "parkeervlak" => Layer::Parking,
        "spoorbaan" => Layer::RailArea,
        _ => Layer::Paved,
    }
}

// ondersteunendwegdeel (verges and traffic islands): paved strips read as paving,
// everything else, including unknown surfaces, as a verge.
fn support_road_layer(fysiek_voorkomen: Option<&str>) -> Layer {
    match fysiek_voorkomen.unwrap_or_default() {
        "open verharding" | "gesloten verharding" => Layer::Paved,
        _ => Layer::Verge,
    }
}

// onbegroeidterreindeel fysiek_voorkomen. Unknown surfaces are drawn as unpaved ground.
fn bare_ground_layer(fysiek_voorkomen: Option<&str>) -> Layer {
    match fysiek_voorkomen.unwrap_or_default() {
        "erf" => Layer::Yard,
        "open verharding" | "gesloten verharding" | "half verhard" => Layer::Paved,
        _ => Layer::Unpaved,
    }
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
    fn a_footpath_keeps_its_bgt_function_as_kind() {
        let (layer, kind) = layer_for("wegdeel", &record(json!({"functie": "voetpad"})));
        assert_eq!(layer, Layer::Footpath);
        assert_eq!(kind.as_deref(), Some("voetpad"));
    }

    #[test]
    fn road_functions_map_to_their_layers() {
        assert_eq!(road_layer(Some("rijbaan lokale weg")), Layer::Road);
        assert_eq!(road_layer(Some("fietspad")), Layer::Cycleway);
        assert_eq!(road_layer(Some("parkeervlak")), Layer::Parking);
        assert_eq!(road_layer(Some("spoorbaan")), Layer::RailArea);
    }

    #[test]
    fn an_unknown_road_function_becomes_plain_paving() {
        assert_eq!(road_layer(Some("hoverboardbaan")), Layer::Paved);
        assert_eq!(road_layer(None), Layer::Paved);
    }

    #[test]
    fn a_yard_is_a_yard_and_unknown_bare_ground_is_unpaved() {
        assert_eq!(bare_ground_layer(Some("erf")), Layer::Yard);
        assert_eq!(bare_ground_layer(Some("gesloten verharding")), Layer::Paved);
        assert_eq!(bare_ground_layer(Some("zand")), Layer::Unpaved);
        assert_eq!(bare_ground_layer(None), Layer::Unpaved);
    }

    #[test]
    fn a_paved_verge_is_paving_and_a_grass_verge_is_a_verge() {
        assert_eq!(support_road_layer(Some("open verharding")), Layer::Paved);
        assert_eq!(support_road_layer(Some("groenvoorziening")), Layer::Verge);
    }

    #[test]
    fn a_tree_takes_its_kind_from_plus_type() {
        let tree = record(json!({"type": "niet-bgt", "plus_type": "boom"}));
        let (layer, kind) = layer_for("vegetatieobject_punt", &tree);
        assert_eq!(layer, Layer::Tree);
        assert_eq!(kind.as_deref(), Some("boom"));
    }
}
