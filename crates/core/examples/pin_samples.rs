//! Renders the Westermarkt fixture with one pin of every kind, in every theme,
//! in the three views, and as a sheet framing a sub-selection, into
//! target/render-samples/pins/. For looking at pins and sub-selections.
//!
//! cargo run --release -p openarchi-core --example pin_samples

use std::path::Path;

use openarchi_core::area::Area;
use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::crs::rd_to_wgs84;
use openarchi_core::export::render_png;
use openarchi_core::pin::{Pin, PinIcon};
use openarchi_core::project::Look;
use openarchi_core::render::{fitting_scale, render_svg, RenderInput};
use openarchi_core::scene::Scene;
use openarchi_core::view::ViewMode;

const FIXTURE: &str = "tests/fixtures/westermarkt.scene.json";
const SAMPLE_WIDTH_PX: u32 = 1800;
/// The sub-selection: 70 m around the Westerkerk tower, west of the centre.
const SELECTION_OFFSET_M: (f64, f64) = (-60.0, -10.0);
const SELECTION_RADIUS_M: f64 = 70.0;

/// One pin of each kind, in metres from the site centre, with its label.
const PINS: [(PinIcon, &str, f64, f64); 6] = [
    (PinIcon::Pin, "Westerkerk", -60.0, -10.0),
    (PinIcon::Star, "Anne Frank House", -40.0, 60.0),
    (PinIcon::Flag, "Tram stop", 70.0, -30.0),
    (PinIcon::Dot, "", 120.0, 120.0),
    (PinIcon::Number, "Entrance", -20.0, -110.0),
    (PinIcon::Number, "", 150.0, -60.0),
];

fn main() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = manifest.join("../../target/render-samples/pins");
    std::fs::create_dir_all(&out_dir).expect("create the samples folder");
    let text = std::fs::read_to_string(manifest.join(FIXTURE)).expect("read the fixture");
    let scene: Scene = serde_json::from_str(&text).expect("parse the fixture");
    let pins = pins(&scene);

    for theme in builtin_themes() {
        let look = Look {
            diagram_id: "building-age".to_string(),
            theme_id: theme.id.clone(),
            ..Look::default()
        };
        write(
            &scene,
            &look,
            &pins,
            &out_dir.join(format!("plan--{}.png", theme.id)),
        );
    }

    for (name, mode) in [
        ("axo", ViewMode::Axonometric),
        ("explode", ViewMode::Explode),
    ] {
        let mut look = Look {
            diagram_id: "building-use".to_string(),
            ..Look::default()
        };
        look.view.mode = mode;
        write(
            &scene,
            &look,
            &pins,
            &out_dir.join(format!("{name}--paper.png")),
        );
    }

    let (east, north) = SELECTION_OFFSET_M;
    let mut detail = Look {
        diagram_id: "figure-ground".to_string(),
        theme_id: "ink".to_string(),
        selection: Some(Area::Circle {
            center: rd_to_wgs84(scene.origin.0 + east, scene.origin.1 + north),
            radius_m: SELECTION_RADIUS_M,
        }),
        ..Look::default()
    };
    detail.sheet.scale = fitting_scale(&scene, &detail).expect("fit the selection");
    detail.sheet.title = format!("Westerkerk at 1:{}", detail.sheet.scale);
    write(&scene, &detail, &pins, &out_dir.join("selection--ink.png"));
    std::fs::write(
        out_dir.join("selection--ink.svg"),
        svg(&scene, &detail, &pins),
    )
    .expect("write the SVG");

    println!("samples in {}", out_dir.display());
}

fn pins(scene: &Scene) -> Vec<Pin> {
    PINS.iter()
        .enumerate()
        .map(|(index, &(icon, label, east, north))| Pin {
            id: format!("pin-{}", index + 1),
            icon,
            label: label.to_string(),
            x: scene.origin.0 + east,
            y: scene.origin.1 + north,
        })
        .collect()
}

fn svg(scene: &Scene, look: &Look, pins: &[Pin]) -> String {
    let (diagram, theme) = resolved(look);
    render_svg(&RenderInput {
        scene,
        diagram: &diagram,
        theme: &theme,
        look,
        pins,
    })
    .expect("render")
}

fn write(scene: &Scene, look: &Look, pins: &[Pin], path: &Path) {
    let (diagram, theme) = resolved(look);
    let input = RenderInput {
        scene,
        diagram: &diagram,
        theme: &theme,
        look,
        pins,
    };
    let png = render_png(&input, SAMPLE_WIDTH_PX).expect("render");
    std::fs::write(path, png).expect("write the sample");
}

fn resolved(
    look: &Look,
) -> (
    openarchi_core::diagram::Diagram,
    openarchi_core::theme::Theme,
) {
    let diagram = builtin_diagrams()
        .into_iter()
        .find(|diagram| diagram.id == look.diagram_id)
        .expect("diagram exists");
    let theme = builtin_themes()
        .into_iter()
        .find(|theme| theme.id == look.theme_id)
        .expect("theme exists");
    (diagram, theme)
}
