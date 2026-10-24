//! AHN, the national elevation model: surface (with buildings and trees) and
//! bare terrain, both at 0.5 m in RD New, from the PDOK WCS.
//!
//! The terrain model has no values under buildings and water, so every user of
//! a `Grid` must expect NaN cells.

use std::io::Cursor;
use std::ops::Range;
use std::thread;

use geo::{Coord, Rect};
use tiff::decoder::{Decoder, DecodingResult};

use crate::scene::Credit;
use crate::{Error, Result};

const SERVICE: &str = "PDOK AHN";
const WCS_URL: &str = "https://service.pdok.nl/rws/ahn/wcs/v1_0";
/// Digital surface model: the top of whatever is there, roofs included.
const SURFACE_COVERAGE: &str = "dsm_05m";
/// Digital terrain model: bare ground, with gaps under buildings.
const TERRAIN_COVERAGE: &str = "dtm_05m";
pub const CELL_M: f64 = 0.5;
/// The WCS refuses anything over 4000 cells a side, which is 2000 m.
const MAX_TILE_CELLS: usize = 4000;
/// Fetch a little beyond the site, so buildings on the edge still have ground
/// around them and contour lines do not stop at the boundary.
const MARGIN_M: f64 = 20.0;
/// AHN marks missing cells with the largest f32; anything this high is not terrain.
const NODATA_ABOVE: f32 = 1.0e30;

/// A raster in RD New. Row 0 is the northern edge; NaN means no data.
#[derive(Debug, Clone)]
pub struct Grid {
    pub west: f64,
    pub north: f64,
    pub cell_m: f64,
    pub width: usize,
    pub height: usize,
    pub values: Vec<f32>,
}

impl Grid {
    pub fn value(&self, column: usize, row: usize) -> f32 {
        self.values[row * self.width + column]
    }

    pub fn cell_centre(&self, column: usize, row: usize) -> Coord<f64> {
        Coord {
            x: self.west + (column as f64 + 0.5) * self.cell_m,
            y: self.north - (row as f64 + 0.5) * self.cell_m,
        }
    }

    /// The columns and rows of the cells overlapping `rect`, cut to the grid.
    pub fn cells_within(&self, rect: Rect<f64>) -> (Range<usize>, Range<usize>) {
        let to_index = |offset: f64, limit: usize| {
            (offset / self.cell_m).floor().clamp(0.0, limit as f64) as usize
        };
        let columns = to_index(rect.min().x - self.west, self.width)
            ..to_index(rect.max().x - self.west + self.cell_m, self.width);
        let rows = to_index(self.north - rect.max().y, self.height)
            ..to_index(self.north - rect.min().y + self.cell_m, self.height);
        (columns, rows)
    }
}

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
    }
}

/// Surface and terrain covering `bbox` (RD New) plus a small margin.
pub fn fetch(bbox: Rect<f64>) -> Result<Elevation> {
    let (west, north, width, height) = snapped(bbox);
    let (surface, terrain) = thread::scope(|scope| {
        let surface = scope.spawn(|| fetch_grid(SURFACE_COVERAGE, west, north, width, height));
        let terrain = scope.spawn(|| fetch_grid(TERRAIN_COVERAGE, west, north, width, height));
        (
            surface.join().expect("AHN surface thread panicked"),
            terrain.join().expect("AHN terrain thread panicked"),
        )
    });

    Ok(Elevation {
        surface: surface?,
        terrain: terrain?,
    })
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

/// The columns and rows of one tile, and its cells.
type Tile = (Range<usize>, Range<usize>, Result<Vec<f32>>);

/// One coverage as a single grid, fetched in tiles the WCS accepts.
fn fetch_grid(coverage: &str, west: f64, north: f64, width: usize, height: usize) -> Result<Grid> {
    let column_tiles = tile_ranges(width);
    let row_tiles = tile_ranges(height);

    let tiles: Vec<Tile> = thread::scope(|scope| {
        let mut handles = Vec::new();
        for rows in &row_tiles {
            for columns in &column_tiles {
                let (rows, columns) = (rows.clone(), columns.clone());
                handles.push(scope.spawn(move || {
                    let values = fetch_tile(coverage, west, north, &columns, &rows);
                    (columns, rows, values)
                }));
            }
        }
        handles
            .into_iter()
            .map(|handle| handle.join().expect("AHN tile thread panicked"))
            .collect()
    });

    let mut values = vec![f32::NAN; width * height];
    for (columns, rows, tile) in tiles {
        let tile = tile?;
        let tile_width = columns.len();
        for (tile_row, row) in rows.enumerate() {
            let source = &tile[tile_row * tile_width..(tile_row + 1) * tile_width];
            values[row * width + columns.start..row * width + columns.end].copy_from_slice(source);
        }
    }

    Ok(Grid {
        west,
        north,
        cell_m: CELL_M,
        width,
        height,
        values,
    })
}

/// `cells` split into equal runs no longer than the WCS allows: 5000 -> 0..2500, 2500..5000.
fn tile_ranges(cells: usize) -> Vec<Range<usize>> {
    let count = cells.div_ceil(MAX_TILE_CELLS).max(1);
    let size = cells.div_ceil(count);
    (0..count)
        .map(|index| index * size..((index + 1) * size).min(cells))
        .collect()
}

fn fetch_tile(
    coverage: &str,
    west: f64,
    north: f64,
    columns: &Range<usize>,
    rows: &Range<usize>,
) -> Result<Vec<f32>> {
    let x_min = west + columns.start as f64 * CELL_M;
    let x_max = west + columns.end as f64 * CELL_M;
    let y_max = north - rows.start as f64 * CELL_M;
    let y_min = north - rows.end as f64 * CELL_M;
    let subset_x = format!("x({x_min},{x_max})");
    let subset_y = format!("y({y_min},{y_max})");
    let query = [
        ("service", "WCS"),
        ("version", "2.0.1"),
        ("request", "GetCoverage"),
        ("coverageId", coverage),
        ("format", "image/tiff"),
        ("subset", subset_x.as_str()),
        ("subset", subset_y.as_str()),
    ];

    let body = crate::sources::http::get(SERVICE, WCS_URL, &query)?;
    decode(&body, columns.len(), rows.len())
}

/// The cells of a float GeoTIFF, row by row from the north, NaN for no data.
/// Fails when the size is not what we asked for, since then the cells would not
/// line up with the grid.
pub fn decode(body: &[u8], width: usize, height: usize) -> Result<Vec<f32>> {
    let data_error = |detail: String| Error::Data {
        service: SERVICE.to_string(),
        detail,
    };

    let mut decoder = Decoder::new(Cursor::new(body)).map_err(|error| {
        data_error(format!(
            "not a GeoTIFF ({error}): {}",
            String::from_utf8_lossy(&body[..body.len().min(300)])
        ))
    })?;
    let (tiff_width, tiff_height) = decoder
        .dimensions()
        .map_err(|error| data_error(error.to_string()))?;
    if (tiff_width as usize, tiff_height as usize) != (width, height) {
        return Err(data_error(format!(
            "asked for {width}x{height} cells, got {tiff_width}x{tiff_height}"
        )));
    }

    let DecodingResult::F32(values) = decoder
        .read_image()
        .map_err(|error| data_error(error.to_string()))?
    else {
        return Err(data_error(
            "elevation is not stored as 32-bit floats".to_string(),
        ));
    };

    Ok(values
        .into_iter()
        .map(|value| {
            if value.is_finite() && value < NODATA_ABOVE {
                value
            } else {
                f32::NAN
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_large_side_is_split_into_tiles_the_service_accepts() {
        assert_eq!(tile_ranges(1200), vec![0..1200]);
        assert_eq!(tile_ranges(5000), vec![0..2500, 2500..5000]);
        assert!(tile_ranges(9001)
            .iter()
            .all(|range| range.len() <= MAX_TILE_CELLS));
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
