//! Fetches a circular site and writes the scene as JSON.
//!
//! cargo run -p openarchi-core --example fetch_site -- 4.8840 52.3745 300 westermarkt.scene.json

use std::collections::BTreeMap;
use std::process::ExitCode;
use std::time::Instant;

use openarchi_core::area::{Area, LonLat};
use openarchi_core::site::{build_scene, Progress};

const USAGE: &str = "usage: fetch_site <lon> <lat> <radius_m> <out.json>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [lon, lat, radius_m, out_path] = args.as_slice() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let (Ok(lon), Ok(lat), Ok(radius_m)) = (lon.parse(), lat.parse(), radius_m.parse()) else {
        eprintln!("lon, lat and radius_m must be numbers\n{USAGE}");
        return ExitCode::FAILURE;
    };

    let area = Area::Circle {
        center: LonLat { lon, lat },
        radius_m,
    };
    let started = Instant::now();
    let report = |progress: Progress| {
        eprintln!(
            "[{:5.1}s] {}/{} {}",
            started.elapsed().as_secs_f64(),
            progress.done,
            progress.total,
            progress.step
        );
    };

    let scene = match build_scene(&area, &report) {
        Ok(scene) => scene,
        Err(error) => {
            eprintln!("fetch failed: {error}");
            return ExitCode::FAILURE;
        }
    };

    let json = serde_json::to_string(&scene).expect("a scene always serialises");
    if let Err(error) = std::fs::write(out_path, &json) {
        eprintln!("could not write {out_path}: {error}");
        return ExitCode::FAILURE;
    }

    let mut per_layer = BTreeMap::new();
    for feature in &scene.features {
        *per_layer.entry(feature.layer.label()).or_insert(0) += 1;
    }
    eprintln!(
        "{} features, {:.1} MB, {:.1}s",
        scene.features.len(),
        json.len() as f64 / 1e6,
        started.elapsed().as_secs_f64()
    );
    for (layer, count) in per_layer {
        eprintln!("  {layer:<12} {count}");
    }
    for warning in &scene.warnings {
        eprintln!("warning: {warning}");
    }

    ExitCode::SUCCESS
}
