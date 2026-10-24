//! Terrain contour lines from the AHN terrain model.
//!
//! The raw 0.5 m terrain is too noisy for contours and has holes under every
//! building. So the terrain is first averaged into 5 m cells, the holes are
//! filled from their edges, and the result is smoothed. Contours under
//! buildings are therefore interpolated, which is fine: buildings draw on top.

use contour::ContourBuilder;
use geo::Geometry;

use crate::scene::{Attributes, Feature, Layer};
use crate::sources::ahn::Grid;
use crate::sources::heights::percentile;

/// Size of the cells contours are traced on.
const COARSE_CELL_M: f64 = 5.0;
const SMOOTHING_PASSES: usize = 2;
/// Contour intervals we choose from, smallest first. NL is flat, so the small
/// ones are the usual answer.
const INTERVALS_M: [f64; 10] = [0.1, 0.25, 0.5, 1.0, 2.0, 2.5, 5.0, 10.0, 25.0, 50.0];
/// The smallest interval that gives at most this many lines wins, which leaves
/// a site with roughly 8 to 20 lines.
const MAX_LINES: usize = 20;

/// Contour lines over `terrain`, in RD New, with their level in `ground_m`.
pub fn contour_lines(terrain: &Grid) -> Vec<Feature> {
    let coarse = smoothed(&filled(&coarsened(terrain)));
    let finite = coarse
        .values
        .iter()
        .copied()
        .filter(|value| value.is_finite());
    let lowest = finite.clone().fold(f32::INFINITY, f32::min);
    let highest = finite.fold(f32::NEG_INFINITY, f32::max);
    if lowest > highest {
        return Vec::new();
    }

    let levels = levels_between(f64::from(lowest), f64::from(highest));
    let values: Vec<f64> = coarse
        .values
        .iter()
        .map(|&value| f64::from(value))
        .collect();
    // Grid index (column, row) maps to x = west + column * cell, y = north - row * cell.
    let builder = ContourBuilder::new(coarse.width, coarse.height, true)
        .x_origin(coarse.west)
        .y_origin(coarse.north)
        .x_step(coarse.cell_m)
        .y_step(-coarse.cell_m);
    let Ok(lines) = builder.lines(&values, &levels) else {
        return Vec::new();
    };

    lines
        .into_iter()
        .filter(|line| !line.geometry().0.is_empty())
        .map(|line| {
            let (geometry, level) = line.into_inner();
            Feature {
                layer: Layer::Contour,
                geometry: Geometry::MultiLineString(geometry),
                attributes: Attributes {
                    ground_m: Some(level),
                    ..Attributes::default()
                },
            }
        })
        .collect()
}

/// Round contour levels between `lowest` and `highest` at the smallest
/// interval that keeps them to `MAX_LINES`. 0.3 to 5.1 -> 0.5, 0.75, ... 5.0.
pub fn levels_between(lowest: f64, highest: f64) -> Vec<f64> {
    for interval in INTERVALS_M {
        let first = (lowest / interval).ceil() as i64;
        let last = (highest / interval).floor() as i64;
        if last - first < MAX_LINES as i64 {
            return (first..=last).map(|step| step as f64 * interval).collect();
        }
    }

    Vec::new()
}

/// `grid` in cells of `COARSE_CELL_M`, each the median of the finite cells it covers.
fn coarsened(grid: &Grid) -> Grid {
    let block = (COARSE_CELL_M / grid.cell_m).round().max(1.0) as usize;
    let width = grid.width.div_ceil(block);
    let height = grid.height.div_ceil(block);

    let mut values = Vec::with_capacity(width * height);
    for coarse_row in 0..height {
        for coarse_column in 0..width {
            let mut cells = Vec::with_capacity(block * block);
            for row in coarse_row * block..((coarse_row + 1) * block).min(grid.height) {
                for column in coarse_column * block..((coarse_column + 1) * block).min(grid.width) {
                    cells.push(grid.value(column, row));
                }
            }
            values.push(percentile(cells, 0.5).map_or(f32::NAN, |value| value as f32));
        }
    }

    Grid {
        cell_m: grid.cell_m * block as f64,
        width,
        height,
        values,
        ..*grid
    }
}

/// `grid` with every NaN cell filled from its neighbours, growing inwards from
/// the edges of each hole. A grid with no finite cell at all stays as it is.
fn filled(grid: &Grid) -> Grid {
    let mut values = grid.values.clone();
    while values.iter().any(|value| value.is_nan()) {
        let previous = values.clone();
        let mut changed = false;
        for row in 0..grid.height {
            for column in 0..grid.width {
                if previous[row * grid.width + column].is_nan() {
                    let mean =
                        neighbour_mean(&previous, grid.width, grid.height, column, row, false);
                    if mean.is_finite() {
                        values[row * grid.width + column] = mean;
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }

    Grid { values, ..*grid }
}

/// `grid` after `SMOOTHING_PASSES` of a 3 by 3 mean.
fn smoothed(grid: &Grid) -> Grid {
    let mut values = grid.values.clone();
    for _ in 0..SMOOTHING_PASSES {
        let previous = values.clone();
        for row in 0..grid.height {
            for column in 0..grid.width {
                values[row * grid.width + column] =
                    neighbour_mean(&previous, grid.width, grid.height, column, row, true);
            }
        }
    }

    Grid { values, ..*grid }
}

/// Mean of the finite cells around (column, row), the cell itself only when
/// `include_centre`. NaN when there are none.
fn neighbour_mean(
    values: &[f32],
    width: usize,
    height: usize,
    column: usize,
    row: usize,
    include_centre: bool,
) -> f32 {
    let mut sum = 0.0;
    let mut count = 0;
    for neighbour_row in row.saturating_sub(1)..=(row + 1).min(height - 1) {
        for neighbour_column in column.saturating_sub(1)..=(column + 1).min(width - 1) {
            let is_centre = neighbour_row == row && neighbour_column == column;
            let value = values[neighbour_row * width + neighbour_column];
            if value.is_finite() && (include_centre || !is_centre) {
                sum += value;
                count += 1;
            }
        }
    }

    if count == 0 {
        f32::NAN
    } else {
        sum / count as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_five_metre_rise_gets_quarter_metre_contours() {
        let levels = levels_between(0.3, 5.1);
        assert_eq!(levels.first(), Some(&0.5));
        assert_eq!(levels.last(), Some(&5.0));
        assert_eq!(levels.len(), 19);
    }

    #[test]
    fn a_hilly_site_gets_a_coarser_interval_and_at_most_twenty_lines() {
        let levels = levels_between(10.0, 95.0);
        assert!(levels.len() <= MAX_LINES);
        assert!(levels.len() >= 8);
        assert_eq!(levels[1] - levels[0], 5.0);
    }

    #[test]
    fn a_hole_is_filled_from_its_edges() {
        let mut values = vec![1.0; 25];
        values[12] = f32::NAN;
        let grid = Grid {
            west: 0.0,
            north: 5.0,
            cell_m: 1.0,
            width: 5,
            height: 5,
            values,
        };
        assert_eq!(filled(&grid).values[12], 1.0);
    }

    #[test]
    fn a_slope_gives_straight_contours_at_the_right_place() {
        // Terrain rising 1 m per 10 m eastwards, 200 by 100 m, at 0.5 m cells.
        let (width, height) = (400, 200);
        let values = (0..width * height)
            .map(|index| ((index % width) as f32 * 0.5 + 0.25) / 10.0)
            .collect();
        let terrain = Grid {
            west: 0.0,
            north: 100.0,
            cell_m: 0.5,
            width,
            height,
            values,
        };

        let lines = contour_lines(&terrain);
        assert!(
            lines.len() >= 8 && lines.len() <= MAX_LINES,
            "{} lines",
            lines.len()
        );

        // The 10 m contour must run near x = 100, away from the grid's edges.
        let ten = lines
            .iter()
            .find(|line| line.attributes.ground_m == Some(10.0))
            .unwrap();
        let Geometry::MultiLineString(geometry) = &ten.geometry else {
            panic!()
        };
        let middle_points: Vec<_> = geometry
            .0
            .iter()
            .flat_map(|line| line.0.iter())
            .filter(|point| point.y > 20.0 && point.y < 80.0 && point.x > 5.0 && point.x < 195.0)
            .collect();
        assert!(!middle_points.is_empty());
        assert!(
            middle_points
                .iter()
                .all(|point| (point.x - 100.0).abs() < 3.0),
            "{middle_points:?}"
        );
    }
}
