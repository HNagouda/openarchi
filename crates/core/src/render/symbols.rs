//! Point symbols for places and stops: a few plain shapes, one per class, so
//! groups stay apart in a one-colour palette and for readers who cannot tell
//! the category colours apart.

use super::geometry::{mm, push_paper_points};

/// The shapes in the order classes take them, in one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Circle,
    Square,
    Diamond,
    Triangle,
    InvertedTriangle,
    Hexagon,
    /// A ring, light enough for a group with hundreds of places (bicycle parking).
    Ring,
}

const SHAPES: [Shape; 7] = [
    Shape::Circle,
    Shape::Square,
    Shape::Diamond,
    Shape::Triangle,
    Shape::InvertedTriangle,
    Shape::Hexagon,
    Shape::Ring,
];

/// How far each shape's corners reach, relative to a circle of the same
/// radius, so every shape covers about the same area and none looks heavier.
const SQUARE_REACH: f64 = 1.25;
const DIAMOND_REACH: f64 = 1.3;
const TRIANGLE_REACH: f64 = 1.45;
const HEXAGON_REACH: f64 = 1.1;
/// A ring's outer edge, and its hole as a share of that, so it reads as a ring at 1 mm.
const RING_REACH: f64 = 1.1;
const RING_HOLE: f64 = 0.45;

/// The shape for the class at `index`; classes past the last shape start over.
pub fn shape_for(index: usize) -> Shape {
    SHAPES[index % SHAPES.len()]
}

/// The symbol as an SVG element centred on (x, y) with radius `radius` in
/// millimetres, followed by `attributes`.
pub fn element(shape: Shape, (x, y): (f64, f64), radius: f64, attributes: &str) -> String {
    let corners = |count: usize, reach: f64, turn: f64| -> Vec<(f64, f64)> {
        (0..count)
            .map(|index| {
                let angle = turn + std::f64::consts::TAU * index as f64 / count as f64;
                (
                    x + radius * reach * angle.sin(),
                    y - radius * reach * angle.cos(),
                )
            })
            .collect()
    };
    let points = match shape {
        Shape::Circle => {
            return format!(
                r#"<circle cx="{}" cy="{}" r="{}"{attributes}/>"#,
                super::geometry::mm(x),
                super::geometry::mm(y),
                super::geometry::mm(radius)
            );
        }
        Shape::Square => corners(4, SQUARE_REACH, std::f64::consts::FRAC_PI_4),
        Shape::Diamond => corners(4, DIAMOND_REACH, 0.0),
        Shape::Triangle => corners(3, TRIANGLE_REACH, 0.0),
        Shape::InvertedTriangle => corners(3, TRIANGLE_REACH, std::f64::consts::PI),
        Shape::Hexagon => corners(6, HEXAGON_REACH, std::f64::consts::FRAC_PI_6),
        Shape::Ring => return ring((x, y), radius * RING_REACH, attributes),
    };
    let mut data = String::new();
    push_paper_points(&mut data, &points, true);

    format!(r#"<path d="{data}"{attributes}/>"#)
}

/// Two circles as one path; the even-odd rule leaves the inner one open.
fn ring((x, y): (f64, f64), outer: f64, attributes: &str) -> String {
    let inner = outer * RING_HOLE;
    let circle = |radius: f64| {
        format!(
            "M{} {}a{r} {r} 0 1 0 {d} 0a{r} {r} 0 1 0 -{d} 0z",
            mm(x - radius),
            mm(y),
            r = mm(radius),
            d = mm(2.0 * radius)
        )
    };

    format!(
        r#"<path d="{}{}" fill-rule="evenodd"{attributes}/>"#,
        circle(outer),
        circle(inner)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_take_shapes_in_order_and_start_over() {
        assert_eq!(shape_for(0), Shape::Circle);
        assert_eq!(shape_for(1), Shape::Square);
        assert_eq!(shape_for(SHAPES.len()), Shape::Circle);
    }

    #[test]
    fn a_ring_is_two_circles_with_the_inner_one_open() {
        let ring = element(Shape::Ring, (10.0, 10.0), 1.0, "");
        assert_eq!(ring.matches('z').count(), 2);
        assert!(ring.contains("fill-rule=\"evenodd\""));
    }

    #[test]
    fn a_square_is_a_closed_path_and_a_circle_a_circle() {
        assert!(element(Shape::Square, (10.0, 10.0), 1.0, "").starts_with("<path d=\"M"));
        assert!(element(Shape::Square, (10.0, 10.0), 1.0, "").contains('z'));
        assert!(element(Shape::Circle, (10.0, 10.0), 1.0, " fill=\"red\"")
            .contains("r=\"1\" fill=\"red\""));
    }
}
