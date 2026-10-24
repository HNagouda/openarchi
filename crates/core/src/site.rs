//! Turns a drawn area into a scene: fetch every source, clip, clean, join.
//!
//! Each source runs on its own thread and reports back over a channel, so the
//! progress callback is only ever called from the caller's thread. A failed
//! source becomes a warning; the scene is only an error when nothing arrived.

use std::sync::mpsc;
use std::thread;
use std::time::SystemTime;

use geo::{
    Area as _, BoundingRect, Centroid, Coord, Geometry, LineString, MapCoords, Polygon, Rect,
};

use crate::area::{Area, LonLat, MAX_AREA_M2};
use crate::crs::wgs84_to_rd;
use crate::scene::{Credit, Feature, Layer, Scene};
use crate::sources::ahn::{self, Elevation};
use crate::sources::clip::{self, Clipped};
use crate::sources::{bag, bgt, brk, contours, heights};
use crate::{Error, Result};

const RD_NEW: &str = "EPSG:28992";
/// Enough segments that a 1 km circle deviates less than 30 cm from a true circle.
const CIRCLE_SEGMENTS: usize = 128;
/// A generous box around the Netherlands, in degrees. Exact borders are left to
/// the data: outside the country PDOK simply returns nothing.
const NETHERLANDS_WEST: f64 = 3.2;
const NETHERLANDS_EAST: f64 = 7.3;
const NETHERLANDS_SOUTH: f64 = 50.7;
const NETHERLANDS_NORTH: f64 = 53.7;

/// One step of a fetch, for a progress bar: "Buildings", 2 of 7.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Progress {
    pub step: String,
    pub done: usize,
    pub total: usize,
}

/// Every data source of a scene, in one place.
#[derive(Debug, Clone, Copy)]
enum Source {
    Bgt,
    Bag,
    Brk,
    Ahn,
}

const SOURCES: [Source; 4] = [Source::Bgt, Source::Bag, Source::Brk, Source::Ahn];

/// What a source hands back: map features, or the elevation that heights and
/// contours are computed from once the buildings are in.
enum Fetched {
    Features(Vec<Feature>),
    Elevation(Elevation),
}

impl Source {
    fn step(self) -> &'static str {
        match self {
            Source::Bgt => "Roads, water and green",
            Source::Bag => "Buildings",
            Source::Brk => "Parcels",
            Source::Ahn => "Elevation",
        }
    }

    fn credit(self) -> Credit {
        match self {
            Source::Bgt => bgt::credit(),
            Source::Bag => bag::credit(),
            Source::Brk => brk::credit(),
            Source::Ahn => ahn::credit(),
        }
    }

    fn fetch(self, bbox: Rect<f64>) -> Result<Fetched> {
        match self {
            Source::Bgt => bgt::fetch(bbox).map(Fetched::Features),
            Source::Bag => bag::fetch(bbox).map(Fetched::Features),
            Source::Brk => brk::fetch(bbox).map(Fetched::Features),
            Source::Ahn => ahn::fetch(bbox).map(Fetched::Elevation),
        }
    }
}

/// Fetches and assembles everything inside `area`. Fails only when the area is
/// invalid or every source failed; a single failed source becomes a warning.
pub fn build_scene(area: &Area, on_progress: &dyn Fn(Progress)) -> Result<Scene> {
    let boundary = boundary_in_rd(area)?;
    let origin = site_centre(area, &boundary);
    let bbox = boundary
        .bounding_rect()
        .ok_or_else(|| Error::Invalid("The drawn area has no points.".to_string()))?;

    // Every source, then heights and contours, then clipping.
    let total = SOURCES.len() + 2;
    let report = |step: &str, done: usize| {
        on_progress(Progress {
            step: step.to_string(),
            done,
            total,
        });
    };
    report("Starting", 0);

    let mut features = Vec::new();
    let mut elevation = None;
    let mut credits = Vec::new();
    let mut warnings = Vec::new();
    for (source, result) in fetch_all(bbox, &report) {
        match result {
            Ok(Fetched::Features(fetched)) => features.extend(fetched),
            Ok(Fetched::Elevation(fetched)) => elevation = Some(fetched),
            Err(error) => {
                warnings.push(format!("{} could not be fetched: {error}", source.step()));
                continue;
            }
        }
        credits.push(source.credit());
    }

    if credits.is_empty() {
        return Err(Error::Network {
            service: "PDOK".to_string(),
            detail: warnings.join("; "),
        });
    }

    if let Some(elevation) = &elevation {
        add_building_heights(&mut features, elevation);
        features.extend(contours::contour_lines(&elevation.terrain));
    }
    report("Heights and contours", SOURCES.len() + 1);

    let (features, unclippable) = clip_and_localise(features, &boundary, origin);
    if unclippable > 0 {
        warnings.push(format!(
            "{unclippable} features had broken shapes and were left out."
        ));
    }
    report("Clipping", total);

    Ok(Scene {
        area: area.clone(),
        crs: RD_NEW.to_string(),
        origin,
        boundary: boundary.map_coords(|coord| to_local_centimetres(coord, origin)),
        features,
        credits,
        fetched_at: humantime::format_rfc3339_seconds(SystemTime::now()).to_string(),
        warnings,
    })
}

/// Every source's result, reporting each one as it arrives.
fn fetch_all(bbox: Rect<f64>, report: &dyn Fn(&str, usize)) -> Vec<(Source, Result<Fetched>)> {
    let (sender, receiver) = mpsc::channel();
    thread::scope(|scope| {
        for source in SOURCES {
            let sender = sender.clone();
            scope.spawn(move || {
                // The receiver outlives every sender, so this send cannot fail.
                let _ = sender.send((source, source.fetch(bbox)));
            });
        }
        drop(sender);

        // Runs on the caller's thread while the sources are still fetching.
        let mut results = Vec::new();
        for (source, result) in receiver.iter() {
            results.push((source, result));
            report(source.step(), results.len());
        }
        results
    })
}

// Heights use the whole footprint, before clipping, so a building cut by the
// boundary keeps the height of the whole building.
fn add_building_heights(features: &mut [Feature], elevation: &Elevation) {
    for feature in features
        .iter_mut()
        .filter(|feature| feature.layer == Layer::Building)
    {
        if let Geometry::Polygon(footprint) = &feature.geometry {
            let (height_m, ground_m) = heights::building_height(footprint, elevation);
            feature.attributes.height_m = height_m;
            feature.attributes.ground_m = ground_m;
        }
    }
}

/// Features cut to `boundary` and moved into the local frame, plus how many
/// had shapes too broken to cut.
fn clip_and_localise(
    features: Vec<Feature>,
    boundary: &Polygon<f64>,
    origin: (f64, f64),
) -> (Vec<Feature>, usize) {
    let mut kept = Vec::with_capacity(features.len());
    let mut unclippable = 0;
    for feature in features {
        match clip::clip(boundary, feature.geometry) {
            Clipped::Inside(geometry) => kept.push(Feature {
                geometry: geometry.map_coords(|coord| to_local_centimetres(coord, origin)),
                ..feature
            }),
            Clipped::Outside => {}
            Clipped::Broken => unclippable += 1,
        }
    }

    (kept, unclippable)
}

// Sources are surveyed to the centimetre; more digits only make project files bigger.
fn to_local_centimetres(coord: Coord<f64>, origin: (f64, f64)) -> Coord<f64> {
    Coord {
        x: ((coord.x - origin.0) * 100.0).round() / 100.0,
        y: ((coord.y - origin.1) * 100.0).round() / 100.0,
    }
}

/// The drawn area in RD New, after checking it is something we can fetch.
fn boundary_in_rd(area: &Area) -> Result<Polygon<f64>> {
    let boundary = match area {
        Area::Circle { center, radius_m } => {
            if !radius_m.is_finite() || *radius_m <= 0.0 {
                return Err(Error::Invalid(
                    "The circle needs a radius above 0 m.".to_string(),
                ));
            }
            check_in_netherlands(&[*center])?;
            circle(wgs84_to_rd(*center), *radius_m)
        }
        Area::Rectangle {
            south_west,
            north_east,
        } => {
            if south_west.lon >= north_east.lon || south_west.lat >= north_east.lat {
                return Err(Error::Invalid(
                    "The rectangle's south-west corner must lie south-west of its north-east corner.".to_string(),
                ));
            }
            let south_east = LonLat {
                lon: north_east.lon,
                lat: south_west.lat,
            };
            let north_west = LonLat {
                lon: south_west.lon,
                lat: north_east.lat,
            };
            ring_in_rd(&[*south_west, south_east, *north_east, north_west])?
        }
        Area::Polygon { points } => {
            if points.len() < 3 {
                return Err(Error::Invalid(
                    "A drawn area needs at least 3 points.".to_string(),
                ));
            }
            ring_in_rd(points)?
        }
    };

    let size = boundary.unsigned_area();
    if size <= 0.0 || !size.is_finite() {
        return Err(Error::Invalid("The drawn area has no surface.".to_string()));
    }
    if size > MAX_AREA_M2 {
        return Err(Error::Invalid(format!(
            "This area is {:.1} km². The largest site openarchi fetches is {:.0} km², so draw a smaller one.",
            size / 1e6,
            MAX_AREA_M2 / 1e6
        )));
    }

    Ok(boundary)
}

fn ring_in_rd(points: &[LonLat]) -> Result<Polygon<f64>> {
    check_in_netherlands(points)?;
    let ring: Vec<Coord<f64>> = points
        .iter()
        .map(|&point| wgs84_to_rd(point).into())
        .collect();
    // Polygon::new closes the ring itself.
    Ok(Polygon::new(LineString::new(ring), vec![]))
}

fn check_in_netherlands(points: &[LonLat]) -> Result<()> {
    let is_inside = |point: &LonLat| {
        (NETHERLANDS_WEST..=NETHERLANDS_EAST).contains(&point.lon)
            && (NETHERLANDS_SOUTH..=NETHERLANDS_NORTH).contains(&point.lat)
    };
    if points.iter().all(is_inside) {
        Ok(())
    } else {
        Err(Error::Invalid(
            "openarchi only has map data for the Netherlands, and this area lies outside it."
                .to_string(),
        ))
    }
}

// Measured straight in RD metres: RD's scale error is at most 1 in 10 000.
fn circle(centre: (f64, f64), radius_m: f64) -> Polygon<f64> {
    let ring: Vec<Coord<f64>> = (0..CIRCLE_SEGMENTS)
        .map(|index| {
            let angle = std::f64::consts::TAU * index as f64 / CIRCLE_SEGMENTS as f64;
            Coord {
                x: centre.0 + radius_m * angle.cos(),
                y: centre.1 + radius_m * angle.sin(),
            }
        })
        .collect();
    Polygon::new(LineString::new(ring), vec![])
}

/// The point features are measured from: the circle's centre, otherwise the
/// middle of the drawn shape.
fn site_centre(area: &Area, boundary: &Polygon<f64>) -> (f64, f64) {
    match area {
        Area::Circle { center, .. } => wgs84_to_rd(*center),
        _ => boundary
            .centroid()
            .map(|point| point.x_y())
            .unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WESTERMARKT: LonLat = LonLat {
        lon: 4.8840,
        lat: 52.3745,
    };

    #[test]
    fn a_circle_of_300_m_covers_about_its_true_area() {
        let boundary = boundary_in_rd(&Area::Circle {
            center: WESTERMARKT,
            radius_m: 300.0,
        })
        .unwrap();
        let true_area = std::f64::consts::PI * 300.0 * 300.0;
        assert!((boundary.unsigned_area() - true_area).abs() / true_area < 0.001);
    }

    #[test]
    fn a_zero_radius_is_refused_with_a_readable_message() {
        let error = boundary_in_rd(&Area::Circle {
            center: WESTERMARKT,
            radius_m: 0.0,
        })
        .unwrap_err();
        assert!(error.to_string().contains("radius above 0 m"));
    }

    #[test]
    fn an_area_outside_the_netherlands_is_refused() {
        let paris = LonLat {
            lon: 2.35,
            lat: 48.85,
        };
        let error = boundary_in_rd(&Area::Circle {
            center: paris,
            radius_m: 300.0,
        })
        .unwrap_err();
        assert!(error.to_string().contains("Netherlands"));
    }

    #[test]
    fn an_area_above_the_maximum_is_refused() {
        let error = boundary_in_rd(&Area::Circle {
            center: WESTERMARKT,
            radius_m: 2000.0,
        })
        .unwrap_err();
        assert!(error.to_string().contains("draw a smaller one"));
    }

    #[test]
    fn a_polygon_with_two_points_is_refused() {
        let points = vec![
            WESTERMARKT,
            LonLat {
                lon: 4.89,
                lat: 52.38,
            },
        ];
        assert!(matches!(
            boundary_in_rd(&Area::Polygon { points }),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    fn a_swapped_rectangle_is_refused() {
        let area = Area::Rectangle {
            south_west: LonLat {
                lon: 4.89,
                lat: 52.38,
            },
            north_east: WESTERMARKT,
        };
        assert!(matches!(boundary_in_rd(&area), Err(Error::Invalid(_))));
    }

    #[test]
    fn the_circle_centre_is_the_origin() {
        let area = Area::Circle {
            center: WESTERMARKT,
            radius_m: 300.0,
        };
        let boundary = boundary_in_rd(&area).unwrap();
        assert_eq!(site_centre(&area, &boundary), wgs84_to_rd(WESTERMARKT));
    }
}
