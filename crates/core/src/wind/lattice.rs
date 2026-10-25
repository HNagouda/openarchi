//! A two-dimensional lattice Boltzmann flow solver: D2Q9 with the BGK
//! collision (Qian, d'Humières & Lallemand, "Lattice BGK models for
//! Navier-Stokes equation", Europhysics Letters 17, 1992; reviewed in Chen &
//! Doolen, "Lattice Boltzmann method for fluid flows", Annual Review of Fluid
//! Mechanics 30, 1998).
//!
//! The grid runs with the wind: air enters along the left edge at a steady
//! speed, leaves along the right edge, and the top and bottom edges are far
//! field, held at the incoming flow. Solid cells reflect what streams into
//! them (half-way bounce-back), which gives walls that hold the air still.
//! The flow behind buildings never settles completely, since wakes shed
//! eddies, so the answer is the velocity averaged over the last part of the run.

/// The nine lattice directions: rest, the four sides, the four diagonals.
const DIRECTIONS: usize = 9;
const CX: [i32; DIRECTIONS] = [0, 1, 0, -1, 0, 1, -1, -1, 1];
const CY: [i32; DIRECTIONS] = [0, 0, 1, 0, -1, 1, 1, -1, -1];
const WEIGHTS: [f32; DIRECTIONS] = [
    4.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
];
/// The direction straight back, for bounce-back.
const OPPOSITE: [usize; DIRECTIONS] = [0, 3, 4, 1, 2, 7, 8, 5, 6];

/// The incoming speed in lattice units. The method is only accurate well below
/// the lattice speed of sound (1/√3); 0.1 keeps compressibility errors near 3 %.
const INFLOW_SPEED: f32 = 0.1;
/// The BGK relaxation time. Viscosity is (τ − ½)/3; closer to ½ is less viscous
/// and closer to real air, but plain BGK grows unstable as τ nears ½. 0.55 ran
/// stable on every test site, where 0.53 already drew longer, jet-like streets.
// #! NOTE[wind, method] : A Smagorinsky subgrid model (Hou et al. 1996) at τ 0.505 let more air into dense streets but blew up on the coarse grid of a large site; plain BGK stays.
const RELAXATION_TIME: f32 = 0.55;
/// How long the air runs, in passes through the grid at the incoming speed:
/// first to wash out the start, then averaged. On the three test sites, one
/// pass in all puts under 1 % of cells in another band than five passes do.
const SETTLING_PASSES: f32 = 0.75;
const AVERAGED_PASSES: f32 = 0.25;

/// A grid of cells, numbered row by row from the upwind, lower corner.
pub struct Obstacles {
    pub columns: usize,
    pub rows: usize,
    pub solid: Vec<bool>,
}

/// The time-averaged velocity of every cell as a share of the incoming wind,
/// along the grid and across it. Solid cells are still.
pub fn mean_flow(obstacles: &Obstacles) -> Vec<(f32, f32)> {
    let (columns, rows) = (obstacles.columns, obstacles.rows);
    let inflow = equilibrium(1.0, INFLOW_SPEED, 0.0);
    let mut current = vec![0.0f32; DIRECTIONS * columns * rows];
    for cell in 0..columns * rows {
        if !obstacles.solid[cell] {
            for direction in 0..DIRECTIONS {
                current[direction * columns * rows + cell] = inflow[direction];
            }
        }
    }
    let mut next = current.clone();
    let mut sum = vec![(0.0f32, 0.0f32); columns * rows];

    let pass = columns as f32 / INFLOW_SPEED;
    let settling = (SETTLING_PASSES * pass) as usize;
    let averaged = ((AVERAGED_PASSES * pass) as usize).max(1);
    for step in 0..settling + averaged {
        let sum = (step >= settling).then_some(sum.as_mut_slice());
        advance(obstacles, &inflow, &current, &mut next, sum);
        std::mem::swap(&mut current, &mut next);
    }

    let scale = 1.0 / (averaged as f32 * INFLOW_SPEED);
    sum.iter().map(|&(x, y)| (x * scale, y * scale)).collect()
}

/// One step: every fluid cell pulls what streams into it, then relaxes
/// towards equilibrium. Adds the new velocities to `sum` when given.
fn advance(
    obstacles: &Obstacles,
    inflow: &[f32; DIRECTIONS],
    current: &[f32],
    next: &mut [f32],
    mut sum: Option<&mut [(f32, f32)]>,
) {
    let (columns, rows) = (obstacles.columns, obstacles.rows);
    let count = columns * rows;

    let omega = 1.0 / RELAXATION_TIME;
    let mut f = [0.0f32; DIRECTIONS];
    for y in 0..rows {
        for x in 0..columns {
            let cell = y * columns + x;
            if obstacles.solid[cell] {
                continue;
            }
            // Past the outflow edge the air is at rest pressure and moves on as
            // it left this cell, never back in, which keeps eddies leaving the grid from reflecting.
            let leaving = if x == columns - 1 {
                let (velocity_x, velocity_y) = velocity(current, count, cell);
                equilibrium(1.0, velocity_x.max(0.0), velocity_y)
            } else {
                *inflow
            };
            for direction in 0..DIRECTIONS {
                let from_x = x as i32 - CX[direction];
                let from_y = y as i32 - CY[direction];
                f[direction] = if from_x < 0 || from_y < 0 || from_y >= rows as i32 {
                    inflow[direction]
                } else if from_x >= columns as i32 {
                    leaving[direction]
                } else {
                    let from = from_y as usize * columns + from_x as usize;
                    if obstacles.solid[from] {
                        current[OPPOSITE[direction] * count + cell]
                    } else {
                        current[direction * count + from]
                    }
                };
            }

            let density: f32 = f.iter().sum();
            let velocity_x = (f[1] - f[3] + f[5] - f[6] - f[7] + f[8]) / density;
            let velocity_y = (f[2] - f[4] + f[5] + f[6] - f[7] - f[8]) / density;
            let settled = equilibrium(density, velocity_x, velocity_y);
            for direction in 0..DIRECTIONS {
                next[direction * count + cell] =
                    f[direction] + omega * (settled[direction] - f[direction]);
            }
            if let Some(sum) = sum.as_deref_mut() {
                sum[cell].0 += velocity_x;
                sum[cell].1 += velocity_y;
            }
        }
    }
}

/// The velocity of one cell from its populations.
fn velocity(populations: &[f32], count: usize, cell: usize) -> (f32, f32) {
    let f: [f32; DIRECTIONS] =
        std::array::from_fn(|direction| populations[direction * count + cell]);
    let density: f32 = f.iter().sum();
    (
        (f[1] - f[3] + f[5] - f[6] - f[7] + f[8]) / density,
        (f[2] - f[4] + f[5] + f[6] - f[7] - f[8]) / density,
    )
}

/// The equilibrium share of each direction at a density and velocity.
fn equilibrium(density: f32, velocity_x: f32, velocity_y: f32) -> [f32; DIRECTIONS] {
    let speed_squared = velocity_x * velocity_x + velocity_y * velocity_y;
    let mut settled = [0.0f32; DIRECTIONS];
    for direction in 0..DIRECTIONS {
        let along = CX[direction] as f32 * velocity_x + CY[direction] as f32 * velocity_y;
        settled[direction] = WEIGHTS[direction]
            * density
            * (1.0 + 3.0 * along + 4.5 * along * along - 1.5 * speed_squared);
    }
    settled
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open(columns: usize, rows: usize) -> Obstacles {
        Obstacles {
            columns,
            rows,
            solid: vec![false; columns * rows],
        }
    }

    #[test]
    fn open_ground_keeps_the_incoming_wind() {
        let flow = mean_flow(&open(40, 20));
        for &(along, across) in &flow {
            assert!((along - 1.0).abs() < 0.01, "{along}");
            assert!(across.abs() < 0.01, "{across}");
        }
    }

    #[test]
    fn a_block_shelters_the_ground_behind_it() {
        let mut obstacles = open(80, 40);
        for y in 15..25 {
            for x in 20..30 {
                obstacles.solid[y * 80 + x] = true;
            }
        }
        let flow = mean_flow(&obstacles);
        let speed = |x: usize, y: usize| {
            let (along, across) = flow[y * 80 + x];
            (along * along + across * across).sqrt()
        };
        assert!(flow.iter().all(|(a, b)| a.is_finite() && b.is_finite()));
        assert!(speed(33, 20) < 0.5, "behind {}", speed(33, 20));
        assert!(speed(25, 30) > 1.0, "beside {}", speed(25, 30));
    }
}
