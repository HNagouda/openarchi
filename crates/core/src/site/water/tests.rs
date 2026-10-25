use std::sync::atomic::{AtomicUsize, Ordering};

use geo::{polygon, LineString, Polygon};

use super::*;
use crate::area::{Area, LonLat};
use crate::scene::{Attributes, Feature};

const ORIGIN: (f64, f64) = (120_600.0, 487_600.0);

/// A scene with terrain over 200 m around the origin and one canal 20 m wide across it.
fn scene_with_canal() -> Scene {
    let canal = Feature {
        layer: Layer::Water,
        geometry: Geometry::Polygon(polygon![
            (x: -100.0, y: -10.0),
            (x: 100.0, y: -10.0),
            (x: 100.0, y: 10.0),
            (x: -100.0, y: 10.0),
        ]),
        attributes: Attributes::default(),
    };
    Scene {
        area: Area::Circle {
            center: LonLat {
                lon: 4.88,
                lat: 52.37,
            },
            radius_m: 100.0,
        },
        crs: "EPSG:28992".to_string(),
        origin: ORIGIN,
        boundary: Polygon::new(LineString::new(vec![]), vec![]),
        features: vec![canal],
        credits: vec![],
        fetched_at: String::new(),
        warnings: vec![],
        measured: Default::default(),
        walk: None,
        editions: Default::default(),
        context: vec![],
        terrain: Some(Grid {
            west: -100.0,
            north: 100.0,
            cell_m: 4.0,
            width: 50,
            height: 50,
            values: vec![1.0; 2500],
        }),
        water_bed: None,
        water_levels: Vec::new(),
        solar: None,
        wind: None,
    }
}

/// A level line in RD New.
fn level(name: &str, level_m: f64, points: Vec<(f64, f64)>) -> WaterLevel {
    WaterLevel {
        name: name.to_string(),
        level_m,
        line: LineString::from(points),
    }
}

fn bed_of(tile: Tile) -> Grid {
    let rect = tile.rect();
    Grid {
        west: rect.min().x,
        north: rect.max().y,
        cell_m: 1.0,
        width: 250,
        height: 250,
        values: vec![-3.0; 250 * 250],
    }
}

#[test]
fn a_canal_gets_its_bed_and_level_in_the_scene_frame() {
    let cache_dir = tempfile::tempdir().unwrap();
    let cache = TileCache::at(cache_dir.path());
    let mut scene = scene_with_canal();
    let levels = |_rect| {
        Ok(vec![level(
            "Amstellands boezempeil",
            -0.4,
            vec![(120_500.0, 487_600.0), (120_700.0, 487_600.0)],
        )])
    };
    let fetchers = Fetchers {
        bed: &|tile| Ok(bed_of(tile)),
        levels: &levels,
    };
    keep_with(&mut scene, &cache, "test", &fetchers);

    let bed = scene.water_bed.expect("a bed");
    assert_eq!(bed.cell_m, 4.0);
    assert!(bed.values.iter().all(|value| *value == -3.0));
    assert_eq!(scene.water_levels.len(), 1);
    assert_eq!(scene.water_levels[0].line.0[0].x, -100.0);
    assert!(scene
        .credits
        .iter()
        .any(|credit| credit.name.contains("Rijkswaterstaat")));
}

#[test]
fn tiles_asked_once_come_from_the_cache_the_second_time() {
    let cache_dir = tempfile::tempdir().unwrap();
    let cache = TileCache::at(cache_dir.path());
    let asked = AtomicUsize::new(0);
    let bed = |tile| {
        asked.fetch_add(1, Ordering::SeqCst);
        Ok(bed_of(tile))
    };
    let fetchers = Fetchers {
        bed: &bed,
        levels: &|_rect| Ok(Vec::new()),
    };
    keep_with(&mut scene_with_canal(), &cache, "test", &fetchers);
    let first = asked.load(Ordering::SeqCst);
    keep_with(&mut scene_with_canal(), &cache, "test", &fetchers);

    assert!(first > 0);
    assert_eq!(asked.load(Ordering::SeqCst), first);
}

#[test]
fn a_failed_survey_is_a_warning_and_no_bed() {
    let cache_dir = tempfile::tempdir().unwrap();
    let cache = TileCache::at(cache_dir.path());
    let mut scene = scene_with_canal();
    let fetchers = Fetchers {
        bed: &|_tile| {
            Err(Error::Data {
                service: "Rijkswaterstaat".to_string(),
                detail: "down".to_string(),
            })
        },
        levels: &|_rect| Ok(Vec::new()),
    };
    keep_with(&mut scene, &cache, "test", &fetchers);

    assert!(scene.water_bed.is_none());
    assert!(scene
        .warnings
        .iter()
        .any(|warning| warning.contains("bed survey")));
    assert!(!scene
        .credits
        .iter()
        .any(|credit| credit.name.contains("Rijkswaterstaat")));
}

#[test]
fn a_scene_without_water_asks_nothing() {
    let cache_dir = tempfile::tempdir().unwrap();
    let cache = TileCache::at(cache_dir.path());
    let mut scene = scene_with_canal();
    scene
        .features
        .retain(|feature| feature.layer != Layer::Water);
    let fetchers = Fetchers {
        bed: &|_tile| panic!("asked for a bed"),
        levels: &|_rect| panic!("asked for levels"),
    };
    keep_with(&mut scene, &cache, "test", &fetchers);

    assert!(scene.water_bed.is_none());
}
