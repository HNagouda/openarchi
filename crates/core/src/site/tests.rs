use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use geo::{polygon, Geometry, Intersects};
use serde_json::json;
use tempfile::TempDir;

use super::download::{Dataset, Payload, CELLS_PER_TILE};
use super::*;
use crate::area::LonLat;
use crate::crs::wgs84_to_rd;
use crate::sources::cache::MAX_AGE;
use crate::sources::edition::Edition;
use crate::sources::grid::{Tile, TILE_M};
use crate::sources::pdok::Record;
use crate::sources::raster::Grid;

const WESTERMARKT: LonLat = LonLat {
    lon: 4.8840,
    lat: 52.3745,
};
const RADIUS_M: f64 = 200.0;
/// About 50 m east, roughly how far a user nudges a site.
const NUDGE_DEGREES: f64 = 0.00075;

fn site(center: LonLat) -> Area {
    Area::Circle {
        center,
        radius_m: RADIUS_M,
    }
}

/// A 10 m building straddling the tile edge nearest to the Westermarkt.
fn building_on_a_tile_edge() -> Geometry<f64> {
    let (x, y) = wgs84_to_rd(WESTERMARKT);
    let edge = (x / TILE_M).round() * TILE_M;
    Geometry::Polygon(polygon![
        (x: edge - 5.0, y: y - 5.0),
        (x: edge + 5.0, y: y - 5.0),
        (x: edge + 5.0, y: y + 5.0),
        (x: edge - 5.0, y: y + 5.0),
    ])
}

/// PDOK as a test sees it: one building, wherever a tile overlaps it, flat
/// ground at 1 m with roofs at 11 m, 50 everywhere on every environmental
/// raster, and nothing else. Counts every request.
fn fake_pdok(requests: &AtomicUsize, dataset: &Dataset, tile: Tile) -> Result<Payload> {
    requests.fetch_add(1, Ordering::SeqCst);
    let building = building_on_a_tile_edge();
    match dataset {
        Dataset::Bag if building.intersects(&tile.rect()) => Ok(Payload::Records(vec![Record {
            id: Some("pand-1".to_string()),
            properties: json!({"identificatie": "0363100012345678", "status": "Pand in gebruik"})
                .as_object()
                .unwrap()
                .clone(),
            geometry: building,
        }])),
        Dataset::AhnSurface => Ok(Payload::Cells(vec![11.0; CELLS_PER_TILE])),
        Dataset::AhnTerrain => Ok(Payload::Cells(vec![1.0; CELLS_PER_TILE])),
        Dataset::Raster(..) => Ok(Payload::Raster(Grid {
            west: tile.rect().min().x,
            north: tile.rect().max().y,
            cell_m: 25.0,
            width: 10,
            height: 10,
            values: vec![50.0; 100],
        })),
        Dataset::Osm(_) => Ok(Payload::Osm(br#"{"elements": []}"#.to_vec())),
        _ => Ok(Payload::Records(Vec::new())),
    }
}

/// Builds the scene for `area` against the fake, in the editions named in
/// code, and returns it with the number of requests it made.
fn fetch_counting(area: &Area, cache: &TileCache) -> (Scene, usize) {
    fetch_counting_in(area, cache, &Editions::known())
}

fn fetch_counting_in(area: &Area, cache: &TileCache, editions: &Editions) -> (Scene, usize) {
    let requests = AtomicUsize::new(0);
    let fetch = |dataset: &Dataset, tile| fake_pdok(&requests, dataset, tile);
    let scene = build_scene_with(area, cache, editions, &fetch, &|_| {}).unwrap();
    (scene, requests.load(Ordering::SeqCst))
}

fn age_every_file(folder: &Path, by: Duration) {
    for entry in fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            age_every_file(&path, by);
        } else {
            let file = fs::File::options().write(true).open(&path).unwrap();
            file.set_modified(SystemTime::now() - by).unwrap();
        }
    }
}

#[test]
fn fetching_the_same_site_again_makes_no_requests() {
    let folder = TempDir::new().unwrap();
    let cache = TileCache::at(folder.path());

    let (_, first) = fetch_counting(&site(WESTERMARKT), &cache);
    let (scene, second) = fetch_counting(&site(WESTERMARKT), &cache);

    assert!(first > 0);
    assert_eq!(second, 0);
    let credited = SOURCES
        .iter()
        .filter(|source| source.credit().is_some())
        .count();
    assert_eq!(scene.credits.len(), credited);
    assert_eq!(scene.measured.no2_ug_m3.map(|stats| stats.mean), Some(50.0));
}

#[test]
fn a_new_edition_is_fetched_fresh_and_the_scene_names_its_year() {
    let folder = TempDir::new().unwrap();
    let cache = TileCache::at(folder.path());
    let area = site(WESTERMARKT);
    let boundary = boundary::boundary_in_rd(&area).unwrap();
    let bbox = boundary.bounding_rect().unwrap();
    let mut newer = Editions::known();
    newer.no2 = Edition {
        year: 2025,
        id: "alo__rivm_nsl_20270401_gm_NO22025".to_string(),
    };
    let new_tiles = download::tiles_for(&boundary, bbox, &newer)
        .into_iter()
        .filter(|(dataset, _)| matches!(dataset, Dataset::Raster(Raster::No2, _)))
        .count();

    let (old, _) = fetch_counting(&area, &cache);
    let (new, refetched) = fetch_counting_in(&area, &cache, &newer);

    assert_eq!(refetched, new_tiles);
    assert_eq!(old.year_of(Layer::No2), Some(2024));
    assert_eq!(new.year_of(Layer::No2), Some(2025));
    assert_eq!(new.year_of(Layer::RoadNoise), Some(2022));
}

#[test]
fn a_nudged_site_only_fetches_the_tiles_it_moved_into() {
    let folder = TempDir::new().unwrap();
    let cache = TileCache::at(folder.path());
    let nudged = LonLat {
        lon: WESTERMARKT.lon + NUDGE_DEGREES,
        lat: WESTERMARKT.lat,
    };

    let (_, first) = fetch_counting(&site(WESTERMARKT), &cache);
    let (_, second) = fetch_counting(&site(nudged), &cache);

    assert!(second > 0);
    assert!(
        second < first / 2,
        "{second} of {first} tiles fetched again"
    );
}

#[test]
fn a_building_on_a_tile_edge_appears_once_with_its_height() {
    let folder = TempDir::new().unwrap();
    let cache = TileCache::at(folder.path());

    let (scene, _) = fetch_counting(&site(WESTERMARKT), &cache);

    let buildings: Vec<&Feature> = scene
        .features
        .iter()
        .filter(|feature| feature.layer == Layer::Building)
        .collect();
    assert_eq!(buildings.len(), 1);
    assert_eq!(buildings[0].attributes.height_m, Some(10.0));
}

#[test]
fn tiles_older_than_the_maximum_age_are_fetched_again() {
    let folder = TempDir::new().unwrap();
    let cache = TileCache::at(folder.path());

    let (_, first) = fetch_counting(&site(WESTERMARKT), &cache);
    age_every_file(folder.path(), MAX_AGE + Duration::from_secs(60));
    let (_, second) = fetch_counting(&site(WESTERMARKT), &cache);

    assert_eq!(second, first);
}

#[test]
fn a_failed_tile_leaves_its_source_out_and_a_retry_fetches_only_that_tile() {
    let folder = TempDir::new().unwrap();
    let cache = TileCache::at(folder.path());
    let area = site(WESTERMARKT);
    let boundary = boundary::boundary_in_rd(&area).unwrap();
    let editions = Editions::known();
    let (_, failing_tile) =
        download::tiles_for(&boundary, boundary.bounding_rect().unwrap(), &editions)
            .into_iter()
            .find(|(dataset, _)| *dataset == Dataset::Brk)
            .unwrap();
    let requests = AtomicUsize::new(0);
    let one_parcel_tile_fails = |dataset: &Dataset, tile| {
        if *dataset == Dataset::Brk && tile == failing_tile {
            return Err(Error::Network {
                service: "PDOK BRK".to_string(),
                detail: "timed out".to_string(),
            });
        }
        fake_pdok(&requests, dataset, tile)
    };

    let scene =
        build_scene_with(&area, &cache, &editions, &one_parcel_tile_fails, &|_| {}).unwrap();
    let (_, retried) = fetch_counting(&area, &cache);

    let credited = SOURCES
        .iter()
        .filter(|source| source.credit().is_some())
        .count();
    assert_eq!(scene.credits.len(), credited - 1);
    assert!(scene.warnings[0].starts_with("Parcels could not be fetched"));
    assert_eq!(retried, 1);
}
