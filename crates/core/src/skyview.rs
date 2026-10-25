//! The sky view factor of the ground: how much of the sky a point on the
//! ground sees, from 0 at the foot of a deep shaft to 1 on open land.
//!
//! The factor is the cosine-weighted one, the share of the sky's light a
//! level surface would receive from a sky equally bright everywhere, as
//! Johnson & Watson define it for urban canyons ("The determination of
//! view-factors in urban canyons", Journal of Climate and Applied
//! Meteorology 23, 1984) and Oke uses for urban climate. It is found from
//! the horizon: walk out from each ground cell in `DIRECTIONS` directions,
//! keep the steepest elevation angle anything rises to within
//! `SEARCH_RADIUS_M`, and take SVF = 1 - mean(sin² horizon), the horizon
//! method on a height raster that Zakšek, Oštir & Kokalj describe ("Sky-view
//! factor as a relief visualization technique", Remote Sensing 3, 2011).
//!
//! What hides the sky: buildings in the site and its neighbourhood as prisms
//! of their footprint and roof height, and the terrain where the scene kept
//! it. The same factor weights the diffuse light in `radiation.rs`.

mod surface;

use std::thread;

use crate::scene::{Feature, Layer, Scene};
use crate::sunlight::raster::Raster;
use crate::sunlight::{banded, cells, raster_over, Cell};
use surface::Surface;

/// Directions the horizon is searched in, evenly round the compass.
pub const DIRECTIONS: usize = 32;
/// How far out the horizon is searched. A 30 m block 150 m away still rises 11°.
pub const SEARCH_RADIUS_M: f64 = 150.0;
/// At most this many cells, a quarter of what the hours of sun allow: the
/// factor changes slowly over the ground, and each cell walks 32 lines.
pub const MAX_CELLS: f64 = 40_000.0;

// #! NOTE[skyview, trees] : Trees are left out of the sky view and the radiation: the scene has tree positions but no height or crown, crowns are partly see-through, and bare half the year. Count them as solid crowns, like the shadow diagram does?

/// The sky view factor per grid cell. Cells outside the site or under a roof have none.
pub struct SkyGrid {
    pub(crate) raster: Raster,
    pub(crate) factors: Vec<Option<f64>>,
}

/// The sky view factor of every ground cell of the site.
pub fn sky_view(scene: &Scene) -> SkyGrid {
    let raster = raster_over(&scene.boundary, MAX_CELLS);
    let cells = cells(scene, &raster);
    let factors = factors(scene, &raster, &cells);

    SkyGrid { raster, factors }
}

/// The grid as bands of `breaks`, each carrying the lower edge of its class.
pub fn bands(grid: &SkyGrid, breaks: &[f64]) -> Vec<Feature> {
    banded(&grid.raster, &grid.factors, breaks, Layer::SkyView)
}

/// The factor of each ground cell of `raster`, `None` elsewhere. Every cell
/// stands on its own, so the cells are shared out over the processor's cores.
pub(crate) fn factors(scene: &Scene, raster: &Raster, cells: &[Cell]) -> Vec<Option<f64>> {
    let surface = surface::surface(scene, raster, SEARCH_RADIUS_M);
    let directions: Vec<(f64, f64)> = (0..DIRECTIONS)
        .map(|index| {
            let azimuth = std::f64::consts::TAU * index as f64 / DIRECTIONS as f64;
            (azimuth.sin(), azimuth.cos())
        })
        .collect();
    let cores = thread::available_parallelism().map_or(1, |cores| cores.get());
    let chunk = cells.len().div_ceil(cores).max(1);

    thread::scope(|scope| {
        let parts: Vec<_> = cells
            .chunks(chunk)
            .enumerate()
            .map(|(part, part_cells)| {
                let (surface, directions) = (&surface, &directions);
                scope.spawn(move || {
                    let first = part * chunk;
                    part_cells
                        .iter()
                        .enumerate()
                        .map(|(offset, &cell)| {
                            (cell == Cell::Ground)
                                .then(|| factor_at(surface, raster, first + offset, directions))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        parts
            .into_iter()
            // A panic in a part is a bug and goes on up.
            .flat_map(|part| {
                part.join()
                    .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
            })
            .collect()
    })
}

/// 1 - mean(sin² horizon) over every direction, for the cell `index`.
fn factor_at(surface: &Surface, raster: &Raster, index: usize, directions: &[(f64, f64)]) -> f64 {
    let point = raster.centre(index);
    let base = surface.height_at(point);
    let mut hidden = 0.0;
    for &direction in directions {
        let tangent = horizon_tangent(surface, raster.cell_m, point, base, direction);
        // sin² of the angle whose tangent is t is t² / (1 + t²).
        hidden += tangent * tangent / (1.0 + tangent * tangent);
    }

    1.0 - hidden / directions.len() as f64
}

/// The tangent of the steepest elevation angle seen from `point` at height
/// `base`, walking out one cell at a time. Ground lower than the point
/// counts as a level horizon, so the sky is the hemisphere above it.
fn horizon_tangent(
    surface: &Surface,
    step_m: f64,
    point: (f64, f64),
    base: f64,
    (east, north): (f64, f64),
) -> f64 {
    let mut steepest: f64 = 0.0;
    let mut distance = step_m;
    while distance <= SEARCH_RADIUS_M {
        // Nothing farther out can rise above the horizon found so far.
        if surface.highest - base <= steepest * distance {
            break;
        }
        let height = surface.height_at((point.0 + east * distance, point.1 + north * distance));
        steepest = steepest.max((height - base) / distance);
        distance += step_m;
    }

    steepest
}

#[cfg(test)]
pub(crate) mod tests;
