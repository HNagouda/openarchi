//! Renders every built-in diagram in every built-in theme to PNG, plus a few
//! sheet layouts, into target/render-samples/, and times a 1 km site.
//!
//! cargo run --release -p openarchi-core --example render_samples [diagram] [theme]
//! cargo run --release -p openarchi-core --example render_samples views [folder]
//!
//! The second form renders the axonometric, explode, sun path and shadow
//! samples, with timings, into `folder` (target/render-samples/views by default).
//!
//! cargo run --release -p openarchi-core --example render_samples scene <scene.json> <folder> [diagram]
//!
//! The third form renders a scene written by `fetch_site` in every theme, for
//! looking at layers the checked-in fixture does not hold.
//!
//! cargo run --release -p openarchi-core --example render_samples osm <scene.json> <name>
//!
//! The third form renders the OpenStreetMap diagrams in every theme from a
//! scene fetched with `fetch_site`, into target/render-samples/osm/. The
//! checked-in fixture predates OpenStreetMap, so these need a live fetch.
//!
//! cargo run --release -p openarchi-core --example render_samples context <scene.json> [diagram]
//!
//! The last form draws a fetched scene with its neighbourhood in every theme,
//! as plan, axonometric and explode, into target/render-samples/context/. The
//! fixture has no neighbourhood either.

mod big_site;
mod fetched;
mod layouts;
mod views;

use std::path::{Path, PathBuf};

use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::export::render_png;
use openarchi_core::project::Look;
use openarchi_core::render::RenderInput;
use openarchi_core::scene::Scene;
use openarchi_core::sheet::Sheet;

use self::big_site::time_big_site;
use self::fetched::{add_solar_year, write_context, write_osm, write_scene};
use self::layouts::{write_custom_text, write_layouts};
use self::views::write_views;

const FIXTURE: &str = "tests/fixtures/westermarkt.scene.json";
const RECORDED_SOLAR_YEAR: &str = "tests/fixtures/analysis-cache/tiles-v1/pvgis-tmy-5km/24_97.zst";
const SAMPLE_WIDTH_PX: u32 = 1800;
const VIEWS_ARGUMENT: &str = "views";
const SCENE_ARGUMENT: &str = "scene";
const OSM_ARGUMENT: &str = "osm";
const CONTEXT_ARGUMENT: &str = "context";

fn main() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = manifest.join("../../target/render-samples");
    std::fs::create_dir_all(&out_dir).expect("create the samples folder");
    let text =
        std::fs::read_to_string(manifest.join(FIXTURE)).expect("read the Westermarkt fixture");
    let mut scene: Scene = serde_json::from_str(&text).expect("parse the Westermarkt fixture");
    add_solar_year(&mut scene, manifest);

    let only_diagram = std::env::args().nth(1);
    let only_theme = std::env::args().nth(2);
    if only_diagram.as_deref() == Some(SCENE_ARGUMENT) {
        let arguments: Vec<String> = std::env::args().skip(2).collect();
        let [scene_path, folder, rest @ ..] = arguments.as_slice() else {
            panic!("usage: render_samples scene <scene.json> <folder> [diagram]");
        };
        write_scene(Path::new(scene_path), Path::new(folder), rest.first());
        return;
    }
    if only_diagram.as_deref() == Some(OSM_ARGUMENT) {
        let args: Vec<String> = std::env::args().skip(2).collect();
        let [scene_path, name] = args.as_slice() else {
            panic!("usage: render_samples osm <scene.json> <name>");
        };
        let text = std::fs::read_to_string(scene_path).expect("read the scene");
        let osm_scene: Scene = serde_json::from_str(&text).expect("parse the scene");
        write_osm(&osm_scene, name, &out_dir.join(OSM_ARGUMENT));
        return;
    }
    if only_diagram.as_deref() == Some(CONTEXT_ARGUMENT) {
        let args: Vec<String> = std::env::args().skip(2).collect();
        let [scene_path, rest @ ..] = args.as_slice() else {
            panic!("usage: render_samples context <scene.json> [diagram]");
        };
        let text = std::fs::read_to_string(scene_path).expect("read the scene");
        let context_scene: Scene = serde_json::from_str(&text).expect("parse the scene");
        let diagram_id = rest.first().map_or("context", String::as_str);
        write_context(&context_scene, diagram_id, &out_dir.join(CONTEXT_ARGUMENT));
        return;
    }
    if only_diagram.as_deref() == Some(VIEWS_ARGUMENT) {
        let views_dir = only_theme
            .map(PathBuf::from)
            .unwrap_or_else(|| out_dir.join("views"));
        write_views(&scene, &views_dir);
        return;
    }
    let sheet = Sheet {
        title: "Westermarkt".to_string(),
        subtitle: "Amsterdam, 300 m around the Westerkerk".to_string(),
        ..Sheet::default()
    };

    // A diagram with variants gets a sample per variant: "air-quality-pm10--paper.png".
    let drawings: Vec<_> = builtin_diagrams()
        .into_iter()
        .filter(|diagram| only_diagram.as_deref().is_none_or(|id| id == diagram.id))
        .flat_map(|diagram| {
            diagram
                .drawings()
                .into_iter()
                .map(move |(variant, _)| (diagram.clone(), variant))
        })
        .collect();
    for (diagram, variant) in drawings {
        let stem = match &variant {
            Some(variant) => format!("{}-{variant}", diagram.id),
            None => diagram.id.clone(),
        };
        for theme in builtin_themes() {
            if only_theme.as_deref().is_some_and(|id| id != theme.id) {
                continue;
            }
            let look = Look {
                diagram_id: diagram.id.clone(),
                variant: variant.clone(),
                theme_id: theme.id.clone(),
                sheet: sheet.clone(),
                ..Look::default()
            };
            let input = RenderInput {
                scene: &scene,
                diagram: &diagram,
                theme: &theme,
                look: &look,
                pins: &[],
                highlights: &[],
                model: None,
            };
            let png = render_png(&input, SAMPLE_WIDTH_PX).expect("render the sample");
            std::fs::write(out_dir.join(format!("{stem}--{}.png", theme.id)), png)
                .expect("write the sample");
        }
    }
    if only_diagram.is_none() {
        write_layouts(&scene, &out_dir);
        write_custom_text(&scene, &out_dir);
        time_big_site(&scene, &out_dir);
    }
    println!("samples in {}", out_dir.display());
}
