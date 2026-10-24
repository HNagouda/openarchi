//! Getting every tile a site needs: from the cache where it holds a fresh
//! copy, from PDOK where it does not.
//!
//! Missing tiles go to a fixed pool of worker threads rather than one thread
//! per tile. A 1 km site needs about 900 tiles, and PDOK is a shared public
//! service, so the number of requests in flight stays bounded.

use std::sync::{mpsc, Mutex, PoisonError};
use std::thread;
use std::vec;

use geo::{Polygon, Rect};

use super::editions::Editions;
use super::environment::{self, Raster, RIVM_RASTERS};
use super::Source;
use crate::sources::cache::TileCache;
use crate::sources::edition::Edition;
use crate::sources::grid::{tiles_covering, tiles_touching, Tile, TILE_M};
use crate::sources::pdok::Record;
use crate::sources::raster::{self, Grid};
use crate::sources::{ahn, bag, bag_units, bgt, brk, cbs, osm, wcs};
use crate::{Error, Result};

/// Requests in flight at once. With 32, an uncached 1 km site arrives in about
/// 12 s; 16 took twice as long. Fetching a 1 km site as a few large boxes kept
/// about 50 requests in flight, so this asks no more of PDOK than before.
const WORKERS: usize = 32;

/// The progress step shown while tiles come out of the cache.
const REUSED_STEP: &str = "Reusing data downloaded earlier";

/// Elevation cells in one tile, which a cached raster must hold exactly.
pub const CELLS_PER_TILE: usize = ((TILE_M / ahn::CELL_M) * (TILE_M / ahn::CELL_M)) as usize;
const CELL_BYTES: usize = std::mem::size_of::<f32>();

/// One thing fetched per tile and cached on its own.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Dataset {
    /// One BGT collection, such as "wegdeel".
    Bgt(&'static str),
    Bag,
    Brk,
    AhnSurface,
    AhnTerrain,
    /// One edition of an environmental raster, placed by its own GeoTIFF.
    Raster(Raster, Edition),
    /// The CBS 100 m statistics of one year.
    Cbs(i32),
    /// BAG verblijfsobjecten, the units inside buildings.
    BagUnits,
    /// One Overpass answer for every site centred in the tile, reaching this
    /// many metres around the tile's centre. See `sources/osm`.
    Osm(u32),
    /// One BGT collection or the BAG buildings in a tile of the neighbourhood
    /// around the site. Cached with the site's own tiles of the same data.
    ContextBgt(&'static str),
    ContextBag,
    /// AHN in 2 m cells, for the heights of buildings around the site.
    ContextSurface,
    ContextTerrain,
}

/// What one tile of a dataset holds.
pub enum Payload {
    Records(Vec<Record>),
    /// Elevation cells, row by row from the north.
    Cells(Vec<f32>),
    Raster(Grid),
    /// An Overpass answer as it arrived, parsed when the scene is put together.
    Osm(Vec<u8>),
}

/// How a tile is fetched when the cache does not have it: `fetch_from_pdok`,
/// or a stand-in in tests.
pub type Fetcher<'a> = dyn Fn(&Dataset, Tile) -> Result<Payload> + Sync + 'a;

/// One tile of one dataset, fetched or failed.
pub struct Download {
    pub dataset: Dataset,
    pub tile: Tile,
    pub payload: Result<Payload>,
}

impl Dataset {
    pub fn source(&self) -> Source {
        match self {
            Dataset::Bgt(_) => Source::Bgt,
            Dataset::Bag => Source::Bag,
            Dataset::Brk => Source::Brk,
            Dataset::AhnSurface | Dataset::AhnTerrain => Source::Ahn,
            Dataset::Raster(Raster::Flood, _) => Source::Liwo,
            Dataset::Raster(..) => Source::Rivm,
            Dataset::Cbs(_) => Source::Cbs,
            Dataset::BagUnits => Source::BagUnits,
            Dataset::Osm(_) => Source::Osm,
            Dataset::ContextBgt(_)
            | Dataset::ContextBag
            | Dataset::ContextSurface
            | Dataset::ContextTerrain => Source::Context,
        }
    }

    /// The dataset's folder in the cache: "bgt-wegdeel", "ahn-dsm_05m". Every
    /// edition has a folder of its own, so a new one is fetched fresh and never
    /// mixed with tiles of the one before.
    fn cache_name(&self) -> String {
        match self {
            Dataset::Bgt(collection) | Dataset::ContextBgt(collection) => {
                format!("bgt-{collection}")
            }
            Dataset::Bag | Dataset::ContextBag => "bag-pand".to_string(),
            Dataset::Brk => "brk-perceel".to_string(),
            Dataset::AhnSurface => format!("ahn-{}", ahn::SURFACE_COVERAGE),
            Dataset::AhnTerrain => format!("ahn-{}", ahn::TERRAIN_COVERAGE),
            Dataset::Raster(_, edition) => format!("wcs-{}", edition.id),
            Dataset::Cbs(year) => format!("cbs-vierkant100-{year}"),
            Dataset::BagUnits => "bag-verblijfsobject".to_string(),
            Dataset::Osm(radius_m) => format!("{}-r{radius_m}", osm::CACHE_NAME),
            Dataset::ContextSurface => format!("ahn-{}-coarse", ahn::SURFACE_COVERAGE),
            Dataset::ContextTerrain => format!("ahn-{}-coarse", ahn::TERRAIN_COVERAGE),
        }
    }
}

/// Every dataset and tile a site needs. Map data only for tiles the boundary
/// reaches; elevation for the whole box plus margin, because building heights
/// and contours need ground beyond the boundary.
pub fn tiles_for(
    boundary: &Polygon<f64>,
    bbox: Rect<f64>,
    editions: &Editions,
) -> Vec<(Dataset, Tile)> {
    let mut vector_datasets: Vec<Dataset> =
        bgt::COLLECTIONS.into_iter().map(Dataset::Bgt).collect();
    vector_datasets.push(Dataset::Bag);
    vector_datasets.push(Dataset::Brk);
    vector_datasets.push(Dataset::BagUnits);
    vector_datasets.extend(cbs::years(editions.population).map(Dataset::Cbs));
    let mut rasters: Vec<Dataset> = RIVM_RASTERS
        .into_iter()
        .map(|raster| Dataset::Raster(raster, editions.raster(raster).clone()))
        .collect();
    rasters.push(Dataset::Raster(Raster::Flood, editions.flood.clone()));

    let mut jobs = Vec::new();
    for tile in tiles_touching(boundary) {
        for dataset in &vector_datasets {
            jobs.push((dataset.clone(), tile));
        }
    }
    for tile in tiles_covering(ahn::extent(bbox)) {
        jobs.push((Dataset::AhnSurface, tile));
        jobs.push((Dataset::AhnTerrain, tile));
    }
    for tile in environment::tiles(bbox) {
        for dataset in &rasters {
            jobs.push((dataset.clone(), tile));
        }
    }

    jobs
}

/// The one OpenStreetMap job of a site centred at `origin`: the tile the
/// centre falls in, and a reach that covers the walk and the whole site.
pub fn osm_job(origin: (f64, f64), boundary: &Polygon<f64>) -> (Dataset, Tile) {
    let site_radius_m = boundary
        .exterior()
        .points()
        .map(|point| ((point.x() - origin.0).powi(2) + (point.y() - origin.1).powi(2)).sqrt())
        .fold(0.0, f64::max);
    let tile = Tile {
        column: (origin.0 / TILE_M).floor() as i64,
        row: (origin.1 / TILE_M).floor() as i64,
    };

    (Dataset::Osm(osm::radius_for(site_radius_m)), tile)
}

pub fn fetch_from_pdok(dataset: &Dataset, tile: Tile) -> Result<Payload> {
    let rect = tile.rect();
    match dataset {
        Dataset::Bgt(collection) => bgt::fetch_tile(collection, rect).map(Payload::Records),
        Dataset::Bag => bag::fetch_tile(rect).map(Payload::Records),
        Dataset::Brk => brk::fetch_tile(rect).map(Payload::Records),
        Dataset::AhnSurface => ahn::fetch_tile(ahn::SURFACE_COVERAGE, rect).map(Payload::Cells),
        Dataset::AhnTerrain => ahn::fetch_tile(ahn::TERRAIN_COVERAGE, rect).map(Payload::Cells),
        Dataset::Raster(raster, edition) => {
            wcs::fetch(&raster.coverage(edition), rect).map(Payload::Raster)
        }
        Dataset::Cbs(year) => cbs::fetch_tile(*year, rect).map(Payload::Records),
        Dataset::BagUnits => bag_units::fetch_tile(rect).map(Payload::Records),
        // Not PDOK, but fetched and cached through the same pool and cache.
        Dataset::Osm(radius_m) => osm::fetch(tile, *radius_m).map(Payload::Osm),
        Dataset::ContextBgt(collection) => bgt::fetch_tile(collection, rect).map(Payload::Records),
        Dataset::ContextBag => bag::fetch_tile(rect).map(Payload::Records),
        Dataset::ContextSurface => {
            ahn::fetch_coarse_tile(ahn::SURFACE_COVERAGE, rect).map(Payload::Raster)
        }
        Dataset::ContextTerrain => {
            ahn::fetch_coarse_tile(ahn::TERRAIN_COVERAGE, rect).map(Payload::Raster)
        }
    }
}

/// Every tile in `jobs`, from `cache` when it has a fresh copy and from
/// `fetch` otherwise. `report` runs on the caller's thread with the number of
/// tiles done. Also returns the errors from writing the cache: they cost the
/// next fetch some time, never this one its data.
pub fn download(
    jobs: Vec<(Dataset, Tile)>,
    cache: &TileCache,
    fetch: &Fetcher<'_>,
    report: &dyn Fn(&str, usize),
) -> (Vec<Download>, Vec<Error>) {
    let mut downloads = Vec::with_capacity(jobs.len());
    let mut misses = Vec::new();
    for (dataset, tile) in jobs {
        match from_cache(cache, &dataset, tile) {
            Some(payload) => downloads.push(Download {
                dataset,
                tile,
                payload: Ok(payload),
            }),
            None => misses.push((dataset, tile)),
        }
    }
    if !downloads.is_empty() {
        report(REUSED_STEP, downloads.len());
    }

    let mut cache_errors = Vec::new();
    let queue = Mutex::new(misses.into_iter());
    let (sender, receiver) = mpsc::channel();
    thread::scope(|scope| {
        for _ in 0..WORKERS {
            let sender = sender.clone();
            let queue = &queue;
            scope.spawn(move || {
                while let Some((dataset, tile)) = next_job(queue) {
                    let payload = fetch(&dataset, tile);
                    let stored = match &payload {
                        Ok(payload) => store(cache, &dataset, tile, payload),
                        Err(_) => Ok(()),
                    };
                    let download = Download {
                        dataset,
                        tile,
                        payload,
                    };
                    // The receiver outlives every sender, so this send cannot fail.
                    let _ = sender.send((download, stored));
                }
            });
        }
        drop(sender);

        // Runs on the caller's thread while the workers are still fetching.
        for (download, stored) in receiver.iter() {
            report(download.dataset.source().step(), downloads.len() + 1);
            downloads.push(download);
            if let Err(error) = stored {
                cache_errors.push(error);
            }
        }
    });

    (downloads, cache_errors)
}

fn next_job(queue: &Mutex<vec::IntoIter<(Dataset, Tile)>>) -> Option<(Dataset, Tile)> {
    // Taking the next item cannot leave the iterator half changed, so a lock
    // poisoned by a panicking worker is still safe to use.
    queue.lock().unwrap_or_else(PoisonError::into_inner).next()
}

// A tile the cache cannot read or decode is fetched again, which also
// replaces the broken copy. That is the whole repair, so the error goes no further.
fn from_cache(cache: &TileCache, dataset: &Dataset, tile: Tile) -> Option<Payload> {
    let bytes = cache.read(&dataset.cache_name(), tile).ok()??;
    decode(dataset, &bytes).ok()
}

fn store(cache: &TileCache, dataset: &Dataset, tile: Tile, payload: &Payload) -> Result<()> {
    let bytes = match payload {
        Payload::Records(records) => serde_json::to_vec(records).map_err(|error| {
            Error::Invalid(format!(
                "could not encode {dataset:?} for the cache: {error}"
            ))
        })?,
        Payload::Cells(cells) => cells.iter().flat_map(|cell| cell.to_le_bytes()).collect(),
        Payload::Raster(grid) => raster::to_bytes(grid),
        Payload::Osm(body) => body.clone(),
    };
    cache.write(&dataset.cache_name(), tile, &bytes)
}

fn decode(dataset: &Dataset, bytes: &[u8]) -> Result<Payload> {
    let corrupt = |detail: String| Error::Invalid(format!("cached {dataset:?} tile: {detail}"));
    match dataset {
        Dataset::AhnSurface | Dataset::AhnTerrain => {
            if bytes.len() != CELLS_PER_TILE * CELL_BYTES {
                return Err(corrupt(format!("{} bytes", bytes.len())));
            }
            let (chunks, _) = bytes.as_chunks::<CELL_BYTES>();
            let cells = chunks
                .iter()
                .map(|chunk| f32::from_le_bytes(*chunk))
                .collect();
            Ok(Payload::Cells(cells))
        }
        Dataset::Raster(..) | Dataset::ContextSurface | Dataset::ContextTerrain => {
            raster::from_bytes(bytes).map(Payload::Raster)
        }
        Dataset::Bgt(_)
        | Dataset::Bag
        | Dataset::Brk
        | Dataset::Cbs(_)
        | Dataset::BagUnits
        | Dataset::ContextBgt(_)
        | Dataset::ContextBag => serde_json::from_slice(bytes)
            .map(Payload::Records)
            .map_err(|error| corrupt(error.to_string())),
        Dataset::Osm(_) => Ok(Payload::Osm(bytes.to_vec())),
    }
}
