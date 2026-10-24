//! Writes what the app's dev-only IPC mock serves: the catalog, the
//! Westermarkt fixture's area and layers, and every diagram in every theme as
//! an SVG preview and a PNG thumbnail. With it the front end runs in a plain
//! browser, which is how the screenshots in docs/screenshots are taken.
//!
//! cargo run --release -p openarchi-core --example app_fixtures   # writes app/dev-fixtures/

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::export::render_png;
use openarchi_core::project::Look;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::scene::{Layer, Scene};
use openarchi_core::sheet::Sheet;
use serde_json::json;

const FIXTURE: &str = "tests/fixtures/westermarkt.scene.json";
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
