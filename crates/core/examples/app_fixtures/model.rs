//! The imported model the mock answers with: the test L-shaped block on a
//! plot north of the Westerkerk, in the shape the app's `ModelView` has
//! (app/src-tauri/src/commands/model.rs), with what it covers.

use std::fs;
use std::path::Path;

use openarchi_core::model::{self, Model, Placement, UpAxis};
use openarchi_core::scene::Scene;
use serde_json::json;

/// A plot of about 740 m² north of the Westerkerk, in metres from the scene's origin.
const PLOT: (f64, f64) = (12.0, 62.0);
const TURN_DEG: f64 = 20.0;
const MODEL_FILE: &str = "tests/fixtures/models/l-block-y-up.obj";

pub fn placed_model(scene: &Scene) -> Model {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(MODEL_FILE);
    let placement = Placement {
        x: scene.origin.0 + PLOT.0,
        y: scene.origin.1 + PLOT.1,
        rotation_deg: TURN_DEG,
        base_m: None,
    };
    model::import(&path, None, Some(UpAxis::Y), scene, Some(placement)).expect("import the block")
}

pub fn write_model(scene: &Scene, out_dir: &Path) {
    let model = placed_model(scene);
    let footprint: Vec<Vec<[f64; 2]>> = model
        .shape()
        .footprint
        .iter()
        .map(|polygon| polygon.exterior().0.iter().map(|c| [c.x, c.y]).collect())
        .collect();
    let view = json!({
        "key": "model-1",
        "file_name": model.file_name,
        "unit": model.unit,
        "up": model.up,
        "size_m": model.mesh.size(),
        "triangles": model.mesh.triangles.len(),
        "placement": model.placement,
        "replaced": model.replaced,
        "footprint": footprint,
        "ground_m": model::ground_m(&model, scene),
        "covered": model::covered_buildings(&model, scene),
    });
    fs::write(
        out_dir.join("model.json"),
        serde_json::to_string_pretty(&view).expect("write the model as JSON"),
    )
    .expect("write the model fixture");
}
