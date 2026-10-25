//! The height of everything that can hide the sky: the ground, and the
//! buildings on it as prisms of their footprint and roof height, on a grid
//! that reaches past the site as far as the horizon is searched.
//!
//! Heights are metres above the terrain model's datum (NAP in the
//! Netherlands). A scene without terrain is taken as flat ground at 0, with
//! each building standing its roof height above it.

use crate::render::building_height;
use crate::scene::{Layer, Scene};
use crate::sources::raster::Grid;
use crate::sunlight::raster::Raster;
use crate::sunlight::{polygons, rings};

/// The ground, and the buildings on it, on a grid of square cells.
pub struct Surface {
    lattice: Raster,
    heights: Vec<f64>,
    /// The ground level where nothing is known, also used past the grid's edge.
    level: f64,
    pub highest: f64,
}

impl Surface {
    /// The height under a point; the ground level past the grid's edge.
    pub fn height_at(&self, (x, y): (f64, f64)) -> f64 {
        let column = ((x - self.lattice.min.0) / self.lattice.cell_m).floor();
        let row = ((y - self.lattice.min.1) / self.lattice.cell_m).floor();
        let is_inside = column >= 0.0
            && row >= 0.0
            && column < self.lattice.columns as f64
            && row < self.lattice.rows as f64;
        if !is_inside {
            return self.level;
        }

        self.heights[row as usize * self.lattice.columns + column as usize]
    }
}

/// The surface over `site` and `reach_m` around it, on the same cells, so a
/// ground cell of the site never lands on a building cell of the surface.
pub fn surface(scene: &Scene, site: &Raster, reach_m: f64) -> Surface {
    let margin = (reach_m / site.cell_m).ceil() as usize;
    let lattice = Raster {
        min: (
            site.min.0 - margin as f64 * site.cell_m,
            site.min.1 - margin as f64 * site.cell_m,
        ),
        cell_m: site.cell_m,
        columns: site.columns + 2 * margin,
        rows: site.rows + 2 * margin,
    };
    let terrain = scene.terrain.as_ref();
    let level = terrain.map_or(0.0, ground_level);
    let mut heights: Vec<f64> = (0..lattice.len())
        .map(|index| {
            terrain
                .and_then(|grid| terrain_at(grid, lattice.centre(index)))
                .unwrap_or(level)
        })
        .collect();

    let buildings = scene
        .features
        .iter()
        .chain(&scene.context)
        .filter(|feature| feature.layer == Layer::Building);
    for building in buildings {
        let (height_m, _) = building_height(building);
        // Without terrain the ground is flat at 0, so a building's own ground level would lift it off it.
        let ground_m = match terrain {
            Some(_) => building.attributes.ground_m.unwrap_or(level),
            None => level,
        };
        let top = ground_m + height_m;
        for polygon in polygons(&building.geometry) {
            lattice.fill_rings(&rings(polygon), |index| {
                heights[index] = heights[index].max(top);
            });
        }
    }

    let highest = heights.iter().copied().fold(level, f64::max);
    Surface {
        lattice,
        heights,
        level,
        highest,
    }
}

/// The terrain cell under a point, when it was measured. The terrain model
/// has no values under buildings and water.
fn terrain_at(grid: &Grid, (x, y): (f64, f64)) -> Option<f64> {
    let column = ((x - grid.west) / grid.cell_m).floor();
    let row = ((grid.north - y) / grid.cell_m).floor();
    let is_inside =
        column >= 0.0 && row >= 0.0 && column < grid.width as f64 && row < grid.height as f64;
    if !is_inside {
        return None;
    }

    let value = grid.value(column as usize, row as usize);
    value.is_finite().then_some(f64::from(value))
}

/// The median measured height, standing in for ground the terrain model left out.
fn ground_level(grid: &Grid) -> f64 {
    let mut measured: Vec<f32> = grid
        .values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect();
    if measured.is_empty() {
        return 0.0;
    }

    measured.sort_by(f32::total_cmp);
    f64::from(measured[measured.len() / 2])
}
