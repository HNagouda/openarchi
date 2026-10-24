//! Writes the Netherlands outline bundled with the app for the locator map.
//!
//! cargo run --release -p openarchi-core --example netherlands_outline   # writes assets/places/
//!
//! Fetches the four CBS land regions (`landsdeel_gegeneraliseerd`, CBS via
//! PDOK, CC BY 4.0) for `YEAR` once, joins them, and keeps a version a
//! locator map can use: simplified to `TOLERANCE_M`, without islands and
//! holes too small to see, coordinates rounded to whole metres in RD New.
//! Run it again only when the coast or the border changes.

use std::fs;
use std::path::Path;

use geo::{Area, MapCoords, MultiPolygon, Polygon, Simplify};
use openarchi_core::sources::boundaries::{country_credit, fetch_country};
use serde_json::json;

const OUT_FILE: &str = "assets/places/netherlands.geojson";
const YEAR: i32 = 2025;
/// On a 40 mm map of a 300 km country one millimetre is 7.5 km, so a 200 m
/// wiggle is invisible, and the file stays at a few tens of kilobytes.
const TOLERANCE_M: f64 = 200.0;
/// Islands and holes smaller than this vanish at locator scale anyway.
const MIN_AREA_M2: f64 = 2_000_000.0;

fn main() {
    let country = fetch_country(YEAR).expect("fetch the CBS land regions from PDOK");
    let simplified = country.simplify(&TOLERANCE_M);
    let kept: Vec<Polygon<f64>> = simplified
        .0
        .into_iter()
        .filter(|polygon| polygon.unsigned_area() >= MIN_AREA_M2)
        .map(without_small_holes)
        .collect();
    let rounded = MultiPolygon::new(kept).map_coords(|coord| geo::Coord {
        x: coord.x.round(),
        y: coord.y.round(),
    });

    // A Feature rather than a bare geometry, so the file says where it came from.
    let credit = country_credit();
    let feature = json!({
        "type": "Feature",
        "properties": {
            "name": "Nederland",
            "crs": "EPSG:28992",
            "source": format!("{}, landsdeel_gegeneraliseerd {YEAR}", credit.name),
            "license": credit.license,
            "made_by": "crates/core/examples/netherlands_outline.rs",
        },
        "geometry": geojson::Geometry::from(&rounded),
    });
    let text = serde_json::to_string(&feature).expect("serialise the outline");
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(OUT_FILE);
    fs::create_dir_all(path.parent().expect("the asset folder")).expect("create the folder");
    fs::write(&path, &text).expect("write the outline");

    let points: usize = rounded
        .0
        .iter()
        .map(|polygon| polygon.exterior().0.len())
        .sum();
    println!(
        "{} polygons, {points} outer points, {} bytes in {}",
        rounded.0.len(),
        text.len(),
        path.display()
    );
}

fn without_small_holes(polygon: Polygon<f64>) -> Polygon<f64> {
    let (exterior, holes) = polygon.into_inner();
    let holes = holes
        .into_iter()
        .filter(|hole| Polygon::new(hole.clone(), vec![]).unsigned_area() >= MIN_AREA_M2)
        .collect();
    Polygon::new(exterior, holes)
}
