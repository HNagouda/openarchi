//! Walk times from the site centre: 5, 10 and 15 minute bands over the
//! OpenStreetMap footpath network, and what lies inside each band.
//!
//! Walking is computed here rather than asked of a routing service, because
//! the public ones need a key and have daily quotas. The network comes in the
//! site's one Overpass query; a shortest-path search from the centre gives
//! the walk to every junction, and `surface` turns that into areas.
//!
//! The walk starts at the site centre, not at the boundary. A drawn site is
//! usually the context around a plot in its middle, so the question is how
//! far one gets from the plot.

mod graph;
mod surface;

use geo::{Geometry, LineString};
use serde::Serialize;

use self::graph::Graph;
use self::surface::Surface;
use crate::scene::{Attributes, Feature, Layer, Scene};
use crate::sources::osm::{
    self, Osm, BUSINESS_GROUPS, FACILITY_GROUPS, PARKING_GROUPS, TRANSIT_GROUPS,
};

/// An unhurried adult pace, about 5 km/h.
pub const SPEED_M_S: f64 = 1.4;
/// The bands drawn and counted, in minutes, in one place.
pub const BAND_MINUTES: [u32; 3] = [5, 10, 15];
const LONGEST_BAND_MIN: u32 = BAND_MINUTES[BAND_MINUTES.len() - 1];
/// The furthest a walk of the longest band gets: 1260 m.
pub const REACH_M: f64 = LONGEST_BAND_MIN as f64 * 60.0 * SPEED_M_S;
/// How far from a mapped street or path a walker is assumed to reach: across
/// a square, into a front garden, to a door set back from the street.
pub const OFF_NETWORK_M: f64 = 40.0;

/// The walk around one site, in RD New, before the scene moves it into its frame.
pub struct WalkResult {
    /// One area per band in `BAND_MINUTES`, with `walk_min` set to the band.
    pub bands: Vec<Feature>,
    /// The drawn streets and paths, as lines.
    pub paths: Vec<Feature>,
    /// Places and stops within the longest band, with `walk_min` set.
    pub places: Vec<Feature>,
}

/// The walk from `centre` over the ways in `osm`. `None` when no way passes
/// near enough to the centre to start a walk.
pub fn compute(osm: &Osm, centre: (f64, f64)) -> Option<WalkResult> {
    let graph = Graph::build(&osm.ways);
    let distances = graph.distances_from(centre, REACH_M);
    if distances.is_empty() {
        return None;
    }
    let surface = Surface::build(&graph, &distances, REACH_M)?;

    let limits_m: Vec<f64> = BAND_MINUTES
        .iter()
        .map(|&minutes| metres_for(minutes))
        .collect();
    let bands = surface
        .areas_within(&limits_m)
        .into_iter()
        .zip(BAND_MINUTES)
        .map(|(area, minutes)| Feature {
            layer: Layer::WalkBand,
            geometry: Geometry::MultiPolygon(area),
            attributes: Attributes {
                walk_min: Some(f64::from(minutes)),
                ..Attributes::default()
            },
        })
        .collect();

    let mut places = Vec::new();
    for place in osm.places.iter().chain(&osm.stops) {
        let Geometry::Point(point) = place.geometry else {
            continue;
        };
        let metres = surface.metres_at((point.x(), point.y()));
        if metres <= REACH_M {
            let mut reached = place.clone();
            reached.attributes.walk_min = Some(round_to_tenth(metres / SPEED_M_S / 60.0));
            places.push(reached);
        }
    }

    Some(WalkResult {
        bands,
        paths: drawn_paths(osm, &surface),
        places,
    })
}

/// The ways worth drawing, kept only where they come within the surface, so
/// the context does not run on past the furthest band.
fn drawn_paths(osm: &Osm, surface: &Surface) -> Vec<Feature> {
    osm.ways
        .iter()
        .filter(|way| way.is_drawn)
        .filter(|way| {
            way.points
                .iter()
                .any(|&point| surface.metres_at(point).is_finite())
        })
        .map(|way| Feature {
            layer: Layer::WalkPath,
            geometry: Geometry::LineString(LineString::from(way.points.clone())),
            attributes: Attributes::default(),
        })
        .collect()
}

fn metres_for(minutes: u32) -> f64 {
    f64::from(minutes) * 60.0 * SPEED_M_S
}

fn round_to_tenth(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

/// What lies within each walk band: the 15 minute neighbourhood.
#[derive(Debug, Clone, Serialize)]
pub struct Neighbourhood {
    /// One entry per band in `BAND_MINUTES`. Each counts everything within
    /// that walk, so the 10 minute counts include the 5 minute ones.
    pub bands: Vec<BandCounts>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BandCounts {
    pub minutes: u32,
    pub businesses: Vec<GroupCount>,
    /// Places to go to: education, culture, civic, health, sport and play.
    pub facilities: Vec<GroupCount>,
    /// Places to leave a car or a bike, kept apart from the facilities.
    pub parking: Vec<GroupCount>,
    pub transit_stops: Vec<GroupCount>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupCount {
    /// Stable, snake_case: "food_drink", "education", "tram".
    pub group: String,
    pub label: String,
    pub count: usize,
}

/// Places by group within each walk band, or `None` when the scene has no
/// walk (fetched before walks existed, or OpenStreetMap could not be reached).
pub fn neighbourhood(scene: &Scene) -> Option<Neighbourhood> {
    let walk = scene.walk.as_ref()?;
    let bands = BAND_MINUTES
        .iter()
        .map(|&minutes| {
            let within = |layer: Layer| {
                walk.places.iter().filter(move |place| {
                    place.layer == layer
                        && place
                            .attributes
                            .walk_min
                            .is_some_and(|walk_min| walk_min <= f64::from(minutes))
                })
            };
            BandCounts {
                minutes,
                businesses: counts(within(Layer::Business), &BUSINESS_GROUPS),
                facilities: counts(within(Layer::Facility), &destination_groups()),
                parking: counts(within(Layer::Facility), &parking_groups()),
                transit_stops: counts(within(Layer::TransitStop), &TRANSIT_GROUPS),
            }
        })
        .collect();

    Some(Neighbourhood { bands })
}

/// The facility groups that are not parking, in legend order.
fn destination_groups() -> Vec<(&'static str, &'static str)> {
    FACILITY_GROUPS
        .into_iter()
        .filter(|(group, _)| !PARKING_GROUPS.contains(group))
        .collect()
}

fn parking_groups() -> Vec<(&'static str, &'static str)> {
    FACILITY_GROUPS
        .into_iter()
        .filter(|(group, _)| PARKING_GROUPS.contains(group))
        .collect()
}

fn counts<'a>(
    places: impl Iterator<Item = &'a Feature>,
    groups: &[(&str, &str)],
) -> Vec<GroupCount> {
    groups
        .iter()
        .zip(osm::count_by_group(places, groups))
        .map(|((group, label), count)| GroupCount {
            group: group.to_string(),
            label: label.to_string(),
            count,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::area::{Area, LonLat};
    use crate::scene::Walk;
    use crate::sources::osm::Way;
    use geo::{Point, Polygon};

    fn test_place(layer: Layer, group: &str, (x, y): (f64, f64)) -> Feature {
        Feature {
            layer,
            geometry: Geometry::Point(Point::new(x, y)),
            attributes: Attributes {
                group: Some(group.to_string()),
                ..Attributes::default()
            },
        }
    }

    /// A street running east from the centre for 2 km, with a cafe 300 m
    /// along it, a school 1 km along, and a shop 2 km along.
    fn street_with_places() -> Osm {
        Osm {
            places: vec![
                test_place(Layer::Business, "food_drink", (300.0, 10.0)),
                test_place(Layer::Facility, "education", (1000.0, 0.0)),
                test_place(Layer::Business, "shop", (2000.0, 0.0)),
            ],
            stops: vec![test_place(Layer::TransitStop, "tram", (600.0, 5.0))],
            ways: vec![Way {
                nodes: vec![1, 2, 3],
                points: vec![(0.0, 0.0), (1000.0, 0.0), (2000.0, 0.0)],
                is_drawn: true,
            }],
        }
    }

    #[test]
    fn places_get_their_walk_time_and_places_out_of_reach_are_left_out() {
        let walk = compute(&street_with_places(), (0.0, 0.0)).unwrap();

        let minutes: Vec<f64> = walk
            .places
            .iter()
            .map(|place| place.attributes.walk_min.unwrap())
            .collect();
        // 300 m and 10 m across is 310 m, 3.7 minutes at 1.4 m/s; the cells are 10 m.
        assert!((minutes[0] - 3.7).abs() <= 0.2, "{minutes:?}");
        assert_eq!(walk.places.len(), 3);
        assert_eq!(walk.bands.len(), BAND_MINUTES.len());
    }

    #[test]
    fn the_neighbourhood_counts_each_band_cumulatively() {
        let walk = compute(&street_with_places(), (0.0, 0.0)).unwrap();
        let scene = Scene {
            area: Area::Circle {
                center: LonLat {
                    lon: 4.88,
                    lat: 52.37,
                },
                radius_m: 100.0,
            },
            crs: "EPSG:28992".to_string(),
            origin: (0.0, 0.0),
            boundary: Polygon::new(LineString::new(vec![]), vec![]),
            features: vec![],
            credits: vec![],
            fetched_at: String::new(),
            warnings: vec![],
            measured: Default::default(),
            walk: Some(Walk {
                bands: walk.bands,
                paths: walk.paths,
                places: walk.places,
            }),
            editions: Default::default(),
            context: vec![],
            terrain: None,
            water_bed: None,
            water_levels: Vec::new(),
            solar: None,
            wind: None,
        };

        let neighbourhood = neighbourhood(&scene).unwrap();
        let food = |band: &BandCounts| band.businesses[0].count;
        let schools = |band: &BandCounts| band.facilities[0].count;

        assert_eq!(food(&neighbourhood.bands[0]), 1);
        assert_eq!(schools(&neighbourhood.bands[0]), 0);
        assert_eq!(schools(&neighbourhood.bands[1]), 0);
        // 1000 m is 11.9 minutes, inside the 15 minute band.
        assert_eq!(schools(&neighbourhood.bands[2]), 1);
        assert_eq!(neighbourhood.bands[1].transit_stops[2].count, 1);
    }
}
