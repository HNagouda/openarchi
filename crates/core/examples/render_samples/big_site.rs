use std::path::{Path, PathBuf};
use std::time::Instant;

use geo::{Coord, Geometry, MapCoords, Polygon};
use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::export::render_png;
use openarchi_core::project::Look;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::scene::Scene;
use openarchi_core::sheet::Sheet;
use openarchi_core::view::{View, ViewMode};

use crate::SAMPLE_WIDTH_PX;

/// Copies of the fixture laid side by side to stand in for a 1 km radius site.
const BIG_SITE_GRID: i32 = 3;
const BIG_SITE_SPACING_M: f64 = 600.0;

/// A stand-in for a 1 km radius site: the fixture tiled 3 × 3, about 600 m apart.
pub(crate) fn time_big_site(scene: &Scene, out_dir: &Path) {
    let mut big = scene.clone();
    big.features.clear();
    for column in -BIG_SITE_GRID / 2..=BIG_SITE_GRID / 2 {
        for row in -BIG_SITE_GRID / 2..=BIG_SITE_GRID / 2 {
            let (dx, dy) = (
                column as f64 * BIG_SITE_SPACING_M,
                row as f64 * BIG_SITE_SPACING_M,
            );
            for feature in &scene.features {
                let mut moved = feature.clone();
                moved.geometry = shift(&feature.geometry, dx, dy);
                big.features.push(moved);
            }
        }
    }
    big.boundary = circle(1000.0);
    let polygons = big
        .features
        .iter()
        .filter(|f| matches!(f.geometry, Geometry::Polygon(_) | Geometry::MultiPolygon(_)))
        .count();

    let diagrams = builtin_diagrams();
    let themes = builtin_themes();
    let runs = [
        ("context", ViewMode::Plan),
        ("building-use", ViewMode::Plan),
        ("building-use", ViewMode::Axonometric),
    ];
    for (diagram_id, mode) in runs {
        let look = Look {
            sheet: Sheet {
                scale: 7500,
                title: "1 km test".to_string(),
                ..Sheet::default()
            },
            view: View {
                mode,
                ..View::default()
            },
            ..Look::default()
        };
        let diagram = diagrams
            .iter()
            .find(|d| d.id == diagram_id)
            .expect("diagram exists");
        let input = RenderInput {
            scene: &big,
            diagram,
            theme: &themes[0],
            look: &look,
            pins: &[],
            highlights: &[],
            model: None,
        };
        let started = Instant::now();
        let svg = render_svg(&input).expect("render the big site");
        let svg_time = started.elapsed();
        let started = Instant::now();
        let png = render_png(&input, SAMPLE_WIDTH_PX).expect("rasterise the big site");
        let png_time = started.elapsed();
        println!(
            "1 km site, {diagram_id} {mode:?}: {} features ({polygons} polygons), SVG {:.0} KB in {:.0} ms, PNG {SAMPLE_WIDTH_PX} px in {:.0} ms (SVG included)",
            big.features.len(),
            svg.len() as f64 / 1024.0,
            svg_time.as_secs_f64() * 1000.0,
            png_time.as_secs_f64() * 1000.0
        );
        let name = format!("big-site--{diagram_id}--{mode:?}").to_lowercase();
        std::fs::write(out_dir.join(format!("{name}.png")), png).expect("write the big site");
    }
    let _: PathBuf = out_dir.to_path_buf();
}

fn shift(geometry: &Geometry<f64>, dx: f64, dy: f64) -> Geometry<f64> {
    geometry.map_coords(|c| Coord {
        x: c.x + dx,
        y: c.y + dy,
    })
}

fn circle(radius: f64) -> Polygon<f64> {
    let points: Vec<Coord<f64>> = (0..256)
        .map(|index| {
            let angle = index as f64 / 256.0 * std::f64::consts::TAU;
            Coord {
                x: radius * angle.cos(),
                y: radius * angle.sin(),
            }
        })
        .collect();
    Polygon::new(points.into(), vec![])
}
