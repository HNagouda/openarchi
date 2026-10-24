//! Renders sections of a scene in every built-in theme, for looking at them.
//!
//! cargo run --release -p openarchi-core --example section_samples -- <scene.json> <name> [angle_deg] [length_m]
//!
//! The line runs through the scene's centre at `angle_deg` counterclockwise
//! from east (0 by default), `length_m` long (240 by default). Into
//! target/render-samples/sections/<name>/ go: the section in every theme, the
//! plan with its cut marker in every theme, the axonometric with the marker,
//! a section of the building age diagram, a section on a sheet framing a
//! smaller area, the same scene without its terrain, and a PDF of the section.
//! Fetch a scene with `fetch_site` first; the checked-in fixture has no terrain.

use std::path::{Path, PathBuf};

use openarchi_core::area::Area;
use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::crs::rd_to_wgs84;
use openarchi_core::export::{export, render_png, ExportFormat};
use openarchi_core::project::Look;
use openarchi_core::render::RenderInput;
use openarchi_core::scene::Scene;
use openarchi_core::section::{SectionLine, DEFAULT_DEPTH_M};
use openarchi_core::sheet::Sheet;
use openarchi_core::theme::Theme;
use openarchi_core::view::{View, ViewMode};

const USAGE: &str = "usage: section_samples <scene.json> <name> [angle_deg] [length_m]";
const SAMPLE_WIDTH_PX: u32 = 1800;
const DEFAULT_LENGTH_M: f64 = 240.0;
/// The smaller area of the sub-selection sample, around the centre.
const AREA_RADIUS_M: f64 = 80.0;
const PDF_DPI: u32 = 150;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(scene_path), Some(name)) = (args.first(), args.get(1)) else {
        eprintln!("{USAGE}");
        std::process::exit(1);
    };
    let angle_deg: f64 = args
        .get(2)
        .map_or(0.0, |text| text.parse().expect("an angle"));
    let length_m: f64 = args
        .get(3)
        .map_or(DEFAULT_LENGTH_M, |text| text.parse().expect("a length"));
    let text = std::fs::read_to_string(scene_path).expect("read the scene");
    let scene: Scene = serde_json::from_str(&text).expect("parse the scene");
    let out_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/render-samples/sections")
        .join(name);
    std::fs::create_dir_all(&out_dir).expect("create the folder");

    let line = line_through_centre(&scene, angle_deg, length_m);
    let section = look_of("context", ViewMode::Section, &line);
    let plan = look_of("context", ViewMode::Plan, &line);
    let axo = look_of("building-height", ViewMode::Axonometric, &line);
    for theme in builtin_themes() {
        write(
            &scene,
            &section,
            &theme,
            &out_dir.join(format!("section--{}.png", theme.id)),
        );
        write(
            &scene,
            &plan,
            &theme,
            &out_dir.join(format!("plan--{}.png", theme.id)),
        );
    }
    let paper = theme("paper");
    write(&scene, &axo, &paper, &out_dir.join("axo--paper.png"));
    let ages = look_of("building-age", ViewMode::Section, &line);
    write(
        &scene,
        &ages,
        &paper,
        &out_dir.join("section-building-age--paper.png"),
    );

    let (x, y) = scene.origin;
    let framed = Look {
        selection: Some(Area::Circle {
            center: rd_to_wgs84(x, y),
            radius_m: AREA_RADIUS_M,
        }),
        ..section.clone()
    };
    write(
        &scene,
        &framed,
        &paper,
        &out_dir.join("section-area--paper.png"),
    );
    let flat = Scene {
        terrain: None,
        ..scene.clone()
    };
    write(
        &flat,
        &section,
        &paper,
        &out_dir.join("section-flat--paper.png"),
    );

    let diagrams = builtin_diagrams();
    let diagram = diagrams
        .iter()
        .find(|d| d.id == "context")
        .expect("context");
    let input = RenderInput {
        scene: &scene,
        diagram,
        theme: &paper,
        look: &section,
        pins: &[],
    };
    export(
        &input,
        ExportFormat::Pdf,
        PDF_DPI,
        &out_dir.join("section--paper.pdf"),
    )
    .expect("write the PDF");
    export(
        &input,
        ExportFormat::Svg,
        PDF_DPI,
        &out_dir.join("section--paper.svg"),
    )
    .expect("write the SVG");
    let plan_input = RenderInput {
        look: &plan,
        ..input
    };
    export(
        &plan_input,
        ExportFormat::Svg,
        PDF_DPI,
        &out_dir.join("plan--paper.svg"),
    )
    .expect("write the plan SVG");
    println!("samples in {}", out_dir.display());
}

/// A line through the scene's centre, in RD New metres.
fn line_through_centre(scene: &Scene, angle_deg: f64, length_m: f64) -> SectionLine {
    let (x, y) = scene.origin;
    let half = length_m / 2.0;
    let (dx, dy) = (
        half * angle_deg.to_radians().cos(),
        half * angle_deg.to_radians().sin(),
    );
    SectionLine {
        start: (x - dx, y - dy),
        end: (x + dx, y + dy),
        depth_m: DEFAULT_DEPTH_M,
    }
}

fn look_of(diagram_id: &str, mode: ViewMode, line: &SectionLine) -> Look {
    Look {
        diagram_id: diagram_id.to_string(),
        sheet: Sheet {
            title: "Section through the site".to_string(),
            subtitle: "Looking north".to_string(),
            show_context: true,
            ..Sheet::default()
        },
        view: View {
            mode,
            ..View::default()
        },
        section: Some(line.clone()),
        ..Look::default()
    }
}

fn theme(id: &str) -> Theme {
    builtin_themes()
        .into_iter()
        .find(|theme| theme.id == id)
        .expect("a built-in theme")
}

fn write(scene: &Scene, look: &Look, theme: &Theme, path: &PathBuf) {
    let diagrams = builtin_diagrams();
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == look.diagram_id)
        .expect("a built-in diagram");
    let look = Look {
        theme_id: theme.id.clone(),
        ..look.clone()
    };
    let input = RenderInput {
        scene,
        diagram,
        theme,
        look: &look,
        pins: &[],
    };
    let png = render_png(&input, SAMPLE_WIDTH_PX).expect("render the sample");
    std::fs::write(path, png).expect("write the sample");
}
