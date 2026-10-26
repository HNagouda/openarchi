//! The aerial photo the 3D view lays on the ground in its realistic finish,
//! fetched only when the view first asks for it, never with the site.
//!
//! It comes in two levels so it stays a few MB: a finer photo over the site
//! and a coarser one over the neighbourhood, which the view sees from further
//! away and through haze. Each level is the 250 m tiles of the shared grid,
//! fetched and cached one by one like every other source, and sent as the
//! JPEGs the service returned; the view puts them side by side into one
//! texture per level. A tile's size is the largest power of two that keeps
//! its level within a texture side, so a small site gets about half a metre
//! a pixel and a large one about two.
//
// #! NOTE[3d, countries] : Only PDOK's photo of the Netherlands is asked for; another country needs its own source and a choice here

use std::sync::{Mutex, PoisonError};
use std::thread;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use geo::{BoundingRect, Rect};
use serde::Serialize;

use crate::scene::{polygons, Scene};
use crate::sources::aerial;
use crate::sources::cache::TileCache;
use crate::sources::grid::{tiles_covering, Tile, TILE_M};
use crate::{Error, Result};

/// The longest texture side of each level, in pixels: 2048 is about 16 MB of
/// GPU memory, which every WebGL2 device has to spare; the neighbourhood's
/// photo takes a quarter of that.
const SITE_PX: u32 = 2048;
const SURROUNDINGS_PX: u32 = 1024;
/// Tile sizes to choose from, finest first. 512 is about half a metre a pixel,
/// near the 25 cm the photo has; finer would grow the tiles and not the view.
const TILE_PX: [u32; 4] = [512, 256, 128, 64];
/// Requests in flight at once: a site's photo is tens of tiles, not hundreds.
const WORKERS: usize = 8;

#[derive(Debug, Clone, Serialize)]
pub struct Photo {
    /// The site first, then the neighbourhood when the scene has one.
    pub levels: Vec<PhotoLevel>,
    /// "Beeldmateriaal Nederland, aerial photo, via PDOK (CC BY 4.0)", shown wherever the photo shows.
    pub credit: String,
    /// Tiles that could not be fetched; the photo is shown without them.
    pub warnings: Vec<String>,
}

/// One texture's worth of tiles, in the scene's local metres.
#[derive(Debug, Clone, Serialize)]
pub struct PhotoLevel {
    pub west: f64,
    pub south: f64,
    pub columns: u32,
    pub rows: u32,
    pub tile_m: f64,
    pub tile_px: u32,
    pub tiles: Vec<PhotoTile>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PhotoTile {
    /// Counted from the level's west and south edges.
    pub column: u32,
    pub row: u32,
    /// The JPEG, in base64.
    pub jpeg: String,
}

/// The photo over `scene` and its neighbourhood, from `cache` or PDOK. An
/// error only when no tile at all could be had.
pub fn photo(scene: &Scene, cache: &TileCache) -> Result<Photo> {
    let mut levels = Vec::new();
    let mut warnings = Vec::new();
    let site = scene
        .boundary
        .bounding_rect()
        .ok_or_else(|| Error::Invalid("The site has no boundary.".to_string()))?;
    levels.push(level(scene, site, SITE_PX, cache, &mut warnings));
    if let Some(around) = surroundings(scene, site) {
        levels.push(level(scene, around, SURROUNDINGS_PX, cache, &mut warnings));
    }
    let tile_count: usize = levels.iter().map(|level| level.tiles.len()).sum();
    if tile_count == 0 {
        return Err(Error::Data {
            service: "PDOK luchtfoto".to_string(),
            detail: warnings.join("; "),
        });
    }

    let credit = aerial::credit();
    Ok(Photo {
        levels,
        credit: format!("{} ({})", credit.name, credit.license),
        warnings,
    })
}

/// The box round the site and every feature of its neighbourhood, `None` without one.
fn surroundings(scene: &Scene, site: Rect<f64>) -> Option<Rect<f64>> {
    let mut reach = site;
    let mut has_any = false;
    for feature in &scene.context {
        for polygon in polygons(&feature.geometry) {
            if let Some(rect) = polygon.bounding_rect() {
                reach = union(reach, rect);
                has_any = true;
            }
        }
    }
    has_any.then_some(reach)
}

fn union(a: Rect<f64>, b: Rect<f64>) -> Rect<f64> {
    Rect::new(
        (a.min().x.min(b.min().x), a.min().y.min(b.min().y)),
        (a.max().x.max(b.max().x), a.max().y.max(b.max().y)),
    )
}

/// The tiles over `local` (a box in the scene's metres) at the finest size
/// that keeps the level within `max_px`.
fn level(
    scene: &Scene,
    local: Rect<f64>,
    max_px: u32,
    cache: &TileCache,
    warnings: &mut Vec<String>,
) -> PhotoLevel {
    let (east, north) = scene.origin;
    let real = Rect::new(
        (local.min().x + east, local.min().y + north),
        (local.max().x + east, local.max().y + north),
    );
    let tiles = tiles_covering(real);
    let first = tiles[0];
    let columns = tiles
        .iter()
        .map(|tile| tile.column)
        .max()
        .unwrap_or(first.column)
        - first.column
        + 1;
    let rows = tiles.iter().map(|tile| tile.row).max().unwrap_or(first.row) - first.row + 1;
    let tile_px = tile_px(columns.max(rows) as u32, max_px);

    let fetched = fetch_all(&tiles, tile_px, cache);
    let mut kept = Vec::new();
    for (tile, result) in tiles.iter().zip(fetched) {
        match result {
            Ok((jpeg, kept_warning)) => {
                warnings.extend(kept_warning);
                kept.push(PhotoTile {
                    column: (tile.column - first.column) as u32,
                    row: (tile.row - first.row) as u32,
                    jpeg: BASE64.encode(jpeg),
                });
            }
            Err(error) => warnings.push(error.to_string()),
        }
    }

    PhotoLevel {
        west: first.rect().min().x - east,
        south: first.rect().min().y - north,
        columns: columns as u32,
        rows: rows as u32,
        tile_m: TILE_M,
        tile_px,
        tiles: kept,
    }
}

/// The largest tile size that fits `across` tiles into `max_px`, or the smallest there is.
fn tile_px(across: u32, max_px: u32) -> u32 {
    TILE_PX
        .into_iter()
        .find(|px| across * px <= max_px)
        .unwrap_or(TILE_PX[TILE_PX.len() - 1])
}

/// Every tile's JPEG, in the order of `tiles`, from the cache or else from
/// PDOK, with a warning when a fetched tile could not be kept.
fn fetch_all(
    tiles: &[Tile],
    tile_px: u32,
    cache: &TileCache,
) -> Vec<Result<(Vec<u8>, Option<String>)>> {
    let dataset = dataset(tile_px);
    let queue = Mutex::new(tiles.iter().copied().enumerate());
    let results = Mutex::new(Vec::new());
    thread::scope(|scope| {
        for _ in 0..WORKERS {
            scope.spawn(|| loop {
                // Taking the next item cannot leave the iterator half changed,
                // so a lock poisoned by a panicking worker is still safe to use.
                let next = queue.lock().unwrap_or_else(PoisonError::into_inner).next();
                let Some((index, tile)) = next else {
                    break;
                };
                let rect = tile.rect();
                let bounds = [rect.min().x, rect.min().y, rect.max().x, rect.max().y];
                let result = cache.read_or_fetch(&dataset, tile, || aerial::fetch(bounds, tile_px));
                results
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push((index, result));
            });
        }
    });

    let mut results = results.into_inner().unwrap_or_else(PoisonError::into_inner);
    results.sort_by_key(|(index, _)| *index);
    results.into_iter().map(|(_, result)| result).collect()
}

/// The cache folder of the photo at one tile size: "luchtfotorgb-Actueel_ortho25-512".
fn dataset(tile_px: u32) -> String {
    format!("luchtfotorgb-{}-{tile_px}", aerial::LAYER)
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use geo::Rect;

    use super::*;
    use crate::area::{Area, LonLat};

    /// A 250 m tile round the Westerkerk from Actueel_ortho25, 2026-10-26.
    const WESTERKERK: &[u8] = include_bytes!("../../tests/fixtures/luchtfotorgb-westerkerk.jpg");
    /// A tile corner by the Westermarkt in RD New, so a small site spans four tiles.
    const ORIGIN: (f64, f64) = (120_750.0, 487_750.0);

    fn site(radius_m: f64) -> Scene {
        Scene {
            area: Area::Circle {
                center: LonLat {
                    lon: 4.8837,
                    lat: 52.3747,
                },
                radius_m,
            },
            crs: "EPSG:28992".to_string(),
            origin: ORIGIN,
            boundary: Rect::new((-radius_m, -radius_m), (radius_m, radius_m)).to_polygon(),
            features: Vec::new(),
            credits: Vec::new(),
            fetched_at: String::new(),
            warnings: Vec::new(),
            measured: Default::default(),
            walk: None,
            editions: Default::default(),
            context: Vec::new(),
            terrain: None,
            water_bed: None,
            water_levels: Vec::new(),
            solar: None,
            wind: None,
        }
    }

    #[test]
    fn a_small_site_gets_fine_tiles_and_a_large_one_coarse_tiles() {
        assert_eq!(tile_px(3, SITE_PX), 512);
        assert_eq!(tile_px(4, SITE_PX), 512);
        assert_eq!(tile_px(5, SITE_PX), 256);
        assert_eq!(tile_px(9, SITE_PX), 128);
        assert_eq!(tile_px(60, SITE_PX), 64);
        assert_eq!(tile_px(6, SURROUNDINGS_PX), 128);
    }

    #[test]
    fn the_photo_comes_from_the_cache_tile_by_tile_without_asking_pdok() {
        let scene = site(100.0);
        let folder = tempfile::tempdir().unwrap();
        let cache = TileCache::at(folder.path());
        let real = Rect::new(
            (ORIGIN.0 - 100.0, ORIGIN.1 - 100.0),
            (ORIGIN.0 + 100.0, ORIGIN.1 + 100.0),
        );
        for tile in tiles_covering(real) {
            cache.write(&dataset(512), tile, WESTERKERK).unwrap();
        }

        let photo = photo(&scene, &cache).unwrap();

        assert_eq!(photo.levels.len(), 1, "no neighbourhood, no second level");
        let level = &photo.levels[0];
        assert_eq!((level.columns, level.rows, level.tile_px), (2, 2, 512));
        assert_eq!(level.tiles.len(), 4);
        assert!(photo.warnings.is_empty(), "{:?}", photo.warnings);
        assert!(photo.credit.contains("Beeldmateriaal"));
        // The level starts on the grid: the tile edge south-west of the site, in local metres.
        assert_eq!(
            (level.west, level.south),
            (120_500.0 - ORIGIN.0, 487_500.0 - ORIGIN.1)
        );
    }

    #[test]
    #[ignore = "asks PDOK"]
    fn the_live_photo_of_the_westermarkt_arrives_within_a_few_mb() {
        let scene = site(300.0);
        let folder = tempfile::tempdir().unwrap();
        let cache = TileCache::at(folder.path());

        let started = Instant::now();
        let photo = photo(&scene, &cache).unwrap();
        let bytes: usize = photo
            .levels
            .iter()
            .flat_map(|level| &level.tiles)
            .map(|tile| tile.jpeg.len())
            .sum();
        println!(
            "{} tiles, {:.0} KB of base64 in {:.1} s",
            photo.levels[0].tiles.len(),
            bytes as f64 / 1024.0,
            started.elapsed().as_secs_f64()
        );

        assert!(photo.warnings.is_empty(), "{:?}", photo.warnings);
        assert!(bytes < 4 * 1024 * 1024);
    }
}
