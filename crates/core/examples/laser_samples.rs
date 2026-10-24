//! Laser cutting files for the recorded Westermarkt site, with a PNG of each
//! bed to look at, at three scales and for a small sheet area.
//!
//!     cargo run --release -p openarchi-core --example laser_samples   # writes target/laser-samples/

use std::fs;
use std::path::Path;

use openarchi_core::area::Area;
use openarchi_core::crs::rd_to_wgs84;
use openarchi_core::export::laser::{export, LaserFormat, LaserSettings};
use openarchi_core::export::svg_to_png;
use openarchi_core::scene::Scene;

const FIXTURE: &str = "tests/fixtures/westermarkt.scene.json";
/// Wide enough to read a 1 mm id on a 600 mm bed when zoomed in.
const PNG_WIDTH_PX: u32 = 6000;
/// A block around the Westerkerk, as a student might frame on a sheet.
const AREA_RADIUS_M: f64 = 80.0;

fn main() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = manifest.join("../../target/laser-samples");
    fs::create_dir_all(&out_dir).expect("create the output folder");
    let text = fs::read_to_string(manifest.join(FIXTURE)).expect("read the fixture");
    let scene: Scene = serde_json::from_str(&text).expect("parse the fixture");
    let area = Area::Circle {
        center: rd_to_wgs84(scene.origin.0, scene.origin.1),
        radius_m: AREA_RADIUS_M,
    };

    for (name, scale, selection) in [
        ("whole-1000", 1000, None),
        ("whole-2500", 2500, None),
        ("area-500", 500, Some(&area)),
    ] {
        let folder = out_dir.join(name);
        fs::create_dir_all(&folder).expect("create the sample folder");
        let settings = LaserSettings {
            scale,
            thickness_mm: 2.0,
            bed_width_mm: 600.0,
            bed_height_mm: 400.0,
            has_base_plate: true,
            has_buildings: true,
            has_contours: true,
            format: LaserFormat::Svg,
        };
        let written = export(&scene, selection, &settings, &folder, "westermarkt").expect("export");
        for file in &written.files {
            let svg = fs::read_to_string(file).expect("read a bed back");
            // The cutter's hairlines vanish in a picture, so the PNG draws them 0.2 mm wide.
            // A white page under it, since the bed itself has no background to show.
            let visible = svg
                .replace("stroke-width=\"0.01\"", "stroke-width=\"0.2\"")
                .replace(
                    "<g id=\"engrave\">",
                    "<rect width=\"100%\" height=\"100%\" fill=\"white\"/>\n<g id=\"engrave\">",
                );
            let png = svg_to_png(&visible, PNG_WIDTH_PX).expect("render the bed");
            fs::write(Path::new(file).with_extension("png"), png).expect("write the PNG");
        }
        println!("{name}: {:?}", written.summary);
    }
    println!("samples in {}", out_dir.display());
}
