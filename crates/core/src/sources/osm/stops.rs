//! Transit stops as a passenger sees them: one point per named stop, with the
//! lines that call there.
//!
//! OpenStreetMap maps a stop as several nodes: a platform on each side of the
//! street, a stop position on the track for each direction, sometimes a
//! station node as well. Drawn one by one they turn a single tram stop into a
//! cluster of four dots. So nodes with the same name close together become
//! one stop, at the middle of its platforms, and the lines of every route
//! relation that lists any of its nodes are joined onto it.

use std::collections::{BTreeMap, BTreeSet};

use geo::{Geometry, Point};

use super::tags::{self, Tags, TRANSIT_GROUPS};
use crate::scene::{Attributes, Feature, Layer};

/// Nodes of one name further apart than this are different stops, such as two
/// "Kerkstraat" stops in different parts of town. Large enough to join the
/// platforms of a big station.
const SAME_STOP_M: f64 = 250.0;
/// Unnamed nodes are only joined with ones this close: the two sides of a street.
const SAME_UNNAMED_STOP_M: f64 = 40.0;

/// One stop node as Overpass returned it, in RD New.
#[derive(Debug, Clone)]
pub struct StopNode {
    pub id: i64,
    pub position: (f64, f64),
    pub tags: Tags,
}

/// One public transport route relation: its mode, its line number and the
/// nodes it lists.
#[derive(Debug, Clone)]
pub struct Route {
    pub mode: &'static str,
    /// "13", "N80"; empty for routes without a `ref`, such as many train services.
    pub line: String,
    pub nodes: Vec<i64>,
}

/// One feature per stop, with `name`, `group` (the most important mode) and
/// `lines` ("Tram 13", "Bus 21"), in RD New.
pub fn merge(nodes: &[StopNode], routes: &[Route]) -> Vec<Feature> {
    let mut lines_of_node: BTreeMap<i64, BTreeSet<(usize, String)>> = BTreeMap::new();
    for route in routes {
        let Some(rank) = mode_rank(route.mode) else {
            continue;
        };
        for node in &route.nodes {
            lines_of_node
                .entry(*node)
                .or_default()
                .insert((rank, route.line.clone()));
        }
    }

    let mut stops = Vec::new();
    for cluster in clusters(nodes) {
        let members: Vec<&StopNode> = cluster.iter().map(|&index| &nodes[index]).collect();
        stops.push(stop_feature(&members, &lines_of_node));
    }

    stops
}

/// Groups of node indices that are one stop, in a stable order.
fn clusters(nodes: &[StopNode]) -> Vec<Vec<usize>> {
    let mut clusters: Vec<Vec<usize>> = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        let name = stop_name(&node.tags);
        let reach = if name.is_some() {
            SAME_STOP_M
        } else {
            SAME_UNNAMED_STOP_M
        };
        let joined = clusters.iter_mut().find(|cluster| {
            let first = &nodes[cluster[0]];
            stop_name(&first.tags) == name && distance(first.position, node.position) <= reach
        });
        match joined {
            Some(cluster) => cluster.push(index),
            None => clusters.push(vec![index]),
        }
    }

    clusters
}

fn stop_feature(
    members: &[&StopNode],
    lines_of_node: &BTreeMap<i64, BTreeSet<(usize, String)>>,
) -> Feature {
    let mut lines: BTreeSet<(usize, String)> = BTreeSet::new();
    for member in members {
        if let Some(found) = lines_of_node.get(&member.id) {
            lines.extend(found.iter().cloned());
        }
    }
    // Route relations are often incomplete, so a stop tagged as a tram stop stays
    // a tram stop even when only night buses list it.
    let route_rank = lines.iter().map(|(rank, _)| *rank).min();
    let own_rank = members
        .iter()
        .filter_map(|member| mode_rank(tags::stop_mode(&member.tags)))
        .min();
    let rank = route_rank.into_iter().chain(own_rank).min().unwrap_or(0);

    // Platforms are where people wait; stop positions only stand in when a stop has no platform.
    let platforms: Vec<&&StopNode> = members
        .iter()
        .filter(|member| !tags::is_stop_position(&member.tags))
        .collect();
    let placed: Vec<(f64, f64)> = if platforms.is_empty() {
        members.iter().map(|member| member.position).collect()
    } else {
        platforms.iter().map(|member| member.position).collect()
    };

    let mut sorted_lines: Vec<(usize, String)> = lines
        .into_iter()
        .filter(|(_, line)| !line.is_empty())
        .collect();
    sorted_lines.sort_by_key(|(rank, line)| (*rank, line_order(line)));

    Feature {
        layer: Layer::TransitStop,
        geometry: Geometry::Point(Point::from(middle(&placed))),
        attributes: Attributes {
            source_id: Some(format!("osm/node/{}", members[0].id)),
            kind: Some(TRANSIT_GROUPS[rank].0.to_string()),
            name: stop_name(&members[0].tags),
            group: Some(TRANSIT_GROUPS[rank].0.to_string()),
            lines: sorted_lines
                .into_iter()
                .map(|(rank, line)| format!("{} {line}", TRANSIT_GROUPS[rank].1))
                .collect(),
            ..Attributes::default()
        },
    }
}

/// Where a mode sits in `TRANSIT_GROUPS`, most important first.
fn mode_rank(mode: &str) -> Option<usize> {
    TRANSIT_GROUPS.iter().position(|(group, _)| *group == mode)
}

/// A stop's name without the town prefix Dutch operators often add:
/// "Amsterdam, Westermarkt" -> "Westermarkt".
fn stop_name(tags: &Tags) -> Option<String> {
    let name = tags.get("name")?;
    let short = name
        .split_once(", ")
        .map_or(name.as_str(), |(_, rest)| rest);
    Some(short.trim().to_string()).filter(|name| !name.is_empty())
}

/// Line numbers in natural order: 2, 13, 17, N80, then names.
pub fn line_order(line: &str) -> (u8, u32, String) {
    let digits: String = line.chars().filter(char::is_ascii_digit).collect();
    match (line.parse::<u32>(), digits.parse::<u32>()) {
        (Ok(number), _) => (0, number, String::new()),
        (Err(_), Ok(number)) => (1, number, line.to_string()),
        _ => (2, 0, line.to_string()),
    }
}

fn middle(points: &[(f64, f64)]) -> (f64, f64) {
    let count = points.len().max(1) as f64;
    let x: f64 = points.iter().map(|point| point.0).sum();
    let y: f64 = points.iter().map(|point| point.1).sum();
    (x / count, y / count)
}

fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: i64, x: f64, pairs: &[(&str, &str)]) -> StopNode {
        StopNode {
            id,
            position: (x, 0.0),
            tags: pairs
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        }
    }

    fn route(mode: &'static str, line: &str, nodes: &[i64]) -> Route {
        Route {
            mode,
            line: line.to_string(),
            nodes: nodes.to_vec(),
        }
    }

    #[test]
    fn two_platforms_of_one_stop_become_one_stop_with_both_lines() {
        let nodes = [
            node(
                1,
                0.0,
                &[("public_transport", "platform"), ("name", "Westermarkt")],
            ),
            node(
                2,
                30.0,
                &[("public_transport", "platform"), ("name", "Westermarkt")],
            ),
            node(
                3,
                15.0,
                &[
                    ("public_transport", "stop_position"),
                    ("name", "Westermarkt"),
                ],
            ),
        ];
        let routes = [
            route("tram", "17", &[1, 3]),
            route("tram", "13", &[2]),
            route("bus", "21", &[1]),
            route("tram", "13", &[1]),
        ];

        let stops = merge(&nodes, &routes);

        assert_eq!(stops.len(), 1);
        let attributes = &stops[0].attributes;
        assert_eq!(attributes.name.as_deref(), Some("Westermarkt"));
        assert_eq!(attributes.group.as_deref(), Some("tram"));
        assert_eq!(attributes.lines, ["Tram 13", "Tram 17", "Bus 21"]);
        // The middle of the two platforms, not pulled towards the stop position.
        assert_eq!(stops[0].geometry, Geometry::Point(Point::new(15.0, 0.0)));
    }

    #[test]
    fn stops_of_one_name_far_apart_stay_apart() {
        let nodes = [
            node(1, 0.0, &[("highway", "bus_stop"), ("name", "Kerkstraat")]),
            node(2, 900.0, &[("highway", "bus_stop"), ("name", "Kerkstraat")]),
        ];
        assert_eq!(merge(&nodes, &[]).len(), 2);
    }

    #[test]
    fn a_stop_no_route_lists_takes_its_mode_from_its_own_tags() {
        let nodes = [node(
            1,
            0.0,
            &[("railway", "station"), ("name", "Eindhoven Centraal")],
        )];
        let stops = merge(&nodes, &[]);
        assert_eq!(stops[0].attributes.group.as_deref(), Some("train"));
        assert!(stops[0].attributes.lines.is_empty());
    }

    #[test]
    fn the_town_prefix_is_left_off_the_name() {
        let tags: Tags = [("name".to_string(), "Amsterdam, Westermarkt".to_string())].into();
        assert_eq!(stop_name(&tags).as_deref(), Some("Westermarkt"));
    }
}
