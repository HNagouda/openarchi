//! The height of everything that can block a line of sight or a ray of sun:
//! the ground, the buildings on it as prisms of their footprint and roof
//! height, and dense trees as solid stands of their canopy height
//! (`forest.rs`). The view, the sky view factor and the solar radiation all
//! look over this one surface, so they agree on what stands where.
//!
//! Heights are metres above the terrain model's datum (NAP in the
//! Netherlands). The kept terrain has no value under buildings and water,
//! where the survey saw roofs and water instead of ground. Those gaps are
//! filled from the measured cells around them, ring by ring inwards, so the
//! ground under a building lies at the level of the streets beside it and a
//! line along a street does not drop into a hole. A building's roof stands at
//! its own measured ground level plus its roof height, both taken from the
//! elevation model at the fetch, which is where the survey saw the roof; a
//! building without a measured ground stands on the filled terrain. Where
//! footprints overlap, the taller roof wins. A scene without terrain stands
//! on flat ground at 0, with each building its roof height above it.
//!
//! A wood stands solid from the ground up to its canopy, since trunks and
//! undergrowth close it off. A single tree is left out: sight and light pass
//! under and around its crown, and a 2.5D surface could only hold that crown
//! as a column down to the ground, which hides far more than a tree does.

use super::{rings, Raster};
use crate::forest;
use crate::render::building_height;
use crate::scene::{polygons, Layer, Scene};
use crate::sources::raster::Grid;

/// How many rings of cells a gap in the terrain is filled from its edge.
/// The widest gap is a large building or a harbour basin; 60 cells of 4 to
/// 6 m reach 240 m and more into it from each side.
const MAX_FILL_RINGS: usize = 60;

/// What stands on each cell of a grid.
pub struct Surface {
    raster: Raster,
    /// The top of the ground, the roof or a wood's canopy.
    pub heights: Vec<f64>,
    /// The ground itself, under a roof or a canopy too.
    pub grounds: Vec<f64>,
    pub is_roof: Vec<bool>,
    /// Under a wood's canopy, see `forest.rs`.
    pub is_forest: Vec<bool>,
    /// The ground level past the grid's edge: the median of the measured terrain.
    level: f64,
    /// The highest top on the grid, so a line rising past it can stop looking.
    pub highest: f64,
    /// The ground is the kept terrain, not flat ground at 0.
    pub has_terrain: bool,
}

impl Surface {
    /// The surface on the cells of `raster`.
    pub fn build(scene: &Scene, raster: Raster) -> Surface {
        let terrain = scene.terrain.as_ref().map(filled);
        let level = scene
            .terrain
            .as_ref()
            .and_then(Grid::median)
            .map_or(0.0, f64::from);
        let grounds: Vec<f64> = (0..raster.len())
            .map(|index| match &terrain {
                Some(terrain) => f64::from(nearest_value(terrain, raster.centre(index))),
                None => 0.0,
            })
            .collect();
        let mut heights = grounds.clone();
        let mut is_roof = vec![false; raster.len()];

        let buildings = scene
            .features
            .iter()
            .chain(&scene.context)
            .filter(|feature| feature.layer == Layer::Building);
        for building in buildings {
            let (roof_m, _) = building_height(building);
            // Without terrain the ground is flat at 0, so a building's own ground level would lift it off it.
            let measured_ground_m = building.attributes.ground_m.filter(|_| terrain.is_some());
            for polygon in polygons(&building.geometry) {
                raster.fill_rings(&rings(polygon), |index| {
                    let top = measured_ground_m.unwrap_or(grounds[index]) + roof_m;
                    heights[index] = if is_roof[index] {
                        heights[index].max(top)
                    } else {
                        top
                    };
                    is_roof[index] = true;
                });
            }
        }

        let mut is_forest = vec![false; raster.len()];
        if let Some(forest) = forest::forest(scene) {
            for index in (0..raster.len()).filter(|&index| !is_roof[index]) {
                if let Some(canopy_m) = forest.height_at(raster.centre(index)) {
                    heights[index] = grounds[index] + canopy_m;
                    is_forest[index] = true;
                }
            }
        }

        let highest = heights.iter().copied().fold(level, f64::max);
        Surface {
            raster,
            heights,
            grounds,
            is_roof,
            is_forest,
            level,
            highest,
            has_terrain: terrain.is_some(),
        }
    }

    /// The top of whatever stands under a point; the ground level past the grid's edge.
    pub fn height_at(&self, point: (f64, f64)) -> f64 {
        self.raster
            .index_at(point)
            .map_or(self.level, |index| self.heights[index])
    }

    /// The ground under a point, below any roof or canopy.
    pub fn ground_at(&self, point: (f64, f64)) -> f64 {
        self.raster
            .index_at(point)
            .map_or(self.level, |index| self.grounds[index])
    }

    /// The point lies past the grid, where nothing stands above the ground level.
    pub fn is_beyond(&self, point: (f64, f64)) -> bool {
        self.raster.index_at(point).is_none()
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

    let datum = terrain.median().unwrap_or(0.0);
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

/// The value of the cell under a point, or of the nearest edge cell for a point past the grid.
fn nearest_value(grid: &Grid, (x, y): (f64, f64)) -> f32 {
    let column = ((x - grid.west) / grid.cell_m).floor();
    let row = ((grid.north - y) / grid.cell_m).floor();
    let column = column.clamp(0.0, (grid.width - 1) as f64) as usize;
    let row = row.clamp(0.0, (grid.height - 1) as f64) as usize;

    grid.value(column, row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forest::tests::canopy;
    use crate::grid::raster_over;
    use crate::grid::tests::{building, site, square};

    fn top_at(scene: &Scene, point: (f64, f64)) -> f64 {
        Surface::build(scene, raster_over(&scene.boundary, 10_000.0)).height_at(point)
    }

    #[test]
    fn where_footprints_overlap_the_taller_roof_wins_in_any_order() {
        let low = building(square(0.0, 0.0, 20.0, 20.0), 6.0);
        let high = building(square(10.0, 10.0, 30.0, 30.0), 15.0);
        let low_first = site(vec![low.clone(), high.clone()], Vec::new());
        let high_first = site(vec![high, low], Vec::new());

        assert_eq!(top_at(&low_first, (15.0, 15.0)), 15.0);
        assert_eq!(top_at(&high_first, (15.0, 15.0)), 15.0);
    }

    #[test]
    fn a_roof_stands_on_its_measured_ground_else_on_the_terrain() {
        let mut measured = building(square(0.0, 0.0, 20.0, 20.0), 10.0);
        measured.attributes.ground_m = Some(5.0);
        let unmeasured = building(square(40.0, 0.0, 60.0, 20.0), 10.0);
        let mut scene = site(vec![measured, unmeasured], Vec::new());
        scene.terrain = Some(canopy(&[(-250.0, -250.0, 250.0, 250.0)], 2.0));

        assert_eq!(top_at(&scene, (10.0, 10.0)), 15.0);
        assert_eq!(top_at(&scene, (50.0, 10.0)), 12.0);
    }

    #[test]
    fn without_terrain_a_roof_stands_on_flat_ground_at_zero() {
        let mut lifted = building(square(0.0, 0.0, 20.0, 20.0), 10.0);
        lifted.attributes.ground_m = Some(5.0);

        assert_eq!(top_at(&site(vec![lifted], Vec::new()), (10.0, 10.0)), 10.0);
    }
}
