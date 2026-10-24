//! A raster in RD New, and the few things done with one before it is drawn:
//! joining tiles, keeping a tile in the cache, keeping one in a scene, and
//! reading values inside the site.

use std::ops::Range;

use geo::{Contains, Coord, Point, Polygon, Rect};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// West edge, north edge and cell size as f64, then width and height as u32.
const HEADER_BYTES: usize = 3 * 8 + 2 * 4;
const CELL_BYTES: usize = std::mem::size_of::<f32>();

/// A raster in RD New. Row 0 is the northern edge; NaN means no data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(into = "StoredGrid", try_from = "StoredGrid")]
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

    pub fn south(&self) -> f64 {
        self.north - self.height as f64 * self.cell_m
    }

    pub fn east(&self) -> f64 {
        self.west + self.width as f64 * self.cell_m
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

    /// The cells overlapping `rect`, as a grid of their own on the same lattice.
    pub fn cropped(&self, rect: Rect<f64>) -> Grid {
        let (columns, rows) = self.cells_within(rect);
        let mut values = Vec::with_capacity(columns.len() * rows.len());
        for row in rows.clone() {
            for column in columns.clone() {
                values.push(self.value(column, row));
            }
        }

        Grid {
            west: self.west + columns.start as f64 * self.cell_m,
            north: self.north - rows.start as f64 * self.cell_m,
            cell_m: self.cell_m,
            width: columns.len(),
            height: rows.len(),
            values,
        }
    }
}

/// A grid as JSON. JSON has no NaN, and serde_json writes one as `null` but
/// refuses to read it back, so a cell without data is stored as `null` here.
#[derive(Serialize, Deserialize)]
struct StoredGrid {
    west: f64,
    north: f64,
    cell_m: f64,
    width: usize,
    height: usize,
    values: Vec<Option<f32>>,
}

impl From<Grid> for StoredGrid {
    fn from(grid: Grid) -> Self {
        StoredGrid {
            west: grid.west,
            north: grid.north,
            cell_m: grid.cell_m,
            width: grid.width,
            height: grid.height,
            values: grid
                .values
                .into_iter()
                .map(|value| value.is_finite().then_some(value))
                .collect(),
        }
    }
}

impl TryFrom<StoredGrid> for Grid {
    type Error = String;

    // A damaged file fails here rather than reading past the end of the cells later.
    fn try_from(stored: StoredGrid) -> std::result::Result<Self, Self::Error> {
        if stored.values.len() != stored.width * stored.height {
            return Err(format!(
                "a stored raster of {} by {} cells has {} values",
                stored.width,
                stored.height,
                stored.values.len()
            ));
        }

        Ok(Grid {
            west: stored.west,
            north: stored.north,
            cell_m: stored.cell_m,
            width: stored.width,
            height: stored.height,
            values: stored
                .values
                .into_iter()
                .map(|value| value.unwrap_or(f32::NAN))
                .collect(),
        })
    }
}

/// Lowest, mean and highest value of the cells whose centre lies in an area.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    pub min: f64,
    pub mean: f64,
    pub max: f64,
    /// 0 to 1: the share of the cells in the area that had a value.
    pub coverage: f64,
}

/// Stats of the cells whose centre lies inside `area`, `None` when no cell has a value.
pub fn stats_within(grid: &Grid, area: &Polygon<f64>) -> Option<Stats> {
    let mut sum = 0.0;
    let mut count = 0usize;
    let mut inside = 0usize;
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for row in 0..grid.height {
        for column in 0..grid.width {
            if !area.contains(&Point::from(grid.cell_centre(column, row))) {
                continue;
            }
            inside += 1;
            let value = f64::from(grid.value(column, row));
            if value.is_finite() {
                sum += value;
                count += 1;
                min = min.min(value);
                max = max.max(value);
            }
        }
    }

    (count > 0).then(|| Stats {
        min,
        mean: sum / count as f64,
        max,
        coverage: count as f64 / inside as f64,
    })
}

/// Tiles of one coverage joined into one grid. The tiles share the
/// coverage's own lattice, so every offset is a whole number of cells.
pub fn join(tiles: &[Grid]) -> Option<Grid> {
    let first = tiles.first()?;
    let cell_m = first.cell_m;
    let west = tiles
        .iter()
        .map(|tile| tile.west)
        .fold(f64::INFINITY, f64::min);
    let north = tiles
        .iter()
        .map(|tile| tile.north)
        .fold(f64::NEG_INFINITY, f64::max);
    let east = tiles
        .iter()
        .map(Grid::east)
        .fold(f64::NEG_INFINITY, f64::max);
    let south = tiles.iter().map(Grid::south).fold(f64::INFINITY, f64::min);
    let width = ((east - west) / cell_m).round() as usize;
    let height = ((north - south) / cell_m).round() as usize;

    let mut values = vec![f32::NAN; width * height];
    for tile in tiles {
        let column_offset = ((tile.west - west) / cell_m).round() as usize;
        let row_offset = ((north - tile.north) / cell_m).round() as usize;
        for row in 0..tile.height {
            for column in 0..tile.width {
                let value = tile.value(column, row);
                let (target_column, target_row) = (column + column_offset, row + row_offset);
                if value.is_finite() && target_column < width && target_row < height {
                    values[target_row * width + target_column] = value;
                }
            }
        }
    }

    Some(Grid {
        west,
        north,
        cell_m,
        width,
        height,
        values,
    })
}

/// A grid as bytes for the tile cache: a small header, then the cells.
pub fn to_bytes(grid: &Grid) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(HEADER_BYTES + grid.values.len() * CELL_BYTES);
    bytes.extend(grid.west.to_le_bytes());
    bytes.extend(grid.north.to_le_bytes());
    bytes.extend(grid.cell_m.to_le_bytes());
    bytes.extend((grid.width as u32).to_le_bytes());
    bytes.extend((grid.height as u32).to_le_bytes());
    for value in &grid.values {
        bytes.extend(value.to_le_bytes());
    }

    bytes
}

/// The grid `to_bytes` wrote, or an error when the bytes are cut short.
pub fn from_bytes(bytes: &[u8]) -> Result<Grid> {
    let corrupt = || {
        Error::Invalid(format!(
            "a cached raster of {} bytes is damaged",
            bytes.len()
        ))
    };
    if bytes.len() < HEADER_BYTES {
        return Err(corrupt());
    }
    let float = |at: usize| f64::from_le_bytes(bytes[at..at + 8].try_into().unwrap_or_default());
    let whole =
        |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap_or_default()) as usize;
    let (width, height) = (whole(24), whole(28));
    let (chunks, rest) = bytes[HEADER_BYTES..].as_chunks::<CELL_BYTES>();
    if chunks.len() != width * height || !rest.is_empty() {
        return Err(corrupt());
    }

    Ok(Grid {
        west: float(0),
        north: float(8),
        cell_m: float(16),
        width,
        height,
        values: chunks
            .iter()
            .map(|chunk| f32::from_le_bytes(*chunk))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::LineString;

    fn grid(west: f64, north: f64, width: usize, height: usize, value: f32) -> Grid {
        Grid {
            west,
            north,
            cell_m: 10.0,
            width,
            height,
            values: vec![value; width * height],
        }
    }

    #[test]
    fn two_offset_tiles_join_on_their_own_lattice() {
        // The second tile starts one cell east and one cell south of the first one's corner.
        let joined = join(&[grid(1.6, 100.0, 2, 2, 1.0), grid(21.6, 90.0, 2, 2, 2.0)]).unwrap();
        assert_eq!((joined.west, joined.north), (1.6, 100.0));
        assert_eq!((joined.width, joined.height), (4, 3));
        assert_eq!(joined.value(0, 0), 1.0);
        assert_eq!(joined.value(3, 2), 2.0);
        assert!(joined.value(3, 0).is_nan());
    }

    #[test]
    fn a_grid_survives_the_cache_bytes() {
        let mut original = grid(120_511.6, 487_747.1, 3, 2, 19.5);
        original.values[4] = f32::NAN;
        let back = from_bytes(&to_bytes(&original)).unwrap();
        assert_eq!(
            (back.west, back.north, back.cell_m),
            (120_511.6, 487_747.1, 10.0)
        );
        assert_eq!(back.values[0], 19.5);
        assert!(back.values[4].is_nan());
        assert!(from_bytes(&to_bytes(&original)[..30]).is_err());
    }

    #[test]
    fn a_grid_survives_json_with_its_empty_cells() {
        let mut original = grid(120_511.6, 487_747.1, 3, 2, 19.5);
        original.values[4] = f32::NAN;
        let json = serde_json::to_string(&original).unwrap();
        assert!(json.contains("null"), "{json}");
        let back: Grid = serde_json::from_str(&json).unwrap();
        assert_eq!(back.values[0], 19.5);
        assert!(back.values[4].is_nan());
        let short = json.replace("19.5,null", "null");
        assert!(serde_json::from_str::<Grid>(&short).is_err());
    }

    #[test]
    fn a_cropped_grid_measures_an_area_inside_it_the_same() {
        let mut original = grid(0.0, 100.0, 10, 10, 50.0);
        for (index, value) in original.values.iter_mut().enumerate() {
            *value = index as f32;
        }
        let area = Polygon::new(
            LineString::from(vec![(22.0, 31.0), (58.0, 35.0), (40.0, 72.0), (22.0, 31.0)]),
            vec![],
        );
        let rect = geo::BoundingRect::bounding_rect(&area).unwrap();
        let cropped = original.cropped(rect);
        assert!(cropped.width < original.width && cropped.height < original.height);
        assert_eq!(
            stats_within(&cropped, &area),
            stats_within(&original, &area)
        );
    }

    #[test]
    fn stats_count_only_cells_centred_inside_the_area() {
        let mut values = grid(0.0, 20.0, 2, 2, 50.0);
        values.values[1] = 70.0;
        let left_half = Polygon::new(
            LineString::from(vec![(0.0, 0.0), (10.0, 0.0), (10.0, 20.0), (0.0, 20.0)]),
            vec![],
        );
        let stats = stats_within(&values, &left_half).unwrap();
        assert_eq!((stats.min, stats.mean, stats.max), (50.0, 50.0, 50.0));
        assert_eq!(stats.coverage, 1.0);
    }
}
