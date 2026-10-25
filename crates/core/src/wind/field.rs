//! The grid the flow is solved on, turned so the wind blows along it, and
//! the solved flow read back at any point of the scene.
//!
//! The grid covers the site and a margin of neighbourhood around it, so the
//! buildings upwind shape the air before it reaches the site. Buildings of
//! the site and the neighbourhood are both solid.

use geo::Polygon;

use super::lattice::{self, Obstacles};
use crate::grid::{cell_size, rings, Raster};
use crate::scene::{polygons, Layer, Scene};

/// Neighbourhood around the site that takes part, in metres. On the three test
/// sites, 400 m kept every band's share of the site within 4 points of 200 m,
/// except the windiest at the Westermarkt (16 to 6 %): more buildings upwind
/// shelter more. 400 m took four times as long to solve.
// #! NOTE[wind, margin] : 200 m trades the windiest band's extent for speed; a larger margin with a coarser grid is the other choice.
pub const MARGIN_M: f64 = 200.0;
/// At most this many cells, so a large site coarsens its grid instead of
/// slowing down: about half a second to solve in a release build.
pub const MAX_CELLS: f64 = 40_000.0;

/// The solved flow, in a grid whose x runs downwind.
pub struct Field {
    /// The direction the air moves, a unit vector in the scene's frame (east, north).
    downwind: (f64, f64),
    /// The grid in the wind's frame.
    raster: Raster,
    /// Velocity per cell as a share of the incoming wind, in the wind's frame.
    velocity: Vec<(f32, f32)>,
    solid: Vec<bool>,
}

impl Field {
    /// The cells of the grid around `boundary` for wind from `from_deg`, and
    /// which of them are buildings: what the solver needs, before solving.
    pub fn grid(scene: &Scene, from_deg: f64) -> (Raster, (f64, f64), Obstacles) {
        let towards = (from_deg + 180.0).to_radians();
        let downwind = (towards.sin(), towards.cos());
        let raster = wind_raster(&scene.boundary, downwind);

        let mut solid = vec![false; raster.len()];
        let buildings = scene
            .features
            .iter()
            .chain(&scene.context)
            .filter(|feature| feature.layer == Layer::Building);
        for building in buildings {
            for polygon in polygons(&building.geometry) {
                let turned: Vec<Vec<(f64, f64)>> = rings(polygon)
                    .into_iter()
                    .map(|ring| {
                        ring.into_iter()
                            .map(|point| to_wind(downwind, point))
                            .collect()
                    })
                    .collect();
                raster.fill_rings(&turned, |index| solid[index] = true);
            }
        }
        let obstacles = Obstacles {
            columns: raster.columns,
            rows: raster.rows,
            solid,
        };
        (raster, downwind, obstacles)
    }

    /// Solves the flow over the grid `grid` returned.
    pub fn solve(raster: Raster, downwind: (f64, f64), obstacles: Obstacles) -> Field {
        let velocity = lattice::mean_flow(&obstacles);
        Field {
            downwind,
            raster,
            velocity,
            solid: obstacles.solid,
        }
    }

    pub fn cell_m(&self) -> f64 {
        self.raster.cell_m
    }

    /// The flow at a scene point, as a share of the incoming wind in the
    /// scene's frame, blended from the four nearest cells. Building cells
    /// count as still air, so the flow slows towards a wall. `None` off the
    /// grid or inside a building.
    pub fn at(&self, point: (f64, f64)) -> Option<(f64, f64)> {
        let (along, across) = to_wind(self.downwind, point);
        let raster = &self.raster;
        let x = (along - raster.min.0) / raster.cell_m - 0.5;
        let y = (across - raster.min.1) / raster.cell_m - 0.5;
        let last_x = raster.columns as f64 - 1.0;
        let last_y = raster.rows as f64 - 1.0;
        if !(0.0..=last_x).contains(&x) || !(0.0..=last_y).contains(&y) {
            return None;
        }
        let nearest = y.round() as usize * raster.columns + x.round() as usize;
        if self.solid[nearest] {
            return None;
        }

        let (left, bottom) = (x.floor() as usize, y.floor() as usize);
        let (right, top) = (
            (left + 1).min(raster.columns - 1),
            (bottom + 1).min(raster.rows - 1),
        );
        let (share_x, share_y) = ((x - left as f64) as f32, (y - bottom as f64) as f32);
        let cell = |column: usize, row: usize| self.velocity[row * raster.columns + column];
        let blend = |a: (f32, f32), b: (f32, f32), share: f32| {
            (a.0 + (b.0 - a.0) * share, a.1 + (b.1 - a.1) * share)
        };
        let lower = blend(cell(left, bottom), cell(right, bottom), share_x);
        let upper = blend(cell(left, top), cell(right, top), share_x);
        let (along, across) = blend(lower, upper, share_y);

        Some(to_scene(
            self.downwind,
            (f64::from(along), f64::from(across)),
        ))
    }
}

/// A grid over the boundary and the margin around it, square to the wind.
fn wind_raster(boundary: &Polygon<f64>, downwind: (f64, f64)) -> Raster {
    let (mut low, mut high) = (
        (f64::INFINITY, f64::INFINITY),
        (f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for coord in &boundary.exterior().0 {
        let (along, across) = to_wind(downwind, (coord.x, coord.y));
        low = (low.0.min(along), low.1.min(across));
        high = (high.0.max(along), high.1.max(across));
    }
    let width = (high.0 - low.0).max(0.0) + 2.0 * MARGIN_M;
    let height = (high.1 - low.1).max(0.0) + 2.0 * MARGIN_M;
    let cell_m = cell_size(width, height, MAX_CELLS);

    Raster {
        min: (low.0 - MARGIN_M, low.1 - MARGIN_M),
        cell_m,
        columns: (width / cell_m).ceil() as usize,
        rows: (height / cell_m).ceil() as usize,
    }
}

/// A scene point in the wind's frame: along the wind, and across it to the left.
fn to_wind((east, north): (f64, f64), (x, y): (f64, f64)) -> (f64, f64) {
    (x * east + y * north, -x * north + y * east)
}

/// Back from the wind's frame to the scene's.
fn to_scene((east, north): (f64, f64), (along, across): (f64, f64)) -> (f64, f64) {
    (along * east - across * north, along * north + across * east)
}
