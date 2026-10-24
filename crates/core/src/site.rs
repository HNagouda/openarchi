//! Turns a drawn area into a scene: fetch every tile, join, clip, clean.
//!
//! Every source is fetched in the tiles of one fixed grid (`sources/grid.rs`)
//! and every tile is kept in a cache (`sources/cache.rs`), so drawing a site
//! again, or a site next door, only downloads the tiles not seen recently.
//! Tiles are joined back per source; a feature on a tile edge comes back from
//! each tile it touches and is kept once. A source with a failed tile becomes
//! a warning; the scene is only an error when nothing arrived.

mod boundary;
mod download;
#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::time::SystemTime;

use geo::{BoundingRect, Coord, Geometry, MapCoords, Polygon, Rect};

use self::boundary::{boundary_in_rd, site_centre};
use self::download::{Dataset, Download, Fetcher, Payload};
use crate::area::Area;
use crate::scene::{Credit, Feature, Layer, Scene};
use crate::sources::ahn::{self, CellTile, Elevation};
use crate::sources::cache::TileCache;
use crate::sources::clip::{self, Clipped};
use crate::sources::pdok::Record;
use crate::sources::{bag, bgt, brk, contours, heights};
use crate::{Error, Result};

const RD_NEW: &str = "EPSG:28992";

/// One step of a fetch, for a progress bar: "Buildings", 12 of 140.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Progress {
    pub step: String,
    pub done: usize,
    pub total: usize,
}

/// Every data source of a scene, in one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Source {
    Bgt,
    Bag,
    Brk,
    Ahn,
}

const SOURCES: [Source; 4] = [Source::Bgt, Source::Bag, Source::Brk, Source::Ahn];

impl Source {
    fn step(self) -> &'static str {
        match self {
            Source::Bgt => "Roads, water and green",
            Source::Bag => "Buildings",
            Source::Brk => "Parcels",
            Source::Ahn => "Elevation",
        }
    }

    fn credit(self) -> Credit {
        match self {
            Source::Bgt => bgt::credit(),
            Source::Bag => bag::credit(),
            Source::Brk => brk::credit(),
            Source::Ahn => ahn::credit(),
        }
    }
}

/// Fetches and assembles everything inside `area`, reusing tiles in `cache`.
/// Fails only when the area is invalid or every source failed; a single
/// failed source becomes a warning.
pub fn build_scene(
    area: &Area,
    cache: &TileCache,
    on_progress: &dyn Fn(Progress),
) -> Result<Scene> {
    build_scene_with(area, cache, &download::fetch_from_pdok, on_progress)
}

fn build_scene_with(
    area: &Area,
    cache: &TileCache,
    fetch: &Fetcher<'_>,
    on_progress: &dyn Fn(Progress),
) -> Result<Scene> {
    let boundary = boundary_in_rd(area)?;
    let origin = site_centre(area, &boundary);
    let bbox = boundary
        .bounding_rect()
        .ok_or_else(|| Error::Invalid("The drawn area has no points.".to_string()))?;

    let jobs = download::tiles_for(&boundary, bbox);
    // Every tile, then heights and contours, then clipping.
    let total = jobs.len() + 2;
    let report = |step: &str, done: usize| {
        on_progress(Progress {
            step: step.to_string(),
            done,
            total,
        });
    };
    report("Starting", 0);

    let (downloads, cache_errors) = download::download(jobs, cache, fetch, &report);
    let Joined {
        mut features,
        elevation,
        credits,
        mut warnings,
    } = join(downloads, bbox);
    if credits.is_empty() {
        return Err(Error::Network {
            service: "PDOK".to_string(),
            detail: warnings.join("; "),
        });
    }

    if let Some(elevation) = &elevation {
        add_building_heights(&mut features, elevation);
        features.extend(contours::contour_lines(&elevation.terrain));
    }
    report("Heights and contours", total - 1);

    let (features, unclippable) = clip_and_localise(features, &boundary, origin);
    if unclippable > 0 {
        warnings.push(format!(
            "{unclippable} features had broken shapes and were left out."
        ));
    }
    if let Some(error) = cache_errors.first() {
        warnings.push(format!(
            "{} downloaded tiles could not be kept for next time, the first because: {error}",
            cache_errors.len()
        ));
    }
    if let Err(error) = cache.prune() {
        warnings.push(format!("Old downloaded data could not be cleared: {error}"));
    }
    report("Clipping", total);

    Ok(Scene {
        area: area.clone(),
        crs: RD_NEW.to_string(),
        origin,
        boundary: boundary.map_coords(|coord| to_local_centimetres(coord, origin)),
        features,
        credits,
        fetched_at: humantime::format_rfc3339_seconds(SystemTime::now()).to_string(),
        warnings,
    })
}

/// The downloaded tiles put back together, per source.
#[derive(Default)]
struct Joined {
    features: Vec<Feature>,
    elevation: Option<Elevation>,
    credits: Vec<Credit>,
    warnings: Vec<String>,
}

/// Joins the tiles of each source. A source missing even one tile is left out
/// whole, because a layer with a hole in it would look like real emptiness.
fn join(mut downloads: Vec<Download>, bbox: Rect<f64>) -> Joined {
    // Tiles finish in any order; sorting keeps the scene the same every time.
    downloads.sort_by_key(|download| (download.tile.row, download.tile.column));

    let mut failed: HashMap<Source, Error> = HashMap::new();
    let mut records: HashMap<Dataset, Vec<Record>> = HashMap::new();
    let mut cells: HashMap<Dataset, Vec<CellTile>> = HashMap::new();
    for download in downloads {
        match download.payload {
            Ok(Payload::Records(tile_records)) => records
                .entry(download.dataset)
                .or_default()
                .extend(tile_records),
            Ok(Payload::Cells(tile_cells)) => cells
                .entry(download.dataset)
                .or_default()
                .push((download.tile.rect(), tile_cells)),
            Err(error) => {
                failed.entry(download.dataset.source()).or_insert(error);
            }
        }
    }

    let mut joined = Joined::default();
    let mut take = |dataset| unique(records.remove(&dataset).unwrap_or_default());
    for source in SOURCES {
        if let Some(error) = failed.remove(&source) {
            joined
                .warnings
                .push(format!("{} could not be fetched: {error}", source.step()));
            continue;
        }

        match source {
            Source::Bgt => {
                for collection in bgt::COLLECTIONS {
                    let tile_records = take(Dataset::Bgt(collection));
                    joined
                        .features
                        .extend(bgt::to_features(collection, tile_records));
                }
            }
            Source::Bag => joined.features.extend(bag::to_features(take(Dataset::Bag))),
            Source::Brk => joined.features.extend(brk::to_features(take(Dataset::Brk))),
            Source::Ahn => {
                let surface = cells.remove(&Dataset::AhnSurface).unwrap_or_default();
                let terrain = cells.remove(&Dataset::AhnTerrain).unwrap_or_default();
                joined.elevation = Some(Elevation {
                    surface: ahn::assemble(bbox, &surface),
                    terrain: ahn::assemble(bbox, &terrain),
                });
            }
        }
        joined.credits.push(source.credit());
    }

    joined
}

/// Each PDOK feature once. A feature crossing a tile edge comes back from
/// every tile it touches, always under the same id.
// #! NOTE[cache, dedupe] : Does PDOK give an edited feature a new id? Then tiles cached days apart could show both versions
fn unique(records: Vec<Record>) -> Vec<Record> {
    let mut seen = HashSet::new();
    records
        .into_iter()
        .filter(|record| match &record.id {
            Some(id) => seen.insert(id.clone()),
            None => true,
        })
        .collect()
}

// Heights use the whole footprint, before clipping, so a building cut by the
// boundary keeps the height of the whole building.
fn add_building_heights(features: &mut [Feature], elevation: &Elevation) {
    for feature in features
        .iter_mut()
        .filter(|feature| feature.layer == Layer::Building)
    {
        if let Geometry::Polygon(footprint) = &feature.geometry {
            let (height_m, ground_m) = heights::building_height(footprint, elevation);
            feature.attributes.height_m = height_m;
            feature.attributes.ground_m = ground_m;
        }
    }
}

/// Features cut to `boundary` and moved into the local frame, plus how many
/// had shapes too broken to cut.
fn clip_and_localise(
    features: Vec<Feature>,
    boundary: &Polygon<f64>,
    origin: (f64, f64),
) -> (Vec<Feature>, usize) {
    let mut kept = Vec::with_capacity(features.len());
    let mut unclippable = 0;
    for feature in features {
        match clip::clip(boundary, feature.geometry) {
            Clipped::Inside(geometry) => kept.push(Feature {
                geometry: geometry.map_coords(|coord| to_local_centimetres(coord, origin)),
                ..feature
            }),
            Clipped::Outside => {}
            Clipped::Broken => unclippable += 1,
        }
    }

    (kept, unclippable)
}

// Sources are surveyed to the centimetre; more digits only make project files bigger.
fn to_local_centimetres(coord: Coord<f64>, origin: (f64, f64)) -> Coord<f64> {
    Coord {
        x: ((coord.x - origin.0) * 100.0).round() / 100.0,
        y: ((coord.y - origin.1) * 100.0).round() / 100.0,
    }
}
