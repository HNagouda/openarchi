//! City blocks: the land the street network leaves between its streets.
//!
//! A block is what is left of the site once streets, paths, rail and water
//! are taken out, the "island" of Spacematrix (Berghauser Pont and Haupt,
//! "Spacematrix: Space, Density and Urban Form", 2010). Its coverage (GSI) is
//! building footprint over block area, and its floor space index (FSI) is
//! gross floor area over block area.
//!
//! Blocks are cut from street surfaces, not from street centre lines. A road
//! that stops at the site edge still cuts the land beside it, and the site
//! edge closes every block, so a circle and a drawn polygon work alike.

use geo::{
    Area, BoundingRect, Contains, Euclidean, Geometry, InteriorPoint, Length, LineString, Point,
    Polygon, Rect,
};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;

use crate::floors::gross_floor_area_m2;
use crate::scene::{Attributes, Feature, Layer, Scene};

/// The surfaces that separate blocks, in one place. Verges and banks belong to
/// the street or the canal they line. Paving and yards stay in the block,
/// because most of them are courtyards and front gardens.
const STREET_LAYERS: [Layer; 9] = [
    Layer::Road,
    Layer::Footpath,
    Layer::Cycleway,
    Layer::Parking,
    Layer::RailArea,
    Layer::Verge,
    Layer::Bridge,
    Layer::Water,
    Layer::WaterBank,
];

/// Smaller pieces are traffic islands and leftovers between surfaces, not blocks.
const MIN_BLOCK_AREA_M2: f64 = 200.0;
/// Area over perimeter, about half the width of a long strip. A thinner piece
/// is a sliver between street surfaces, such as a planted strip.
const MIN_BLOCK_DEPTH_M: f64 = 3.0;

/// Every block in the site, each with its coverage and floor space index.
pub fn blocks(scene: &Scene) -> Vec<Feature> {
    let land = land_between_streets(scene);
    let buildings = buildings(scene);

    land.into_iter()
        .filter(is_block)
        .map(|polygon| block(polygon, &buildings))
        .collect()
}

/// The site less every street surface, in one overlay. Surfaces may overlap,
/// such as a road on a bridge over water, so they are counted nonzero, with
/// every outline turned counterclockwise and every hole clockwise.
fn land_between_streets(scene: &Scene) -> Vec<Polygon<f64>> {
    let site = contours(&scene.boundary);
    let streets: Vec<Vec<[f64; 2]>> = scene
        .features
        .iter()
        .filter(|feature| STREET_LAYERS.contains(&feature.layer))
        .flat_map(|feature| polygons(&feature.geometry))
        .flat_map(contours)
        .collect();
    let shapes = site.overlay(&streets, OverlayRule::Difference, FillRule::NonZero);

    shapes.into_iter().map(polygon_of).collect()
}

/// A building reduced to what the block sums: a point inside it and its areas.
struct Building {
    inside: Point<f64>,
    footprint_m2: f64,
    gross_floor_area_m2: Option<f64>,
}

fn buildings(scene: &Scene) -> Vec<Building> {
    scene
        .features
        .iter()
        .filter(|feature| feature.layer == Layer::Building)
        .filter_map(|feature| {
            Some(Building {
                inside: feature.geometry.interior_point()?,
                footprint_m2: feature.geometry.unsigned_area(),
                gross_floor_area_m2: gross_floor_area_m2(feature),
            })
        })
        .collect()
}

fn polygons(geometry: &Geometry<f64>) -> Vec<&Polygon<f64>> {
    match geometry {
        Geometry::Polygon(polygon) => vec![polygon],
        Geometry::MultiPolygon(multi) => multi.0.iter().collect(),
        _ => Vec::new(),
    }
}

/// A polygon's rings as overlay contours: open, the outline counterclockwise
/// and the holes clockwise.
fn contours(polygon: &Polygon<f64>) -> Vec<Vec<[f64; 2]>> {
    let rings = std::iter::once(polygon.exterior()).chain(polygon.interiors());
    rings
        .enumerate()
        .map(|(index, ring)| {
            let is_hole = index > 0;
            // Rings repeat their first point at the end; contours do not.
            let open = &ring.0[..ring.0.len().saturating_sub(1)];
            let mut points: Vec<[f64; 2]> = open.iter().map(|c| [c.x, c.y]).collect();
            if (signed_area(&points) > 0.0) == is_hole {
                points.reverse();
            }
            points
        })
        .collect()
}

/// An overlay shape, its outline first and then its holes, as a polygon.
fn polygon_of(shape: Vec<Vec<[f64; 2]>>) -> Polygon<f64> {
    let mut rings = shape.into_iter().map(|contour| {
        LineString::from(contour.into_iter().map(|[x, y]| (x, y)).collect::<Vec<_>>())
    });
    let exterior = rings.next().unwrap_or_else(|| LineString::new(Vec::new()));
    // Polygon::new closes every ring.
    Polygon::new(exterior, rings.collect())
}

/// Shoelace area: positive for a counterclockwise contour.
fn signed_area(points: &[[f64; 2]]) -> f64 {
    let mut sum = 0.0;
    for index in 0..points.len() {
        let [x1, y1] = points[index];
        let [x2, y2] = points[(index + 1) % points.len()];
        sum += x1 * y2 - x2 * y1;
    }

    sum / 2.0
}

fn is_block(polygon: &Polygon<f64>) -> bool {
    let area = polygon.unsigned_area();
    let perimeter = polygon.exterior().length::<Euclidean>();
    area >= MIN_BLOCK_AREA_M2 && area / perimeter >= MIN_BLOCK_DEPTH_M
}

/// Buildings count towards the block their inside point falls in, whole, so
/// a building is never split between two blocks by a sliver of pavement.
fn block(polygon: Polygon<f64>, buildings: &[Building]) -> Feature {
    let area = polygon.unsigned_area();
    let bounds = polygon.bounding_rect();
    let mut footprint_m2 = 0.0;
    let mut floor_area_m2 = 0.0;
    for building in buildings {
        let is_near = bounds.is_some_and(|rect| contains_point(&rect, building.inside));
        if !is_near || !polygon.contains(&building.inside) {
            continue;
        }
        footprint_m2 += building.footprint_m2;
        floor_area_m2 += building.gross_floor_area_m2.unwrap_or(0.0);
    }

    Feature {
        layer: Layer::Block,
        geometry: Geometry::Polygon(polygon),
        attributes: Attributes {
            coverage: Some((footprint_m2 / area).min(1.0)),
            floor_space_index: Some(floor_area_m2 / area),
            ..Attributes::default()
        },
    }
}

fn contains_point(rect: &Rect<f64>, point: Point<f64>) -> bool {
    let (min, max) = (rect.min(), rect.max());
    point.x() >= min.x && point.x() <= max.x && point.y() >= min.y && point.y() <= max.y
}

#[cfg(test)]
mod tests {
    use geo::polygon;

    use super::*;
    use crate::area::{Area as SiteArea, LonLat};

    fn feature(layer: Layer, geometry: Polygon<f64>, height_m: Option<f64>) -> Feature {
        Feature {
            layer,
            geometry: Geometry::Polygon(geometry),
            attributes: Attributes {
                height_m,
                ..Attributes::default()
            },
        }
    }

    fn square(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Polygon<f64> {
        polygon![
            (x: min_x, y: min_y),
            (x: max_x, y: min_y),
            (x: max_x, y: max_y),
            (x: min_x, y: max_y),
        ]
    }

    /// A 100 m square site with a north-south road through the middle and an
    /// east-west road that stops halfway, at the first road.
    fn site(features: Vec<Feature>) -> Scene {
        Scene {
            area: SiteArea::Circle {
                center: LonLat {
                    lon: 4.9,
                    lat: 52.4,
                },
                radius_m: 50.0,
            },
            crs: "EPSG:28992".to_string(),
            origin: (0.0, 0.0),
            boundary: square(-50.0, -50.0, 50.0, 50.0),
            features,
            credits: Vec::new(),
            fetched_at: String::new(),
            warnings: Vec::new(),
            measured: Default::default(),
            walk: None,
            editions: Default::default(),
            context: Vec::new(),
            terrain: None,
            water_bed: None,
            water_levels: Vec::new(),
            solar: None,
        }
    }

    fn roads() -> Vec<Feature> {
        vec![
            feature(Layer::Road, square(-5.0, -50.0, 5.0, 50.0), None),
            // Overlaps the first road, as a junction does in survey data.
            feature(Layer::Road, square(-50.0, -4.0, 2.0, 4.0), None),
        ]
    }

    #[test]
    fn roads_cut_the_site_into_blocks_and_a_dead_end_road_still_cuts() {
        let blocks = blocks(&site(roads()));

        // East of the road one block, west of it two, split by the side road.
        assert_eq!(blocks.len(), 3);
        let mut areas: Vec<f64> = blocks.iter().map(|b| b.geometry.unsigned_area()).collect();
        areas.sort_by(f64::total_cmp);
        assert!((areas[0] - 45.0 * 46.0).abs() < 1e-6);
        assert!((areas[1] - 45.0 * 46.0).abs() < 1e-6);
        assert!((areas[2] - 45.0 * 100.0).abs() < 1e-6);
    }

    #[test]
    fn a_traffic_island_is_not_a_block() {
        let mut features = roads();
        // A 10 by 10 m island between two carriageways east of the junction.
        features.push(feature(Layer::Road, square(5.0, -10.0, 50.0, -5.0), None));
        features.push(feature(Layer::Road, square(5.0, 5.0, 50.0, 10.0), None));
        features.push(feature(Layer::Road, square(15.0, -5.0, 50.0, 5.0), None));
        let blocks = blocks(&site(features));

        assert!(blocks
            .iter()
            .all(|block| block.geometry.unsigned_area() >= MIN_BLOCK_AREA_M2));
    }

    #[test]
    fn a_block_knows_its_coverage_and_floor_space_index() {
        let mut features = roads();
        // A 20 by 20 m building of four floors in the east block of 45 by 100 m.
        features.push(feature(
            Layer::Building,
            square(20.0, 20.0, 40.0, 40.0),
            Some(12.0),
        ));
        let blocks = blocks(&site(features));
        let east = blocks
            .iter()
            .find(|block| block.geometry.unsigned_area() > 4000.0)
            .unwrap();

        let block_area = 45.0 * 100.0;
        let coverage = east.attributes.coverage.unwrap();
        let floor_space_index = east.attributes.floor_space_index.unwrap();
        assert!((coverage - 400.0 / block_area).abs() < 1e-9);
        assert!((floor_space_index - 1600.0 / block_area).abs() < 1e-9);
    }

    #[test]
    fn a_site_without_streets_is_one_block() {
        assert_eq!(blocks(&site(Vec::new())).len(), 1);
    }
}
