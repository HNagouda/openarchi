//! What a person standing at one point can see of the ground around them.
//!
//! The method is the R2 line-of-sight viewshed of Franklin and Ray (W. R.
//! Franklin and C. K. Ray, "Higher isn't necessarily better: visibility
//! algorithms and experiments", Advances in GIS Research, Sixth International
//! Symposium on Spatial Data Handling, Edinburgh, 1994, pp. 751-770). A height
//! surface is laid out as a grid, a sight line runs from the eye to every cell
//! on the grid's edge, and each line is walked cell by cell keeping the
//! steepest slope it has met so far. A cell is in view when its own slope from
//! the eye is at least that steep: nothing between it and the eye stands
//! higher. Cells the lines pass through are decided on the way, which is what
//! makes R2 fast enough to redraw with every change, where R3 runs one line
//! to each cell.
//!
//! The surface is the one of `grid/surface.rs`, which the sky view and the
//! solar radiation share: the ground, from the kept terrain when the scene
//! has it, with every building of the site and its neighbourhood standing on
//! it as a prism of its footprint and roof height, as the shadow diagram casts
//! them, and every wood as a solid stand of its canopy height (`forest.rs`);
//! single trees are see-through. The eye stands `EYE_HEIGHT_M` above the ground
//! where it is, so a viewpoint placed on a building looks from its roof, and
//! one placed in a wood looks from the floor of the wood.

#[cfg(test)]
mod tests;

use geo::{Centroid, Geometry, Rect};

use crate::grid::{area_of_cells, cell_size, rings, Raster, Surface, FINEST_CELL_M};
use crate::scene::{polygons, Attributes, Feature, Layer, Scene};
use crate::section;
use crate::{Error, Result};

/// The eye above the ground, in metres. 1.7 m is Harsh's choice: the eye
/// level of an average Dutch adult, about 1.80 m tall (CBS), with the eyes
/// about 10 cm below the top of the head. For comparison, the Landscape
/// Institute's Technical Guidance Note 06/19, "Visual representation of
/// development proposals" (2019, appendix 3.1.3), sets its camera at 1.5 m.
pub const EYE_HEIGHT_M: f64 = 1.7;
/// The ground counts as seen when a point this far above it is: Franklin and
/// Ray's target height. The kept terrain is rounded to the decimetre, and on
/// flat ground a sight line grazes it, so without this the rounding alone
/// would hide stray cells of an open square.
const TARGET_HEIGHT_M: f64 = 0.1;
/// At most this many cells, so a large drawing coarsens its grid instead of slowing down.
const MAX_CELLS: f64 = 160_000.0;
/// The surfaces a passer-by stands on: streets, paths and squares. A
/// courtyard or a garden is often walled in, so the default eye avoids them.
const PUBLIC_GROUND: [Layer; 5] = [
    Layer::Road,
    Layer::Footpath,
    Layer::Cycleway,
    Layer::Paved,
    Layer::Parking,
];

/// Which cells of the grid an eye can see.
pub struct Viewshed {
    raster: Raster,
    surface: Surface,
    /// Inside the site's boundary.
    is_site: Vec<bool>,
    is_visible: Vec<bool>,
    /// Where the eye stands, in the scene's frame.
    pub eye: (f64, f64),
    /// The eye stands on a roof rather than on the ground.
    pub is_on_roof: bool,
}

/// A viewpoint (RD New metres) the scene has data around, or an error saying
/// why not: it must lie in the site or its neighbourhood, like a section line.
pub fn check_view_point(scene: &Scene, (x, y): (f64, f64)) -> Result<()> {
    if !x.is_finite() || !y.is_finite() {
        return Err(Error::Invalid(format!(
            "a viewpoint needs a real position, got x {x} and y {y}"
        )));
    }
    let local = (x - scene.origin.0, y - scene.origin.1);
    let is_within = section::reach(scene).is_some_and(|reach| {
        local.0 >= reach.min().x
            && local.0 <= reach.max().x
            && local.1 >= reach.min().y
            && local.1 <= reach.max().y
    });
    if !is_within {
        return Err(Error::Invalid(
            "the viewpoint must lie inside the area the sheet draws: the fetched site and its neighbourhood"
                .to_string(),
        ));
    }

    Ok(())
}

/// The view from `eye` (the scene's frame), over the cells of `extent`.
/// Without an eye it stands on the street or square nearest to the site
/// centre. `extent` grows to hold the site and the eye.
pub fn viewshed(scene: &Scene, eye: Option<(f64, f64)>, extent: Rect<f64>) -> Viewshed {
    let centre = scene
        .boundary
        .centroid()
        .map_or((0.0, 0.0), |point| (point.x(), point.y()));
    let wanted = eye.unwrap_or(centre);
    let raster = raster_over(grown(extent, scene, wanted));
    let surface = Surface::build(scene, raster);
    let is_site = site_cells(scene, &raster);
    let eye_index = match eye {
        Some(eye) => cell_at(&raster, eye),
        None => {
            let is_public = public_cells(scene, &raster);
            nearest_open_ground(&raster, &surface, &is_site, &is_public, centre)
        }
    };
    let eye = match eye {
        Some(eye) => eye,
        None => raster.centre(eye_index),
    };
    // On a roof the eye looks from the roof; under a canopy, from the ground under it.
    let stands_on_m = if surface.is_roof[eye_index] {
        surface.heights[eye_index]
    } else {
        surface.grounds[eye_index]
    };
    let eye_height_m = stands_on_m + EYE_HEIGHT_M;
    let is_visible = sight_lines(&raster, &surface.heights, eye_index, eye_height_m);

    Viewshed {
        is_on_roof: surface.is_roof[eye_index],
        raster,
        surface,
        is_site,
        is_visible,
        eye,
    }
}

impl Viewshed {
    /// The share of the site's open ground in view, 0 to 1, or `None` for a
    /// site without open ground.
    pub fn share_of_site(&self) -> Option<f64> {
        let mut ground = 0usize;
        let mut seen = 0usize;
        for index in 0..self.raster.len() {
            if !self.is_site[index] || self.is_covered(index) {
                continue;
            }
            ground += 1;
            if self.is_visible[index] {
                seen += 1;
            }
        }

        (ground > 0).then(|| seen as f64 / ground as f64)
    }

    /// The scene was drawn on the kept terrain, not on flat ground.
    pub fn has_terrain(&self) -> bool {
        self.surface.has_terrain
    }

    /// Under a roof or a wood's canopy: not open ground, so never counted as in view.
    fn is_covered(&self, index: usize) -> bool {
        self.surface.is_roof[index] || self.surface.is_forest[index]
    }

    /// The open ground in view, as one feature. Roofs are left out: the
    /// buildings are drawn over them.
    pub fn area(&self) -> Feature {
        let area = area_of_cells(&self.raster, |index| {
            self.is_visible[index] && !self.is_covered(index)
        });
        Feature {
            layer: Layer::Viewshed,
            geometry: Geometry::MultiPolygon(area),
            attributes: Attributes::default(),
        }
    }
}

/// R2: one sight line from the eye to each cell on the grid's edge, walked
/// one cell at a time along its longer axis.
fn sight_lines(raster: &Raster, height_m: &[f64], eye_index: usize, eye_m: f64) -> Vec<bool> {
    let mut is_visible = vec![false; raster.len()];
    is_visible[eye_index] = true;
    let eye_cell = (eye_index % raster.columns, eye_index / raster.columns);
    for target in edge_cells(raster) {
        let delta_column = target.0 as f64 - eye_cell.0 as f64;
        let delta_row = target.1 as f64 - eye_cell.1 as f64;
        let steps = delta_column.abs().max(delta_row.abs()) as usize;
        let mut steepest = f64::NEG_INFINITY;
        for step in 1..=steps {
            let along = step as f64 / steps as f64;
            let column = (eye_cell.0 as f64 + delta_column * along).round() as usize;
            let row = (eye_cell.1 as f64 + delta_row * along).round() as usize;
            let index = row * raster.columns + column;
            let distance_m = (delta_column * along).hypot(delta_row * along) * raster.cell_m;
            let slope = (height_m[index] - eye_m) / distance_m;
            let target_slope = slope + TARGET_HEIGHT_M / distance_m;
            if target_slope >= steepest {
                is_visible[index] = true;
            }
            steepest = steepest.max(slope);
        }
    }

    is_visible
}

/// Every cell on the grid's outer edge, as (column, row).
fn edge_cells(raster: &Raster) -> Vec<(usize, usize)> {
    let (columns, rows) = (raster.columns, raster.rows);
    let mut cells = Vec::with_capacity(2 * (columns + rows));
    for column in 0..columns {
        cells.push((column, 0));
        cells.push((column, rows - 1));
    }
    for row in 1..rows.saturating_sub(1) {
        cells.push((0, row));
        cells.push((columns - 1, row));
    }

    cells
}

/// `extent` grown to hold the site and the eye, with a cell to spare.
fn grown(extent: Rect<f64>, scene: &Scene, eye: (f64, f64)) -> Rect<f64> {
    let mut min = (extent.min().x.min(eye.0), extent.min().y.min(eye.1));
    let mut max = (extent.max().x.max(eye.0), extent.max().y.max(eye.1));
    for coord in &scene.boundary.exterior().0 {
        min = (min.0.min(coord.x), min.1.min(coord.y));
        max = (max.0.max(coord.x), max.1.max(coord.y));
    }

    Rect::new(
        (min.0 - FINEST_CELL_M, min.1 - FINEST_CELL_M),
        (max.0 + FINEST_CELL_M, max.1 + FINEST_CELL_M),
    )
}

/// A grid over `extent`, as fine as `MAX_CELLS` allows.
fn raster_over(extent: Rect<f64>) -> Raster {
    let width = extent.width().max(FINEST_CELL_M);
    let height = extent.height().max(FINEST_CELL_M);
    let cell_m = cell_size(width, height, MAX_CELLS);

    Raster {
        min: (extent.min().x, extent.min().y),
        cell_m,
        columns: (width / cell_m).ceil() as usize,
        rows: (height / cell_m).ceil() as usize,
    }
}

/// The cell holding `point`, or the nearest edge cell for a point past the grid.
fn cell_at(raster: &Raster, point: (f64, f64)) -> usize {
    let column = ((point.0 - raster.min.0) / raster.cell_m).floor();
    let row = ((point.1 - raster.min.1) / raster.cell_m).floor();
    let column = column.clamp(0.0, (raster.columns - 1) as f64) as usize;
    let row = row.clamp(0.0, (raster.rows - 1) as f64) as usize;

    row * raster.columns + column
}

/// The cell nearest to `point` on a street or square in the site, or on any
/// open ground in it when it has neither, or the cell under `point` when the
/// site has no open ground at all.
fn nearest_open_ground(
    raster: &Raster,
    surface: &Surface,
    is_site: &[bool],
    is_public: &[bool],
    point: (f64, f64),
) -> usize {
    let is_open =
        |index: usize| is_site[index] && !surface.is_roof[index] && !surface.is_forest[index];
    let is_on_street = |index: usize| is_open(index) && is_public[index];
    nearest_where(raster, point, is_on_street)
        .or_else(|| nearest_where(raster, point, is_open))
        .unwrap_or_else(|| cell_at(raster, point))
}

/// Which cells lie inside the site's boundary.
fn site_cells(scene: &Scene, raster: &Raster) -> Vec<bool> {
    let mut is_site = vec![false; raster.len()];
    raster.fill_rings(&rings(&scene.boundary), |index| is_site[index] = true);
    is_site
}

/// Which cells lie on a street, path or square, see `PUBLIC_GROUND`.
fn public_cells(scene: &Scene, raster: &Raster) -> Vec<bool> {
    let mut is_public = vec![false; raster.len()];
    let public_ground = scene
        .features
        .iter()
        .filter(|feature| PUBLIC_GROUND.contains(&feature.layer));
    for ground in public_ground {
        for polygon in polygons(&ground.geometry) {
            raster.fill_rings(&rings(polygon), |index| is_public[index] = true);
        }
    }

    is_public
}

fn nearest_where(
    raster: &Raster,
    point: (f64, f64),
    is_wanted: impl Fn(usize) -> bool,
) -> Option<usize> {
    let mut nearest = None;
    let mut nearest_m = f64::INFINITY;
    for index in (0..raster.len()).filter(|&index| is_wanted(index)) {
        let (x, y) = raster.centre(index);
        let distance_m = (x - point.0).hypot(y - point.1);
        if distance_m < nearest_m {
            nearest = Some(index);
            nearest_m = distance_m;
        }
    }

    nearest
}
