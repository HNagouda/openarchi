//! The bed and target level of the waters around a site, kept with the scene
//! so a section can draw water rather than ground bridged from bank to bank.
//!
//! Only the 250 m tiles that hold water are asked for, after the scene is
//! built, because only then is it known where the water is. Both answers are
//! cached per tile like every other source. Either can be missing without
//! being an error: most city canals have no surveyed bed, and a river has no
//! target level. A failed request is a warning and leaves the scene as it was.

use std::collections::BTreeSet;
use std::thread;

use geo::{BoundingRect, Geometry, Rect};

use super::terrain;
use crate::scene::{Layer, Scene, WaterLevel};
use crate::sources::cache::TileCache;
use crate::sources::edition;
use crate::sources::grid::{tiles_covering, Tile};
use crate::sources::raster::{self, Grid};
use crate::sources::rws;
use crate::{Error, Result};

/// Name of the answer kept in the tile cache for the newest bed edition.
const BED_ANSWER: &str = "rws-bed";
const LEVELS_DATASET: &str = "rws-streefpeil";
/// Tiles asked for at once. A large site near a river touches a few dozen.
const WORKERS: usize = 8;

/// How one tile's bed and levels are fetched: Rijkswaterstaat, or a stand-in in tests.
pub struct Fetchers<'a> {
    pub bed: &'a (dyn Fn(Tile) -> Result<Grid> + Sync),
    pub levels: &'a (dyn Fn(Rect<f64>) -> Result<Vec<WaterLevel>> + Sync),
}

/// Adds the bed and levels of the scene's waters, from the cache or from
/// Rijkswaterstaat. Asks nothing when the scene has no water.
pub fn keep(scene: &mut Scene, cache: &TileCache) {
    if water_tiles(scene).is_empty() {
        return;
    }
    let (bed_id, edition_error) = edition::resolve(
        cache,
        BED_ANSWER,
        || {
            let ids = rws::list_bed_ids()?;
            rws::newest_bed(&ids).ok_or_else(|| Error::Data {
                service: "Rijkswaterstaat".to_string(),
                detail: "the bed survey lists no editions".to_string(),
            })
        },
        rws::KNOWN_BED_ID.to_string(),
    );
    if let Some(error) = edition_error {
        scene.warnings.push(format!(
            "Rijkswaterstaat could not be asked for its newest bed survey, so {bed_id} was used: {error}"
        ));
    }

    let bed = |tile| rws::fetch_bed(&bed_id, tile);
    let fetchers = Fetchers {
        bed: &bed,
        levels: &rws::fetch_levels,
    };
    keep_with(scene, cache, &bed_id, &fetchers);
}

pub fn keep_with(scene: &mut Scene, cache: &TileCache, bed_id: &str, fetchers: &Fetchers) {
    let tiles = water_tiles(scene);
    if tiles.is_empty() {
        return;
    }
    let bed_dataset = format!("rws-{bed_id}");
    let answers: Vec<(Result<Grid>, Result<Vec<WaterLevel>>)> = tiles
        .chunks(WORKERS)
        .flat_map(|chunk| {
            thread::scope(|scope| {
                let handles: Vec<_> = chunk
                    .iter()
                    .map(|&tile| {
                        let bed_dataset = bed_dataset.as_str();
                        scope.spawn(move || {
                            (
                                bed_tile(cache, bed_dataset, tile, fetchers),
                                level_tile(cache, tile, fetchers),
                            )
                        })
                    })
                    .collect();
                handles
                    .into_iter()
                    .map(|handle| {
                        // Every failure comes back as a value, so a panic is a bug and goes on up.
                        handle
                            .join()
                            .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
                    })
                    .collect::<Vec<_>>()
            })
        })
        .collect();

    let mut beds = Vec::new();
    let mut levels: Vec<WaterLevel> = Vec::new();
    let mut bed_error = None;
    let mut level_error = None;
    for (bed, tile_levels) in answers {
        match bed {
            Ok(grid) => beds.push(grid),
            Err(error) => bed_error = bed_error.or(Some(error)),
        }
        match tile_levels {
            // A fairway crossing a tile edge comes back from every tile it touches.
            Ok(tile_levels) => {
                for level in tile_levels {
                    if !levels.contains(&level) {
                        levels.push(level);
                    }
                }
            }
            Err(error) => level_error = level_error.or(Some(error)),
        }
    }

    // A bed with a missing tile would draw a hole as if the water had no bed there.
    if let Some(error) = bed_error {
        scene.warnings.push(format!(
            "Rijkswaterstaat's bed survey could not be fetched, so sections draw water without its bed: {error}"
        ));
    } else {
        scene.water_bed = raster::join(&beds)
            .and_then(|bed| terrain::kept(&bed, reach(scene, &tiles), scene.origin));
    }
    if let Some(error) = level_error {
        scene.warnings.push(format!(
            "Rijkswaterstaat's target water levels could not be fetched, so sections draw water at its banks: {error}"
        ));
    } else {
        scene.water_levels = levels
            .into_iter()
            .map(|level| WaterLevel {
                line: rws::localised(level.line, scene.origin),
                ..level
            })
            .collect();
    }
    if scene.water_bed.is_some() || !scene.water_levels.is_empty() {
        scene.credits.push(rws::credit());
    }
}

/// The 250 m tiles (RD New) any water of the site or its neighbourhood
/// reaches, within the area the scene keeps terrain for.
fn water_tiles(scene: &Scene) -> Vec<Tile> {
    let Some(reach) = scene_reach(scene) else {
        return Vec::new();
    };
    let mut tiles = BTreeSet::new();
    for feature in scene.features.iter().chain(&scene.context) {
        if feature.layer != Layer::Water {
            continue;
        }
        let Some(bounds) = water_bounds(&feature.geometry) else {
            continue;
        };
        let Some(inside) = overlap(bounds, reach) else {
            continue;
        };
        let (x, y) = scene.origin;
        let rd = Rect::new(
            (inside.min().x + x, inside.min().y + y),
            (inside.max().x + x, inside.max().y + y),
        );
        tiles.extend(
            tiles_covering(rd)
                .into_iter()
                .map(|tile| (tile.row, tile.column)),
        );
    }

    tiles
        .into_iter()
        .map(|(row, column)| Tile { column, row })
        .collect()
}

fn water_bounds(geometry: &Geometry<f64>) -> Option<Rect<f64>> {
    match geometry {
        Geometry::Polygon(_) | Geometry::MultiPolygon(_) => geometry.bounding_rect(),
        _ => None,
    }
}

/// The area the scene keeps terrain for, in its own frame: sections reach no further.
fn scene_reach(scene: &Scene) -> Option<Rect<f64>> {
    let grid = scene.terrain.as_ref()?;
    Some(Rect::new(
        (grid.west, grid.south()),
        (grid.east(), grid.north),
    ))
}

/// The tiles' extent in RD New, cut to the scene's reach.
fn reach(scene: &Scene, tiles: &[Tile]) -> Rect<f64> {
    let (x, y) = scene.origin;
    let tiles_rect = tiles
        .iter()
        .map(|tile| tile.rect())
        .reduce(union)
        .unwrap_or_else(|| Rect::new((x, y), (x, y)));
    match scene_reach(scene) {
        Some(local) => {
            let rd = Rect::new(
                (local.min().x + x, local.min().y + y),
                (local.max().x + x, local.max().y + y),
            );
            overlap(tiles_rect, rd).unwrap_or(tiles_rect)
        }
        None => tiles_rect,
    }
}

fn union(a: Rect<f64>, b: Rect<f64>) -> Rect<f64> {
    Rect::new(
        (a.min().x.min(b.min().x), a.min().y.min(b.min().y)),
        (a.max().x.max(b.max().x), a.max().y.max(b.max().y)),
    )
}

fn overlap(a: Rect<f64>, b: Rect<f64>) -> Option<Rect<f64>> {
    let (west, south) = (a.min().x.max(b.min().x), a.min().y.max(b.min().y));
    let (east, north) = (a.max().x.min(b.max().x), a.max().y.min(b.max().y));
    (west < east && south < north).then(|| Rect::new((west, south), (east, north)))
}

fn bed_tile(cache: &TileCache, dataset: &str, tile: Tile, fetchers: &Fetchers) -> Result<Grid> {
    if let Some(bytes) = cache.read(dataset, tile).ok().flatten() {
        if let Ok(grid) = raster::from_bytes(&bytes) {
            return Ok(grid);
        }
    }
    let grid = (fetchers.bed)(tile)?;
    // A failed write only means fetching the tile again next time.
    let _ = cache.write(dataset, tile, &raster::to_bytes(&grid));

    Ok(grid)
}

fn level_tile(cache: &TileCache, tile: Tile, fetchers: &Fetchers) -> Result<Vec<WaterLevel>> {
    if let Some(bytes) = cache.read(LEVELS_DATASET, tile).ok().flatten() {
        if let Ok(levels) = serde_json::from_slice(&bytes) {
            return Ok(levels);
        }
    }
    let levels = (fetchers.levels)(tile.rect())?;
    if let Ok(bytes) = serde_json::to_vec(&levels) {
        let _ = cache.write(LEVELS_DATASET, tile, &bytes);
    }

    Ok(levels)
}

#[cfg(test)]
mod tests;
