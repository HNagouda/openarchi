//! A walk-distance surface: for every 10 m cell around the site, how far it is
//! on foot from the site centre. Walk bands are its contours.
//!
//! Each reached stretch of street spreads its distance a little way to the
//! side, so a cell's value is the walk to the nearest reached street plus the
//! last few metres straight across. That is the same as buffering the reached
//! streets, but one surface gives every band at once, and the bands nest by
//! construction: the 5 minute area always lies inside the 10 minute one.
//! Unioning thousands of buffered segments as polygons would be slower and
//! fragile on the near-parallel lines of a mapped pavement.

use contour::ContourBuilder;
use geo::{Area, LineString, MultiPolygon, Polygon, Simplify};

use super::graph::{distance, Graph};
use super::OFF_NETWORK_M;

/// Fine enough that a band follows a street, coarse enough that a 1.3 km
/// reach stays under a hundred thousand cells.
const CELL_M: f64 = 10.0;
/// Distance between samples along a street; half a cell, so no cell is skipped.
const SAMPLE_M: f64 = CELL_M / 2.0;
/// Band outlines are simplified to this; it is a fifth of a millimetre at 1:10 000.
const SIMPLIFY_M: f64 = 2.0;
/// Holes and islands smaller than this are left out. They are the deep middles
/// of building blocks and stray cells, which read as noise rather than as
/// places a walk cannot reach. A canal-belt block is about 30 by 150 m.
const MIN_SPECK_M2: f64 = 6000.0;

pub struct Surface {
    /// Centre of the north-west cell, in RD New.
    west: f64,
    north: f64,
    width: usize,
    height: usize,
    /// Metres on foot, row by row from the north. Infinite where no walk reaches.
    metres: Vec<f64>,
}

impl Surface {
    /// The surface over every street reached within `limit_m`. `None` when
    /// nothing was reached.
    pub fn build(graph: &Graph, distances: &[f64], limit_m: f64) -> Option<Surface> {
        let reached = graph
            .points
            .iter()
            .zip(distances)
            .filter(|(_, metres)| **metres <= limit_m)
            .map(|(point, _)| *point);
        let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
        let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (x, y) in reached {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
        if !min_x.is_finite() {
            return None;
        }

        // A margin of empty cells all round, so every band closes into a ring.
        let margin = OFF_NETWORK_M + 2.0 * CELL_M;
        let west = ((min_x - margin) / CELL_M).floor() * CELL_M + CELL_M / 2.0;
        let north = ((max_y + margin) / CELL_M).ceil() * CELL_M - CELL_M / 2.0;
        let width = ((max_x + margin - west) / CELL_M).ceil() as usize + 1;
        let height = ((north - (min_y - margin)) / CELL_M).ceil() as usize + 1;
        let mut surface = Surface {
            west,
            north,
            width,
            height,
            metres: vec![f64::INFINITY; width * height],
        };

        for edge in &graph.edges {
            let (from_m, to_m) = (distances[edge.from], distances[edge.to]);
            if from_m.min(to_m) > limit_m {
                continue;
            }
            let (from, to) = (graph.points[edge.from], graph.points[edge.to]);
            let steps = (edge.length_m / SAMPLE_M).ceil().max(1.0) as usize;
            for step in 0..=steps {
                let along = step as f64 / steps as f64;
                let walked =
                    (from_m + along * edge.length_m).min(to_m + (1.0 - along) * edge.length_m);
                if walked > limit_m {
                    continue;
                }
                let point = (
                    from.0 + along * (to.0 - from.0),
                    from.1 + along * (to.1 - from.1),
                );
                surface.spread(point, walked);
            }
        }

        Some(surface)
    }

    /// Metres on foot to the cell holding `point`, infinite outside the surface.
    pub fn metres_at(&self, (x, y): (f64, f64)) -> f64 {
        let column = ((x - self.west) / CELL_M).round();
        let row = ((self.north - y) / CELL_M).round();
        let is_inside = column >= 0.0
            && row >= 0.0
            && (column as usize) < self.width
            && (row as usize) < self.height;
        if !is_inside {
            return f64::INFINITY;
        }

        self.metres[row as usize * self.width + column as usize]
    }

    /// The area within each of `limits_m`, in the same order, in RD New.
    pub fn areas_within(&self, limits_m: &[f64]) -> Vec<MultiPolygon<f64>> {
        // Contours enclose values at or above a threshold, so the distances
        // are turned negative: within 420 m is at or above -420.
        let unreached = -1.0e9;
        // Rows from the south, so y grows with the row: the contour builder
        // tells outer rings from holes by their turn, which a flipped axis reverses.
        let values: Vec<f64> = self
            .metres
            .chunks(self.width)
            .rev()
            .flatten()
            .map(|&metres| {
                if metres.is_finite() {
                    -metres
                } else {
                    unreached
                }
            })
            .collect();
        let thresholds: Vec<f64> = limits_m.iter().map(|limit| -limit).collect();
        let south = self.north - (self.height - 1) as f64 * CELL_M;
        let builder = ContourBuilder::new(self.width, self.height, true)
            .x_origin(self.west)
            .y_origin(south)
            .x_step(CELL_M)
            .y_step(CELL_M);
        let Ok(contours) = builder.contours(&values, &thresholds) else {
            return Vec::new();
        };

        contours
            .into_iter()
            .map(|contour| without_specks(contour.into_inner().0.simplify(&SIMPLIFY_M)))
            .collect()
    }

    /// Lowers every cell within `OFF_NETWORK_M` of `point` to the walk there.
    fn spread(&mut self, point: (f64, f64), walked: f64) {
        let first_column = ((point.0 - OFF_NETWORK_M - self.west) / CELL_M)
            .floor()
            .max(0.0) as usize;
        let last_column =
            (((point.0 + OFF_NETWORK_M - self.west) / CELL_M).ceil() as usize).min(self.width - 1);
        let first_row = ((self.north - point.1 - OFF_NETWORK_M) / CELL_M)
            .floor()
            .max(0.0) as usize;
        let last_row = (((self.north - point.1 + OFF_NETWORK_M) / CELL_M).ceil() as usize)
            .min(self.height - 1);
        for row in first_row..=last_row {
            for column in first_column..=last_column {
                let centre = (
                    self.west + column as f64 * CELL_M,
                    self.north - row as f64 * CELL_M,
                );
                let across = distance(point, centre);
                if across > OFF_NETWORK_M {
                    continue;
                }
                let cell = &mut self.metres[row * self.width + column];
                *cell = cell.min(walked + across);
            }
        }
    }
}

/// `area` without the holes and islands below `MIN_SPECK_M2`.
fn without_specks(area: MultiPolygon<f64>) -> MultiPolygon<f64> {
    let polygons = area
        .into_iter()
        .filter(|polygon| polygon.unsigned_area() >= MIN_SPECK_M2)
        .map(|polygon| {
            let (exterior, interiors) = polygon.into_inner();
            let kept: Vec<LineString<f64>> = interiors
                .into_iter()
                .filter(|hole| Polygon::new(hole.clone(), vec![]).unsigned_area() >= MIN_SPECK_M2)
                .collect();
            Polygon::new(exterior, kept)
        })
        .collect();

    MultiPolygon::new(polygons)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::osm::Way;
    use geo::{Area, Contains, Point};

    /// One straight street 1 km long, walked from its west end.
    fn street() -> (Graph, Vec<f64>) {
        let ways = [Way {
            nodes: vec![1, 2],
            points: vec![(0.0, 0.0), (1000.0, 0.0)],
            is_drawn: true,
        }];
        let graph = Graph::build(&ways);
        let distances = graph.distances_from((0.0, 0.0), 2000.0);
        (graph, distances)
    }

    #[test]
    fn a_cell_on_the_street_is_as_far_as_the_walk_to_it() {
        let (graph, distances) = street();
        let surface = Surface::build(&graph, &distances, 2000.0).unwrap();

        assert!((surface.metres_at((400.0, 0.0)) - 400.0).abs() <= CELL_M);
        assert!(surface.metres_at((400.0, 500.0)).is_infinite());
    }

    #[test]
    fn bands_nest_and_follow_the_street() {
        let (graph, distances) = street();
        let surface = Surface::build(&graph, &distances, 2000.0).unwrap();
        let areas = surface.areas_within(&[300.0, 600.0]);

        assert_eq!(areas.len(), 2);
        assert!(areas[0].contains(&Point::new(150.0, 0.0)));
        assert!(!areas[0].contains(&Point::new(450.0, 0.0)));
        assert!(areas[1].contains(&Point::new(450.0, 0.0)));
        // A strip about 600 m long and twice the off-network reach wide.
        let expected = 600.0 * 2.0 * OFF_NETWORK_M;
        assert!((areas[1].unsigned_area() / expected - 1.0).abs() < 0.2);
    }
}
