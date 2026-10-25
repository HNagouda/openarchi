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
mod greenery;
mod join;
mod solar;
mod source;
mod terrain;
#[cfg(test)]
mod tests;
mod water;

use std::time::SystemTime;

use geo::{BoundingRect, Coord, MapCoords, Rect};

// Shared with sheet sub-selections, which are drawn and checked like a site.
pub(crate) use self::boundary::boundary_in_rd;
use self::boundary::site_centre;
pub use self::boundary::{measure, AreaSize};
use self::download::Fetcher;
pub(crate) use self::editions::years_before_editions_were_kept;
use self::editions::Editions;
use self::join::{add_building_heights, clip_and_localise, join, unique, Joined};
use self::source::Source;
use crate::area::Area;
use crate::scene::{Feature, Scene, Walk};
use crate::sources::ahn::{self, Elevation};
use crate::sources::cache::TileCache;
use crate::sources::osm::Osm;
use crate::sources::raster::Grid;
use crate::sources::{bands, contours};
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
        // Contours and slope read the same filled and smoothed ground, made once.
        let ground = contours::prepared(&elevation.terrain);
        features.extend(contours::contour_lines(&ground));
        features.extend(environment::slope_bands(&ground, &boundary, &mut measured));
    }
    let terrain = kept_terrain(
        context_terrain,
        context_extent,
        elevation.as_ref(),
        bbox,
        origin,
    );
    report("Heights and contours", total - 2);

    let walk = match osm.as_ref().map(|osm| walk_around(osm, origin)) {
        Some(Ok(Some(walk))) => Some(walk),
        Some(Ok(None)) => {
            warnings.push(
                "No street or path passes near the site centre, so walk times were left out."
                    .to_string(),
            );
            None
        }
        Some(Err(error)) => {
            warnings.push(format!("Walk times were left out: {error}."));
            None
        }
        None => None,
    };
    if let Some(osm) = osm {
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
        wind: None,
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
fn walk_around(osm: &Osm, origin: (f64, f64)) -> Result<Option<Walk>> {
    let Some(walk) = walk::compute(osm, origin)? else {
        return Ok(None);
    };
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

    Ok(Some(Walk {
        bands: localise(walk.bands),
        paths: localise(walk.paths),
        places: localise(walk.places),
    }))
}

// Sources are surveyed to the centimetre; more digits only make project files bigger.
fn to_local_centimetres(coord: Coord<f64>, origin: (f64, f64)) -> Coord<f64> {
    Coord {
        x: ((coord.x - origin.0) * 100.0).round() / 100.0,
        y: ((coord.y - origin.1) * 100.0).round() / 100.0,
    }
}
