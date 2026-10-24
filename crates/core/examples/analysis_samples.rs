//! Renders the analysis of the Westermarkt fixture in every theme, as one
//! SVG and PNG page per theme and as the A4 PDF the app exports, for looking
//! at the charts and pages while working on them.
//!
//! cargo run --release -p openarchi-core --example analysis_samples   # writes target/analysis-samples/
//!
//! Climate, rain and the location lookups go through target/tile-cache, so
//! only the first run needs the network. `--record` also copies what was
//! fetched into tests/fixtures/analysis-cache, which the offline tests read.

use std::fs;
use std::path::Path;

use openarchi_core::analysis::{self, report_pages, report_svg, section_svg, PARTS};
use openarchi_core::catalog::builtin_themes;
use openarchi_core::export::{svg_pages_to_pdf, svg_to_png};
use openarchi_core::scene::Scene;
use openarchi_core::sources::cache::TileCache;

const FIXTURE: &str = "tests/fixtures/westermarkt.scene.json";
const CACHE_DIR: &str = "../../target/tile-cache";
const OUT_DIR: &str = "../../target/analysis-samples";
const RECORDED_DIR: &str = "tests/fixtures/analysis-cache";
/// The cache folders the analysis fills, in one place.
const DATASETS: [&str; 4] = [
    "pvgis-tmy-5km",
    "knmi-daily-rain",
    "pdok-gemeente-10m",
    "pdok-address-10m",
];
const PNG_WIDTH_PX: u32 = 900;

fn main() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let text = fs::read_to_string(manifest.join(FIXTURE)).expect("read the Westermarkt fixture");
    let scene: Scene = serde_json::from_str(&text).expect("parse the Westermarkt fixture");
    let cache_dir = manifest.join(CACHE_DIR);
    let cache = TileCache::at(&cache_dir);
    let out_dir = manifest.join(OUT_DIR);
    fs::create_dir_all(&out_dir).expect("create the samples folder");

    for theme in builtin_themes() {
        let mut sections = Vec::new();
        let mut failures = Vec::new();
        for part in PARTS {
            match analysis::part(part, &scene, &theme, &cache) {
                Ok(found) => sections.extend(found),
                Err(error) => failures.push((part.title().to_string(), error.to_string())),
            }
        }
        let svg = report_svg(&sections, &failures, &theme, "Westermarkt").expect("draw the page");
        let png = svg_to_png(&svg, PNG_WIDTH_PX).expect("rasterise the page");
        fs::write(out_dir.join(format!("{}.svg", theme.id)), &svg).expect("write the SVG");
        fs::write(out_dir.join(format!("{}.png", theme.id)), png).expect("write the PNG");
        let pages = report_pages(&sections, &failures, &theme, "Westermarkt").expect("lay out A4");
        let pdf = svg_pages_to_pdf(&pages, "Westermarkt").expect("make the PDF");
        fs::write(out_dir.join(format!("{}-a4.pdf", theme.id)), pdf).expect("write the PDF");

        // One page per section as well, small enough to look at one by one.
        let section_dir = out_dir.join(&theme.id);
        fs::create_dir_all(&section_dir).expect("create the theme folder");
        for section in &sections {
            let svg = section_svg(section, &theme, "Westermarkt").expect("draw the section");
            let png = svg_to_png(&svg, PNG_WIDTH_PX).expect("rasterise the section");
            fs::write(section_dir.join(format!("{}.png", section.id)), png).expect("write the PNG");
        }
        for failure in &failures {
            eprintln!("{}: {} failed: {}", theme.id, failure.0, failure.1);
        }
    }

    if std::env::args().any(|argument| argument == "--record") {
        record(&cache_dir.join("tiles-v1"), &manifest.join(RECORDED_DIR));
    }
    println!("samples in {}", out_dir.display());
}

fn record(from: &Path, to: &Path) {
    for dataset in DATASETS {
        let source = from.join(dataset);
        let target = to.join("tiles-v1").join(dataset);
        fs::create_dir_all(&target).expect("create the fixture folder");
        for entry in fs::read_dir(&source).expect("the dataset was fetched") {
            let path = entry.expect("read the cache folder").path();
            let name = path.file_name().expect("a file name");
            fs::copy(&path, target.join(name)).expect("copy the cached answer");
        }
    }
    println!("recorded the fetched answers in {}", to.display());
}
