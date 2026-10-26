//! Site edits on the Westermarkt fixture: the test L-shaped block placed
//! north of the Westerkerk, with the trees in its way taken away under it, and
//! by hand two more trees and a building removed, a tree moved and a row of
//! three trees planted. Each sample is drawn before and after, framed on the
//! plot, with its SVG size and time. Writes PNGs to target/render-samples/edits/.
//!
//! cargo run --release -p openarchi-core --example edits_samples

use std::path::Path;
use std::time::Instant;

use openarchi_core::area::{Area, LonLat};
use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::crs::rd_to_wgs84;
use openarchi_core::edits::{edited, standing_trees, SiteEdits};
use openarchi_core::export::render_png;
use openarchi_core::model::{self, Model, UpAxis};
use openarchi_core::project::Look;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::scene::{Layer, Scene};
use openarchi_core::sheet::Sheet;
use openarchi_core::view::{SunMoment, View, ViewMode};

const SAMPLE_WIDTH_PX: u32 = 1600;
/// The plot north of the Westerkerk, in metres from the scene's origin, as model_samples has it.
const PLOT: (f64, f64) = (12.0, 62.0);
const TURN_DEG: f64 = 20.0;
/// The sheets frame this much ground around the plot, at this scale.
const FRAME_RADIUS_M: f64 = 70.0;
const FRAME_SCALE: u32 = 500;
/// The planted row: its first tree, in metres from the plot, and the spacing east.
const ROW_START: (f64, f64) = (-40.0, -32.0);
const ROW_SPACING_M: f64 = 7.0;
const ROW_LENGTH: usize = 3;
/// How far the moved tree goes, east and north.
const MOVE_M: (f64, f64) = (0.0, -8.0);

/// File name, diagram, theme and view mode.
const SAMPLES: [(&str, &str, &str, ViewMode); 5] = [
    ("plan--context--paper", "context", "paper", ViewMode::Plan),
    ("plan--shadow--paper", "shadow", "paper", ViewMode::Plan),
    (
        "plan--sun-hours--paper",
        "sun-hours",
        "paper",
        ViewMode::Plan,
    ),
    (
        "axo--context--paper",
        "context",
        "paper",
        ViewMode::Axonometric,
    ),
    (
        "axo--building-use--pastel",
        "building-use",
        "pastel",
        ViewMode::Axonometric,
    ),
];

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let scene: Scene = serde_json::from_slice(
        &std::fs::read(root.join("tests/fixtures/westermarkt.scene.json")).expect("read the scene"),
    )
    .expect("parse the scene");
    let model = placed_block(&scene, root);
    let edits = sample_edits(&scene, &model);
    println!(
        "edits: {}",
        serde_json::to_string(&edits).expect("edits as JSON")
    );
    let started = Instant::now();
    let after = edited(&scene, &edits).into_owned();
    println!(
        "edited scene of {} features in {:.1} ms",
        scene.features.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    let out_dir = root.join("../../target/render-samples/edits");
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
        let look = framed_look(&scene, diagram_id, theme_id, mode);
        let before = RenderInput {
            scene: &scene,
            diagram,
            theme,
            look: &look,
            pins: &[],
            highlights: &[],
            model: None,
        };
        let with_edits = RenderInput {
            scene: &after,
            model: Some(&model),
            ..before
        };
        for (suffix, input) in [("before", &before), ("after", &with_edits)] {
            let started = Instant::now();
            let svg = render_svg(input).expect("render the sheet");
            let ms = started.elapsed().as_secs_f64() * 1000.0;
            println!(
                "{name}--{suffix}: SVG {:.1} KB in {ms:.0} ms",
                svg.len() as f64 / 1024.0
            );
            let png = render_png(input, SAMPLE_WIDTH_PX).expect("rasterise the sheet");
            std::fs::write(out_dir.join(format!("{name}--{suffix}.png")), png)
                .expect("write the sample");
        }
    }
    println!("samples in {}", out_dir.display());
}

/// The L-shaped test block on the plot, turned.
fn placed_block(scene: &Scene, root: &Path) -> Model {
    let path = root.join("tests/fixtures/models/l-block-y-up.obj");
    let mut placement = model::Placement::centre_of(scene);
    placement.x += PLOT.0;
    placement.y += PLOT.1;
    placement.rotation_deg = TURN_DEG;
    model::import(&path, None, Some(UpAxis::Y), scene, Some(placement)).expect("import the block")
}

/// What the model stands on, and the hand edits around the plot.
fn sample_edits(scene: &Scene, model: &Model) -> SiteEdits {
    let picks = model::picked(model, scene, &SiteEdits::default());
    let mut edits = SiteEdits::default().with_model_picks(&picks);
    let plot = (scene.origin.0 + PLOT.0, scene.origin.1 + PLOT.1);
    let mut near: Vec<_> = standing_trees(scene, &edits)
        .into_iter()
        .filter(|tree| !edits.is_removed(&tree.id))
        .collect();
    let distance = |x: f64, y: f64| (x - PLOT.0).hypot(y - PLOT.1);
    near.sort_by(|a, b| distance(a.at.x(), a.at.y()).total_cmp(&distance(b.at.x(), b.at.y())));
    edits = edits.with_removed(&near[0].id).with_removed(&near[1].id);
    let moved = &near[2];
    edits = edits.with_moved(
        &moved.id,
        moved.at.x() + scene.origin.0 + MOVE_M.0,
        moved.at.y() + scene.origin.1 + MOVE_M.1,
    );
    if let Some(building) = nearest_building(scene, &edits) {
        edits = edits.with_removed(&building);
    }
    for index in 0..ROW_LENGTH {
        let east = ROW_START.0 + ROW_SPACING_M * index as f64;
        (edits, _) = edits.with_planted(plot.0 + east, plot.1 + ROW_START.1);
    }
    edits
        .check(scene)
        .expect("the sample edits stand on the site");

    edits
}

/// The building nearest the plot that the model does not already replace.
fn nearest_building(scene: &Scene, edits: &SiteEdits) -> Option<String> {
    scene
        .features
        .iter()
        .filter(|feature| feature.layer == Layer::Building)
        .filter_map(|feature| {
            let id = feature.attributes.source_id.clone()?;
            let point = geo::InteriorPoint::interior_point(&feature.geometry)?;
            let distance = (point.x() - PLOT.0).hypot(point.y() - PLOT.1);
            (!edits.is_removed(&id)).then_some((distance, id))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, id)| id)
}

/// A sheet framing the plot, so trees read as more than specks.
fn framed_look(scene: &Scene, diagram_id: &str, theme_id: &str, mode: ViewMode) -> Look {
    let centre = rd_to_wgs84(scene.origin.0 + PLOT.0, scene.origin.1 + PLOT.1);
    Look {
        diagram_id: diagram_id.to_string(),
        theme_id: theme_id.to_string(),
        sheet: Sheet {
            title: "Westermarkt".to_string(),
            subtitle: "Site edits around a proposal north of the Westerkerk".to_string(),
            scale: FRAME_SCALE,
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
        selection: Some(Area::Circle {
            center: LonLat {
                lon: centre.lon,
                lat: centre.lat,
            },
            radius_m: FRAME_RADIUS_M,
        }),
        ..Look::default()
    }
}
