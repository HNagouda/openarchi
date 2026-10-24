//! Renders every built-in diagram in every built-in theme to PNG, plus a few
//! sheet layouts, into target/render-samples/, and times a 1 km site.
//!
//! cargo run --release -p openarchi-core --example render_samples [diagram] [theme]

use std::path::{Path, PathBuf};
use std::time::Instant;

use geo::{Coord, Geometry, MapCoords, Polygon};
use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::export::{export, render_png, ExportFormat};
use openarchi_core::project::Look;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::scene::Scene;
use openarchi_core::sheet::{Align, Frame, Paper, Position, Sheet, TextBox, TextStyle};

const FIXTURE: &str = "tests/fixtures/westermarkt.scene.json";
const SAMPLE_WIDTH_PX: u32 = 1800;
/// Copies of the fixture laid side by side to stand in for a 1 km radius site.
const BIG_SITE_GRID: i32 = 3;
const BIG_SITE_SPACING_M: f64 = 600.0;

fn main() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = manifest.join("../../target/render-samples");
    std::fs::create_dir_all(&out_dir).expect("create the samples folder");
    let text =
        std::fs::read_to_string(manifest.join(FIXTURE)).expect("read the Westermarkt fixture");
    let scene: Scene = serde_json::from_str(&text).expect("parse the Westermarkt fixture");

    let only_diagram = std::env::args().nth(1);
    let only_theme = std::env::args().nth(2);
    let sheet = Sheet {
        title: "Westermarkt".to_string(),
        subtitle: "Amsterdam, 300 m around the Westerkerk".to_string(),
        ..Sheet::default()
    };

    for diagram in builtin_diagrams() {
        if only_diagram.as_deref().is_some_and(|id| id != diagram.id) {
            continue;
        }
        for theme in builtin_themes() {
            if only_theme.as_deref().is_some_and(|id| id != theme.id) {
                continue;
            }
            let look = Look {
                diagram_id: diagram.id.clone(),
                theme_id: theme.id.clone(),
                sheet: sheet.clone(),
                ..Look::default()
            };
            let input = RenderInput {
                scene: &scene,
                diagram: &diagram,
                theme: &theme,
                look: &look,
            };
            let png = render_png(&input, SAMPLE_WIDTH_PX).expect("render the sample");
            std::fs::write(
                out_dir.join(format!("{}--{}.png", diagram.id, theme.id)),
                png,
            )
            .expect("write the sample");
        }
    }
    if only_diagram.is_none() {
        write_layouts(&scene, &out_dir);
        write_custom_text(&scene, &out_dir);
        time_big_site(&scene, &out_dir);
    }
    println!("samples in {}", out_dir.display());
}

/// The same diagram on portrait, square and A4 sheets, and in each frame shape.
fn write_layouts(scene: &Scene, out_dir: &Path) {
    let diagrams = builtin_diagrams();
    let themes = builtin_themes();
    let variants = [
        ("a3-portrait", Paper::A3, false, 2500, Frame::Circle),
        ("square", Paper::Square, false, 3500, Frame::Circle),
        ("a4-landscape", Paper::A4, true, 4000, Frame::Circle),
        ("a1-landscape", Paper::A1, true, 1000, Frame::Circle),
        ("frame-square", Paper::A3, true, 2500, Frame::Square),
        ("frame-area", Paper::A3, true, 2500, Frame::Area),
        ("cropped", Paper::A4, false, 1500, Frame::Circle),
    ];
    for (name, paper, landscape, scale, frame) in variants {
        let sheet = Sheet {
            paper,
            landscape,
            scale,
            frame,
            title: "Westermarkt, Amsterdam".to_string(),
            subtitle: "Building use within 300 m of the Westerkerk".to_string(),
            ..Sheet::default()
        };
        let look = Look {
            sheet,
            ..Look::default()
        };
        let diagram = diagrams
            .iter()
            .find(|d| d.id == "building-use")
            .expect("building-use exists");
        let theme = &themes[0];
        let input = RenderInput {
            scene,
            diagram,
            theme,
            look: &look,
        };
        let png = render_png(&input, SAMPLE_WIDTH_PX).expect("render the layout");
        std::fs::write(out_dir.join(format!("layout--{name}.png")), png).expect("write the layout");
    }

    let look = Look {
        sheet: Sheet {
            title: "Westermarkt".to_string(),
            ..Sheet::default()
        },
        ..Look::default()
    };
    let input = RenderInput {
        scene,
        diagram: &diagrams[0],
        theme: &themes[0],
        look: &look,
    };
    for format in [ExportFormat::Svg, ExportFormat::Pdf, ExportFormat::Dxf] {
        let path = out_dir.join(format!("westermarkt.{}", format.extension()));
        export(&input, format, 150, &path).expect("export");
    }
}

/// A sheet with the title moved onto the drawing, a restyled subtitle and two
/// text boxes in every family, as PNG and PDF, to check type and embedding.
fn write_custom_text(scene: &Scene, out_dir: &Path) {
    let position = |x_mm, y_mm| Some(Position { x_mm, y_mm });
    let sheet = Sheet {
        title: "Westermarkt".to_string(),
        subtitle: "Building use within 300 m of the Westerkerk".to_string(),
        title_position: position(24.0, 22.0),
        title_style: TextStyle {
            family: Some("Source Serif 4".to_string()),
            italic: Some(true),
            size_pt: Some(40.0),
            ..TextStyle::default()
        },
        subtitle_style: TextStyle {
            family: Some("IBM Plex Mono".to_string()),
            colour: Some("@accent".to_string()),
            ..TextStyle::default()
        },
        texts: vec![
            TextBox {
                id: "text-1".to_string(),
                text: "Westerkerk\n1631, Hendrick de Keyser".to_string(),
                style: TextStyle {
                    bold: Some(true),
                    size_pt: Some(12.0),
                    align: Some(Align::Centre),
                    ..TextStyle::default()
                },
                position: position(150.0, 150.0),
            },
            TextBox {
                id: "text-2".to_string(),
                text: "Prinsengracht".to_string(),
                style: TextStyle {
                    family: Some("IBM Plex Mono".to_string()),
                    italic: Some(true),
                    bold: Some(true),
                    size_pt: Some(10.0),
                    align: Some(Align::Right),
                    colour: Some("#1D4E89".to_string()),
                    opacity: Some(0.8),
                },
                position: position(280.0, 260.0),
            },
        ],
        ..Sheet::default()
    };
    let look = Look {
        diagram_id: "building-use".to_string(),
        sheet,
        ..Look::default()
    };
    let diagrams = builtin_diagrams();
    let themes = builtin_themes();
    let diagram = diagrams
        .iter()
        .find(|d| d.id == "building-use")
        .expect("building-use exists");
    let input = RenderInput {
        scene,
        diagram,
        theme: &themes[0],
        look: &look,
    };
    let png = render_png(&input, SAMPLE_WIDTH_PX).expect("render the custom text");
    std::fs::write(out_dir.join("text--custom.png"), png).expect("write the custom text");
    export(
        &input,
        ExportFormat::Pdf,
        150,
        &out_dir.join("text--custom.pdf"),
    )
    .expect("export the custom text");
}

/// A stand-in for a 1 km radius site: the fixture tiled 3 × 3, about 600 m apart.
fn time_big_site(scene: &Scene, out_dir: &Path) {
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
    let look = Look {
        sheet: Sheet {
            scale: 7500,
            title: "1 km test".to_string(),
            ..Sheet::default()
        },
        ..Look::default()
    };
    for diagram_id in ["context", "building-use"] {
        let diagram = diagrams
            .iter()
            .find(|d| d.id == diagram_id)
            .expect("diagram exists");
        let input = RenderInput {
            scene: &big,
            diagram,
            theme: &themes[0],
            look: &look,
        };
        let started = Instant::now();
        let svg = render_svg(&input).expect("render the big site");
        let svg_time = started.elapsed();
        let started = Instant::now();
        let png = render_png(&input, SAMPLE_WIDTH_PX).expect("rasterise the big site");
        let png_time = started.elapsed();
        println!(
            "1 km site, {diagram_id}: {} features ({polygons} polygons), SVG {:.0} KB in {:.0} ms, PNG {SAMPLE_WIDTH_PX} px in {:.0} ms (SVG included)",
            big.features.len(),
            svg.len() as f64 / 1024.0,
            svg_time.as_secs_f64() * 1000.0,
            png_time.as_secs_f64() * 1000.0
        );
        std::fs::write(out_dir.join(format!("big-site--{diagram_id}.png")), png)
            .expect("write the big site");
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
