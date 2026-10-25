//! Writes the DXF of every diagram of a saved project, a section across its
//! site and a sheet framed to its first highlight, and prints how long each
//! took and how large it is. Open the files in a CAD program, or read them
//! back with ezdxf, to check what a CAD user gets.
//!
//! cargo run --release -p openarchi-core --example dxf_samples -- <library> <project-id> [out folder]
//!
//! THEME=<palette id> draws in that palette instead of the project's, and
//! DXF_LAYERS=all writes every fetched layer instead of those the sheet shows.

use std::path::{Path, PathBuf};
use std::time::Instant;

use geo::BoundingRect;
use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::export::{export_with_dxf_layers, DxfLayers, ExportFormat};
use openarchi_core::library::Library;
use openarchi_core::project::Look;
use openarchi_core::render::RenderInput;
use openarchi_core::section::SectionLine;
use openarchi_core::view::ViewMode;

const DEFAULT_OUT: &str = "target/dxf-samples";

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    let library = Library::at(Path::new(&arguments[1])).expect("open the library");
    let project = library.load(&arguments[2]).expect("load the project");
    let out = PathBuf::from(arguments.get(3).map_or(DEFAULT_OUT, String::as_str)).join(&project.id);
    std::fs::create_dir_all(&out).expect("make the out folder");
    let scene = project.scene.as_ref().expect("a fetched site");
    let themes = builtin_themes();
    let theme_id = std::env::var("THEME").unwrap_or(project.look.theme_id.clone());
    let theme = themes
        .iter()
        .find(|theme| theme.id == theme_id)
        .unwrap_or(&themes[0]);
    let dxf_layers = match std::env::var("DXF_LAYERS").as_deref() {
        Ok("all") => DxfLayers::All,
        _ => DxfLayers::Shown,
    };
    let diagrams = builtin_diagrams();
    let write = |name: &str, look: &Look| {
        let diagram = diagrams
            .iter()
            .find(|diagram| diagram.id == look.diagram_id)
            .expect("a built-in diagram");
        let input = RenderInput {
            scene,
            diagram,
            theme,
            look,
            pins: &project.pins,
            highlights: &project.highlights,
            model: None,
        };
        let path = out.join(format!("{name}.dxf"));
        let started = Instant::now();
        export_with_dxf_layers(&input, ExportFormat::Dxf, 0, dxf_layers, &path)
            .expect("export the DXF");
        let size = std::fs::metadata(&path).expect("written").len();
        println!(
            "{name:<40} {:>8.1?} {:>7} KB",
            started.elapsed(),
            size / 1024
        );
    };

    for diagram in &diagrams {
        for (variant, _) in diagram.drawings() {
            let look = Look {
                diagram_id: diagram.id.clone(),
                variant: variant.clone(),
                ..project.look.clone()
            };
            let name = match &variant {
                Some(variant) => format!("{}-{variant}", diagram.id),
                None => diagram.id.clone(),
            };
            write(&name, &look);
        }
    }

    // West to east through the middle of the site, unless the sheet has a line of its own.
    let bounds = scene.boundary.bounding_rect().expect("a boundary");
    let middle_y = (bounds.min().y + bounds.max().y) / 2.0 + scene.origin.1;
    let line = project.look.section.clone().unwrap_or(SectionLine {
        start: (bounds.min().x + scene.origin.0, middle_y),
        end: (bounds.max().x + scene.origin.0, middle_y),
        depth_m: openarchi_core::section::DEFAULT_DEPTH_M,
        scale: None,
        exaggeration: 1,
    });
    let mut plan_with_line = project.look.clone();
    plan_with_line.section = Some(line);
    plan_with_line.view.mode = ViewMode::Plan;
    write("plan-with-section-line", &plan_with_line);
    let mut section = plan_with_line.clone();
    section.view.mode = ViewMode::Section;
    section.sheet.show_context = true;
    write("section", &section);

    if let Some(highlight) = project.highlights.first() {
        let framed = Look {
            selection: Some(highlight.shape.to_area()),
            ..project.look.clone()
        };
        write("framed-to-first-highlight", &framed);
    }
}
