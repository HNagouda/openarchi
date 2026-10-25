//! Scene metres to paper millimetres, and geometry to SVG path data.
//!
//! Coordinates are rounded to 0.01 mm, far below what a plotter or a printer
//! can show, and written as integers of hundredths. Path steps are relative and
//! computed from the rounded points, so rounding never accumulates along a ring.

use std::fmt::Write;

use geo::orient::{Direction, Orient};
use geo::{BoundingRect, Coord, Geometry, LineString, Polygon};

use super::layout::Rect;
use super::simplify;

/// Hundredths of a millimetre per millimetre.
const HUNDREDTHS: f64 = 100.0;

/// How the ground maps onto the paper: a fixed scale around a centre, and in
/// the tilted views a turn and a foreshortening of the ground.
///
/// The view frame is the ground turned by the rotation: `u` runs left to right
/// on the paper and `v` away from the viewer, so a larger `v` is further back.
#[derive(Debug, Clone, Copy)]
pub struct Projection {
    /// Paper millimetres per ground metre: 1000 / scale.
    pub mm_per_m: f64,
    /// The ground point that lands on `paper_centre`.
    pub ground_centre: (f64, f64),
    pub paper_centre: (f64, f64),
    /// Cosine and sine of the rotation; (1, 0) in plan.
    rotation: (f64, f64),
    /// How much the ground shortens up the paper: 1 in plan, sin(tilt) when tilted.
    pub squash: f64,
    /// How far a metre of height rises on the paper per metre of ground: 0 in plan, cos(tilt) when tilted.
    pub rise: f64,
}

impl Projection {
    /// Centres the boundary's bounding box in `zone` at `scale` (2500 for 1:2500).
    pub fn new(boundary: &Polygon<f64>, scale: u32, zone: &Rect) -> Projection {
        Projection {
            mm_per_m: 1000.0 / scale as f64,
            ground_centre: ground_centre(boundary),
            paper_centre: zone.centre(),
            rotation: (1.0, 0.0),
            squash: 1.0,
            rise: 0.0,
        }
    }

    /// A tilted view: the ground turned counterclockwise by `rotation_deg`,
    /// then seen from `tilt_deg` above the horizon.
    pub fn tilted(
        ground_centre: (f64, f64),
        mm_per_m: f64,
        rotation_deg: f64,
        tilt_deg: f64,
    ) -> Projection {
        let rotation = rotation_deg.to_radians();
        let tilt = tilt_deg.to_radians();
        Projection {
            mm_per_m,
            ground_centre,
            paper_centre: (0.0, 0.0),
            rotation: (rotation.cos(), rotation.sin()),
            squash: tilt.sin(),
            rise: tilt.cos(),
        }
    }

    /// True for a plain plan, where circles stay circles and squares stay upright.
    pub fn is_plan(&self) -> bool {
        self.squash == 1.0 && self.rotation == (1.0, 0.0)
    }

    /// The same view with the ground centre landing somewhere else on the paper.
    pub fn moved_to(self, paper_centre: (f64, f64)) -> Projection {
        Projection {
            paper_centre,
            ..self
        }
    }

    /// The same view at another scale.
    pub fn scaled_to(self, mm_per_m: f64) -> Projection {
        Projection { mm_per_m, ..self }
    }

    /// The same view laid flat, for drawing on the ground plane only.
    pub fn flat(self) -> Projection {
        Projection { rise: 0.0, ..self }
    }

    /// Ground metres to the view frame, in metres: (across, away from the viewer).
    pub fn to_view(self, x: f64, y: f64) -> (f64, f64) {
        self.turn(x - self.ground_centre.0, y - self.ground_centre.1)
    }

    /// A ground direction (east, north) in the view frame.
    pub fn turn(self, east: f64, north: f64) -> (f64, f64) {
        let (cos, sin) = self.rotation;
        (east * cos - north * sin, east * sin + north * cos)
    }

    /// Ground metres (y north) to paper millimetres (y down).
    pub fn to_paper(self, x: f64, y: f64) -> (f64, f64) {
        self.to_paper_at(x, y, 0.0)
    }

    /// A point `z` metres above the ground to paper millimetres.
    pub fn to_paper_at(self, x: f64, y: f64, z: f64) -> (f64, f64) {
        let (u, v) = self.to_view(x, y);
        let (centre_x, centre_y) = self.paper_centre;
        (
            centre_x + u * self.mm_per_m,
            centre_y - (v * self.squash + z * self.rise) * self.mm_per_m,
        )
    }

    /// Paper millimetres back to ground metres, on the ground.
    pub fn to_ground(self, x: f64, y: f64) -> (f64, f64) {
        let (centre_x, centre_y) = self.paper_centre;
        let u = (x - centre_x) / self.mm_per_m;
        let v = -(y - centre_y) / (self.mm_per_m * self.squash);
        let (cos, sin) = self.rotation;
        (
            self.ground_centre.0 + u * cos + v * sin,
            self.ground_centre.1 - u * sin + v * cos,
        )
    }

    /// Which way north points on the paper, in degrees clockwise from straight up.
    pub fn north_angle_deg(self) -> f64 {
        let (cos, sin) = self.rotation;
        // North (0, 1) lands at u = -sin, v = cos.
        (-sin).atan2(cos * self.squash).to_degrees()
    }

    pub fn to_hundredths(self, coord: Coord<f64>) -> (i64, i64) {
        let (x, y) = self.to_paper(coord.x, coord.y);
        (
            (x * HUNDREDTHS).round() as i64,
            (y * HUNDREDTHS).round() as i64,
        )
    }
}

/// The middle of the boundary's bounding box, or the origin for an empty boundary.
pub fn ground_centre(boundary: &Polygon<f64>) -> (f64, f64) {
    boundary
        .bounding_rect()
        .map(|rect| (rect.center().x, rect.center().y))
        .unwrap_or((0.0, 0.0))
}

/// How far the furthest of `points` lies from `centre`, in metres; 0 for no points.
pub fn furthest_from(points: impl IntoIterator<Item = (f64, f64)>, centre: (f64, f64)) -> f64 {
    points
        .into_iter()
        .map(|(x, y)| ((x - centre.0).powi(2) + (y - centre.1).powi(2)).sqrt())
        .fold(0.0, f64::max)
}

/// How far the boundary reaches from `centre`, so a frame around it never cuts the site.
pub fn boundary_reach(boundary: &Polygon<f64>, centre: (f64, f64)) -> f64 {
    furthest_from(boundary.exterior().0.iter().map(|c| (c.x, c.y)), centre)
}

/// 1234 -> "12.34", 1200 -> "12", -5 -> "-0.05".
pub fn push_hundredths(out: &mut String, value: i64) {
    if value < 0 {
        out.push('-');
    }
    let magnitude = value.unsigned_abs();
    let whole = magnitude / 100;
    let fraction = magnitude % 100;
    let _ = write!(out, "{whole}");
    if fraction == 0 {
        return;
    }
    if fraction.is_multiple_of(10) {
        let _ = write!(out, ".{}", fraction / 10);
    } else {
        let _ = write!(out, ".{fraction:02}");
    }
}

/// A millimetre value rounded to 0.01 mm, as text.
pub fn mm(value: f64) -> String {
    decimal(value)
}

/// Any number rounded to two decimals, as text: opacities, angles.
pub fn decimal(value: f64) -> String {
    let mut out = String::new();
    push_hundredths(&mut out, (value * HUNDREDTHS).round() as i64);
    out
}

/// What a geometry turns into on paper.
#[derive(Debug, Default)]
pub struct PaperShapes {
    /// Path data for filled areas, holes included. Outer rings and holes
    /// turn opposite ways, so areas joined into one path fill as their union.
    pub areas: Vec<String>,
    /// Path data for open lines.
    pub lines: Vec<String>,
    /// Point centres in millimetres.
    pub points: Vec<(f64, f64)>,
}

/// Splits any geometry into areas, lines and points on paper.
pub fn paper_shapes(geometry: &Geometry<f64>, projection: &Projection, shapes: &mut PaperShapes) {
    match geometry {
        Geometry::Polygon(polygon) => push_polygon(polygon, projection, shapes),
        Geometry::MultiPolygon(multi) => {
            for polygon in &multi.0 {
                push_polygon(polygon, projection, shapes);
            }
        }
        Geometry::Rect(rect) => push_polygon(&rect.to_polygon(), projection, shapes),
        Geometry::Triangle(triangle) => push_polygon(&triangle.to_polygon(), projection, shapes),
        Geometry::LineString(line) => push_line(line, projection, shapes),
        Geometry::MultiLineString(multi) => {
            for line in &multi.0 {
                push_line(line, projection, shapes);
            }
        }
        Geometry::Line(line) => push_line(
            &LineString::from(vec![line.start, line.end]),
            projection,
            shapes,
        ),
        Geometry::Point(point) => shapes
            .points
            .push(projection.to_paper(point.x(), point.y())),
        Geometry::MultiPoint(multi) => {
            for point in &multi.0 {
                shapes
                    .points
                    .push(projection.to_paper(point.x(), point.y()));
            }
        }
        Geometry::GeometryCollection(collection) => {
            for member in &collection.0 {
                paper_shapes(member, projection, shapes);
            }
        }
    }
}

/// Path data for one polygon, every ring closed, the outer ring and the holes
/// turning opposite ways. Empty when the outer ring collapses below 0.01 mm.
pub fn polygon_path(polygon: &Polygon<f64>, projection: &Projection) -> String {
    let polygon = polygon.orient(Direction::Default);
    let mut data = String::new();
    if !push_ring(&mut data, polygon.exterior(), projection, true) {
        return String::new();
    }
    for interior in polygon.interiors() {
        push_ring(&mut data, interior, projection, true);
    }

    data
}

fn push_polygon(polygon: &Polygon<f64>, projection: &Projection, shapes: &mut PaperShapes) {
    let data = polygon_path(polygon, projection);
    if !data.is_empty() {
        shapes.areas.push(data);
    }
}

fn push_line(line: &LineString<f64>, projection: &Projection, shapes: &mut PaperShapes) {
    let mut data = String::new();
    if push_ring(&mut data, line, projection, false) {
        shapes.lines.push(data);
    }
}

/// Appends "M x y l dx dy ... [z]" and returns false if fewer than the needed
/// distinct points survive rounding (3 for a ring, 2 for a line).
fn push_ring(
    data: &mut String,
    ring: &LineString<f64>,
    projection: &Projection,
    closed: bool,
) -> bool {
    let points = ring.0.iter().map(|coord| projection.to_hundredths(*coord));
    push_points(data, points, closed)
}

/// Appends paper points in millimetres as path data, like `push_ring`.
pub fn push_paper_points(data: &mut String, points: &[(f64, f64)], closed: bool) -> bool {
    let hundredths = points.iter().map(|&(x, y)| {
        (
            (x * HUNDREDTHS).round() as i64,
            (y * HUNDREDTHS).round() as i64,
        )
    });
    push_points(data, hundredths, closed)
}

fn push_points(data: &mut String, rounded: impl Iterator<Item = (i64, i64)>, closed: bool) -> bool {
    let mut points: Vec<(i64, i64)> = Vec::new();
    for point in rounded {
        if points.last() != Some(&point) {
            points.push(point);
        }
    }
    let mut points = simplify::simplified(&points);
    if closed && points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    let needed = if closed { 3 } else { 2 };
    if points.len() < needed {
        return false;
    }

    let (mut previous_x, mut previous_y) = points[0];
    data.push('M');
    push_hundredths(data, previous_x);
    data.push(' ');
    push_hundredths(data, previous_y);
    data.push('l');
    for (index, &(x, y)) in points.iter().enumerate().skip(1) {
        if index > 1 {
            data.push(' ');
        }
        push_hundredths(data, x - previous_x);
        data.push(' ');
        push_hundredths(data, y - previous_y);
        previous_x = x;
        previous_y = y;
    }
    if closed {
        data.push('z');
    }

    true
}

#[cfg(test)]
mod tests {
    use geo::{polygon, Winding};

    use super::*;

    /// The paper turn of every ring in `data`, as written: relative steps after a move.
    fn ring_turns(data: &str) -> Vec<bool> {
        data.split('M')
            .filter(|ring| !ring.is_empty())
            .map(|ring| {
                let numbers: Vec<f64> = ring
                    .trim_end_matches('z')
                    .replace('l', " ")
                    .split_whitespace()
                    .map(|number| number.parse().unwrap())
                    .collect();
                let mut point = (numbers[0], numbers[1]);
                let mut corners = vec![point];
                for step in numbers[2..].chunks(2) {
                    point = (point.0 + step[0], point.1 + step[1]);
                    corners.push(point);
                }
                // The path closes with `z`; the winding test needs the ring closed.
                corners.push(corners[0]);
                LineString::from(corners).is_ccw()
            })
            .collect()
    }

    #[test]
    fn a_hole_turns_the_other_way_from_its_outer_ring_whichever_way_they_came() {
        // Both rings clockwise, as some sources deliver them.
        let square = polygon!(
            exterior: [(x: 0.0, y: 0.0), (x: 0.0, y: 100.0), (x: 100.0, y: 100.0), (x: 100.0, y: 0.0)],
            interiors: [[(x: 20.0, y: 20.0), (x: 20.0, y: 80.0), (x: 80.0, y: 80.0), (x: 80.0, y: 20.0)]],
        );
        let zone = Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 200.0,
        };
        let projection = Projection::new(&square, 1000, &zone);

        let turns = ring_turns(&polygon_path(&square, &projection));

        assert_eq!(turns.len(), 2);
        assert_ne!(turns[0], turns[1]);
    }
}
