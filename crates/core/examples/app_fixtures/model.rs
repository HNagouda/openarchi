//! The imported model the mock answers with: the test L-shaped block on a
//! plot north of the Westerkerk, in the shape the app's `ModelView` has
//! (app/src-tauri/src/commands/model.rs), with what it covers.

use std::fs;
use std::path::Path;

use std::time::Instant;

use openarchi_core::catalog::builtin_themes;
use openarchi_core::model::{self, Model, Placement, UpAxis};
use openarchi_core::scene::Scene;
use openarchi_core::view3d;
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
    write_view3d(scene, &model, out_dir);
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

/// What the 3D view draws of the site with the model, in the paper palette,
/// and the sun on the equinox, timed, since the view waits on both when it opens.
fn write_view3d(scene: &Scene, model: &Model, out_dir: &Path) {
    let theme = builtin_themes()
        .into_iter()
        .find(|theme| theme.id == "paper")
        .expect("the paper palette");
    let started = Instant::now();
    let view = view3d::view(scene, &theme, Some(model)).expect("the 3D view");
    let text = serde_json::to_string(&view).expect("the 3D view as JSON");
    println!(
        "3D view: {} building triangles, {:.0} KB of JSON in {:.0} ms",
        view.buildings.triangle_count,
        text.len() as f64 / 1024.0,
        started.elapsed().as_secs_f64() * 1000.0
    );
    fs::write(out_dir.join("view3d.json"), text).expect("write the 3D view");
    let day = view3d::sun_day(scene, 3, 21);
    fs::write(
        out_dir.join("sun-day.json"),
        serde_json::to_string(&day).expect("the sun's day as JSON"),
    )
    .expect("write the sun's day");
}
