//! The walk network as a graph, and the shortest walk from the site centre to
//! every junction on it.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use crate::sources::osm::Way;

/// Network nodes this close to the centre are all places a walk may start,
/// each after the straight walk to it. One nearest node could be a dead end
/// in a courtyard; several give every street around the centre a fair start.
const START_RADIUS_M: f64 = 60.0;
/// Without a node that close, the walk starts at the nearest node within this.
const MAX_START_M: f64 = 400.0;
/// Heap keys are whole millimetres, which sort exactly where floats do not.
const MM_PER_M: f64 = 1000.0;

/// One stretch of way between two neighbouring points.
#[derive(Debug, Clone, Copy)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub length_m: f64,
}

pub struct Graph {
    /// Every point of every way, in RD New. Ways that share a node share its index.
    pub points: Vec<(f64, f64)>,
    pub edges: Vec<Edge>,
    /// For each point, the edges that touch it: (neighbour, length in metres).
    neighbours: Vec<Vec<(usize, f64)>>,
}

impl Graph {
    pub fn build(ways: &[Way]) -> Graph {
        let mut index_of: HashMap<i64, usize> = HashMap::new();
        let mut points = Vec::new();
        let mut edges = Vec::new();
        for way in ways {
            let mut previous: Option<usize> = None;
            for (&node, &point) in way.nodes.iter().zip(&way.points) {
                let index = *index_of.entry(node).or_insert_with(|| {
                    points.push(point);
                    points.len() - 1
                });
                if let Some(from) = previous.filter(|&from| from != index) {
                    edges.push(Edge {
                        from,
                        to: index,
                        length_m: distance(points[from], points[index]),
                    });
                }
                previous = Some(index);
            }
        }

        let mut neighbours = vec![Vec::new(); points.len()];
        for edge in &edges {
            neighbours[edge.from].push((edge.to, edge.length_m));
            neighbours[edge.to].push((edge.from, edge.length_m));
        }

        Graph {
            points,
            edges,
            neighbours,
        }
    }

    /// The shortest walk in metres from `centre` to every point, through the
    /// nearest points of the network. Points further than `limit_m` stay
    /// infinite. Empty when no point lies near enough to start from.
    pub fn distances_from(&self, centre: (f64, f64), limit_m: f64) -> Vec<f64> {
        let mut best = vec![f64::INFINITY; self.points.len()];
        let mut queue = BinaryHeap::new();
        for (index, start_m) in self.starts(centre) {
            best[index] = start_m;
            queue.push(Reverse((millimetres(start_m), index)));
        }
        if queue.is_empty() {
            return Vec::new();
        }

        while let Some(Reverse((key, index))) = queue.pop() {
            if key > millimetres(best[index]) {
                continue;
            }
            for &(next, length_m) in &self.neighbours[index] {
                let through = best[index] + length_m;
                if through < best[next] && through <= limit_m {
                    best[next] = through;
                    queue.push(Reverse((millimetres(through), next)));
                }
            }
        }

        best
    }

    /// Where a walk from `centre` enters the network: (point, metres to it).
    fn starts(&self, centre: (f64, f64)) -> Vec<(usize, f64)> {
        let near: Vec<(usize, f64)> = self
            .points
            .iter()
            .enumerate()
            .map(|(index, &point)| (index, distance(centre, point)))
            .filter(|&(_, metres)| metres <= START_RADIUS_M)
            .collect();
        if !near.is_empty() {
            return near;
        }

        let nearest = self
            .points
            .iter()
            .enumerate()
            .map(|(index, &point)| (index, distance(centre, point)))
            .min_by(|a, b| a.1.total_cmp(&b.1));
        nearest
            .filter(|&(_, metres)| metres <= MAX_START_M)
            .into_iter()
            .collect()
    }
}

fn millimetres(metres: f64) -> u64 {
    (metres * MM_PER_M).round() as u64
}

pub fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn way(nodes: &[i64], points: &[(f64, f64)]) -> Way {
        Way {
            nodes: nodes.to_vec(),
            points: points.to_vec(),
            is_drawn: true,
        }
    }

    #[test]
    fn ways_that_share_a_node_are_joined_there() {
        // An L: east 100 m, then north 100 m on a second way.
        let ways = [
            way(&[1, 2], &[(0.0, 0.0), (100.0, 0.0)]),
            way(&[2, 3], &[(100.0, 0.0), (100.0, 100.0)]),
        ];
        let graph = Graph::build(&ways);
        let distances = graph.distances_from((0.0, 0.0), 1000.0);

        assert_eq!(graph.points.len(), 3);
        assert!((distances[2] - 200.0).abs() < 1e-9);
    }

    #[test]
    fn ways_that_cross_without_a_shared_node_are_not_joined() {
        // A bridge over a street: no node in common, so no way across.
        let ways = [
            way(&[1, 2], &[(-50.0, 0.0), (50.0, 0.0)]),
            way(&[3, 4], &[(0.0, -500.0), (0.0, 500.0)]),
        ];
        let graph = Graph::build(&ways);
        let distances = graph.distances_from((-50.0, 0.0), 2000.0);

        assert!(distances[2].is_infinite());
    }

    #[test]
    fn points_past_the_limit_stay_unreached() {
        let ways = [way(&[1, 2, 3], &[(0.0, 0.0), (500.0, 0.0), (1500.0, 0.0)])];
        let distances = Graph::build(&ways).distances_from((0.0, 0.0), 1000.0);

        assert_eq!(distances[1], 500.0);
        assert!(distances[2].is_infinite());
    }

    #[test]
    fn a_centre_far_from_any_way_has_no_walk() {
        let ways = [way(&[1, 2], &[(0.0, 0.0), (10.0, 0.0)])];
        let graph = Graph::build(&ways);
        assert!(graph.distances_from((5000.0, 0.0), 1000.0).is_empty());
    }
}
