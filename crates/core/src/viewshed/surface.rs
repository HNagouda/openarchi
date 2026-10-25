//! The height surface a viewshed looks over: the ground, with the buildings
//! standing on it.
//!
//! The kept terrain has no value under buildings and water, where the survey
//! saw roofs and water instead of ground. Those gaps are filled from the
//! measured cells around them, ring by ring inwards, so the ground under a
//! building lies at the level of the streets beside it and a sight line along
//! a street does not drop into a hole. A building stands on that ground with
//! its roof height, which is measured from the ground around it. A scene
//! without terrain stands on flat ground at 0.
//!
//! Trees are left out. Their crowns stand above eye height, so a sight line
//! from a standing eye down to the ground passes under them, and a 2.5D
//! surface can only hold a crown as a solid column down to the ground, which
//! would hide far more than a tree does.

// #! NOTE[view, trees] : Trees left out of the viewshed; count crowns as solid in summer, when they hide most?

use geo::{Geometry, Polygon};

use crate::render::building_height;
use crate::scene::{Layer, Scene};
use crate::sources::raster::Grid;
use crate::sunlight::raster::Raster;

/// How many rings of cells a gap in the terrain is filled from its edge.
/// The widest gap is a large building or a harbour basin; 60 cells of 4 to
/// 6 m reach 240 m and more into it from each side.
const MAX_FILL_RINGS: usize = 60;
/// The surfaces a passer-by stands on: streets, paths and squares. A
/// courtyard or a garden is often walled in, so the default eye avoids them.
const PUBLIC_GROUND: [Layer; 5] = [
    Layer::Road,
    Layer::Footpath,
    Layer::Cycleway,
    Layer::Paved,
    Layer::Parking,
];

/// What stands on each cell of a viewshed's grid.
pub struct Surface {
    /// The top of the ground or the roof, in the terrain's heights (above NAP
    /// in the Netherlands), or above flat ground at 0 without terrain.
    pub height_m: Vec<f64>,
    pub is_roof: Vec<bool>,
    /// Inside the site's boundary.
    pub is_site: Vec<bool>,
    /// On a street, path or square, see `PUBLIC_GROUND`.
    pub is_public: Vec<bool>,
    pub has_terrain: bool,
}

pub fn build(scene: &Scene, raster: &Raster) -> Surface {
    let ground = scene.terrain.as_ref().map(filled);
    let mut height_m: Vec<f64> = (0..raster.len())
        .map(|index| {
            let (x, y) = raster.centre(index);
            ground.as_ref().map_or(0.0, |grid| ground_at(grid, x, y))
        })
        .collect();
    let mut is_roof = vec![false; raster.len()];
    let mut is_site = vec![false; raster.len()];
    raster.fill_rings(&rings(&scene.boundary), |index| is_site[index] = true);
    let mut is_public = vec![false; raster.len()];
    let public_ground = scene
        .features
        .iter()
        .filter(|feature| PUBLIC_GROUND.contains(&feature.layer));
    for surface in public_ground {
        for polygon in polygons(&surface.geometry) {
            raster.fill_rings(&rings(polygon), |index| is_public[index] = true);
        }
    }

    let buildings = scene
        .features
        .iter()
        .chain(&scene.context)
        .filter(|feature| feature.layer == Layer::Building);
    for building in buildings {
        let (roof_m, _) = building_height(building);
        for polygon in polygons(&building.geometry) {
            raster.fill_rings(&rings(polygon), |index| {
                if !is_roof[index] {
                    height_m[index] += roof_m;
                    is_roof[index] = true;
                }
            });
        }
    }

    Surface {
        height_m,
        is_roof,
        is_site,
        is_public,
        has_terrain: ground.is_some(),
    }
}

/// The terrain with every gap filled: each pass gives the unmeasured cells
/// next to measured ones the mean of those neighbours. Cells still empty
/// after `MAX_FILL_RINGS` passes take the median of the measured ground.
fn filled(terrain: &Grid) -> Grid {
    let mut values = terrain.values.clone();
    let (width, height) = (terrain.width, terrain.height);
    for _ in 0..MAX_FILL_RINGS {
        let before = values.clone();
        let mut is_changed = false;
        for row in 0..height {
            for column in 0..width {
                let index = row * width + column;
                if before[index].is_finite() {
                    continue;
                }
                let neighbours = [
                    (column > 0).then(|| index - 1),
                    (column + 1 < width).then(|| index + 1),
                    (row > 0).then(|| index - width),
                    (row + 1 < height).then(|| index + width),
                ];
                let measured: Vec<f32> = neighbours
                    .into_iter()
                    .flatten()
                    .map(|neighbour| before[neighbour])
                    .filter(|value| value.is_finite())
                    .collect();
                if !measured.is_empty() {
                    values[index] = measured.iter().sum::<f32>() / measured.len() as f32;
                    is_changed = true;
                }
            }
        }
        if !is_changed {
            break;
        }
    }

    let datum = median_height(terrain).unwrap_or(0.0);
    for value in &mut values {
        if !value.is_finite() {
            *value = datum;
        }
    }

    Grid {
        values,
        ..terrain.clone()
    }
}

/// The ground under (`x`, `y`), in the grid's frame, or at the nearest edge
/// cell for a point past the grid.
fn ground_at(grid: &Grid, x: f64, y: f64) -> f64 {
    let column = ((x - grid.west) / grid.cell_m).floor();
    let row = ((grid.north - y) / grid.cell_m).floor();
    let column = column.clamp(0.0, (grid.width - 1) as f64) as usize;
    let row = row.clamp(0.0, (grid.height - 1) as f64) as usize;

    f64::from(grid.value(column, row))
}

/// The median of the measured cells, or `None` when none was measured.
fn median_height(grid: &Grid) -> Option<f32> {
    let mut measured: Vec<f32> = grid
        .values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect();
    if measured.is_empty() {
        return None;
    }
    measured.sort_by(f32::total_cmp);

    Some(measured[measured.len() / 2])
}

fn rings(polygon: &Polygon<f64>) -> Vec<Vec<(f64, f64)>> {
    std::iter::once(polygon.exterior())
        .chain(polygon.interiors())
        .map(|ring| ring.0.iter().map(|c| (c.x, c.y)).collect())
        .collect()
}

fn polygons(geometry: &Geometry<f64>) -> Vec<&Polygon<f64>> {
    match geometry {
        Geometry::Polygon(polygon) => vec![polygon],
        Geometry::MultiPolygon(multi) => multi.0.iter().collect(),
        _ => Vec::new(),
    }
}
