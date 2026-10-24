//! The bare ground kept with a scene, so a section can draw its profile.
//!
//! AHN's terrain model comes in 0.5 m cells, which over a neighbourhood would
//! make the project file many times larger than all the rest of the scene. A
//! section reads the ground as a line at 1:200 or smaller, where a few metres
//! between heights draw the same, and Dutch ground is gentle, so the kept grid
//! has cells of a few metres and heights to the decimetre. Each kept cell is
//! the median of the measured cells inside it, so one tall kerb or a gap under
//! a building does not move it.

use geo::{Coord, Rect};

use super::to_local_centimetres;
use crate::sources::raster::Grid;

/// The smallest kept cell. Twice the 2 m neighbourhood cells, and still
/// several heights across the narrowest Dutch dike crest.
const MIN_CELL_M: f64 = 4.0;
/// A larger area gets larger cells, so no project keeps more than this many
/// cells across: 300 by 300 cells is 90,000 heights, about 40 kB in the
/// packed project file, where a 200 m site's scene takes about 1.2 MB.
const MAX_CELLS_ACROSS: f64 = 300.0;
/// AHN is accurate to about 5 cm; a section at 1:200 draws 10 cm as half a millimetre.
const KEPT_PRECISION_M: f64 = 0.1;

/// `terrain` cut to `extent` (both RD New), in larger cells, moved into the
/// scene's frame. `None` when no cell inside `extent` has a value.
pub fn kept(terrain: &Grid, extent: Rect<f64>, origin: (f64, f64)) -> Option<Grid> {
    let cut = terrain.cropped(extent);
    let longest_m = cut.width.max(cut.height) as f64 * cut.cell_m;
    let wanted_cell_m = MIN_CELL_M.max(longest_m / MAX_CELLS_ACROSS);
    let factor = (wanted_cell_m / cut.cell_m).ceil().max(1.0) as usize;
    let coarse = coarsened(&cut, factor);
    if coarse.values.iter().all(|value| value.is_nan()) {
        return None;
    }

    let corner = to_local_centimetres(
        Coord {
            x: coarse.west,
            y: coarse.north,
        },
        origin,
    );
    Some(Grid {
        west: corner.x,
        north: corner.y,
        ..coarse
    })
}

/// Blocks of `factor` by `factor` cells as one cell each, holding the median
/// of their measured cells to the decimetre, or NaN when none was measured.
fn coarsened(grid: &Grid, factor: usize) -> Grid {
    let width = grid.width.div_ceil(factor);
    let height = grid.height.div_ceil(factor);
    let mut values = Vec::with_capacity(width * height);
    let mut block = Vec::with_capacity(factor * factor);
    for block_row in 0..height {
        for block_column in 0..width {
            block.clear();
            let rows = block_row * factor..((block_row + 1) * factor).min(grid.height);
            let columns = block_column * factor..((block_column + 1) * factor).min(grid.width);
            for row in rows {
                for column in columns.clone() {
                    let value = grid.value(column, row);
                    if value.is_finite() {
                        block.push(value);
                    }
                }
            }
            values.push(median_to_decimetre(&mut block));
        }
    }

    Grid {
        west: grid.west,
        north: grid.north,
        cell_m: grid.cell_m * factor as f64,
        width,
        height,
        values,
    }
}

fn median_to_decimetre(values: &mut [f32]) -> f32 {
    if values.is_empty() {
        return f32::NAN;
    }
    values.sort_by(f32::total_cmp);
    let median = f64::from(values[values.len() / 2]);

    ((median / KEPT_PRECISION_M).round() * KEPT_PRECISION_M) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2 m cells over 100 m, all at 1.23 m NAP but for a building of no data in one corner.
    fn neighbourhood() -> Grid {
        let mut values = vec![1.23; 50 * 50];
        for row in 0..4 {
            for column in 0..4 {
                values[row * 50 + column] = f32::NAN;
            }
        }
        Grid {
            west: 1000.0,
            north: 2100.0,
            cell_m: 2.0,
            width: 50,
            height: 50,
            values,
        }
    }

    #[test]
    fn a_small_area_is_kept_in_four_metre_cells_to_the_decimetre() {
        let extent = Rect::new((1000.0, 2000.0), (1100.0, 2100.0));
        let kept = kept(&neighbourhood(), extent, (1050.0, 2050.0)).unwrap();
        assert_eq!(kept.cell_m, 4.0);
        assert_eq!((kept.width, kept.height), (25, 25));
        assert_eq!((kept.west, kept.north), (-50.0, 50.0));
        assert_eq!(kept.value(10, 10), 1.2);
        // The corner block of four by four 2 m cells had no data at all.
        assert!(kept.value(0, 0).is_nan() && kept.value(1, 1).is_nan());
        assert_eq!(kept.value(2, 2), 1.2);
    }

    #[test]
    fn a_large_area_gets_larger_cells_so_the_file_stays_small() {
        let grid = Grid {
            width: 1000,
            height: 1000,
            values: vec![0.5; 1000 * 1000],
            ..neighbourhood()
        };
        let extent = Rect::new((1000.0, 100.0), (3000.0, 2100.0));
        let kept = kept(&grid, extent, (0.0, 0.0)).unwrap();
        assert_eq!(kept.cell_m, 8.0);
        assert!(kept.width as f64 <= MAX_CELLS_ACROSS);
    }

    #[test]
    fn terrain_without_a_single_value_is_not_kept() {
        let empty = Grid {
            values: vec![f32::NAN; 50 * 50],
            ..neighbourhood()
        };
        let extent = Rect::new((1000.0, 2000.0), (1100.0, 2100.0));
        assert!(kept(&empty, extent, (0.0, 0.0)).is_none());
    }
}
