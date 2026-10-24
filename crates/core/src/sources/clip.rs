//! Cutting features to the drawn boundary.

use std::panic::{self, AssertUnwindSafe};

use geo::{
    BooleanOps, BoundingRect, Contains, Geometry, Intersects, MultiLineString, MultiPoint,
    MultiPolygon, Polygon,
};

/// What is left of a feature after cutting it to the boundary.
#[derive(Debug)]
pub enum Clipped {
    Inside(Geometry<f64>),
    Outside,
    /// The shape was too broken to cut, for example a ring that crosses itself.
    Broken,
}

/// `geometry` cut to `boundary`. Points stay points, lines stay lines and areas stay areas.
pub fn clip(boundary: &Polygon<f64>, geometry: Geometry<f64>) -> Clipped {
    // Cheap rejection first: most features of a bbox query near the corners of a
    // circle fall wholly outside it.
    let Some(rect) = geometry.bounding_rect() else {
        return Clipped::Outside;
    };
    if !boundary.intersects(&rect) {
        return Clipped::Outside;
    }

    // geo's boolean operations can panic on degenerate input. One odd source
    // polygon must not lose the whole site, so it is counted and left out.
    match panic::catch_unwind(AssertUnwindSafe(|| clip_unchecked(boundary, geometry))) {
        Ok(Some(inside)) => Clipped::Inside(inside),
        Ok(None) => Clipped::Outside,
        Err(_) => Clipped::Broken,
    }
}

fn clip_unchecked(boundary: &Polygon<f64>, geometry: Geometry<f64>) -> Option<Geometry<f64>> {
    match geometry {
        Geometry::Point(point) => boundary.contains(&point).then_some(Geometry::Point(point)),
        Geometry::MultiPoint(points) => {
            let inside: Vec<_> = points
                .into_iter()
                .filter(|point| boundary.contains(point))
                .collect();
            (!inside.is_empty()).then(|| Geometry::MultiPoint(MultiPoint::new(inside)))
        }
        Geometry::LineString(line) => clip_lines(boundary, MultiLineString::new(vec![line])),
        Geometry::MultiLineString(lines) => clip_lines(boundary, lines),
        Geometry::Polygon(polygon) => simplest_area(boundary.intersection(&polygon)),
        Geometry::MultiPolygon(polygons) => {
            simplest_area(MultiPolygon::new(vec![boundary.clone()]).intersection(&polygons))
        }
        // Rects, triangles and collections do not come from our sources.
        _ => None,
    }
}

fn clip_lines(boundary: &Polygon<f64>, lines: MultiLineString<f64>) -> Option<Geometry<f64>> {
    let mut inside = boundary.clip(&lines, false);
    match inside.0.len() {
        0 => None,
        1 => Some(Geometry::LineString(inside.0.remove(0))),
        _ => Some(Geometry::MultiLineString(inside)),
    }
}

// A single piece stays a plain polygon, so a clipped building is still one shape.
fn simplest_area(mut polygons: MultiPolygon<f64>) -> Option<Geometry<f64>> {
    match polygons.0.len() {
        0 => None,
        1 => Some(Geometry::Polygon(polygons.0.remove(0))),
        _ => Some(Geometry::MultiPolygon(polygons)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{line_string, point, polygon, Area};

    fn square_of_ten() -> Polygon<f64> {
        polygon![(x: 0.0, y: 0.0), (x: 10.0, y: 0.0), (x: 10.0, y: 10.0), (x: 0.0, y: 10.0)]
    }

    #[test]
    fn a_polygon_half_outside_keeps_only_the_inside_half() {
        let half_out =
            polygon![(x: 5.0, y: 0.0), (x: 15.0, y: 0.0), (x: 15.0, y: 10.0), (x: 5.0, y: 10.0)];
        let Clipped::Inside(Geometry::Polygon(clipped)) =
            clip(&square_of_ten(), Geometry::Polygon(half_out))
        else {
            panic!("expected a polygon")
        };
        assert!((clipped.unsigned_area() - 50.0).abs() < 1e-9);
    }

    #[test]
    fn a_polygon_wholly_outside_is_dropped() {
        let outside = polygon![(x: 20.0, y: 0.0), (x: 30.0, y: 0.0), (x: 30.0, y: 10.0)];
        assert!(matches!(
            clip(&square_of_ten(), Geometry::Polygon(outside)),
            Clipped::Outside
        ));
    }

    #[test]
    fn a_line_crossing_the_edge_is_cut_at_the_edge() {
        let crossing = line_string![(x: 5.0, y: 5.0), (x: 20.0, y: 5.0)];
        let Clipped::Inside(Geometry::LineString(clipped)) =
            clip(&square_of_ten(), Geometry::LineString(crossing))
        else {
            panic!("expected a line")
        };
        let end = clipped.0.last().unwrap();
        assert!((end.x - 10.0).abs() < 1e-9);
    }

    #[test]
    fn points_are_kept_only_inside() {
        assert!(matches!(
            clip(&square_of_ten(), Geometry::Point(point!(x: 5.0, y: 5.0))),
            Clipped::Inside(_)
        ));
        assert!(matches!(
            clip(&square_of_ten(), Geometry::Point(point!(x: 50.0, y: 5.0))),
            Clipped::Outside
        ));
    }
}
