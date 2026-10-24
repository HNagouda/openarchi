//! Scene metres to paper millimetres, and geometry to SVG path data.
//!
//! Coordinates are rounded to 0.01 mm, far below what a plotter or a printer
//! can show, and written as integers of hundredths. Path steps are relative and
//! computed from the rounded points, so rounding never accumulates along a ring.

use std::fmt::Write;

use geo::{BoundingRect, Coord, Geometry, LineString, Polygon};

use super::layout::Rect;

/// Hundredths of a millimetre per millimetre.
const HUNDREDTHS: f64 = 100.0;

/// How the ground maps onto the paper: a fixed scale, centred in the drawing zone.
#[derive(Debug, Clone, Copy)]
pub struct Projection {
    /// Paper millimetres per ground metre: 1000 / scale.
    pub mm_per_m: f64,
    /// The ground point that lands on `paper_centre`.
    ground_centre: (f64, f64),
    pub paper_centre: (f64, f64),
}

impl Projection {
    /// Centres the boundary's bounding box in `zone` at `scale` (2500 for 1:2500).
    pub fn new(boundary: &Polygon<f64>, scale: u32, zone: &Rect) -> Projection {
        let ground_centre = boundary
            .bounding_rect()
            .map(|rect| (rect.center().x, rect.center().y))
            .unwrap_or((0.0, 0.0));
        Projection {
            mm_per_m: 1000.0 / scale as f64,
            ground_centre,
            paper_centre: zone.centre(),
        }
    }

    /// Ground metres (y north) to paper millimetres (y down).
    pub fn to_paper(self, x: f64, y: f64) -> (f64, f64) {
        let (centre_x, centre_y) = self.paper_centre;
        (
            centre_x + (x - self.ground_centre.0) * self.mm_per_m,
            centre_y - (y - self.ground_centre.1) * self.mm_per_m,
        )
    }

    /// Paper millimetres back to ground metres.
    pub fn to_ground(self, x: f64, y: f64) -> (f64, f64) {
        let (centre_x, centre_y) = self.paper_centre;
        (
            self.ground_centre.0 + (x - centre_x) / self.mm_per_m,
            self.ground_centre.1 - (y - centre_y) / self.mm_per_m,
        )
    }

    fn to_hundredths(self, coord: Coord<f64>) -> (i64, i64) {
        let (x, y) = self.to_paper(coord.x, coord.y);
        (
            (x * HUNDREDTHS).round() as i64,
            (y * HUNDREDTHS).round() as i64,
        )
    }
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
    /// Path data for filled areas, holes included (draw with evenodd).
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

/// Path data for one polygon, every ring closed. Empty when the outer ring
/// collapses below 0.01 mm.
pub fn polygon_path(polygon: &Polygon<f64>, projection: &Projection) -> String {
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
    let mut points: Vec<(i64, i64)> = Vec::with_capacity(ring.0.len());
    for coord in &ring.0 {
        let point = projection.to_hundredths(*coord);
        if points.last() != Some(&point) {
            points.push(point);
        }
    }
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
