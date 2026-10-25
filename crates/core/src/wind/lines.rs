//! Flow lines over the site's open ground, evenly spaced, with arrowheads
//! pointing downwind.
//!
//! The spacing follows Jobard & Lefer ("Creating evenly-spaced streamlines of
//! arbitrary density", Visualization in Scientific Computing 1997): a line
//! starts only at a seed at least one spacing from every line drawn so far,
//! and grows forwards and backwards until it comes within half a spacing of
//! another line. Seeds sit on a regular grid. Lines bunch where streets
//! squeeze the flow together and stop where the air is nearly still.

use std::collections::HashMap;

use super::field::Field;
use crate::sunlight::raster::Raster;
use crate::sunlight::Cell;

/// Lines this many flow cells apart.
const SPACING_CELLS: f64 = 4.0;
/// A line stops this close to another, as a share of the spacing (Jobard & Lefer's d_test).
const STOP_SHARE: f64 = 0.5;
/// Each step moves half a flow cell, so a line follows the blend between cells.
const STEP_CELLS: f64 = 0.5;
/// Below this share of the incoming wind the air is all but still and its direction is noise.
const STILL_SHARE: f64 = 0.1;
/// A line shorter than this many spacings says nothing and is dropped.
const SHORTEST_SPACINGS: f64 = 1.5;
/// An arrowhead every this many spacings along a line, the first half that from its start.
const ARROW_EVERY_SPACINGS: f64 = 3.0;
/// The arrowhead's length in metres, 2.4 mm at the default 1:2500; its width is half its length.
const ARROW_M: f64 = 6.0;
/// A line's width in metres, 0.3 mm at 1:2500, wide enough for a paper-coloured edge.
const RIBBON_M: f64 = 0.75;
const ARROW_WIDTH_SHARE: f64 = 0.5;
/// So a line that circles in an eddy ends.
const MOST_STEPS: usize = 4_000;

/// Points of a line, upwind first.
pub type Line = Vec<(f64, f64)>;

/// The flow lines over the ground cells of `site`, and an arrowhead
/// triangle for each place along them that gets one.
pub fn flow_lines(
    field: &Field,
    site: &Raster,
    cells: &[Cell],
) -> (Vec<Line>, Vec<[(f64, f64); 3]>) {
    let spacing = SPACING_CELLS * field.cell_m();
    let mut taken = Taken::new(spacing);
    let mut lines = Vec::new();
    let seed_every = ((spacing / site.cell_m).round() as usize).max(1);
    for row in (0..site.rows).step_by(seed_every) {
        for column in (0..site.columns).step_by(seed_every) {
            let index = row * site.columns + column;
            if cells[index] != Cell::Ground {
                continue;
            }
            let seed = site.centre(index);
            if taken.is_near(seed, spacing) || moving(field, seed).is_none() {
                continue;
            }
            let walk = Walk {
                field,
                site,
                cells,
                taken: &taken,
                spacing,
            };
            let mut line = walk.grown(seed, -1.0);
            line.reverse();
            line.pop();
            line.extend(walk.grown(seed, 1.0));
            if length(&line) >= SHORTEST_SPACINGS * spacing {
                taken.add(&line);
                lines.push(line);
            }
        }
    }

    let arrows = lines
        .iter()
        .flat_map(|line| arrowheads(line, spacing))
        .collect();
    (lines, arrows)
}

/// The unit direction of the flow at a point, or `None` where the air is still.
fn moving(field: &Field, point: (f64, f64)) -> Option<(f64, f64)> {
    let (x, y) = field.at(point)?;
    let speed = x.hypot(y);
    (speed >= STILL_SHARE).then(|| (x / speed, y / speed))
}

/// What a line growing from a seed needs to know.
struct Walk<'a> {
    field: &'a Field,
    site: &'a Raster,
    cells: &'a [Cell],
    taken: &'a Taken,
    spacing: f64,
}

impl Walk<'_> {
    /// The points from `seed` on, downwind for `sense` 1 and upwind for -1,
    /// in midpoint (second order Runge-Kutta) steps along the flow's direction.
    fn grown(&self, seed: (f64, f64), sense: f64) -> Line {
        let step = STEP_CELLS * self.field.cell_m() * sense;
        let stop_at = STOP_SHARE * self.spacing;
        // A line circling in an eddy meets its own tail; the last stretch is
        // left out of the test, or every point would be near the one before.
        let lag = (2.0 * self.spacing / step.abs()).ceil() as usize;
        let mut own = Taken::new(self.spacing);
        let mut line = vec![seed];
        let mut point = seed;
        for _ in 0..MOST_STEPS {
            let Some(here) = moving(self.field, point) else {
                break;
            };
            let middle = (point.0 + here.0 * step / 2.0, point.1 + here.1 * step / 2.0);
            let Some(there) = moving(self.field, middle) else {
                break;
            };
            point = (point.0 + there.0 * step, point.1 + there.1 * step);
            let is_crowded = self.taken.is_near(point, stop_at) || own.is_near(point, stop_at);
            if !self.is_ground(point) || is_crowded {
                break;
            }
            line.push(point);
            if line.len() > lag {
                own.add_point(line[line.len() - 1 - lag]);
            }
        }
        line
    }

    fn is_ground(&self, (x, y): (f64, f64)) -> bool {
        let site = self.site;
        let column = ((x - site.min.0) / site.cell_m).floor();
        let row = ((y - site.min.1) / site.cell_m).floor();
        if column < 0.0 || row < 0.0 || column >= site.columns as f64 || row >= site.rows as f64 {
            return false;
        }
        self.cells[row as usize * site.columns + column as usize] == Cell::Ground
    }
}

/// Points of the lines drawn so far, in buckets one spacing wide, so a
/// nearness test only looks at the buckets around a point.
struct Taken {
    bucket_m: f64,
    buckets: HashMap<(i64, i64), Vec<(f64, f64)>>,
}

impl Taken {
    fn new(bucket_m: f64) -> Taken {
        Taken {
            bucket_m,
            buckets: HashMap::new(),
        }
    }

    fn bucket(&self, (x, y): (f64, f64)) -> (i64, i64) {
        (
            (x / self.bucket_m).floor() as i64,
            (y / self.bucket_m).floor() as i64,
        )
    }

    fn add(&mut self, line: &Line) {
        for &point in line {
            self.add_point(point);
        }
    }

    fn add_point(&mut self, point: (f64, f64)) {
        let bucket = self.bucket(point);
        self.buckets.entry(bucket).or_default().push(point);
    }

    /// Whether a drawn line passes within `distance` (at most one bucket) of `point`.
    fn is_near(&self, point: (f64, f64), distance: f64) -> bool {
        let (column, row) = self.bucket(point);
        for neighbour_row in row - 1..=row + 1 {
            for neighbour_column in column - 1..=column + 1 {
                let Some(points) = self.buckets.get(&(neighbour_column, neighbour_row)) else {
                    continue;
                };
                if points
                    .iter()
                    .any(|other| (other.0 - point.0).hypot(other.1 - point.1) < distance)
                {
                    return true;
                }
            }
        }
        false
    }
}

/// The outline of a line widened to `RIBBON_M`: down its left side and back up its right.
pub fn ribbon(line: &Line) -> Vec<(f64, f64)> {
    let half = RIBBON_M / 2.0;
    let mut left = Vec::with_capacity(line.len());
    let mut right = Vec::with_capacity(line.len());
    for (index, &point) in line.iter().enumerate() {
        // The direction at a point is the chord through its neighbours.
        let before = line[index.saturating_sub(1)];
        let after = line[(index + 1).min(line.len() - 1)];
        let (dx, dy) = (after.0 - before.0, after.1 - before.1);
        let length = dx.hypot(dy);
        if length == 0.0 {
            continue;
        }
        let (side_x, side_y) = (-dy / length * half, dx / length * half);
        left.push((point.0 + side_x, point.1 + side_y));
        right.push((point.0 - side_x, point.1 - side_y));
    }
    right.reverse();
    left.extend(right);
    left
}

fn length(line: &Line) -> f64 {
    line.windows(2)
        .map(|pair| (pair[1].0 - pair[0].0).hypot(pair[1].1 - pair[0].1))
        .sum()
}

/// Triangles pointing downwind, spread along the line.
fn arrowheads(line: &Line, spacing: f64) -> Vec<[(f64, f64); 3]> {
    let every = ARROW_EVERY_SPACINGS * spacing;
    let arrow = ARROW_M;
    let mut arrows = Vec::new();
    let mut next_at = every / 2.0;
    let mut walked = 0.0;
    for pair in line.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        let step = (to.0 - from.0).hypot(to.1 - from.1);
        if step == 0.0 {
            continue;
        }
        let (along_x, along_y) = ((to.0 - from.0) / step, (to.1 - from.1) / step);
        while walked + step >= next_at {
            let share = (next_at - walked) / step;
            let tip = (
                from.0 + (to.0 - from.0) * share,
                from.1 + (to.1 - from.1) * share,
            );
            let base = (tip.0 - along_x * arrow, tip.1 - along_y * arrow);
            let half = arrow * ARROW_WIDTH_SHARE / 2.0;
            arrows.push([
                tip,
                (base.0 - along_y * half, base.1 + along_x * half),
                (base.0 + along_y * half, base.1 - along_x * half),
            ]);
            next_at += every;
        }
        walked += step;
    }
    arrows
}
