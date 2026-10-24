//! The ground along a section line, sampled from the terrain kept with the scene.
//!
//! The terrain model has no values under buildings and water, and a section
//! nearly always crosses both. Such gaps are bridged in a straight line from
//! the ground on either side, which is what the ground under a building is
//! close to, and what a quay-to-quay canal bed is not; see the note below.

use super::Axis;
use crate::sources::raster::Grid;

/// A section samples the ground at most this many times, however long it is.
const MAX_SAMPLES: usize = 2000;

/// Ground heights along the line, in metres above NAP when measured.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    /// (metres from A, ground height), from A to A', at least two of them.
    pub samples: Vec<(f64, f64)>,
    /// False for a scene kept without terrain: the ground is drawn flat at 0.
    pub is_measured: bool,
}

impl Profile {
    /// The ground height `along` metres from A, between the two nearest samples.
    pub fn height_at(&self, along: f64) -> f64 {
        let after = self
            .samples
            .iter()
            .position(|&(at, _)| at >= along)
            .unwrap_or(self.samples.len() - 1);
        if after == 0 {
            return self.samples[0].1;
        }
        let (a_at, a_height) = self.samples[after - 1];
        let (b_at, b_height) = self.samples[after];
        if b_at <= a_at || along >= b_at {
            return b_height;
        }

        a_height + (b_height - a_height) * (along - a_at) / (b_at - a_at)
    }

    pub fn lowest(&self) -> f64 {
        self.samples
            .iter()
            .map(|&(_, height)| height)
            .fold(f64::INFINITY, f64::min)
    }

    pub fn highest(&self) -> f64 {
        self.samples
            .iter()
            .map(|&(_, height)| height)
            .fold(f64::NEG_INFINITY, f64::max)
    }
}

/// The ground along `axis` from `terrain`, or flat at 0 when there is none or
/// it has no value anywhere along the line.
// #! NOTE[sections, water] : Canals and ponds are bridged at bank height; draw the water level and bed instead?
pub fn ground_profile(terrain: Option<&Grid>, axis: &Axis) -> Profile {
    let Some(grid) = terrain else {
        return flat(axis.length);
    };
    let step = (grid.cell_m / 2.0).max(axis.length / MAX_SAMPLES as f64);
    let count = (axis.length / step).ceil() as usize + 1;
    let mut samples: Vec<(f64, Option<f64>)> = (0..count)
        .map(|index| {
            let along = (index as f64 * step).min(axis.length);
            let (x, y) = axis.point_at(along, 0.0);
            (along, height_at(grid, x, y))
        })
        .collect();
    if samples.iter().all(|(_, height)| height.is_none()) {
        return flat(axis.length);
    }
    bridge_gaps(&mut samples);

    Profile {
        samples: samples
            .into_iter()
            .map(|(along, height)| (along, height.unwrap_or_default()))
            .collect(),
        is_measured: true,
    }
}

/// The terrain at (`x`, `y`) in the grid's frame, bilinear between the four
/// nearest cell centres. Missing neighbours are left out of the blend, and
/// `None` means none of the four has a value.
pub fn height_at(grid: &Grid, x: f64, y: f64) -> Option<f64> {
    // Position in cell units, measured between cell centres.
    let column = (x - grid.west) / grid.cell_m - 0.5;
    let row = (grid.north - y) / grid.cell_m - 0.5;
    let (left, top) = (column.floor(), row.floor());
    let (right_share, bottom_share) = (column - left, row - top);

    let mut sum = 0.0;
    let mut weight = 0.0;
    for (column_step, row_step) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
        let (at_column, at_row) = (left + column_step, top + row_step);
        let is_inside = at_column >= 0.0
            && at_row >= 0.0
            && (at_column as usize) < grid.width
            && (at_row as usize) < grid.height;
        if !is_inside {
            continue;
        }
        let value = f64::from(grid.value(at_column as usize, at_row as usize));
        if !value.is_finite() {
            continue;
        }
        let across = if column_step == 0.0 {
            1.0 - right_share
        } else {
            right_share
        };
        let down = if row_step == 0.0 {
            1.0 - bottom_share
        } else {
            bottom_share
        };
        sum += value * across * down;
        weight += across * down;
    }

    (weight > 0.0).then(|| sum / weight)
}

/// Fills each run of missing heights on a straight line between the heights
/// on either side; a run at either end takes the nearest height.
fn bridge_gaps(samples: &mut [(f64, Option<f64>)]) {
    let known: Vec<usize> = (0..samples.len())
        .filter(|&index| samples[index].1.is_some())
        .collect();
    let (Some(&first), Some(&last)) = (known.first(), known.last()) else {
        return;
    };
    for index in 0..samples.len() {
        if samples[index].1.is_some() {
            continue;
        }
        let height = if index < first {
            samples[first].1
        } else if index > last {
            samples[last].1
        } else {
            let before = known
                .iter()
                .rev()
                .find(|&&at| at < index)
                .copied()
                .unwrap_or(first);
            let after = known
                .iter()
                .find(|&&at| at > index)
                .copied()
                .unwrap_or(last);
            let (a_at, a_height) = (samples[before].0, samples[before].1.unwrap_or_default());
            let (b_at, b_height) = (samples[after].0, samples[after].1.unwrap_or_default());
            let share = (samples[index].0 - a_at) / (b_at - a_at);
            Some(a_height + (b_height - a_height) * share)
        };
        samples[index].1 = height;
    }
}

fn flat(length: f64) -> Profile {
    Profile {
        samples: vec![(0.0, 0.0), (length, 0.0)],
        is_measured: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::section::SectionLine;

    /// 10 m cells, 10 by 3, rising 1 m per cell from west to east: 0.5 m at the first centre.
    fn slope() -> Grid {
        let mut values = Vec::new();
        for _ in 0..3 {
            for column in 0..10 {
                values.push(column as f32 + 0.5);
            }
        }
        Grid {
            west: 0.0,
            north: 30.0,
            cell_m: 10.0,
            width: 10,
            height: 3,
            values,
        }
    }

    fn axis(from: (f64, f64), to: (f64, f64)) -> Axis {
        let line = SectionLine {
            start: from,
            end: to,
            depth_m: 50.0,
        };
        line.axis((0.0, 0.0))
    }

    #[test]
    fn heights_between_cell_centres_are_blended() {
        let grid = slope();
        assert_eq!(height_at(&grid, 5.0, 15.0), Some(0.5));
        assert_eq!(height_at(&grid, 10.0, 15.0), Some(1.0));
        assert_eq!(height_at(&grid, 500.0, 15.0), None);
    }

    #[test]
    fn a_line_up_the_slope_climbs_with_it() {
        let profile = ground_profile(Some(&slope()), &axis((5.0, 15.0), (95.0, 15.0)));
        assert!(profile.is_measured);
        assert!((profile.height_at(0.0) - 0.5).abs() < 1e-9);
        assert!((profile.height_at(45.0) - 5.0).abs() < 1e-9);
        assert!((profile.highest() - 9.5).abs() < 1e-9);
    }

    #[test]
    fn a_gap_under_a_building_is_bridged_in_a_straight_line() {
        let mut grid = slope();
        for row in 0..3 {
            for column in 3..7 {
                grid.values[row * 10 + column] = f32::NAN;
            }
        }
        let profile = ground_profile(Some(&grid), &axis((5.0, 15.0), (95.0, 15.0)));
        // The cells either side of the gap hold 2.5 and 7.5, so the middle reads 5.
        assert!((profile.height_at(45.0) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn no_terrain_gives_flat_ground_that_says_so() {
        let profile = ground_profile(None, &axis((0.0, 0.0), (80.0, 0.0)));
        assert!(!profile.is_measured);
        assert_eq!(profile.height_at(40.0), 0.0);
        assert_eq!(profile.samples.last(), Some(&(80.0, 0.0)));
    }
}
