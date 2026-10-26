//! Every tree and building of the fixture in RD New metres, for the mock's
//! edit tool: it picks, lists and draws site edits from these the way
//! `openarchi_core::edits` does from the scene.

use std::fs;
use std::path::Path;

use geo::Geometry;
use openarchi_core::scene::{polygons, Layer, Scene};
use serde_json::{json, Value};

pub fn write_site_things(scene: &Scene, out_dir: &Path) {
    let (east, north) = scene.origin;
    let rd = |x: f64, y: f64| [x + east, y + north];
    let mut trees: Vec<Value> = Vec::new();
    let mut buildings: Vec<Value> = Vec::new();
    for feature in &scene.features {
        let Some(id) = &feature.attributes.source_id else {
            continue;
        };
        match (feature.layer, &feature.geometry) {
            (Layer::Tree, Geometry::Point(point)) => trees.push(json!({
                "id": id,
                "kind": feature.attributes.kind,
                "at": rd(point.x(), point.y()),
            })),
            (Layer::Building, geometry) => {
                let outline: Vec<Vec<[f64; 2]>> = polygons(geometry)
                    .iter()
                    .map(|polygon| {
                        polygon
                            .exterior()
                            .points()
                            .map(|p| rd(p.x(), p.y()))
                            .collect()
                    })
                    .collect();
                buildings.push(json!({
                    "id": id,
                    "year_built": feature.attributes.year_built,
                    "height_m": feature.attributes.height_m,
                    "outline": outline,
                }));
            }
            _ => {}
        }
    }

    let things = json!({ "origin": [east, north], "trees": trees, "buildings": buildings });
    fs::write(
        out_dir.join("site-things.json"),
        serde_json::to_string(&things).expect("write the trees and buildings as JSON"),
    )
    .expect("write the site things fixture");
}
