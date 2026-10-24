//! A fixed grid of square tiles over RD New, the unit every source is fetched
//! and cached in.
//!
//! The grid is anchored at RD's own origin rather than at the site, so two
//! sites that overlap share tiles: moving a site 50 m only fetches the row of
//! tiles it moved into.

use geo::{BoundingRect, Coord, Intersects, Polygon, Rect};

/// Small enough that a nudged site reuses most of its tiles, large enough that
/// a 1 km radius site stays well under a hundred tiles per dataset. A multiple
/// of the AHN cell, so raster tiles line up with the elevation grid.
pub const TILE_M: f64 = 250.0;

/// One tile, counted in tiles from the RD origin: column 482 starts at x = 120 500.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tile {
    pub column: i64,
    pub row: i64,
}

impl Tile {
    pub fn rect(self) -> Rect<f64> {
        let min = Coord {
            x: self.column as f64 * TILE_M,
            y: self.row as f64 * TILE_M,
        };
        Rect::new(
            min,
            Coord {
                x: min.x + TILE_M,
                y: min.y + TILE_M,
            },
        )
    }
}

/// Every tile that overlaps `rect`, row by row from the south-west.
pub fn tiles_covering(rect: Rect<f64>) -> Vec<Tile> {
    let first_column = (rect.min().x / TILE_M).floor() as i64;
    let first_row = (rect.min().y / TILE_M).floor() as i64;
    // A rect ending exactly on a tile edge does not reach into the next tile.
    let end_column = ((rect.max().x / TILE_M).ceil() as i64).max(first_column + 1);
    let end_row = ((rect.max().y / TILE_M).ceil() as i64).max(first_row + 1);

    let mut tiles = Vec::new();
    for row in first_row..end_row {
        for column in first_column..end_column {
            tiles.push(Tile { column, row });
        }
    }

    tiles
}

/// The tiles `boundary` actually reaches. For a circle this skips the corner
/// tiles of its bounding box, which hold nothing the site will keep.
pub fn tiles_touching(boundary: &Polygon<f64>) -> Vec<Tile> {
    let Some(bbox) = boundary.bounding_rect() else {
        return Vec::new();
    };
    tiles_covering(bbox)
        .into_iter()
        .filter(|tile| boundary.intersects(&tile.rect()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{LineString, Polygon};

    fn rect(min: (f64, f64), max: (f64, f64)) -> Rect<f64> {
        Rect::new(Coord { x: min.0, y: min.1 }, Coord { x: max.0, y: max.1 })
    }

    #[test]
    fn a_rect_inside_one_tile_is_that_tile() {
        let tiles = tiles_covering(rect((120_010.0, 487_010.0), (120_100.0, 487_100.0)));
        assert_eq!(
            tiles,
            vec![Tile {
                column: 480,
                row: 1948
            }]
        );
        assert_eq!(tiles[0].rect().min().x, 120_000.0);
    }

    #[test]
    fn a_rect_ending_on_a_tile_edge_does_not_take_the_next_tile() {
        let tiles = tiles_covering(rect((120_000.0, 487_000.0), (120_500.0, 487_250.0)));
        assert_eq!(tiles.len(), 2);
    }

    #[test]
    fn a_circle_skips_the_corner_tiles_of_its_box() {
        // A 600 m circle centred on a tile corner: its box spans 4 by 4 tiles,
        // and the four corner tiles lie outside the circle.
        let centre = (120_000.0, 487_000.0);
        let ring: Vec<Coord<f64>> = (0..128)
            .map(|index| {
                let angle = std::f64::consts::TAU * index as f64 / 128.0;
                Coord {
                    x: centre.0 + 300.0 * angle.cos(),
                    y: centre.1 + 300.0 * angle.sin(),
                }
            })
            .collect();
        let circle = Polygon::new(LineString::new(ring), vec![]);
        assert_eq!(tiles_touching(&circle).len(), 12);
    }
}
