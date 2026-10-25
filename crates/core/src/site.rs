//! Turns a drawn area into a scene: fetch every tile, join, clip, clean.
//!
//! Every source is fetched in the tiles of one fixed grid (`sources/grid.rs`)
//! and every tile is kept in a cache (`sources/cache.rs`), so drawing a site
//! again, or a site next door, only downloads the tiles not seen recently.
//! Tiles are joined back per source; a feature on a tile edge comes back from
//! each tile it touches and is kept once. A source with a failed tile becomes
//! a warning; the scene is only an error when nothing arrived.

mod boundary;
mod context;
mod download;
mod editions;
mod environment;
mod solar;
mod source;
mod terrain;
#[cfg(test)]
mod tests;
mod water;

use std::collections::{HashMap, HashSet};
use std::time::SystemTime;

use geo::{BoundingRect, Coord, Geometry, MapCoords, Polygon, Rect};

// Shared with sheet sub-selections, which are drawn and checked like a site.
pub(crate) use self::boundary::boundary_in_rd;
use self::boundary::site_centre;
use self::download::{Dataset, Download, Fetcher, Payload};
pub(crate) use self::editions::years_before_editions_were_kept;
use self::editions::Editions;
use self::environment::{Raster, RIVM_RASTERS};
use self::source::{Source, BASE_SOURCES, SOURCES};
use crate::area::Area;
use crate::environment::Measured;
use crate::scene::{Credit, Feature, Layer, Scene, Walk};
use crate::sources::ahn::{self, CellTile, Elevation};
use crate::sources::cache::TileCache;
use crate::sources::clip::{self, Clipped};
use crate::sources::osm::{self, Osm};
use crate::sources::pdok::Record;
use crate::sources::raster::{self, Grid};
use crate::sources::{bag, bag_units, bands, bgt, brk, cbs, contours, heights};
use crate::walk;
use crate::{Error, Result};

const RD_NEW: &str = "EPSG:28992";

/// One step of a fetch, for a progress bar: "Buildings", 12 of 140.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Progress {
    pub step: String,
    pub done: usize,
    pub total: usize,
}

/// Fetches and assembles everything inside `area`, reusing tiles in `cache`.
/// Fails only when the area is invalid or every source failed; a single
/// failed source becomes a warning.
pub fn build_scene(
    area: &Area,
    cache: &TileCache,
    on_progress: &dyn Fn(Progress),
) -> Result<Scene> {
    let (editions, edition_warnings) = editions::find(cache);
    let mut scene = build_scene_with(
        area,
        cache,
        &editions,
        &download::fetch_from_pdok,
        on_progress,
    )?;
    scene.warnings.extend(edition_warnings);
    on_progress(Progress {
        step: "Water beds and levels".to_string(),
        done: 0,
        total: 1,
    });
    water::keep(&mut scene, cache);
    on_progress(Progress {
        step: "Typical year of sunlight".to_string(),
        done: 0,
        total: 1,
    });
    solar::keep(&mut scene, cache);

    Ok(scene)
}

fn build_scene_with(
    area: &Area,
    cache: &TileCache,
    editions: &Editions,
    fetch: &Fetcher<'_>,
    on_progress: &dyn Fn(Progress),
) -> Result<Scene> {
    let boundary = boundary_in_rd(area)?;
    let origin = site_centre(area, &boundary);
    let bbox = boundary
        .bounding_rect()
        .ok_or_else(|| Error::Invalid("The drawn area has no points.".to_string()))?;

    // The one Overpass query takes 30 s or more, so it goes first: queued
    // behind several hundred tiles it would start last and finish last.
    let mut jobs = vec![download::osm_job(origin, &boundary)];
    jobs.extend(download::tiles_for(&boundary, bbox, editions));
    let context_extent = context::extent(&boundary, origin);
    jobs.extend(context::jobs(&boundary, context_extent));
    // Every tile, then heights and contours, then walk times, then clipping.
    let total = jobs.len() + 3;
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
        osm,
        credits,
        mut warnings,
        mut measured,
        has_base,
        context,
        context_terrain,
    } = join(downloads, bbox, &boundary, editions);
    if !has_base {
        return Err(Error::Network {
            service: "PDOK".to_string(),
            detail: warnings.join("; "),
        });
    }

    if let Some(elevation) = &elevation {
        add_building_heights(&mut features, elevation);
        features.extend(contours::contour_lines(&elevation.terrain));
        let (slope, stats) = environment::slope_bands(&elevation.terrain, &boundary);
        features.extend(slope);
        measured.slope_percent = stats;
    }
    let terrain = kept_terrain(
        context_terrain,
        context_extent,
        elevation.as_ref(),
        bbox,
        origin,
    );
    report("Heights and contours", total - 2);

    let walk = osm.as_ref().and_then(|osm| walk_around(osm, origin));
    if let Some(osm) = osm {
        if walk.is_none() {
            warnings.push(
                "No street or path passes near the site centre, so walk times were left out."
                    .to_string(),
            );
        }
        features.extend(osm.places);
        features.extend(osm.stops);
    }
    report("Walk times", total - 1);

    let (features, unclippable) = clip_and_localise(features, &boundary, origin);
    let features = bands::without_hidden_bands(features);
    let (context, broken_context) = context::cut(context, &boundary, context_extent, origin);
    let unclippable = unclippable + broken_context;
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
        measured: environment::localised(measured, origin),
        walk,
        editions: editions.years(),
        context,
        terrain,
        water_bed: None,
        water_levels: Vec::new(),
        solar: None,
    })
}

/// The ground a section reads its profile from: the neighbourhood's terrain,
/// which reaches as far as a sheet draws, or the site's own when the
/// neighbourhood could not be fetched.
fn kept_terrain(
    context_terrain: Option<Grid>,
    context_extent: Rect<f64>,
    elevation: Option<&Elevation>,
    bbox: Rect<f64>,
    origin: (f64, f64),
) -> Option<Grid> {
    if let Some(grid) = context_terrain {
        return terrain::kept(&grid, context_extent, origin);
    }

    elevation.and_then(|elevation| terrain::kept(&elevation.terrain, ahn::extent(bbox), origin))
}

/// The walk from the site centre, moved into the scene's frame. Not clipped:
/// a walk does not stop at the boundary.
fn walk_around(osm: &Osm, origin: (f64, f64)) -> Option<Walk> {
    let result = walk::compute(osm, origin)?;
    let localise = |features: Vec<Feature>| -> Vec<Feature> {
        features
            .into_iter()
            .map(|feature| Feature {
                geometry: feature
                    .geometry
                    .map_coords(|coord| to_local_centimetres(coord, origin)),
                ..feature
            })
            .collect()
    };

    Some(Walk {
        bands: localise(result.bands),
        paths: localise(result.paths),
        places: localise(result.places),
    })
}

/// The downloaded tiles put back together, per source.
#[derive(Default)]
struct Joined {
    features: Vec<Feature>,
    elevation: Option<Elevation>,
    osm: Option<Osm>,
    credits: Vec<Credit>,
    warnings: Vec<String>,
    measured: Measured,
    /// At least one of `BASE_SOURCES` arrived.
    has_base: bool,
    /// The neighbourhood around the site, in RD New and not cut yet.
    context: Vec<Feature>,
    /// The neighbourhood's bare ground in 2 m cells, RD New, when every tile of it arrived.
    context_terrain: Option<Grid>,
}

/// Joins the tiles of each source. A source missing even one tile is left out
/// whole, because a layer with a hole in it would look like real emptiness.
fn join(
    mut downloads: Vec<Download>,
    bbox: Rect<f64>,
    boundary: &Polygon<f64>,
    editions: &Editions,
) -> Joined {
    // Tiles finish in any order; sorting keeps the scene the same every time.
    downloads.sort_by_key(|download| (download.tile.row, download.tile.column));

    let mut failed: HashMap<Source, Error> = HashMap::new();
    let mut records: HashMap<Dataset, Vec<Record>> = HashMap::new();
    let mut cells: HashMap<Dataset, Vec<CellTile>> = HashMap::new();
    let mut rasters: HashMap<Dataset, Vec<Grid>> = HashMap::new();
    let mut osm_body: Option<Vec<u8>> = None;
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
            Ok(Payload::Raster(grid)) => rasters.entry(download.dataset).or_default().push(grid),
            Ok(Payload::Osm(body)) => osm_body = Some(body),
            Err(error) => {
                failed.entry(download.dataset.source()).or_insert(error);
            }
        }
    }

    let mut joined = Joined::default();
    // Before the site takes its records: the neighbourhood reads the site's
    // tiles too. Without all of them it would have a hole that looks real.
    let has_context = [Source::Context, Source::Bgt, Source::Bag]
        .iter()
        .all(|source| !failed.contains_key(source));
    if has_context {
        // Joined before the neighbourhood takes its rasters, for the section profile.
        joined.context_terrain = rasters
            .get(&Dataset::ContextTerrain)
            .and_then(|tiles| raster::join(tiles));
        joined.context = context::features(&records, &mut rasters);
    }
    let mut take = |dataset| unique(records.remove(&dataset).unwrap_or_default());
    // Units first, so the buildings can count them.
    let units = (!failed.contains_key(&Source::BagUnits))
        .then(|| bag_units::totals_per_building(&take(Dataset::BagUnits)));
    let mut bands_of = |raster: Raster, joined: &mut Joined| {
        let dataset = Dataset::Raster(raster, editions.raster(raster).clone());
        let tiles = rasters.remove(&dataset).unwrap_or_default();
        let features = environment::raster_bands(raster, &tiles, boundary, &mut joined.measured);
        joined.features.extend(features);
    };
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
            Source::Bag => joined
                .features
                .extend(bag::to_features(take(Dataset::Bag), units.as_ref())),
            Source::Brk => joined.features.extend(brk::to_features(take(Dataset::Brk))),
            Source::Ahn => {
                let surface = cells.remove(&Dataset::AhnSurface).unwrap_or_default();
                let terrain = cells.remove(&Dataset::AhnTerrain).unwrap_or_default();
                joined.elevation = Some(Elevation {
                    surface: ahn::assemble(bbox, &surface),
                    terrain: ahn::assemble(bbox, &terrain),
                });
            }
            Source::Rivm => {
                for raster in RIVM_RASTERS {
                    bands_of(raster, &mut joined);
                }
            }
            Source::Liwo => bands_of(Raster::Flood, &mut joined),
            Source::Cbs => {
                let years = cbs::years(editions.population).map(|year| take(Dataset::Cbs(year)));
                joined.features.extend(cbs::to_features(years.into()));
            }
            Source::BagUnits | Source::Context => {}
            Source::Osm => match osm::parse(&osm_body.take().unwrap_or_default()) {
                Ok(parsed) => joined.osm = Some(parsed),
                Err(error) => {
                    joined
                        .warnings
                        .push(format!("{} could not be read: {error}", source.step()));
                    continue;
                }
            },
        }
        joined.has_base |= BASE_SOURCES.contains(&source);
        joined.credits.extend(source.credit());
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
