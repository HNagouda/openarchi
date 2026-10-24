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

mod raster;

use geo::{BooleanOps, Geometry, LineString, MultiPolygon, Polygon};

use crate::render::{building_height, shadow_per_metre, signed_area, tree_shadow_ring};
use crate::scene::{Attributes, Feature, Layer, Scene};
use crate::sun::{self, SunPosition};
use raster::Raster;

/// Minutes between two sun positions.
pub const STEP_MINUTES: u32 = 10;
const MINUTES_PER_DAY: u32 = 24 * 60;
/// The finest grid, in metres: a little over a parking bay is as fine as a site sheet reads.
const FINEST_CELL_M: f64 = 2.0;
/// At most this many cells, so a large site coarsens its grid instead of slowing down.
const MAX_CELLS: f64 = 160_000.0;

/// Hours of sun per grid cell. Cells outside the site or under a roof have none.
pub struct SunGrid {
    raster: Raster,
    hours: Vec<Option<f64>>,
}

/// What lies on a grid cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cell {
    Outside,
    Ground,
    Roof,
}

/// One thing that casts a shadow, reduced to what the shadow needs.
enum Caster {
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
    let raster = raster_over(&scene.boundary);
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
    let mut squares: Vec<Vec<Polygon<f64>>> = vec![Vec::new(); breaks.len() + 1];
    for (index, hours) in grid.hours.iter().enumerate() {
        let Some(hours) = hours else {
            continue;
        };
        let class = breaks.iter().filter(|&&limit| *hours >= limit).count();
        squares[class].push(cell_square(&grid.raster, index));
    }

    squares
        .into_iter()
        .enumerate()
        .filter(|(_, class_squares)| !class_squares.is_empty())
        .map(|(class, class_squares)| {
            // Squares never overlap, so one overlay merges them into the band's outline.
            let band = MultiPolygon::new(class_squares).union(&MultiPolygon::new(Vec::new()));
            let lower_edge = if class == 0 { 0.0 } else { breaks[class - 1] };
            Feature {
                layer: Layer::SunHours,
                geometry: Geometry::MultiPolygon(band),
                attributes: Attributes {
                    value: Some(lower_edge),
                    ..Attributes::default()
                },
            }
        })
        .collect()
}

/// A grid over the boundary's bounds, as fine as `MAX_CELLS` allows.
fn raster_over(boundary: &Polygon<f64>) -> Raster {
    let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
    let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for coord in &boundary.exterior().0 {
        min_x = min_x.min(coord.x);
        min_y = min_y.min(coord.y);
        max_x = max_x.max(coord.x);
        max_y = max_y.max(coord.y);
    }
    let width = (max_x - min_x).max(0.0);
    let height = (max_y - min_y).max(0.0);
    let cell_m = FINEST_CELL_M.max((width * height / MAX_CELLS).sqrt());

    Raster {
        min: (min_x, min_y),
        cell_m,
        columns: (width / cell_m).ceil() as usize,
        rows: (height / cell_m).ceil() as usize,
    }
}

/// Which cells lie in the site, and which of those under a roof.
fn cells(scene: &Scene, raster: &Raster) -> Vec<Cell> {
    let mut cells = vec![Cell::Outside; raster.len()];
    raster.fill_rings(&rings(&scene.boundary), |index| cells[index] = Cell::Ground);
    let buildings = scene
        .features
        .iter()
        .filter(|feature| feature.layer == Layer::Building);
    for building in buildings {
        for polygon in polygons(&building.geometry) {
            raster.fill_rings(&rings(polygon), |index| {
                if cells[index] == Cell::Ground {
                    cells[index] = Cell::Roof;
                }
            });
        }
    }

    cells
}

/// Buildings and trees in the site and its neighbourhood.
fn casters(scene: &Scene) -> Vec<Caster> {
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
fn shade(
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

fn rings(polygon: &Polygon<f64>) -> Vec<Vec<(f64, f64)>> {
    std::iter::once(polygon.exterior())
        .chain(polygon.interiors())
        .map(|ring| ring.0.iter().map(|c| (c.x, c.y)).collect())
        .collect()
}

fn polygons(geometry: &Geometry<f64>) -> Vec<&Polygon<f64>> {
    match geometry {
        Geometry::Polygon(polygon) => vec![polygon],
        Geometry::MultiPolygon(multi) => multi.0.iter().collect(),
        _ => Vec::new(),
    }
}

fn cell_square(raster: &Raster, index: usize) -> Polygon<f64> {
    let (x, y) = raster.centre(index);
    let half = raster.cell_m / 2.0;
    Polygon::new(
        LineString::from(vec![
            (x - half, y - half),
            (x + half, y - half),
            (x + half, y + half),
            (x - half, y + half),
            (x - half, y - half),
        ]),
        Vec::new(),
    )
}

#[cfg(test)]
mod tests;
