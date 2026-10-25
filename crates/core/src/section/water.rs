//! The waters a section line crosses, and at what level each stands.
//!
//! A water's level comes from a published target level when a fairway line
//! with one runs through it. Without one, the water is drawn at the lower of
//! its two banks: AHN measures the ground down to the water's edge, so the
//! bank is the nearest real evidence of where the surface lies, but it is not
//! a level, and the legend says so. The banks are read from the terrain alone,
//! bridged across the water, because a river cut into several polygons has
//! ends in mid-stream, where the surveyed bed would put the surface on the bottom.

use geo::{Geometry, Intersects};

use super::cut::{self, Stretch};
use super::profile::{self, Profile};
use super::Axis;
use crate::scene::{Layer, Scene};

/// Bed heights are looked for along a water every this many metres.
const BED_PROBE_M: f64 = 1.0;
/// Pieces of water closer than this along the line are one water. The site
/// and its neighbourhood cut one canal into two polygons at the boundary.
const TOUCHING_M: f64 = 0.05;

/// One water the line crosses.
#[derive(Debug, Clone, PartialEq)]
pub struct Water {
    pub stretch: Stretch,
    /// The surface, in metres above NAP.
    pub level_m: f64,
    /// The published target level's name, or `None` when drawn at the bank.
    pub level_name: Option<String>,
    /// True when the surveyed bed reaches this stretch; the profile then follows it.
    pub has_bed: bool,
}

/// Every water the line crosses, A to A'. Empty for a scene without terrain,
/// whose flat ground has no level to put water at.
pub fn crossed(scene: &Scene, axis: &Axis) -> Vec<Water> {
    let banks = profile::ground_profile(scene.terrain.as_ref(), None, axis);
    if !banks.is_measured {
        return Vec::new();
    }
    let mut pieces: Vec<(Stretch, &Geometry<f64>)> = Vec::new();
    for feature in scene.features.iter().chain(&scene.context) {
        if feature.layer != Layer::Water {
            continue;
        }
        for stretch in cut::cut_stretches(&feature.geometry, axis) {
            pieces.push((stretch, &feature.geometry));
        }
    }
    pieces.sort_by(|a, b| a.0.from.total_cmp(&b.0.from));

    let mut joined: Vec<(Stretch, Vec<&Geometry<f64>>)> = Vec::new();
    for (stretch, geometry) in pieces {
        match joined.last_mut() {
            Some((last, geometries)) if stretch.from <= last.to + TOUCHING_M => {
                last.to = last.to.max(stretch.to);
                geometries.push(geometry);
            }
            _ => joined.push((stretch, vec![geometry])),
        }
    }

    joined
        .into_iter()
        .map(|(stretch, geometries)| water(scene, axis, &banks, &geometries, stretch))
        .collect()
}

fn water(
    scene: &Scene,
    axis: &Axis,
    banks: &Profile,
    geometries: &[&Geometry<f64>],
    stretch: Stretch,
) -> Water {
    // #! NOTE[sections, water] : Two fairways with different target levels in one water (at a lock) take the first; split the water at the lock?
    let published = scene.water_levels.iter().find(|level| {
        geometries
            .iter()
            .any(|geometry| level.line.intersects(*geometry))
    });
    let (level_m, level_name) = match published {
        Some(level) => (level.level_m, Some(level.name.clone())),
        None => (
            banks
                .height_at(stretch.from)
                .min(banks.height_at(stretch.to)),
            None,
        ),
    };

    Water {
        stretch,
        level_m,
        level_name,
        has_bed: has_bed(scene, axis, stretch),
    }
}

fn has_bed(scene: &Scene, axis: &Axis, stretch: Stretch) -> bool {
    let Some(bed) = &scene.water_bed else {
        return false;
    };
    let steps = ((stretch.to - stretch.from) / BED_PROBE_M).ceil() as usize;
    (0..=steps).any(|step| {
        let along = (stretch.from + step as f64 * BED_PROBE_M).min(stretch.to);
        let (x, y) = axis.point_at(along, 0.0);
        profile::height_at(bed, x, y).is_some()
    })
}

#[cfg(test)]
mod tests {
    use geo::{polygon, LineString, Polygon};

    use super::*;
    use crate::area::{Area, LonLat};
    use crate::scene::{Attributes, Feature, WaterLevel};
    use crate::section::SectionLine;
    use crate::sources::raster::Grid;

    /// Flat ground at +1 over 200 m, with a canal from x 40 to 60.
    fn scene() -> Scene {
        let canal = Feature {
            layer: Layer::Water,
            geometry: Geometry::Polygon(polygon![
                (x: 40.0, y: -50.0),
                (x: 60.0, y: -50.0),
                (x: 60.0, y: 50.0),
                (x: 40.0, y: 50.0),
            ]),
            attributes: Attributes::default(),
        };
        Scene {
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
            features: vec![canal],
            credits: vec![],
            fetched_at: String::new(),
            warnings: vec![],
            measured: Default::default(),
            walk: None,
            editions: Default::default(),
            context: vec![],
            terrain: Some(Grid {
                west: -100.0,
                north: 100.0,
                cell_m: 4.0,
                width: 50,
                height: 50,
                values: vec![1.0; 2500],
            }),
            water_bed: None,
            water_levels: Vec::new(),
        }
    }

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

    #[test]
    fn a_canal_without_a_published_level_stands_at_its_bank() {
        let scene = scene();
        let waters = crossed(&scene, &axis());
        assert_eq!(waters.len(), 1);
        assert!((waters[0].stretch.from - 40.0).abs() < 1e-9);
        assert!((waters[0].level_m - 1.0).abs() < 1e-9);
        assert_eq!(waters[0].level_name, None);
        assert!(!waters[0].has_bed);
    }

    #[test]
    fn a_fairway_through_the_canal_gives_its_level() {
        let mut scene = scene();
        scene.water_levels.push(WaterLevel {
            name: "Amstellands boezempeil".to_string(),
            level_m: -0.4,
            line: LineString::from(vec![(50.0, -80.0), (50.0, 80.0)]),
        });
        scene.water_bed = Some(Grid {
            west: 40.0,
            north: 50.0,
            cell_m: 4.0,
            width: 5,
            height: 25,
            values: vec![-3.0; 125],
        });
        let waters = crossed(&scene, &axis());
        assert_eq!(waters[0].level_m, -0.4);
        assert!(waters[0].has_bed);
    }
}
