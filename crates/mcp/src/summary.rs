//! A short description of a fetched site, so an assistant can talk about it
//! without reading thousands of features.

use std::cmp::Reverse;
use std::collections::BTreeMap;

use openarchi_core::scene::{Layer, Scene};
use serde::Serialize;

/// How many building uses the summary names, most common first.
const TOP_USES: usize = 5;

/// BAG joins mixed use like "woonfunctie, winkelfunctie".
const USE_SEPARATOR: &str = ", ";

#[derive(Debug, Serialize)]
pub struct SiteSummary {
    pub fetched_at: String,
    pub features_per_layer: BTreeMap<Layer, usize>,
    pub buildings: BuildingSummary,
    /// Sources that failed during the fetch. The rest of the site is still there.
    pub warnings: Vec<String>,
    pub sources: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BuildingSummary {
    pub count: usize,
    pub with_height: usize,
    pub without_height: usize,
    pub height_m: Option<Spread>,
    pub with_year: usize,
    pub oldest_year: Option<i32>,
    pub newest_year: Option<i32>,
    /// Most common BAG uses, counting each use of a mixed-use building once.
    pub top_uses: Vec<UseCount>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Spread {
    pub min: f64,
    pub median: f64,
    pub max: f64,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct UseCount {
    pub use_class: String,
    pub count: usize,
}

pub fn summarize(scene: &Scene) -> SiteSummary {
    let mut features_per_layer = BTreeMap::new();
    for feature in &scene.features {
        *features_per_layer.entry(feature.layer).or_insert(0) += 1;
    }

    SiteSummary {
        fetched_at: scene.fetched_at.clone(),
        features_per_layer,
        buildings: summarize_buildings(scene),
        warnings: scene.warnings.clone(),
        sources: scene
            .credits
            .iter()
            .map(|credit| credit.name.clone())
            .collect(),
    }
}

fn summarize_buildings(scene: &Scene) -> BuildingSummary {
    let buildings: Vec<_> = scene
        .features
        .iter()
        .filter(|feature| feature.layer == Layer::Building)
        .map(|feature| &feature.attributes)
        .collect();

    let heights: Vec<f64> = buildings.iter().filter_map(|a| a.height_m).collect();
    let years: Vec<i32> = buildings.iter().filter_map(|a| a.year_built).collect();
    let uses = buildings.iter().filter_map(|a| a.use_class.as_deref());

    BuildingSummary {
        count: buildings.len(),
        with_height: heights.len(),
        without_height: buildings.len() - heights.len(),
        height_m: spread(heights),
        with_year: years.len(),
        oldest_year: years.iter().min().copied(),
        newest_year: years.iter().max().copied(),
        top_uses: top_uses(uses),
    }
}

fn spread(mut values: Vec<f64>) -> Option<Spread> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);

    let middle = values.len() / 2;
    let median = if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    };
    Some(Spread {
        min: values[0],
        median,
        max: values[values.len() - 1],
    })
}

fn top_uses<'a>(use_classes: impl Iterator<Item = &'a str>) -> Vec<UseCount> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for use_class in use_classes.flat_map(|joined| joined.split(USE_SEPARATOR)) {
        *counts.entry(use_class).or_insert(0) += 1;
    }

    let mut ranked: Vec<UseCount> = counts
        .into_iter()
        .map(|(use_class, count)| UseCount {
            use_class: use_class.to_string(),
            count,
        })
        .collect();
    // Stable sort, so ties stay in alphabetical order.
    ranked.sort_by_key(|entry| Reverse(entry.count));
    ranked.truncate(TOP_USES);
    ranked
}

#[cfg(test)]
mod tests {
    use geo::{Geometry, LineString, Point, Polygon};
    use openarchi_core::area::{Area, LonLat};
    use openarchi_core::scene::{Attributes, Feature};

    use super::*;

    fn building(
        height_m: Option<f64>,
        year_built: Option<i32>,
        use_class: Option<&str>,
    ) -> Feature {
        Feature {
            layer: Layer::Building,
            geometry: Geometry::Point(Point::new(0.0, 0.0)),
            attributes: Attributes {
                height_m,
                year_built,
                use_class: use_class.map(str::to_string),
                ..Attributes::default()
            },
        }
    }

    fn scene(features: Vec<Feature>) -> Scene {
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
            boundary: Polygon::new(
                LineString::from(vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]),
                vec![],
            ),
            features,
            credits: vec![],
            fetched_at: "2026-10-24T12:00:00Z".to_string(),
            warnings: vec!["AHN did not answer".to_string()],
        }
    }

    #[test]
    fn buildings_are_counted_with_and_without_height() {
        let site = scene(vec![
            building(Some(9.0), Some(1890), Some("woonfunctie")),
            building(Some(3.0), Some(1650), Some("woonfunctie, winkelfunctie")),
            building(Some(30.0), None, Some("kantoorfunctie")),
            building(None, Some(2004), Some("woonfunctie")),
        ]);

        let summary = summarize(&site);
        let buildings = summary.buildings;

        assert_eq!(buildings.count, 4);
        assert_eq!(buildings.with_height, 3);
        assert_eq!(buildings.without_height, 1);
        assert_eq!(
            buildings.height_m,
            Some(Spread {
                min: 3.0,
                median: 9.0,
                max: 30.0
            })
        );
        assert_eq!(
            (buildings.oldest_year, buildings.newest_year),
            (Some(1650), Some(2004))
        );
        assert_eq!(
            buildings.top_uses[0],
            UseCount {
                use_class: "woonfunctie".to_string(),
                count: 3
            }
        );
        assert_eq!(summary.features_per_layer[&Layer::Building], 4);
        assert_eq!(summary.warnings, vec!["AHN did not answer"]);
    }

    #[test]
    fn median_of_an_even_count_is_the_mean_of_the_middle_two() {
        let middle = spread(vec![4.0, 1.0, 10.0, 6.0]).unwrap();
        assert_eq!(middle.median, 5.0);
    }

    #[test]
    fn site_without_buildings_has_no_height_spread() {
        let summary = summarize(&scene(vec![]));
        assert_eq!(summary.buildings.count, 0);
        assert_eq!(summary.buildings.height_m, None);
        assert!(summary.buildings.top_uses.is_empty());
    }
}
