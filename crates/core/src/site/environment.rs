//! The environmental rasters of a site, from tiles to bands: noise, air
//! quality, flood depth and slope.
//!
//! Each raster is measured inside the site before it becomes bands, because
//! the bands keep only which class a place is in, and is kept cut to the
//! site's box so a sheet area can be measured again. The band edges are the
//! breaks of the diagram that draws the layer, so the map and its legend can
//! never disagree.

use geo::{BoundingRect, Coord, Polygon, Rect};

use super::to_local_centimetres;

use crate::catalog;
use crate::diagram::Scheme;
use crate::environment::Measured;
use crate::scene::{Feature, Layer};
use crate::sources::bands::{self, Gaps, Lowest};
use crate::sources::edition::Edition;
use crate::sources::grid::{tiles_covering, Tile};
use crate::sources::raster::{self, Grid};
use crate::sources::slope::slope_percent;
use crate::sources::wcs::Coverage;
use crate::sources::{liwo, rivm};

/// Rasters reach this far past the site, so the blur and the band edges
/// along the boundary have real cells on both sides.
const MARGIN_M: f64 = 30.0;
/// Water shallower than this is a wet street, not a flood worth a band.
const FLOOD_MIN_DEPTH_M: f64 = 0.01;

/// Every raster of this module, in one place. The greenery rasters are
/// `site::greenery::Greenery`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Raster {
    RoadNoise,
    AllNoise,
    No2,
    Pm10,
    Pm25,
    Flood,
}

pub const RIVM_RASTERS: [Raster; 5] = [
    Raster::RoadNoise,
    Raster::AllNoise,
    Raster::No2,
    Raster::Pm10,
    Raster::Pm25,
];

impl Raster {
    /// The coverage that holds `edition` of this raster.
    pub fn coverage(self, edition: &Edition) -> Coverage<'_> {
        match self {
            Raster::Flood => Coverage {
                id: &edition.id,
                ..liwo::FLOOD_DEPTH
            },
            _ => rivm::coverage(&edition.id),
        }
    }

    pub fn layer(self) -> Layer {
        match self {
            Raster::RoadNoise => Layer::RoadNoise,
            Raster::AllNoise => Layer::AllNoise,
            Raster::No2 => Layer::No2,
            Raster::Pm10 => Layer::Pm10,
            Raster::Pm25 => Layer::Pm25,
            Raster::Flood => Layer::Flood,
        }
    }
}

/// The tiles a raster is fetched in: the site's box plus a margin.
pub fn tiles(bbox: Rect<f64>) -> Vec<Tile> {
    let margin = geo::Coord {
        x: MARGIN_M,
        y: MARGIN_M,
    };
    tiles_covering(Rect::new(bbox.min() - margin, bbox.max() + margin))
}

/// The bands of one raster from its tiles. Its figures inside `boundary` go
/// into `measured`, with the raster cut to the boundary's box when they found a value.
pub fn raster_bands(
    raster: Raster,
    tiles: &[Grid],
    boundary: &Polygon<f64>,
    measured: &mut Measured,
) -> Vec<Feature> {
    let Some(grid) = raster::join(tiles) else {
        return Vec::new();
    };
    let (lowest, gaps) = match raster {
        // A cell LIWO leaves empty stays dry in every scenario.
        Raster::Flood => (Lowest::From(FLOOD_MIN_DEPTH_M), Gaps::Means(0.0)),
        _ => (Lowest::Everything, Gaps::Unknown),
    };

    measured_bands(&grid, raster.layer(), boundary, 0.0, lowest, gaps, measured)
}

/// Slope bands from `ground`, the terrain as `contours::prepared` leaves it.
/// Its figures inside `boundary` go into `measured`, with the slope raster
/// cut to the boundary's box: 5 m cells, like the tree heights.
pub fn slope_bands(
    ground: &Grid,
    boundary: &Polygon<f64>,
    measured: &mut Measured,
) -> Vec<Feature> {
    let slope = slope_percent(ground);
    measured_bands(
        &slope,
        Layer::Slope,
        boundary,
        0.0,
        Lowest::Everything,
        Gaps::Unknown,
        measured,
    )
}

/// The bands of `grid`, the raster of `layer`. Its figures inside `boundary`
/// go into `measured`, and when they found a value, the raster cut to the
/// boundary's box widened by `reach_m`, so a sheet area can be measured again.
pub(super) fn measured_bands(
    grid: &Grid,
    layer: Layer,
    boundary: &Polygon<f64>,
    reach_m: f64,
    lowest: Lowest,
    gaps: Gaps,
    measured: &mut Measured,
) -> Vec<Feature> {
    let stats = raster::stats_within(grid, boundary);
    measured.set(layer, stats);
    // Without a value over the whole site there is none in a part of it either, so nothing is kept.
    if let (Some(_), Some(bbox)) = (stats, boundary.bounding_rect()) {
        let reach = Coord {
            x: reach_m,
            y: reach_m,
        };
        let kept = Rect::new(bbox.min() - reach, bbox.max() + reach);
        measured.grids.insert(layer, grid.cropped(kept));
    }

    bands::banded(grid, layer, &breaks_for(layer), lowest, gaps)
}

/// `measured` with its kept rasters moved into the scene's frame, which is
/// RD New minus `origin`, to the centimetre like the boundary they are measured in.
pub fn localised(mut measured: Measured, origin: (f64, f64)) -> Measured {
    for grid in measured.grids.values_mut() {
        let corner = to_local_centimetres(
            Coord {
                x: grid.west,
                y: grid.north,
            },
            origin,
        );
        grid.west = corner.x;
        grid.north = corner.y;
    }

    measured
}

/// The class breaks of the built-in diagram, or diagram variant, that classifies `layer`.
pub(super) fn breaks_for(layer: Layer) -> Vec<f64> {
    let classify = catalog::builtin_diagrams()
        .iter()
        .flat_map(|diagram| diagram.drawings())
        .filter_map(|(_, drawing)| drawing.classify)
        .find(|classify| classify.layer == layer);
    match classify.map(|classify| classify.scheme) {
        Some(Scheme::Sequential { breaks, .. }) if !breaks.is_empty() => breaks,
        // A broken asset, caught by the catalog tests long before a user sees it.
        _ => panic!("no built-in diagram classifies {layer:?} in sequential bands"),
    }
}
