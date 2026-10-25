//! What a section line meets: the stretches of a footprint the cut passes
//! through, and the footprints and trees standing beyond it, up to a depth.
//!
//! A building the line crosses is both: cut where the line passes through it,
//! and seen in elevation where it stands beyond the line. The renderer draws
//! the elevation first and the cut on top.

use geo::{BooleanOps, BoundingRect, Centroid, Contains, Geometry, MultiPolygon, Point, Polygon};

use super::Axis;
use crate::scene::polygons;

/// Along-line positions closer than this are one crossing, so a line
/// running exactly through a corner does not split a cut in two.
const SAME_CROSSING_M: f64 = 1e-6;

/// A stretch along the line, in metres from A.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stretch {
    pub from: f64,
    pub to: f64,
}

/// A footprint or a tree seen beyond the line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Beyond {
    /// The part of the line it stands in front of.
    pub stretch: Stretch,
    /// How far beyond the line its nearest part is, in metres.
    pub depth: f64,
    /// The middle of the part within reach, in the scene's frame, to read the ground under it.
    pub centre: (f64, f64),
}

/// The stretches of the line inside `geometry`, A to A'. Empty for a line that misses it.
pub fn cut_stretches(geometry: &Geometry<f64>, axis: &Axis) -> Vec<Stretch> {
    let mut stretches = Vec::new();
    for polygon in polygons(geometry) {
        let mut crossings = vec![0.0, axis.length];
        let rings = std::iter::once(polygon.exterior()).chain(polygon.interiors());
        for ring in rings {
            for edge in ring.lines() {
                let (a_along, a_depth) = axis.station(edge.start.x, edge.start.y);
                let (b_along, b_depth) = axis.station(edge.end.x, edge.end.y);
                let is_crossing = (a_depth <= 0.0) != (b_depth <= 0.0);
                if is_crossing {
                    let share = a_depth / (a_depth - b_depth);
                    let along = a_along + (b_along - a_along) * share;
                    if along > 0.0 && along < axis.length {
                        crossings.push(along);
                    }
                }
            }
        }
        crossings.sort_by(f64::total_cmp);
        crossings.dedup_by(|a, b| (*a - *b).abs() < SAME_CROSSING_M);

        // Between two crossings the line is either all inside or all outside.
        for pair in crossings.windows(2) {
            let (x, y) = axis.point_at((pair[0] + pair[1]) / 2.0, 0.0);
            if polygon.contains(&Point::new(x, y)) {
                push_joined(
                    &mut stretches,
                    Stretch {
                        from: pair[0],
                        to: pair[1],
                    },
                );
            }
        }
    }

    stretches
}

/// The part of `geometry` within `depth_m` beyond the line and between its
/// ends, or `None` when nothing of it is there.
pub fn beyond(geometry: &Geometry<f64>, axis: &Axis, depth_m: f64) -> Option<Beyond> {
    if let Geometry::Point(point) = geometry {
        return beyond_point(*point, axis, depth_m);
    }
    let band = band(axis, depth_m);
    let band_box = band.bounding_rect()?;
    let shapes: Vec<Polygon<f64>> = polygons(geometry)
        .into_iter()
        .filter(|polygon| {
            polygon.bounding_rect().is_some_and(|rect| {
                rect.min().x <= band_box.max().x
                    && rect.max().x >= band_box.min().x
                    && rect.min().y <= band_box.max().y
                    && rect.max().y >= band_box.min().y
            })
        })
        .cloned()
        .collect();
    if shapes.is_empty() {
        return None;
    }
    let within = MultiPolygon(shapes).intersection(&MultiPolygon(vec![band]));
    let centre = within.centroid()?;

    let mut from = f64::INFINITY;
    let mut to = f64::NEG_INFINITY;
    let mut depth = f64::INFINITY;
    for polygon in &within.0 {
        for coord in &polygon.exterior().0 {
            let (along, beyond) = axis.station(coord.x, coord.y);
            from = from.min(along);
            to = to.max(along);
            depth = depth.min(beyond);
        }
    }

    (to > from).then_some(Beyond {
        stretch: Stretch { from, to },
        depth: depth.max(0.0),
        centre: (centre.x(), centre.y()),
    })
}

/// A point, such as a tree, beyond the line and between its ends.
fn beyond_point(point: Point<f64>, axis: &Axis, depth_m: f64) -> Option<Beyond> {
    let (along, depth) = axis.station(point.x(), point.y());
    let is_within = (0.0..=depth_m).contains(&depth) && (0.0..=axis.length).contains(&along);
    is_within.then_some(Beyond {
        stretch: Stretch {
            from: along,
            to: along,
        },
        depth,
        centre: (point.x(), point.y()),
    })
}

/// The ground from the line to `depth_m` beyond it, between its ends.
fn band(axis: &Axis, depth_m: f64) -> Polygon<f64> {
    let corners = vec![
        axis.point_at(0.0, 0.0),
        axis.point_at(axis.length, 0.0),
        axis.point_at(axis.length, depth_m),
        axis.point_at(0.0, depth_m),
    ];
    Polygon::new(corners.into(), vec![])
}

/// Adds `stretch`, joining it to the last one when they touch.
fn push_joined(stretches: &mut Vec<Stretch>, stretch: Stretch) {
    if let Some(last) = stretches.last_mut() {
        if (last.to - stretch.from).abs() < SAME_CROSSING_M {
            last.to = stretch.to;
            return;
        }
    }
    stretches.push(stretch);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::section::SectionLine;
    use geo::polygon;

    /// 100 m from (0, 0) east to (100, 0), looking north.
    fn axis() -> Axis {
        SectionLine {
            start: (0.0, 0.0),
            end: (100.0, 0.0),
            depth_m: 50.0,
            scale: None,
            exaggeration: 1,
        }
        .axis((0.0, 0.0))
    }

    fn square(west: f64, south: f64, size: f64) -> Geometry<f64> {
        Geometry::Polygon(polygon![
            (x: west, y: south),
            (x: west + size, y: south),
            (x: west + size, y: south + size),
            (x: west, y: south + size),
        ])
    }

    #[test]
    fn a_building_across_the_line_is_cut_where_the_line_runs_through_it() {
        let building = square(20.0, -5.0, 10.0);
        assert_eq!(
            cut_stretches(&building, &axis()),
            vec![Stretch {
                from: 20.0,
                to: 30.0
            }]
        );
    }

    #[test]
    fn a_courtyard_splits_the_cut_in_two() {
        let mut block = polygon![
            (x: 10.0, y: -20.0), (x: 50.0, y: -20.0), (x: 50.0, y: 20.0), (x: 10.0, y: 20.0),
        ];
        block.interiors_push(vec![
            (20.0, -10.0),
            (40.0, -10.0),
            (40.0, 10.0),
            (20.0, 10.0),
        ]);
        let stretches = cut_stretches(&Geometry::Polygon(block), &axis());
        assert_eq!(
            stretches,
            vec![
                Stretch {
                    from: 10.0,
                    to: 20.0
                },
                Stretch {
                    from: 40.0,
                    to: 50.0
                }
            ]
        );
    }

    #[test]
    fn a_cut_building_also_stands_beyond_the_line() {
        let seen = beyond(&square(20.0, -5.0, 10.0), &axis(), 50.0).unwrap();
        assert_eq!(
            seen.stretch,
            Stretch {
                from: 20.0,
                to: 30.0
            }
        );
        assert_eq!(seen.depth, 0.0);
    }

    #[test]
    fn a_building_beyond_is_seen_but_not_cut() {
        let building = square(60.0, 20.0, 10.0);
        assert!(cut_stretches(&building, &axis()).is_empty());
        let seen = beyond(&building, &axis(), 50.0).unwrap();
        assert_eq!(
            seen.stretch,
            Stretch {
                from: 60.0,
                to: 70.0
            }
        );
        assert_eq!(seen.depth, 20.0);
    }

    #[test]
    fn buildings_behind_the_viewer_or_out_of_reach_are_left_out() {
        assert!(beyond(&square(40.0, -30.0, 10.0), &axis(), 50.0).is_none());
        assert!(beyond(&square(40.0, 80.0, 10.0), &axis(), 50.0).is_none());
    }

    #[test]
    fn a_building_past_the_end_is_cut_to_the_line_s_length() {
        let seen = beyond(&square(90.0, 10.0, 30.0), &axis(), 50.0).unwrap();
        assert_eq!(
            seen.stretch,
            Stretch {
                from: 90.0,
                to: 100.0
            }
        );
    }

    #[test]
    fn a_tree_within_reach_is_seen_at_its_point() {
        let tree = Geometry::Point(Point::new(25.0, 12.0));
        let seen = beyond(&tree, &axis(), 50.0).unwrap();
        assert_eq!(
            seen.stretch,
            Stretch {
                from: 25.0,
                to: 25.0
            }
        );
        assert_eq!(seen.depth, 12.0);
    }
}
