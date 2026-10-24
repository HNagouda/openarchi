//! AHN, the national elevation model: surface (with buildings and trees) and
//! bare terrain, both at 0.5 m in RD New, from the PDOK WCS.
//!
//! The terrain model has no values under buildings and water, so every user of
//! a `Grid` must expect NaN cells.

use geo::{Coord, Rect};

use crate::scene::Credit;
pub use crate::sources::raster::Grid;
use crate::sources::wcs::{self, Coverage};
use crate::{Error, Result};

const SERVICE: &str = "PDOK AHN";
const WCS_URL: &str = "https://service.pdok.nl/rws/ahn/wcs/v1_0";
/// Digital surface model: the top of whatever is there, roofs included.
pub const SURFACE_COVERAGE: &str = "dsm_05m";
/// Digital terrain model: bare ground, with gaps under buildings.
pub const TERRAIN_COVERAGE: &str = "dtm_05m";
pub const CELL_M: f64 = 0.5;
/// PDOK names the axes in lower case.
const AXES: (&str, &str) = ("x", "y");
/// Fetch a little beyond the site, so buildings on the edge still have ground
/// around them and contour lines do not stop at the boundary.
const MARGIN_M: f64 = 20.0;
/// The neighbourhood around a site needs only a height per building, so its
/// elevation comes in 2 m cells: a sixteenth of the cells, and still several
/// on the roof of a terraced house.
const COARSE_SCALE_FACTOR: &str = "0.25";

/// Surface and terrain for the same cells.
#[derive(Debug, Clone)]
pub struct Elevation {
    pub surface: Grid,
    pub terrain: Grid,
}

pub fn credit() -> Credit {
    Credit {
        name: "Actueel Hoogtebestand Nederland (AHN), via PDOK".to_string(),
        license: "CC0 1.0".to_string(),
        url: "https://www.ahn.nl".to_string(),
        layers: Vec::new(),
    }
}

/// The area elevation is fetched for: `bbox` (RD New) plus a small margin,
/// widened to whole cells.
pub fn extent(bbox: Rect<f64>) -> Rect<f64> {
    let (west, north, width, height) = snapped(bbox);
    Rect::new(
        Coord {
            x: west,
            y: north - height as f64 * CELL_M,
        },
        Coord {
            x: west + width as f64 * CELL_M,
            y: north,
        },
    )
}

/// One downloaded raster tile: where it lies, and its cells row by row from the north.
pub type CellTile = (Rect<f64>, Vec<f32>);

/// One coverage over `extent(bbox)`, pieced together from tiles of that
/// coverage. Cells no tile covers stay NaN.
pub fn assemble(bbox: Rect<f64>, tiles: &[CellTile]) -> Grid {
    let (west, north, width, height) = snapped(bbox);
    let mut grid = Grid {
        west,
        north,
        cell_m: CELL_M,
        width,
        height,
        values: vec![f32::NAN; width * height],
    };
    for (rect, cells) in tiles {
        copy_tile(&mut grid, *rect, cells);
    }

    grid
}

/// Copies the part of a tile that falls inside `grid`. Tiles and grid share
/// the AHN cell raster, so offsets are whole cells.
fn copy_tile(grid: &mut Grid, rect: Rect<f64>, cells: &[f32]) {
    let tile_width = cells_across(rect.width());
    let column_offset = ((rect.min().x - grid.west) / CELL_M).round() as i64;
    let row_offset = ((grid.north - rect.max().y) / CELL_M).round() as i64;
    let first_column = column_offset.max(0);
    let end_column = (column_offset + tile_width as i64).min(grid.width as i64);
    if first_column >= end_column {
        return;
    }

    for (tile_row, source) in cells.chunks_exact(tile_width).enumerate() {
        let row = row_offset + tile_row as i64;
        if row < 0 || row >= grid.height as i64 {
            continue;
        }
        let start = (first_column - column_offset) as usize;
        let end = (end_column - column_offset) as usize;
        let target = row as usize * grid.width;
        grid.values[target + first_column as usize..target + end_column as usize]
            .copy_from_slice(&source[start..end]);
    }
}

fn cells_across(metres: f64) -> usize {
    (metres / CELL_M).round() as usize
}

/// West edge, north edge, width and height in cells of `bbox` with the margin,
/// widened to whole cells so tiles line up with the AHN grid.
fn snapped(bbox: Rect<f64>) -> (f64, f64, usize, usize) {
    let west = ((bbox.min().x - MARGIN_M) / CELL_M).floor() * CELL_M;
    let south = ((bbox.min().y - MARGIN_M) / CELL_M).floor() * CELL_M;
    let east = ((bbox.max().x + MARGIN_M) / CELL_M).ceil() * CELL_M;
    let north = ((bbox.max().y + MARGIN_M) / CELL_M).ceil() * CELL_M;
    let width = ((east - west) / CELL_M).round() as usize;
    let height = ((north - south) / CELL_M).round() as usize;
    (west, north, width, height)
}

/// The cells of `coverage` inside `rect` (RD New, on whole cells), row by row
/// from the north. Fails when the answer does not cover exactly `rect`, since
/// then the cells would not line up with the grid.
pub fn fetch_tile(coverage: &'static str, rect: Rect<f64>) -> Result<Vec<f32>> {
    let coverage = Coverage {
        service: SERVICE,
        url: WCS_URL,
        id: coverage,
        axes: AXES,
        scale_factor: None,
    };
    let grid = wcs::fetch(&coverage, rect)?;
    let expected = (cells_across(rect.width()), cells_across(rect.height()));
    let is_on_rect = (grid.west - rect.min().x).abs() < CELL_M / 2.0
        && (grid.north - rect.max().y).abs() < CELL_M / 2.0;
    if (grid.width, grid.height) != expected || !is_on_rect {
        return Err(Error::Data {
            service: SERVICE.to_string(),
            detail: format!(
                "asked for {}x{} cells at ({}, {}), got {}x{} at ({}, {})",
                expected.0,
                expected.1,
                rect.min().x,
                rect.max().y,
                grid.width,
                grid.height,
                grid.west,
                grid.north
            ),
        });
    }

    Ok(grid.values)
}

/// `coverage` inside `rect` (RD New) in 2 m cells, placed where the GeoTIFF
/// says. For building heights around the site, where 0.5 m would be wasted.
pub fn fetch_coarse_tile(coverage: &'static str, rect: Rect<f64>) -> Result<Grid> {
    let coverage = Coverage {
        service: SERVICE,
        url: WCS_URL,
        id: coverage,
        axes: AXES,
        scale_factor: Some(COARSE_SCALE_FACTOR),
    };
    wcs::fetch(&coverage, rect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_tiles_side_by_side_fill_one_grid() {
        // Two 2 by 2 cell tiles, the left all 1.0 and the right all 2.0.
        let left = Rect::new(Coord { x: 0.0, y: 0.0 }, Coord { x: 1.0, y: 1.0 });
        let right = Rect::new(Coord { x: 1.0, y: 0.0 }, Coord { x: 2.0, y: 1.0 });
        let mut grid = Grid {
            west: 0.0,
            north: 1.0,
            cell_m: CELL_M,
            width: 4,
            height: 2,
            values: vec![f32::NAN; 8],
        };
        copy_tile(&mut grid, left, &[1.0; 4]);
        copy_tile(&mut grid, right, &[2.0; 4]);
        assert_eq!(grid.values, vec![1.0, 1.0, 2.0, 2.0, 1.0, 1.0, 2.0, 2.0]);
    }

    #[test]
    fn a_tile_reaching_past_the_grid_is_cut_to_it() {
        let tile = Rect::new(Coord { x: -0.5, y: 0.0 }, Coord { x: 1.0, y: 1.5 });
        let mut grid = Grid {
            west: 0.0,
            north: 1.0,
            cell_m: CELL_M,
            width: 2,
            height: 2,
            values: vec![f32::NAN; 4],
        };
        // Three by three cells, numbered row by row from the north-west.
        let cells: Vec<f32> = (0..9).map(|index| index as f32).collect();
        copy_tile(&mut grid, tile, &cells);
        assert_eq!(grid.values, vec![4.0, 5.0, 7.0, 8.0]);
    }

    #[test]
    fn the_bbox_is_widened_to_whole_cells_with_a_margin() {
        let bbox = Rect::new(Coord { x: 100.2, y: 200.7 }, Coord { x: 110.0, y: 210.0 });
        let (west, north, width, height) = snapped(bbox);
        assert_eq!(west, 80.0);
        assert_eq!(north, 230.0);
        assert_eq!(width, 100);
        assert_eq!(height, 99);
    }

    #[test]
    fn cells_within_a_rect_cover_it_and_stay_on_the_grid() {
        let grid = Grid {
            west: 0.0,
            north: 10.0,
            cell_m: 0.5,
            width: 20,
            height: 20,
            values: vec![0.0; 400],
        };
        let (columns, rows) = grid.cells_within(Rect::new(
            Coord { x: 1.2, y: 8.1 },
            Coord { x: 2.0, y: 9.0 },
        ));
        assert_eq!(columns, 2..5);
        assert_eq!(rows, 2..4);

        let (columns, _) = grid.cells_within(Rect::new(
            Coord { x: -5.0, y: 0.0 },
            Coord { x: 50.0, y: 1.0 },
        ));
        assert_eq!(columns, 0..20);
    }
}
