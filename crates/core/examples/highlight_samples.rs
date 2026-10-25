//! Renders the Westermarkt fixture with a few highlights, in every theme, in
//! plan, axonometric and explode, with "dim the rest", and as a sheet opened
//! from a highlight, into target/render-samples/highlights/. For looking at
//! highlights and pin sizes.
//!
//! cargo run --release -p openarchi-core --example highlight_samples

use std::path::Path;

use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::export::render_png;
use openarchi_core::highlight::{Highlight, HighlightShape, Outline};
use openarchi_core::pin::{Pin, PinIcon, PinSize};
use openarchi_core::project::Look;
use openarchi_core::render::{fitting_scale, render_svg, RenderInput};
use openarchi_core::scene::Scene;
use openarchi_core::view::ViewMode;

const FIXTURE: &str = "tests/fixtures/westermarkt.scene.json";
const SAMPLE_WIDTH_PX: u32 = 1800;

fn main() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = manifest.join("../../target/render-samples/highlights");
    std::fs::create_dir_all(&out_dir).expect("create the samples folder");
    let text = std::fs::read_to_string(manifest.join(FIXTURE)).expect("read the fixture");
    let scene: Scene = serde_json::from_str(&text).expect("parse the fixture");
    let highlights = highlights(&scene);
    let pins = pins(&scene);

    for theme in builtin_themes() {
        let look = Look {
            diagram_id: "building-age".to_string(),
            theme_id: theme.id.clone(),
            ..Look::default()
        };
        let path = out_dir.join(format!("plan--{}.png", theme.id));
        write(&scene, &look, &pins, &highlights, &path);
    }

    let mut dimmed = highlights.clone();
    dimmed[0].dims_rest = true;
    let look = Look {
        diagram_id: "figure-ground".to_string(),
        ..Look::default()
    };
    write(
        &scene,
        &look,
        &pins,
        &dimmed,
        &out_dir.join("dim--paper.png"),
    );

    for (name, mode) in [
        ("axo", ViewMode::Axonometric),
        ("explode", ViewMode::Explode),
    ] {
        let mut look = Look {
            diagram_id: "building-use".to_string(),
            ..Look::default()
        };
        look.view.mode = mode;
        look.view.plates = vec!["building-use".to_string(), "context".to_string()];
        let path = out_dir.join(format!("{name}--paper.png"));
        write(&scene, &look, &pins, &dimmed, &path);
    }

    let mut opened = Look {
        diagram_id: "context".to_string(),
        theme_id: "ink".to_string(),
        selection: Some(highlights[0].shape.to_area()),
        ..Look::default()
    };
    opened.sheet.scale = fitting_scale(&scene, &opened).expect("fit the highlight");
    opened.sheet.title = "Plot".to_string();
    write(
        &scene,
        &opened,
        &pins,
        &highlights,
        &out_dir.join("opened--ink.png"),
    );
    let svg = svg(&scene, &opened, &pins, &highlights);
    std::fs::write(out_dir.join("opened--ink.svg"), svg).expect("write the SVG");

    println!("samples in {}", out_dir.display());
}

/// The plot on the square, a proposed house on it, and a round square to the north-east.
fn highlights(scene: &Scene) -> Vec<Highlight> {
    let (x, y) = scene.origin;
    vec![
        Highlight {
            id: "highlight-1".to_string(),
            label: "Plot".to_string(),
            shape: HighlightShape::Polygon {
                points: vec![
                    [x - 40.0, y - 30.0],
                    [x + 10.0, y - 35.0],
                    [x + 20.0, y + 15.0],
                    [x - 30.0, y + 25.0],
                ],
            },
            outline: Outline::Solid,
            dims_rest: false,
            shows_area: true,
        },
        Highlight {
            id: "highlight-2".to_string(),
            label: "New house".to_string(),
            shape: HighlightShape::Rectangle {
                min: [x - 20.0, y - 15.0],
                max: [x - 2.0, y + 2.0],
            },
            outline: Outline::Dashed,
            dims_rest: false,
            shows_area: false,
        },
        Highlight {
            id: "highlight-3".to_string(),
            label: "Square to redesign".to_string(),
            shape: HighlightShape::Circle {
                center: [x + 90.0, y + 80.0],
                radius_m: 35.0,
            },
            outline: Outline::Solid,
            dims_rest: false,
            shows_area: false,
        },
    ]
}

/// A pin of every size, so they can be compared beside the highlights.
fn pins(scene: &Scene) -> Vec<Pin> {
    let sizes = [
        (PinSize::Small, "Small"),
        (PinSize::Medium, "Medium"),
        (PinSize::Large, "Large"),
        (PinSize::ExtraLarge, "Extra large"),
    ];
    sizes
        .iter()
        .enumerate()
        .map(|(index, &(size, label))| Pin {
            id: format!("pin-{}", index + 1),
            icon: PinIcon::Pin,
            label: label.to_string(),
            x: scene.origin.0 - 150.0 + 65.0 * index as f64,
            y: scene.origin.1 - 110.0,
            size,
        })
        .collect()
}

fn svg(scene: &Scene, look: &Look, pins: &[Pin], highlights: &[Highlight]) -> String {
    let (diagram, theme) = resolved(look);
    let input = RenderInput {
        scene,
        diagram: &diagram,
        theme: &theme,
        look,
        pins,
        highlights,
    };
    render_svg(&input).expect("render")
}

fn write(scene: &Scene, look: &Look, pins: &[Pin], highlights: &[Highlight], path: &Path) {
    let (diagram, theme) = resolved(look);
    let input = RenderInput {
        scene,
        diagram: &diagram,
        theme: &theme,
        look,
        pins,
        highlights,
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
