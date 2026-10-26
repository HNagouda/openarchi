//! What the 3D view draws of the site with the model, from a sheet of each of
//! a few diagrams in every built-in palette, so the mock can show the view
//! the way each sheet opens it. The sun on the equinox, and the aerial photo,
//! which is fetched from PDOK into the tile cache the examples share.

use std::fs;
use std::path::Path;
use std::time::Instant;

use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::model::Model;
use openarchi_core::project::Look;
use openarchi_core::render::RenderInput;
use openarchi_core::scene::Scene;
use openarchi_core::sources::cache::TileCache;
use openarchi_core::view3d;
use openarchi_core::view3d::photo;

/// A plain sheet and two that colour the buildings. The mock falls back to
/// the plain one for every other diagram.
const DIAGRAMS: [&str; 3] = ["context", "building-age", "building-use"];
const FOLDER: &str = "view3d";
/// The cache `fetch_site` fills, so the photo is downloaded once.
const TILE_CACHE: &str = "../../target/tile-cache";

pub fn write(scene: &Scene, model: &Model, out_dir: &Path) {
    let folder = out_dir.join(FOLDER);
    fs::create_dir_all(&folder).expect("make the 3D view folder");
    let diagrams = builtin_diagrams();
    for theme in builtin_themes() {
        for diagram_id in DIAGRAMS {
            let diagram = diagrams
                .iter()
                .find(|diagram| diagram.id == diagram_id)
                .expect("a built-in diagram");
            let look = Look {
                diagram_id: diagram_id.to_string(),
                theme_id: theme.id.clone(),
                ..Look::default()
            };
            let input = RenderInput {
                scene,
                diagram,
                theme: &theme,
                look: &look,
                pins: &[],
                highlights: &[],
                model: Some(model),
            };
            let started = Instant::now();
            let view = view3d::view(&input).expect("the 3D view");
            let text = serde_json::to_string(&view).expect("the 3D view as JSON");
            if theme.id == "paper" && diagram_id == "context" {
                let triangles: usize = view.buildings.iter().map(|s| s.triangle_count).sum();
                println!(
                    "3D view: {triangles} building triangles, {:.0} KB of JSON in {:.0} ms",
                    text.len() as f64 / 1024.0,
                    started.elapsed().as_secs_f64() * 1000.0
                );
            }
            let name = format!("{diagram_id}-{}.json", theme.id);
            fs::write(folder.join(name), text).expect("write the 3D view");
        }
    }

    write_photo(scene, out_dir);
    let day = view3d::sun_day(scene, 3, 21);
    fs::write(
        out_dir.join("sun-day.json"),
        serde_json::to_string(&day).expect("the sun's day as JSON"),
    )
    .expect("write the sun's day");
}

/// The photo, or a note when PDOK cannot be reached: the mock then shows real colours without it.
fn write_photo(scene: &Scene, out_dir: &Path) {
    let cache = TileCache::at(&Path::new(env!("CARGO_MANIFEST_DIR")).join(TILE_CACHE));
    let started = Instant::now();
    match photo::photo(scene, &cache) {
        Ok(photo) => {
            let text = serde_json::to_string(&photo).expect("the photo as JSON");
            let tiles: usize = photo.levels.iter().map(|level| level.tiles.len()).sum();
            println!(
                "aerial photo: {tiles} tiles, {:.0} KB of JSON in {:.1} s",
                text.len() as f64 / 1024.0,
                started.elapsed().as_secs_f64()
            );
            fs::write(out_dir.join("aerial-photo.json"), text).expect("write the photo");
        }
        Err(error) => println!("no aerial photo fixture: {error}"),
    }
}
