//! The drawn area as a polygon in RD New, checked before anything is fetched.

use geo::{Area as _, Centroid, Coord, LineString, Polygon};

use crate::area::{Area, LonLat, MAX_AREA_M2};
use crate::crs::wgs84_to_rd;
use crate::{Error, Result};

/// Enough segments that a 1 km circle deviates less than 30 cm from a true circle.
const CIRCLE_SEGMENTS: usize = 128;
/// A generous box around the Netherlands, in degrees. Exact borders are left to
/// the data: outside the country PDOK simply returns nothing.
const NETHERLANDS_WEST: f64 = 3.2;
const NETHERLANDS_EAST: f64 = 7.3;
const NETHERLANDS_SOUTH: f64 = 50.7;
const NETHERLANDS_NORTH: f64 = 53.7;

/// The drawn area in RD New, after checking it is something we can fetch.
pub fn boundary_in_rd(area: &Area) -> Result<Polygon<f64>> {
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
pub fn site_centre(area: &Area, boundary: &Polygon<f64>) -> (f64, f64) {
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
