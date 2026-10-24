//! Imports a palette the way the app does and renders every diagram in it,
//! with the ratio charts, into target/render-samples/palette/.
//!
//! cargo run --release -p openarchi-core --example palette_samples [palette text] [scene.json]
//!
//! Without arguments it uses five colours from a palette generator and the
//! checked-in Westermarkt scene. Pass a scene written by `fetch_site` to see
//! the OpenStreetMap and environment diagrams too.

use std::path::Path;

use openarchi_core::catalog::builtin_diagrams;
use openarchi_core::export::render_png;
use openarchi_core::palette::{check, parse, PaletteDraft, RoleChoice};
use openarchi_core::project::Look;
use openarchi_core::ratios::{chart_svg, compute, ChartStyle};
use openarchi_core::render::RenderInput;
use openarchi_core::scene::Scene;
use openarchi_core::sheet::Sheet;
use openarchi_core::view::{View, ViewMode};

const FIXTURE: &str = "tests/fixtures/westermarkt.scene.json";
const FIVE_COLOURS: &str = "https://palettes.example/f79f79-f7d08a-e3f09b-87b6a7-5b5941";
const SAMPLE_WIDTH_PX: u32 = 1400;
const PALETTE_ID: &str = "sample-palette";

fn main() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = manifest.join("../../target/render-samples/palette");
    std::fs::create_dir_all(&out_dir).expect("create the samples folder");
    let text = std::env::args().nth(1).unwrap_or(FIVE_COLOURS.to_string());
    let scene_path = std::env::args()
        .nth(2)
        .map(std::path::PathBuf::from)
        .unwrap_or(manifest.join(FIXTURE));
    let scene: Scene =
        serde_json::from_str(&std::fs::read_to_string(scene_path).expect("read the scene"))
            .expect("parse the scene");

    let draft = PaletteDraft {
        name: "Sample".to_string(),
        swatches: parse(&text).expect("read the palette"),
        roles: RoleChoice::default(),
        typography: None,
        based_on: None,
    };
    let checked = check(&draft, PALETTE_ID).expect("build the theme");
    for role in &checked.roles {
        println!("{:<7} {} ({})", role.role, role.hex, role.reason);
    }
    for warning in &checked.warnings {
        println!("warning: {warning}");
    }
    let theme = checked.theme;
    let json = serde_json::to_string_pretty(&theme).expect("theme as JSON");
    std::fs::write(out_dir.join("theme.json"), json).expect("write the theme");

    for diagram in builtin_diagrams() {
        for (variant, _) in diagram.drawings() {
            let stem = match &variant {
                Some(variant) => format!("{}-{variant}", diagram.id),
                None => diagram.id.clone(),
            };
            let look = Look {
                diagram_id: diagram.id.clone(),
                variant,
                theme_id: PALETTE_ID.to_string(),
                sheet: Sheet {
                    title: diagram.name.clone(),
                    subtitle: "Westermarkt, Amsterdam".to_string(),
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
            };
            let png = render_png(&input, SAMPLE_WIDTH_PX).expect("render the sample");
            std::fs::write(out_dir.join(format!("{stem}.png")), png).expect("write the sample");
        }
    }

    let diagrams = builtin_diagrams();
    let context = diagrams
        .iter()
        .find(|d| d.id == "context")
        .expect("context");
    let axo = Look {
        diagram_id: context.id.clone(),
        theme_id: PALETTE_ID.to_string(),
        view: View {
            mode: ViewMode::Axonometric,
            rotation_deg: 30.0,
            ..View::default()
        },
        ..Look::default()
    };
    let input = RenderInput {
        scene: &scene,
        diagram: context,
        theme: &theme,
        look: &axo,
        pins: &[],
    };
    let png = render_png(&input, SAMPLE_WIDTH_PX).expect("render the axonometric");
    std::fs::write(out_dir.join("axo--context.png"), png).expect("write the axonometric");

    let ratios = compute(&scene, &theme).expect("compute the ratios");
    let svg = chart_svg(&ratios, &theme, ChartStyle::Bar, "Westermarkt");
    std::fs::write(out_dir.join("ratios.svg"), svg).expect("write the ratios");
    println!("samples in {}", out_dir.display());
}
