//! The environmental rasters of a site, from tiles to bands: noise, air
//! quality, flood depth and slope.
//!
//! Each raster is measured inside the site before it becomes bands, because
//! the bands keep only which class a place is in. The band edges are the
//! breaks of the diagram that draws the layer, so the map and its legend can
//! never disagree.

use geo::{Polygon, Rect};

use crate::catalog;
use crate::diagram::Scheme;
use crate::environment::Measured;
use crate::scene::{Feature, Layer};
use crate::sources::bands::{self, Gaps, Lowest};
use crate::sources::grid::{tiles_covering, Tile};
use crate::sources::raster::{self, Grid, Stats};
use crate::sources::slope::slope_percent;
use crate::sources::wcs::Coverage;
use crate::sources::{liwo, rivm};

/// Rasters reach this far past the site, so the blur and the band edges
/// along the boundary have real cells on both sides.
const MARGIN_M: f64 = 30.0;
/// Water shallower than this is a wet street, not a flood worth a band.
const FLOOD_MIN_DEPTH_M: f64 = 0.01;

/// Every fetched raster, in one place.
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
    pub fn coverage(self) -> Coverage {
        match self {
            Raster::RoadNoise => rivm::ROAD_NOISE,
            Raster::AllNoise => rivm::ALL_NOISE,
            Raster::No2 => rivm::NO2,
            Raster::Pm10 => rivm::PM10,
            Raster::Pm25 => rivm::PM25,
            Raster::Flood => liwo::FLOOD_DEPTH,
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

/// The bands of one raster from its tiles, and its figures inside `boundary`.
pub fn raster_bands(
    raster: Raster,
    tiles: &[Grid],
    boundary: &Polygon<f64>,
) -> (Vec<Feature>, Option<Stats>) {
    let Some(grid) = raster::join(tiles) else {
        return (Vec::new(), None);
    };
    let stats = raster::stats_within(&grid, boundary);
    let (lowest, gaps) = match raster {
        // A cell LIWO leaves empty stays dry in every scenario.
        Raster::Flood => (Lowest::From(FLOOD_MIN_DEPTH_M), Gaps::Means(0.0)),
        _ => (Lowest::Everything, Gaps::Unknown),
    };
    let features = bands::banded(
        &grid,
        raster.layer(),
        &breaks_for(raster.layer()),
        lowest,
        gaps,
    );

    (features, stats)
}

/// Slope bands from the terrain model, and the slope figures inside `boundary`.
pub fn slope_bands(terrain: &Grid, boundary: &Polygon<f64>) -> (Vec<Feature>, Option<Stats>) {
    let slope = slope_percent(terrain);
    let stats = raster::stats_within(&slope, boundary);
    let features = bands::banded(
        &slope,
        Layer::Slope,
        &breaks_for(Layer::Slope),
        Lowest::Everything,
        Gaps::Unknown,
    );

    (features, stats)
}

/// Keeps a raster's figures in the field the summary reads them from.
pub fn record(measured: &mut Measured, raster: Raster, stats: Option<Stats>) {
    match raster {
        Raster::RoadNoise => measured.road_noise_db = stats,
        Raster::AllNoise => measured.all_noise_db = stats,
        Raster::No2 => measured.no2_ug_m3 = stats,
        Raster::Pm10 => measured.pm10_ug_m3 = stats,
        Raster::Pm25 => measured.pm25_ug_m3 = stats,
        Raster::Flood => measured.flood_depth_m = stats,
    }
}

/// The class breaks of the built-in diagram, or diagram variant, that classifies `layer`.
fn breaks_for(layer: Layer) -> Vec<f64> {
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
