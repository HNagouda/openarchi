//! Times the preview render for every diagram of a saved project, so a slow
//! diagram shows up as a number rather than a feeling.
//!
//! It also times loading the project and encoding it for a save. VIEW
//! (plan, axonometric or explode) changes the view, SELECT_RADIUS frames a sheet
//! area that many metres around the site centre, and DUMP_DIR writes every
//! SVG there to look at.
//!
//! cargo run --release -p openarchi-core --example render_timing -- <library> <project-id>

use std::time::Instant;

use openarchi_core::area::{Area, LonLat};
use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::export::svg_to_png;
use openarchi_core::library::Library;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::view::ViewMode;

/// The diagram column's picture width.
const THUMBNAIL_WIDTH_PX: u32 = 160;

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    let library = Library::at(std::path::Path::new(&arguments[1])).expect("open the library");
    let started = Instant::now();
    let project = library.load(&arguments[2]).expect("load the project");
    println!("load {:?}", started.elapsed());

    let started = Instant::now();
    let copy = project.clone();
    println!("clone project {:?}", started.elapsed());
    let started = Instant::now();
    let bytes = serde_json::to_vec(&copy).expect("encode");
    println!(
        "encode project {:?} ({} KB)",
        started.elapsed(),
        bytes.len() / 1024
    );

    let scene = project.scene.expect("a fetched site");
    // SELECT_RADIUS=150 frames a circle that size at the site's centre, as a sheet area does.
    let selection = std::env::var("SELECT_RADIUS").ok().map(|radius| {
        let center = match &project.area {
            Area::Circle { center, .. } => *center,
            Area::Rectangle {
                south_west,
                north_east,
            } => LonLat {
                lon: (south_west.lon + north_east.lon) / 2.0,
                lat: (south_west.lat + north_east.lat) / 2.0,
            },
            Area::Polygon { points } => points[0],
        };
        Area::Circle {
            center,
            radius_m: radius.parse().expect("a radius in metres"),
        }
    });
    let themes = builtin_themes();
    let theme = themes
        .iter()
        .find(|theme| theme.id == project.look.theme_id)
        .unwrap_or(&themes[0]);
    for diagram in builtin_diagrams() {
        let mut look = project.look.clone();
        look.diagram_id = diagram.id.clone();
        look.variant = None;
        look.selection = selection.clone();
        look.view.mode = match std::env::var("VIEW").as_deref() {
            Ok("plan") => ViewMode::Plan,
            Ok("axonometric") => ViewMode::Axonometric,
            Ok("explode") => ViewMode::Explode,
            _ => look.view.mode,
        };
        let input = RenderInput {
            scene: &scene,
            diagram: &diagram,
            theme,
            look: &look,
            pins: &[],
            highlights: &[],
            model: None,
        };

        let started = Instant::now();
        let svg = render_svg(&input).expect("render");
        let svg_time = started.elapsed();
        if let Ok(dir) = std::env::var("DUMP_DIR") {
            std::fs::write(format!("{dir}/{}.svg", diagram.id), &svg).expect("dump");
        }
        let started = Instant::now();
        svg_to_png(&svg, THUMBNAIL_WIDTH_PX).expect("png");
        let png_time = started.elapsed();
        println!(
            "{:24} svg {:>8.1?}  {:>7} KB   thumbnail png {:>8.1?}",
            diagram.id,
            svg_time,
            svg.len() / 1024,
            png_time
        );
    }
}
