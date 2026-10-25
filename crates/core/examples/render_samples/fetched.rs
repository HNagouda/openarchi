use std::path::Path;

use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::export::render_png;
use openarchi_core::project::Look;
use openarchi_core::radiation::{self, SolarYear};
use openarchi_core::render::RenderInput;
use openarchi_core::scene::Scene;
use openarchi_core::sheet::Sheet;
use openarchi_core::sources::pvgis;
use openarchi_core::view::{View, ViewMode};
use openarchi_core::wind::{self, WindYear};

use crate::{RECORDED_SOLAR_YEAR, SAMPLE_WIDTH_PX};

/// The views the neighbourhood samples are drawn in, with a name for each.
const CONTEXT_VIEWS: [(&str, ViewMode); 3] = [
    ("plan", ViewMode::Plan),
    ("axo", ViewMode::Axonometric),
    ("explode", ViewMode::Explode),
];
/// The diagrams drawn from OpenStreetMap data.
const OSM_DIAGRAMS: [&str; 4] = ["business", "facility", "transit", "walkability"];

/// Every diagram, or just `only`, in every theme, from a fetched scene.
/// The typical year a fetch keeps today, which the fixture predates, from the recorded PVGIS answer.
pub(crate) fn add_solar_year(scene: &mut Scene, manifest: &Path) {
    let packed =
        std::fs::read(manifest.join(RECORDED_SOLAR_YEAR)).expect("read the recorded PVGIS year");
    let bytes = zstd::decode_all(packed.as_slice()).expect("unpack the recorded PVGIS year");
    let tmy = pvgis::parse(&bytes).expect("parse the recorded PVGIS year");
    scene.solar = Some(SolarYear::from_tmy(&tmy).expect("a full typical year"));
    scene.credits.push(radiation::credit());
    scene.wind = Some(WindYear::from_tmy(&tmy).expect("a full typical year"));
    scene.credits.push(wind::credit());
}

pub(crate) fn write_scene(scene_path: &Path, out_dir: &Path, only: Option<&String>) {
    std::fs::create_dir_all(out_dir).expect("create the folder");
    let text = std::fs::read_to_string(scene_path).expect("read the scene");
    let scene: Scene = serde_json::from_str(&text).expect("parse the scene");
    // Every variant too, like the fixture samples: "greenery-canopy--paper.png".
    let drawings: Vec<_> = builtin_diagrams()
        .into_iter()
        .filter(|diagram| only.is_none_or(|id| *id == diagram.id))
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
            let look = Look {
                diagram_id: diagram.id.clone(),
                variant: variant.clone(),
                theme_id: theme.id.clone(),
                sheet: Sheet {
                    title: diagram.name.clone(),
                    subtitle: "300 m around the site".to_string(),
                    ..Sheet::default()
                },
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
            let name = format!("{stem}--{}.png", theme.id);
            std::fs::write(out_dir.join(name), png).expect("write the sample");
        }
    }
    println!("samples in {}", out_dir.display());
}

/// Every OpenStreetMap diagram in every theme, on an A3 sheet at 1:2500.
pub(crate) fn write_osm(scene: &Scene, name: &str, out_dir: &Path) {
    std::fs::create_dir_all(out_dir).expect("create the osm folder");
    let diagrams = builtin_diagrams();
    for diagram in diagrams
        .iter()
        .filter(|d| OSM_DIAGRAMS.contains(&d.id.as_str()))
    {
        for theme in builtin_themes() {
            let look = Look {
                diagram_id: diagram.id.clone(),
                theme_id: theme.id.clone(),
                sheet: Sheet {
                    title: name.to_string(),
                    ..Sheet::default()
                },
                ..Look::default()
            };
            let input = RenderInput {
                scene,
                diagram,
                theme: &theme,
                look: &look,
                pins: &[],
                highlights: &[],
                model: None,
            };
            let png = render_png(&input, SAMPLE_WIDTH_PX).expect("render the sample");
            let file = format!("{name}--{}--{}.png", diagram.id, theme.id);
            std::fs::write(out_dir.join(file), png).expect("write the sample");
        }
    }
    println!("OpenStreetMap samples in {}", out_dir.display());
}

/// One diagram with its neighbourhood in every theme and view.
pub(crate) fn write_context(scene: &Scene, diagram_id: &str, out_dir: &Path) {
    std::fs::create_dir_all(out_dir).expect("create the context folder");
    let diagrams = builtin_diagrams();
    let diagram = diagrams
        .iter()
        .find(|d| d.id == diagram_id)
        .expect("diagram exists");
    for theme in builtin_themes() {
        for (view_name, mode) in CONTEXT_VIEWS {
            let look = Look {
                diagram_id: diagram.id.clone(),
                theme_id: theme.id.clone(),
                sheet: Sheet {
                    title: diagram.name.clone(),
                    subtitle: "With its neighbourhood".to_string(),
                    ..Sheet::default()
                },
                view: View {
                    mode,
                    ..View::default()
                },
                ..Look::default()
            };
            let input = RenderInput {
                scene,
                diagram,
                theme: &theme,
                look: &look,
                pins: &[],
                highlights: &[],
                model: None,
            };
            let png = render_png(&input, SAMPLE_WIDTH_PX).expect("render the sample");
            let file = format!("{}--{view_name}--{}.png", diagram.id, theme.id);
            std::fs::write(out_dir.join(file), png).expect("write the sample");
        }
    }
    println!("neighbourhood samples in {}", out_dir.display());
}
