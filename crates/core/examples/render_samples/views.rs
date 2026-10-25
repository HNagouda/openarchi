use std::path::Path;
use std::time::Instant;

use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::export::render_png;
use openarchi_core::project::Look;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::scene::Scene;
use openarchi_core::sheet::Sheet;
use openarchi_core::view::{SunMoment, View, ViewMode};

use crate::SAMPLE_WIDTH_PX;

/// The view samples: file name, diagram, theme and view mode.
const VIEW_SAMPLES: [(&str, &str, &str, ViewMode); 12] = [
    (
        "axo--building-height--paper",
        "building-height",
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
    ("axo--shadow--mono", "shadow", "mono", ViewMode::Axonometric),
    ("explode--paper", "context", "paper", ViewMode::Explode),
    (
        "explode--midnight",
        "context",
        "midnight",
        ViewMode::Explode,
    ),
    (
        "explode--blueprint",
        "context",
        "blueprint",
        ViewMode::Explode,
    ),
    ("explode--pastel", "context", "pastel", ViewMode::Explode),
    ("explode--wind--paper", "wind", "paper", ViewMode::Explode),
    ("sun-path--paper", "sun-path", "paper", ViewMode::Plan),
    ("shadow--paper", "shadow", "paper", ViewMode::Plan),
];

/// Axonometric, explode, sun path and shadow samples, each timed.
pub(crate) fn write_views(scene: &Scene, out_dir: &Path) {
    std::fs::create_dir_all(out_dir).expect("create the views folder");
    let diagrams = builtin_diagrams();
    let themes = builtin_themes();
    for (name, diagram_id, theme_id, mode) in VIEW_SAMPLES {
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
                subtitle: "Amsterdam, 300 m around the Westerkerk".to_string(),
                ..Sheet::default()
            },
            view: View {
                mode,
                rotation_deg: if mode == ViewMode::Axonometric {
                    30.0
                } else {
                    0.0
                },
                // The wind sample shows that a plan-only diagram still makes a flat plate.
                plates: if diagram_id == "wind" {
                    vec!["wind".to_string(), "context".to_string()]
                } else {
                    View::default().plates
                },
                ..View::default()
            },
            sun: SunMoment {
                hour: if diagram_id == "shadow" { 16 } else { 13 },
                ..SunMoment::default()
            },
            ..Look::default()
        };
        let input = RenderInput {
            scene,
            diagram,
            theme,
            look: &look,
            pins: &[],
            highlights: &[],
            model: None,
        };
        let started = Instant::now();
        let svg = render_svg(&input).expect("render the view");
        let svg_time = started.elapsed();
        let started = Instant::now();
        let png = render_png(&input, SAMPLE_WIDTH_PX).expect("rasterise the view");
        let png_time = started.elapsed();
        println!(
            "{name}: SVG {:.0} KB in {:.0} ms, PNG {SAMPLE_WIDTH_PX} px in {:.0} ms (SVG included)",
            svg.len() as f64 / 1024.0,
            svg_time.as_secs_f64() * 1000.0,
            png_time.as_secs_f64() * 1000.0
        );
        std::fs::write(out_dir.join(format!("{name}.png")), png).expect("write the view");
    }
    println!("views in {}", out_dir.display());
}
