//! Times the layers worked out from a scene when a sheet draws them: the
//! woods that block, the hours of sun, the sky view factor, the solar
//! radiation, the city blocks, the wind solve and the view from the site
//! centre. Each must stay quick enough to draw when a sheet opens, so a slow
//! one shows up as a number here first. This times the work itself; a
//! drawing keeps each result for its next redraw (`memo.rs`).
//!
//! cargo run --release -p openarchi-core --example compute_timing -- <scene.json>...

use std::time::Instant;

use geo::BoundingRect;
use openarchi_core::blocks;
use openarchi_core::forest;
use openarchi_core::radiation;
use openarchi_core::scene::Scene;
use openarchi_core::skyview;
use openarchi_core::sunlight;
use openarchi_core::viewshed;
use openarchi_core::wind::{self, WindSeason};

/// Each measure is run this many times and the fastest kept, so a busy machine does not skew it.
const RUNS: usize = 3;
/// The default sun date of a sheet, 21 March, which BR 209 asks about.
const SUN_DATE: (u32, u32) = (3, 21);

fn main() {
    for path in std::env::args().skip(1) {
        let text = std::fs::read_to_string(&path).expect("read the scene");
        let scene: Scene = serde_json::from_str(&text).expect("parse the scene");
        println!("{path}");
        report("woods", || {
            forest::forest(&scene);
        });
        report("hours of sun", || {
            sunlight::sun_hours(&scene, SUN_DATE.0, SUN_DATE.1);
        });
        report("sky view", || {
            skyview::sky_view(&scene);
        });
        match &scene.solar {
            Some(solar) => report("solar radiation", || {
                radiation::yearly(&scene, solar);
            }),
            None => println!("  solar radiation    no typical year in this scene"),
        }
        report("city blocks", || {
            blocks::blocks(&scene);
        });
        match wind::prevailing(&scene, WindSeason::Year) {
            Some(prevailing) => report("wind", || {
                wind::solved(&scene, prevailing.from_deg);
            }),
            None => println!("  wind               no typical year in this scene"),
        }
        let extent = scene.boundary.bounding_rect().expect("a boundary");
        report("view", || {
            viewshed::viewshed(&scene, None, extent);
        });
    }
}

fn report(name: &str, mut work: impl FnMut()) {
    let fastest = (0..RUNS)
        .map(|_| {
            let started = Instant::now();
            work();
            started.elapsed()
        })
        .min()
        .unwrap_or_default();
    println!("  {name:<18} {fastest:?}");
}
