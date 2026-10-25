use std::fs;

use super::*;
use crate::forest::tests::{canopy, wooded};
use crate::grid::tests::{building, site, square, value_at};
use crate::sources::pvgis::{self, Tmy};

/// The PVGIS year recorded for the Westermarkt, the cell this test site lies in.
const RECORDED_YEAR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/analysis-cache/tiles-v1/pvgis-tmy-5km/24_97.zst"
);

fn recorded() -> Tmy {
    let bytes = zstd::decode_all(fs::read(RECORDED_YEAR).unwrap().as_slice()).unwrap();
    pvgis::parse(&bytes).unwrap()
}

fn kwh_at(grid: &RadiationGrid, x: f64, y: f64) -> Option<f64> {
    value_at(&grid.raster, &grid.kwh_m2, x, y)
}

/// A 40 m wide, 30 m tall block whose north face runs along y = 0.
fn tall_block() -> Feature {
    building(square(-20.0, -20.0, 20.0, 0.0), 30.0)
}

#[test]
fn open_ground_receives_the_typical_years_global_irradiation() {
    let tmy = recorded();
    let solar = SolarYear::from_tmy(&tmy).unwrap();
    let grid = yearly(&site(Vec::new(), Vec::new()), &solar);
    let open = kwh_at(&grid, 0.0, 0.0).unwrap();
    let global: f64 = tmy.hours.iter().map(|hour| hour.global_w_m2).sum::<f64>() / WH_PER_KWH;

    // One average day per month and no beam below 2° keep it close, not exact.
    assert!(
        (open - global).abs() < 0.02 * global,
        "{open} against {global}"
    );
}

#[test]
fn ground_north_of_a_tall_block_receives_less() {
    let solar = SolarYear::from_tmy(&recorded()).unwrap();
    let grid = yearly(&site(vec![tall_block()], Vec::new()), &solar);
    let behind = kwh_at(&grid, 0.0, 5.0).unwrap();
    let in_front = kwh_at(&grid, 0.0, -60.0).unwrap();

    // Behind the block the direct sun is gone most of the year and half the sky with it.
    assert!(behind < 0.6 * in_front, "{behind} against {in_front}");
}

#[test]
fn an_open_roof_receives_what_open_ground_does() {
    let solar = SolarYear::from_tmy(&recorded()).unwrap();
    let scene = site(vec![tall_block()], Vec::new());
    let grid = yearly(&scene, &solar);
    let roof = kwh_at(&grid, 0.0, -10.0).unwrap();
    let open = open_ground_kwh(&scene, &solar);

    assert!((roof - open).abs() < 0.01 * open, "{roof} against {open}");
}

#[test]
fn a_lower_roof_north_of_a_taller_building_receives_less() {
    let solar = SolarYear::from_tmy(&recorded()).unwrap();
    let low_roof = building(square(-20.0, 0.0, 20.0, 20.0), 6.0);
    let grid = yearly(&site(vec![tall_block(), low_roof], Vec::new()), &solar);
    let shaded = kwh_at(&grid, 0.0, 3.0).unwrap();
    let tall = kwh_at(&grid, 0.0, -10.0).unwrap();

    assert!(shaded < 0.7 * tall, "{shaded} against {tall}");
}

#[test]
fn the_floor_of_a_wood_is_shaded_but_a_row_of_street_trees_is_not() {
    let solar = SolarYear::from_tmy(&recorded()).unwrap();
    let wood = wooded(
        site(Vec::new(), Vec::new()),
        canopy(&[(0.0, 0.0, 100.0, 100.0)], 15.0),
    );
    let street = wooded(
        site(Vec::new(), Vec::new()),
        canopy(&[(-240.0, 0.0, 240.0, 10.0)], 15.0),
    );
    let open = open_ground_kwh(&wood, &solar);

    let floor = kwh_at(&yearly(&wood, &solar), 50.0, 50.0).unwrap();
    let under_trees = kwh_at(&yearly(&street, &solar), 0.0, 5.0).unwrap();
    assert!(floor < 0.2 * open, "{floor} against {open}");
    assert!(under_trees > 0.95 * open, "{under_trees} against {open}");
}

#[test]
fn classes_are_shares_of_what_open_ground_receives_on_the_site() {
    let solar = SolarYear::from_tmy(&recorded()).unwrap();
    let mut scene = site(Vec::new(), Vec::new());
    scene.solar = Some(solar.clone());
    let open = open_ground_kwh(&scene, &solar);
    let diagram = crate::catalog::builtin_diagrams()
        .into_iter()
        .find(|diagram| diagram.id == "solar-radiation")
        .unwrap();

    let sited = sited(&diagram, &scene).unwrap();
    let Some(crate::diagram::Scheme::Sequential { breaks, labels }) =
        sited.classify.as_ref().map(|classify| &classify.scheme)
    else {
        panic!("a sequential scheme");
    };
    let top_share = breaks.last().unwrap() / open;
    assert!((top_share - 0.95).abs() < 0.01, "{breaks:?} of {open}");
    assert!(labels[0].starts_with("Under "), "{labels:?}");
    let note = sited.classify.unwrap().note.unwrap();
    assert!(note.starts_with("Shares of the "), "{note}");
}

#[test]
fn bands_are_solar_radiation() {
    let solar = SolarYear::from_tmy(&recorded()).unwrap();
    let grid = yearly(&site(vec![tall_block()], Vec::new()), &solar);
    let bands = bands(&grid, &[300.0, 500.0, 700.0, 850.0, 1000.0]);

    assert!(bands.len() > 1);
    assert!(bands.iter().all(|band| band.layer == Layer::SolarRadiation));
}
