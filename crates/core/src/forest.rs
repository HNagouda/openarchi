//! Trees dense enough to hide what stands behind them, like a wood, as
//! opposed to a street tree or a garden tree that sight, sky and sun pass
//! through between and under the crowns.
//!
//! "Dense" is the forest of the FAO Global Forest Resources Assessment (FAO,
//! "Global Forest Resources Assessment 2020: Terms and Definitions", Forest
//! Resources Assessment Working Paper 188, 2018): land spanning more than
//! 0.5 ha with trees higher than 5 m and a canopy cover of more than 10 %,
//! where a strip of trees counts only when it is more than 20 m wide. On the
//! tree height raster that becomes: cells with a tree taller than 5 m, kept
//! where they fill a 20 m square, joined into patches by their edges, and
//! patches of at least 0.5 ha. A patch is a closed canopy, far above 10 %
//! cover. Open land with scattered trees, which the FAO also counts as
//! forest, is left out, because sight passes between the trees.
//!
//! A forest cell stands solid from the ground up to its canopy height. The
//! view, the sky view factor and the solar radiation all read it from here,
//! so the three agree on which trees block.
//!
//! The canopy is the tree raster the scene keeps, with the cells under a
//! building footprint left out: the tree map took some roofs for crowns. A
//! scene without the raster has no forest.

#[cfg(test)]
pub(crate) mod tests;

use geo::Polygon;

use crate::grid::{rings, Raster};
use crate::scene::{polygons, Layer, Scene};
use crate::sources::raster::Grid;

/// The FAO's thresholds, see the module comment.
pub const MIN_TREE_HEIGHT_M: f64 = 5.0;
pub const MIN_AREA_M2: f64 = 5_000.0;
pub const MIN_WIDTH_M: f64 = 20.0;

/// The canopy height above the ground of each forest cell, and no value elsewhere.
pub struct Forest {
    canopy: Grid,
}

impl Forest {
    /// The canopy height above the ground at a point, `None` off the forest.
    pub fn height_at(&self, (x, y): (f64, f64)) -> Option<f64> {
        let grid = &self.canopy;
        let column = ((x - grid.west) / grid.cell_m).floor();
        let row = ((grid.north - y) / grid.cell_m).floor();
        let is_inside =
            column >= 0.0 && row >= 0.0 && column < grid.width as f64 && row < grid.height as f64;
        if !is_inside {
            return None;
        }

        let height = grid.value(column as usize, row as usize);
        height.is_finite().then_some(f64::from(height))
    }
}

/// The forest of `scene`, or `None` when it kept no tree raster or has no forest.
pub fn forest(scene: &Scene) -> Option<Forest> {
    let raw = scene.measured.grids.get(&Layer::TreeCanopy)?;
    let buildings = scene
        .features
        .iter()
        .chain(&scene.context)
        .filter(|feature| feature.layer == Layer::Building)
        .flat_map(|feature| polygons(&feature.geometry));
    let canopy = without_buildings(raw, buildings);

    let (width, height) = (canopy.width, canopy.height);
    let is_tall: Vec<bool> = canopy
        .values
        .iter()
        .map(|value| f64::from(*value) > MIN_TREE_HEIGHT_M)
        .collect();
    let side = (MIN_WIDTH_M / canopy.cell_m).ceil() as usize;
    let is_wide = filling_squares(&is_tall, width, height, side);
    let min_cells = (MIN_AREA_M2 / (canopy.cell_m * canopy.cell_m)).ceil() as usize;
    let is_forest = in_large_patches(&is_wide, width, height, min_cells);
    if !is_forest.contains(&true) {
        return None;
    }

    let values = canopy
        .values
        .iter()
        .zip(&is_forest)
        .map(|(value, &is_forest)| if is_forest { *value } else { f32::NAN })
        .collect();
    Some(Forest {
        canopy: Grid { values, ..canopy },
    })
}

/// `grid` with no tree in every cell one of `footprints` reaches into: its
/// centre or one of its corners lies inside. The corners count because a
/// tall façade along a cell's edge reads as a "tree" of roof height there,
/// such as the 54 m one beside a TU/e tower, with the cell's centre just outside.
pub fn without_buildings<'a>(
    grid: &Grid,
    footprints: impl IntoIterator<Item = &'a Polygon<f64>>,
) -> Grid {
    let (width, height) = (grid.width, grid.height);
    // The cells numbered from the south-west, as `Raster` counts them, and
    // their corners as the centres of a lattice half a cell further out.
    let centres = Raster {
        min: (grid.west, grid.south()),
        cell_m: grid.cell_m,
        columns: width,
        rows: height,
    };
    let corners = Raster {
        min: (
            grid.west - grid.cell_m / 2.0,
            grid.south() - grid.cell_m / 2.0,
        ),
        cell_m: grid.cell_m,
        columns: width + 1,
        rows: height + 1,
    };
    let mut is_built = vec![false; width * height];
    let mut is_corner_built = vec![false; corners.len()];
    for footprint in footprints {
        let rings = rings(footprint);
        centres.fill_rings(&rings, |index| is_built[index] = true);
        corners.fill_rings(&rings, |index| is_corner_built[index] = true);
    }

    let mut values = grid.values.clone();
    for row in 0..height {
        for column in 0..width {
            let corner =
                |dx: usize, dy: usize| is_corner_built[(row + dy) * (width + 1) + column + dx];
            let is_reached = is_built[row * width + column]
                || corner(0, 0)
                || corner(1, 0)
                || corner(0, 1)
                || corner(1, 1);
            if is_reached {
                // `row` counts from the south; the grid's rows from the north.
                values[(height - 1 - row) * width + column] = f32::NAN;
            }
        }
    }

    Grid {
        values,
        ..grid.clone()
    }
}

/// The cells of `is_set` that lie in some `side` by `side` square of set
/// cells: what is left of the patches where they are at least that wide.
fn filling_squares(is_set: &[bool], width: usize, height: usize, side: usize) -> Vec<bool> {
    let mut is_kept = vec![false; is_set.len()];
    if side == 0 || side > width || side > height {
        return is_kept;
    }

    for top in 0..=height - side {
        for left in 0..=width - side {
            let is_full = (top..top + side)
                .all(|row| (left..left + side).all(|column| is_set[row * width + column]));
            if !is_full {
                continue;
            }
            for row in top..top + side {
                for column in left..left + side {
                    is_kept[row * width + column] = true;
                }
            }
        }
    }

    is_kept
}

/// The cells of `is_set` in a patch of at least `min_cells` cells, joined by
/// their edges, found by filling each patch from its first cell.
fn in_large_patches(is_set: &[bool], width: usize, height: usize, min_cells: usize) -> Vec<bool> {
    let mut is_seen = vec![false; is_set.len()];
    let mut is_kept = vec![false; is_set.len()];
    for start in 0..is_set.len() {
        if !is_set[start] || is_seen[start] {
            continue;
        }
        is_seen[start] = true;
        let mut patch = vec![start];
        let mut next = 0;
        while next < patch.len() {
            let index = patch[next];
            next += 1;
            let (column, row) = (index % width, index / width);
            let neighbours = [
                (column > 0).then(|| index - 1),
                (column + 1 < width).then(|| index + 1),
                (row > 0).then(|| index - width),
                (row + 1 < height).then(|| index + width),
            ];
            for neighbour in neighbours.into_iter().flatten() {
                if is_set[neighbour] && !is_seen[neighbour] {
                    is_seen[neighbour] = true;
                    patch.push(neighbour);
                }
            }
        }
        if patch.len() >= min_cells {
            for index in patch {
                is_kept[index] = true;
            }
        }
    }

    is_kept
}
