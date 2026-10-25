//! Small scenes the grid layers' tests share.

use geo::{polygon, Geometry, Polygon};

use super::Raster;
use crate::area::{Area as SiteArea, LonLat};
use crate::scene::{Attributes, Feature, Layer, Scene};

pub(crate) fn square(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Polygon<f64> {
    polygon![
        (x: min_x, y: min_y),
        (x: max_x, y: min_y),
        (x: max_x, y: max_y),
        (x: min_x, y: max_y),
    ]
}

pub(crate) fn building(footprint: Polygon<f64>, height_m: f64) -> Feature {
    Feature {
        layer: Layer::Building,
        geometry: Geometry::Polygon(footprint),
        attributes: Attributes {
            height_m: Some(height_m),
            ..Attributes::default()
        },
    }
}

/// A 200 m square site in Amsterdam, on flat ground.
pub(crate) fn site(features: Vec<Feature>, context: Vec<Feature>) -> Scene {
    Scene {
        area: SiteArea::Circle {
            center: LonLat {
                lon: 4.9041,
                lat: 52.3676,
            },
            radius_m: 100.0,
        },
        crs: "EPSG:28992".to_string(),
        origin: (0.0, 0.0),
        boundary: square(-100.0, -100.0, 100.0, 100.0),
        features,
        credits: Vec::new(),
        fetched_at: String::new(),
        warnings: Vec::new(),
        measured: Default::default(),
        walk: None,
        editions: Default::default(),
        context,
        terrain: None,
        water_bed: None,
        water_levels: Vec::new(),
        solar: None,
        wind: None,
    }
}

/// The value of the cell under a ground point.
pub(crate) fn value_at(raster: &Raster, values: &[Option<f64>], x: f64, y: f64) -> Option<f64> {
    let column = ((x - raster.min.0) / raster.cell_m) as usize;
    let row = ((y - raster.min.1) / raster.cell_m) as usize;
    values[row * raster.columns + column]
}
