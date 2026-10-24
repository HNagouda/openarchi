//! Writes what the app's dev-only IPC mock serves: the catalog, the
//! Westermarkt fixture's area and layers, and every diagram in every theme as
//! an SVG preview and a PNG thumbnail. With it the front end runs in a plain
//! browser, which is how the screenshots in docs/screenshots are taken.
//!
//! cargo run --release -p openarchi-core --example app_fixtures   # writes app/dev-fixtures/

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use openarchi_core::analysis::{self, PARTS};
use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::export::render_png;
use openarchi_core::project::Look;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::scene::{Layer, Scene};
use openarchi_core::sheet::Sheet;
use openarchi_core::sources::cache::TileCache;
use openarchi_core::sources::grid::Tile;
use serde_json::json;

const FIXTURE: &str = "tests/fixtures/westermarkt.scene.json";
const RECORDED_ANALYSIS: &str = "tests/fixtures/analysis-cache/tiles-v1";
/// Wide enough for a gallery card on a Retina screen.
const THUMBNAIL_WIDTH_PX: u32 = 560;

fn main() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = manifest.join("../../app/dev-fixtures");
    fs::create_dir_all(out_dir.join("previews")).expect("create the previews folder");
    fs::create_dir_all(out_dir.join("thumbs")).expect("create the thumbs folder");

    let text = fs::read_to_string(manifest.join(FIXTURE)).expect("read the Westermarkt fixture");
    let scene: Scene = serde_json::from_str(&text).expect("parse the Westermarkt fixture");
    write_catalog(&scene, &out_dir);
    write_analysis(&scene, &out_dir);

    let sheet = Sheet {
        title: "Westermarkt".to_string(),
        subtitle: "Amsterdam, around the Westerkerk".to_string(),
        ..Sheet::default()
    };
    for diagram in builtin_diagrams() {
        for theme in builtin_themes() {
            let look = Look {
                diagram_id: diagram.id.clone(),
                theme_id: theme.id.clone(),
                sheet: sheet.clone(),
                ..Look::default()
            };
            let input = RenderInput {
                scene: &scene,
                diagram: &diagram,
                theme: &theme,
                look: &look,
            };
            let name = format!("{}--{}", diagram.id, theme.id);
            let svg = render_svg(&input).expect("render the preview");
            fs::write(out_dir.join(format!("previews/{name}.svg")), svg)
                .expect("write the preview");
            let png = render_png(&input, THUMBNAIL_WIDTH_PX).expect("render the thumbnail");
            fs::write(out_dir.join(format!("thumbs/{name}.png")), png)
                .expect("write the thumbnail");
        }
    }

    println!("fixtures in {}", out_dir.display());
}

/// Every analysis part in every theme, as `get_analysis` returns it, from the
/// answers recorded in tests/fixtures/analysis-cache, so no service is asked.
fn write_analysis(scene: &Scene, out_dir: &Path) {
    let folder = out_dir.join("analysis");
    fs::create_dir_all(&folder).expect("create the analysis folder");
    let cache_dir = tempfile::TempDir::new().expect("a temporary cache");
    let cache = TileCache::at(cache_dir.path());
    let recorded = Path::new(env!("CARGO_MANIFEST_DIR")).join(RECORDED_ANALYSIS);
    for dataset in fs::read_dir(&recorded).expect("the recorded analysis answers") {
        let dataset = dataset.expect("a dataset folder").path();
        let name = dataset
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .to_string();
        for file in fs::read_dir(&dataset).expect("the dataset's files") {
            let path = file.expect("a recorded answer").path();
            let stem = path
                .file_stem()
                .expect("a stem")
                .to_string_lossy()
                .to_string();
            let (column, row) = stem.split_once('_').expect("a file named column_row");
            let tile = Tile {
                column: column.parse().expect("a column"),
                row: row.parse().expect("a row"),
            };
            let bytes =
                zstd::decode_all(fs::read(&path).expect("read").as_slice()).expect("unpack");
            // Written fresh, so the cache never takes a recorded answer for a stale one.
            cache.write(&name, tile, &bytes).expect("seed the cache");
        }
    }

    for theme in builtin_themes() {
        for part in PARTS {
            let sections =
                analysis::part(part, scene, &theme, &cache).expect("draw the analysis part");
            let part_name = serde_json::to_value(part).expect("a part name");
            let file = folder.join(format!(
                "{}--{}.json",
                part_name.as_str().expect("text"),
                theme.id
            ));
            fs::write(file, serde_json::to_string(&sections).expect("serialise"))
                .expect("write the part");
        }
    }
}

/// The catalog as the app's `get_catalog` returns it, plus the fixture's area and layers.
fn write_catalog(scene: &Scene, out_dir: &Path) {
    let layers: Vec<_> = Layer::ALL
        .iter()
        .map(|layer| json!({ "id": layer, "label": layer.label() }))
        .collect();
    let present: BTreeSet<Layer> = scene.features.iter().map(|feature| feature.layer).collect();
    let fixture = json!({
        "catalog": {
            "diagrams": builtin_diagrams(),
            "themes": builtin_themes(),
            "warnings": [],
            "layers": layers,
        },
        "area": scene.area,
        "credits": scene.credits,
        "layers_present": present,
    });
    fs::write(
        out_dir.join("fixture.json"),
        serde_json::to_string(&fixture).expect("serialise the fixture"),
    )
    .expect("write the fixture");
}
