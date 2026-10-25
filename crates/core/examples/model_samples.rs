//! The test L-shaped block placed on the Westermarkt fixture, drawn in plan,
//! standing up and in the shadow diagram, in a few palettes, each with its SVG
//! size and time next to the same sheet without the model. Writes PNGs to
//! target/render-samples/model/.
//!
//! cargo run --release -p openarchi-core --example model_samples

use std::path::Path;
use std::time::Instant;

use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::export::render_png;
use openarchi_core::model::{self, Model, UpAxis};
use openarchi_core::project::Look;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::scene::Scene;
use openarchi_core::sheet::Sheet;
use openarchi_core::view::{SunMoment, View, ViewMode};

const SAMPLE_WIDTH_PX: u32 = 1600;
/// A plot of about 740 m² north of the Westerkerk, in metres from the scene's origin.
const PLOT: (f64, f64) = (12.0, 62.0);
const TURN_DEG: f64 = 20.0;

/// File name, diagram, theme and view mode.
const SAMPLES: [(&str, &str, &str, ViewMode); 10] = [
    ("plan--context--paper", "context", "paper", ViewMode::Plan),
    (
        "plan--sun-hours--paper",
        "sun-hours",
        "paper",
        ViewMode::Plan,
    ),
    ("plan--sky-view--paper", "sky-view", "paper", ViewMode::Plan),
    (
        "plan--building-height--midnight",
        "building-height",
        "midnight",
        ViewMode::Plan,
    ),
    (
        "axo--context--paper",
        "context",
        "paper",
        ViewMode::Axonometric,
    ),
    (
        "axo--building-height--midnight",
        "building-height",
        "midnight",
        ViewMode::Axonometric,
    ),
    (
        "axo--building-use--pastel",
        "building-use",
        "pastel",
        ViewMode::Axonometric,
    ),
    ("axo--shadow--mono", "shadow", "mono", ViewMode::Axonometric),
    ("shadow--paper", "shadow", "paper", ViewMode::Plan),
    ("shadow--blueprint", "shadow", "blueprint", ViewMode::Plan),
];

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let scene: Scene = serde_json::from_slice(
        &std::fs::read(root.join("tests/fixtures/westermarkt.scene.json")).expect("read the scene"),
    )
    .expect("parse the scene");
    let model = placed_block(&scene, root);
    println!(
        "model: {} triangles, {} faces, replaces {:?}",
        model.mesh.triangles.len(),
        model.shape().faces.len(),
        model.replaced
    );

    let out_dir = root.join("../../target/render-samples/model");
    std::fs::create_dir_all(&out_dir).expect("create the samples folder");
    let diagrams = builtin_diagrams();
    let themes = builtin_themes();
    for (name, diagram_id, theme_id, mode) in SAMPLES {
        let diagram = diagrams
            .iter()
            .find(|d| d.id == diagram_id)
            .expect("diagram exists");
        let theme = themes
            .iter()
            .find(|t| t.id == theme_id)
            .expect("theme exists");
        let look = Look {
            diagram_id: diagram_id.to_string(),
            theme_id: theme_id.to_string(),
            sheet: Sheet {
                title: "Westermarkt".to_string(),
                subtitle: "A proposal north of the Westerkerk".to_string(),
                ..Sheet::default()
            },
            view: View {
                mode,
                rotation_deg: if mode == ViewMode::Axonometric {
                    30.0
                } else {
                    0.0
                },
                ..View::default()
            },
            sun: SunMoment {
                hour: if diagram_id == "shadow" { 16 } else { 13 },
                ..SunMoment::default()
            },
            ..Look::default()
        };
        let without = RenderInput {
            scene: &scene,
            diagram,
            theme,
            look: &look,
            pins: &[],
            highlights: &[],
            model: None,
        };
        let with = RenderInput {
            model: Some(&model),
            ..without
        };

        let started = Instant::now();
        let bare = render_svg(&without).expect("render without the model");
        let bare_ms = started.elapsed().as_secs_f64() * 1000.0;
        let started = Instant::now();
        let svg = render_svg(&with).expect("render with the model");
        let model_ms = started.elapsed().as_secs_f64() * 1000.0;
        println!(
            "{name}: SVG {:.1} KB in {model_ms:.0} ms with the model, {:.1} KB in {bare_ms:.0} ms without",
            svg.len() as f64 / 1024.0,
            bare.len() as f64 / 1024.0,
        );
        let png = render_png(&with, SAMPLE_WIDTH_PX).expect("rasterise the sheet");
        std::fs::write(out_dir.join(format!("{name}.png")), png).expect("write the sample");
    }
    println!("samples in {}", out_dir.display());
}

/// The L-shaped test block on the plot, turned, replacing what it covers.
fn placed_block(scene: &Scene, root: &Path) -> Model {
    let path = root.join("tests/fixtures/models/l-block-y-up.obj");
    let mut placement = model::Placement::centre_of(scene);
    placement.x += PLOT.0;
    placement.y += PLOT.1;
    placement.rotation_deg = TURN_DEG;
    model::import(&path, None, Some(UpAxis::Y), scene, Some(placement)).expect("import the block")
}
