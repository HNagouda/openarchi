//! Hours of direct sun on the ground over one day, on a grid over the site.
//!
//! The method is a sun-on-ground study as BRE BR 209 describes it (Littlefair,
//! "Site layout planning for daylight and sunlight", third edition, 2022): step
//! the sun through the day, cast the shadows at each step, and add up the time
//! each point of the ground spends outside them. BR 209 asks that at least half
//! of a garden or open space sees two hours of sun on 21 March, the default
//! sun date of a sheet.
//!
//! Here the sun moves every ten minutes, from `sun.rs`. Buildings are prisms of
//! their footprint and roof height, trees a crown on a trunk, both exactly as
//! the shadow diagram casts them, and the neighbourhood around the site casts
//! shadows too. A grid cell counts a step as sunny when its centre lies outside
//! every shadow. A sun lower than the shadow diagram draws counts as no sun, as
//! it would only graze the ground.

use geo::{Geometry, Polygon};

use crate::grid::{banded, cells, raster_over, Cell, Raster};
use crate::render::{building_height, shadow_per_metre, signed_area, tree_shadow_ring};
use crate::scene::{polygons, Feature, Layer, Scene};
use crate::sun::{self, SunPosition};

/// Minutes between two sun positions.
pub const STEP_MINUTES: u32 = 10;
const MINUTES_PER_DAY: u32 = 24 * 60;
/// At most this many cells, so a large site coarsens its grid instead of slowing down.
const MAX_CELLS: f64 = 160_000.0;

/// Hours of sun per grid cell. Cells outside the site or under a roof have none.
pub struct SunGrid {
    raster: Raster,
    hours: Vec<Option<f64>>,
}

/// One thing that casts a shadow, reduced to what the shadow needs.
pub(crate) enum Caster {
    /// Every wall of a building, as its two ends at the foot, each wall turned
    /// so the building lies on its left, and the roof height.
    Building {
        walls: Vec<((f64, f64), (f64, f64))>,
        height_m: f64,
    },
    Tree {
        base: (f64, f64),
    },
}

/// The hours of direct sun on every ground cell of the site, on a day of the year.
pub fn sun_hours(scene: &Scene, month: u32, day: u32) -> SunGrid {
    let raster = raster_over(&scene.boundary, MAX_CELLS);
    let cells = cells(scene, &raster);
    let casters = casters(scene);
    let location = sun::site_location(&scene.area);
    let steps = MINUTES_PER_DAY / STEP_MINUTES;
    let mut shaded_at = vec![u32::MAX; raster.len()];
    let mut sunny_steps = vec![0u32; raster.len()];
    for step in 0..steps {
        // Each step stands for the ten minutes around its middle.
        let utc_hours = (f64::from(step) + 0.5) * f64::from(STEP_MINUTES) / 60.0;
        let sun = sun::position_utc(location, month, day, utc_hours);
        let Some(per_metre) = shadow_per_metre(&sun) else {
            continue;
        };
        for caster in &casters {
            shade(caster, per_metre, &sun, &raster, |index| {
                shaded_at[index] = step;
            });
        }
        for index in 0..raster.len() {
            if cells[index] == Cell::Ground && shaded_at[index] != step {
                sunny_steps[index] += 1;
            }
        }
    }

    let hours = cells
        .iter()
        .zip(&sunny_steps)
        .map(|(&cell, &sunny)| {
            (cell == Cell::Ground).then(|| f64::from(sunny * STEP_MINUTES) / 60.0)
        })
        .collect();
    SunGrid { raster, hours }
}

/// The grid as one area per class of `breaks`, each carrying the lower edge
/// of its class as its value, the way measured bands do. n breaks make up to
/// n + 1 areas; classes no cell falls in are left out.
pub fn bands(grid: &SunGrid, breaks: &[f64]) -> Vec<Feature> {
    banded(&grid.raster, &grid.hours, breaks, Layer::SunHours)
}

/// Buildings and trees in the site and its neighbourhood.
pub(crate) fn casters(scene: &Scene) -> Vec<Caster> {
    let mut casters = Vec::new();
    for feature in scene.features.iter().chain(&scene.context) {
        match (feature.layer, &feature.geometry) {
            (Layer::Building, geometry) => {
                let (height_m, _) = building_height(feature);
                let walls = polygons(geometry).into_iter().flat_map(walls).collect();
                casters.push(Caster::Building { walls, height_m });
            }
            (Layer::Tree, Geometry::Point(point)) => casters.push(Caster::Tree {
                base: (point.x(), point.y()),
            }),
            _ => {}
        }
    }

    casters
}

/// Marks the cells in the shadow of one caster.
///
/// A building's shadow on the ground around it is the band each wall sweeps
/// as the roof edge is moved along the shadow, and only walls facing away
/// from the sun sweep outwards: any point in the shadow lies within one
/// shadow length behind such a wall. The footprint itself is roof, not ground.
pub(crate) fn shade(
    caster: &Caster,
    per_metre: (f64, f64),
    sun: &SunPosition,
    raster: &Raster,
    mut mark: impl FnMut(usize),
) {
    match caster {
        Caster::Building { walls, height_m } => {
            let offset = (per_metre.0 * height_m, per_metre.1 * height_m);
            for &(start, end) in walls {
                // The building lies left of each wall, so (dy, -dx) points out of it.
                let outward = (end.1 - start.1, start.0 - end.0);
                if outward.0 * offset.0 + outward.1 * offset.1 <= 0.0 {
                    continue;
                }
                let band = [
                    start,
                    end,
                    (end.0 + offset.0, end.1 + offset.1),
                    (start.0 + offset.0, start.1 + offset.1),
                ];
                raster.fill_convex(&band, &mut mark);
            }
        }
        Caster::Tree { base } => {
            raster.fill_convex(&tree_shadow_ring(*base, per_metre, sun), &mut mark);
        }
    }
}

/// The walls of one footprint, turned so the building lies on the left of each:
/// the outer ring counterclockwise, the courtyards clockwise.
fn walls(polygon: &Polygon<f64>) -> Vec<((f64, f64), (f64, f64))> {
    let mut walls = Vec::new();
    for (index, ring) in std::iter::once(polygon.exterior())
        .chain(polygon.interiors())
        .enumerate()
    {
        let is_hole = index > 0;
        let mut points: Vec<(f64, f64)> = ring.0.iter().map(|c| (c.x, c.y)).collect();
        let is_counterclockwise = signed_area(&points) > 0.0;
        if is_counterclockwise == is_hole {
            points.reverse();
        }
        walls.extend(points.windows(2).map(|pair| (pair[0], pair[1])));
    }

    walls
}

#[cfg(test)]
mod tests;
