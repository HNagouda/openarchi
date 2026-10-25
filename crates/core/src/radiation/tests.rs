use std::fs;

use super::*;
use crate::skyview::tests::{building, site, square, value_at};
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
fn ground_under_a_roof_has_no_radiation() {
    let solar = SolarYear::from_tmy(&recorded()).unwrap();
    let grid = yearly(&site(vec![tall_block()], Vec::new()), &solar);
    assert_eq!(kwh_at(&grid, 0.0, -10.0), None);
}

#[test]
fn bands_are_solar_radiation() {
    let solar = SolarYear::from_tmy(&recorded()).unwrap();
    let grid = yearly(&site(vec![tall_block()], Vec::new()), &solar);
    let bands = bands(&grid, &[300.0, 500.0, 700.0, 850.0, 1000.0]);

    assert!(bands.len() > 1);
    assert!(bands.iter().all(|band| band.layer == Layer::SolarRadiation));
}
