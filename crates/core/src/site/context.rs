//! The neighbourhood around a site: buildings, roads, water and green
//! outside the boundary, which sheets draw muted so the site reads in its
//! surroundings.
//!
//! Only what makes a recognisable figure-ground is fetched, from services the
//! site already uses, in the same tiles and through the same cache, so the
//! tiles along the site's edge are never fetched twice. Building heights come
//! from AHN in 2 m cells rather than 0.5 m, because a muted neighbour needs a
//! height, not a roof shape.

use std::collections::{HashMap, HashSet};

use geo::{
    BoundingRect, Contains, Coord, Geometry, Intersects, MapCoords, Polygon, Rect, Simplify,
};

use super::download::Dataset;
use super::{to_local_centimetres, unique};
use crate::scene::{Attributes, Feature, Layer};
use crate::sources::ahn::Elevation;
use crate::sources::clip::{self, Clipped};
use crate::sources::grid::{tiles_covering, tiles_touching, Tile};
use crate::sources::pdok::Record;
use crate::sources::raster::{self, Grid};
use crate::sources::{bag, bgt, heights};

/// How far the neighbourhood reaches past the site, as a share of the site's
/// own reach from its centre. A tilted view at the default 35 degrees sees
/// about 1.7 times the site's depth, so three quarters more fills it.
const RING_SHARE: f64 = 0.75;
/// A3 landscape at 1:2500 shows about 730 m across, so a small site needs at
/// least this much around it to fill the drawing.
const MIN_RING_M: f64 = 250.0;
/// The cap, so a large site does not pull in a whole town: at 500 m a 1 km
/// site already fetches about four times its own area.
const MAX_RING_M: f64 = 500.0;
/// The BGT collections of the neighbourhood: roads and paving, water, green.
pub const COLLECTIONS: [&str; 3] = ["wegdeel", "waterdeel", "begroeidterreindeel"];
/// Outlines are simplified to this, in metres. A quarter metre is 0.1 mm at
/// 1:2500, finer than a muted line shows, and it halves the stored size.
const SIMPLIFY_M: f64 = 0.25;

/// The square the neighbourhood is fetched for, centred on the site, in RD New.
pub fn extent(boundary: &Polygon<f64>, origin: (f64, f64)) -> Rect<f64> {
    let site_reach = boundary
        .exterior()
        .points()
        .map(|point| ((point.x() - origin.0).powi(2) + (point.y() - origin.1).powi(2)).sqrt())
        .fold(0.0, f64::max);
    let ring = (site_reach * RING_SHARE).clamp(MIN_RING_M, MAX_RING_M);
    let half = site_reach + ring;

    Rect::new(
        Coord {
            x: origin.0 - half,
            y: origin.1 - half,
        },
        Coord {
            x: origin.0 + half,
            y: origin.1 + half,
        },
    )
}

/// The tiles of the neighbourhood. Map data skips the tiles the site fetches
/// anyway, since `features` reads those too; heights cover the whole square.
pub fn jobs(boundary: &Polygon<f64>, extent: Rect<f64>) -> Vec<(Dataset, Tile)> {
    let site_tiles: HashSet<Tile> = tiles_touching(boundary).into_iter().collect();
    let mut jobs = Vec::new();
    for tile in tiles_covering(extent) {
        if !site_tiles.contains(&tile) {
            for collection in COLLECTIONS {
                jobs.push((Dataset::ContextBgt(collection), tile));
            }
            jobs.push((Dataset::ContextBag, tile));
        }
        jobs.push((Dataset::ContextSurface, tile));
        jobs.push((Dataset::ContextTerrain, tile));
    }

    jobs
}

/// Every neighbourhood feature in RD New, from its own tiles and the site's
/// tiles of the same data, with building heights. Not cut yet.
pub fn features(
    records: &HashMap<Dataset, Vec<Record>>,
    rasters: &mut HashMap<Dataset, Vec<Grid>>,
) -> Vec<Feature> {
    let both = |own: Dataset, site: Dataset| -> Vec<Record> {
        let mut joined = records.get(&own).cloned().unwrap_or_default();
        joined.extend(records.get(&site).cloned().unwrap_or_default());
        unique(joined)
    };

    let mut features = Vec::new();
    for collection in COLLECTIONS {
        let joined = both(Dataset::ContextBgt(collection), Dataset::Bgt(collection));
        features.extend(bgt::to_features(collection, joined));
    }
    let mut buildings = bag::to_features(both(Dataset::ContextBag, Dataset::Bag), None);
    let surface = raster::join(&rasters.remove(&Dataset::ContextSurface).unwrap_or_default());
    let terrain = raster::join(&rasters.remove(&Dataset::ContextTerrain).unwrap_or_default());
    if let (Some(surface), Some(terrain)) = (surface, terrain) {
        add_heights(&mut buildings, &Elevation { surface, terrain });
    }
    features.extend(buildings);

    features.into_iter().map(stripped).collect()
}

/// `features` simplified, cut to the square minus the site, and moved into the
/// scene's frame. Also returns how many had shapes too broken to cut.
pub fn cut(
    features: Vec<Feature>,
    boundary: &Polygon<f64>,
    extent: Rect<f64>,
    origin: (f64, f64),
) -> (Vec<Feature>, usize) {
    let ring = Polygon::new(
        extent.to_polygon().exterior().clone(),
        vec![boundary.exterior().clone()],
    );
    let mut kept = Vec::with_capacity(features.len());
    let mut broken = 0;
    for feature in features {
        let geometry = simplified(feature.geometry);
        let Some(rect) = geometry.bounding_rect() else {
            continue;
        };
        // Most features lie wholly inside the ring and need no cutting.
        let cut = if extent.contains(&rect) && !boundary.intersects(&rect) {
            Clipped::Inside(geometry)
        } else {
            clip::clip(&ring, geometry)
        };
        match cut {
            Clipped::Inside(geometry) => kept.push(Feature {
                geometry: geometry.map_coords(|coord| to_local_centimetres(coord, origin)),
                ..feature
            }),
            Clipped::Outside => {}
            Clipped::Broken => broken += 1,
        }
    }

    (kept, broken)
}

fn add_heights(buildings: &mut [Feature], elevation: &Elevation) {
    for building in buildings.iter_mut() {
        if let Geometry::Polygon(footprint) = &building.geometry {
            building.attributes.height_m = heights::building_height(footprint, elevation).0;
        }
    }
}

/// Only the layer, the shape and a building's height: the neighbourhood is
/// drawn, never classified, so the rest would only make the project bigger.
fn stripped(feature: Feature) -> Feature {
    let height_m = match feature.layer {
        Layer::Building => feature.attributes.height_m,
        _ => None,
    };
    Feature {
        layer: feature.layer,
        geometry: feature.geometry,
        attributes: Attributes {
            height_m,
            ..Attributes::default()
        },
    }
}

fn simplified(geometry: Geometry<f64>) -> Geometry<f64> {
    match geometry {
        Geometry::Polygon(polygon) => Geometry::Polygon(polygon.simplify(&SIMPLIFY_M)),
        Geometry::MultiPolygon(polygons) => Geometry::MultiPolygon(polygons.simplify(&SIMPLIFY_M)),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::polygon;

    fn site() -> Polygon<f64> {
        polygon![(x: -50.0, y: -50.0), (x: 50.0, y: -50.0), (x: 50.0, y: 50.0), (x: -50.0, y: 50.0)]
    }

    fn building(geometry: Polygon<f64>) -> Feature {
        Feature {
            layer: Layer::Building,
            geometry: Geometry::Polygon(geometry),
            attributes: Attributes::default(),
        }
    }

    #[test]
    fn a_small_site_gets_the_smallest_ring() {
        let extent = extent(&site(), (0.0, 0.0));
        let site_reach = 50.0 * 2f64.sqrt();
        assert!((extent.max().x - (site_reach + MIN_RING_M)).abs() < 1e-9);
    }

    #[test]
    fn a_large_site_gets_the_capped_ring() {
        let large = polygon![(x: -2000.0, y: 0.0), (x: 2000.0, y: 0.0), (x: 0.0, y: 10.0)];
        let extent = extent(&large, (0.0, 0.0));
        assert!((extent.max().x - (2000.0 + MAX_RING_M)).abs() < 1e-9);
    }

    #[test]
    fn neighbourhood_tiles_skip_the_site_tiles_for_map_data_but_not_for_heights() {
        let jobs = jobs(&site(), extent(&site(), (0.0, 0.0)));
        let site_tiles = tiles_touching(&site());
        let has_map_data_on_site = jobs
            .iter()
            .any(|(dataset, tile)| *dataset == Dataset::ContextBag && site_tiles.contains(tile));
        let has_heights_on_site = jobs.iter().any(|(dataset, tile)| {
            *dataset == Dataset::ContextSurface && site_tiles.contains(tile)
        });
        assert!(!has_map_data_on_site);
        assert!(has_heights_on_site);
    }

    #[test]
    fn a_building_across_the_boundary_keeps_only_its_outside_part() {
        let across =
            polygon![(x: 40.0, y: 0.0), (x: 60.0, y: 0.0), (x: 60.0, y: 10.0), (x: 40.0, y: 10.0)];
        let extent = extent(&site(), (0.0, 0.0));
        let (kept, broken) = cut(vec![building(across)], &site(), extent, (0.0, 0.0));
        assert_eq!(broken, 0);
        let rect = kept[0].geometry.bounding_rect().unwrap();
        assert!((rect.min().x - 50.0).abs() < 1e-9);
        assert!((rect.max().x - 60.0).abs() < 1e-9);
    }

    #[test]
    fn a_building_inside_the_site_is_not_neighbourhood() {
        let inside = polygon![(x: 0.0, y: 0.0), (x: 10.0, y: 0.0), (x: 10.0, y: 10.0)];
        let extent = extent(&site(), (0.0, 0.0));
        let (kept, _) = cut(vec![building(inside)], &site(), extent, (0.0, 0.0));
        assert!(kept.is_empty());
    }

    #[test]
    fn only_a_building_keeps_its_height() {
        let mut road = building(site());
        road.layer = Layer::Road;
        road.attributes.height_m = Some(3.0);
        road.attributes.kind = Some("rijbaan lokale weg".to_string());
        let stripped_road = stripped(road);
        assert_eq!(stripped_road.attributes.height_m, None);
        assert_eq!(stripped_road.attributes.kind, None);
    }
}
