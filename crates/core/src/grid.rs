//! The square grid the computed layers work on: hours of sun, sky view,
//! solar radiation, the view and the wind all lay one over the site, mark
//! which cells are ground and which roof, and turn a value per cell back into
//! bands to draw.

mod raster;
#[cfg(test)]
pub(crate) mod tests;

use geo::{BooleanOps, BoundingRect, Geometry, LineString, MultiPolygon, Polygon, Rect};

use crate::diagram::class_index;
use crate::scene::{polygons, Attributes, Feature, Layer, Scene};
pub use raster::Raster;

/// The finest grid, in metres: a little over a parking bay is as fine as a site sheet reads.
pub const FINEST_CELL_M: f64 = 2.0;

/// What lies on a grid cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Outside,
    Ground,
    Roof,
}

/// The cell size that lays a `width` by `height` area out in at most
/// `max_cells` cells, never finer than `FINEST_CELL_M`.
pub fn cell_size(width: f64, height: f64, max_cells: f64) -> f64 {
    FINEST_CELL_M.max((width * height / max_cells).sqrt())
}

/// A grid over the boundary's bounds, as fine as `max_cells` allows.
pub fn raster_over(boundary: &Polygon<f64>, max_cells: f64) -> Raster {
    // An empty boundary gets an empty grid.
    let bounds = boundary
        .bounding_rect()
        .unwrap_or(Rect::new((0.0, 0.0), (0.0, 0.0)));

    Raster::over(
        bounds,
        cell_size(bounds.width(), bounds.height(), max_cells),
    )
}

/// Which cells lie in the site, and which of those under a roof.
pub fn cells(scene: &Scene, raster: &Raster) -> Vec<Cell> {
    let mut cells = vec![Cell::Outside; raster.len()];
    raster.fill_rings(&rings(&scene.boundary), |index| cells[index] = Cell::Ground);
    let buildings = scene
        .features
        .iter()
        .filter(|feature| feature.layer == Layer::Building);
    for building in buildings {
        for polygon in polygons(&building.geometry) {
            raster.fill_rings(&rings(polygon), |index| {
                if cells[index] == Cell::Ground {
                    cells[index] = Cell::Roof;
                }
            });
        }
    }

    cells
}

/// A grid of values as one area of `layer` per class of `breaks`, each
/// carrying the lower edge of its class as its value, the way measured bands
/// do. n breaks make up to n + 1 areas; classes no cell falls in are left out.
pub fn banded(
    raster: &Raster,
    values: &[Option<f64>],
    breaks: &[f64],
    layer: Layer,
) -> Vec<Feature> {
    let mut squares: Vec<Vec<Polygon<f64>>> = vec![Vec::new(); breaks.len() + 1];
    for (index, value) in values.iter().enumerate() {
        let Some(value) = value else {
            continue;
        };
        squares[class_index(*value, breaks)].push(cell_square(raster, index));
    }

    squares
        .into_iter()
        .enumerate()
        .filter(|(_, class_squares)| !class_squares.is_empty())
        .map(|(class, class_squares)| {
            // Squares never overlap, so one overlay merges them into the band's outline.
            let band = MultiPolygon::new(class_squares).union(&MultiPolygon::new(Vec::new()));
            let lower_edge = if class == 0 { 0.0 } else { breaks[class - 1] };
            Feature {
                layer,
                geometry: Geometry::MultiPolygon(band),
                attributes: Attributes {
                    value: Some(lower_edge),
                    ..Attributes::default()
                },
            }
        })
        .collect()
}

/// Every ring of a polygon, the outline first, as the point lists `Raster` fills.
pub fn rings(polygon: &Polygon<f64>) -> Vec<Vec<(f64, f64)>> {
    std::iter::once(polygon.exterior())
        .chain(polygon.interiors())
        .map(|ring| ring.0.iter().map(|c| (c.x, c.y)).collect())
        .collect()
}

fn cell_square(raster: &Raster, index: usize) -> Polygon<f64> {
    let (x, y) = raster.centre(index);
    let half = raster.cell_m / 2.0;
    Polygon::new(
        LineString::from(vec![
            (x - half, y - half),
            (x + half, y - half),
            (x + half, y + half),
            (x - half, y + half),
            (x - half, y - half),
        ]),
        Vec::new(),
    )
}
