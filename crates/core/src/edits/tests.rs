use std::borrow::Cow;

use geo::{Geometry, Point};

use super::*;
use crate::area::{Area as SiteArea, LonLat};
use crate::scene::{Attributes, Feature, Layer, Scene};

const ORIGIN: (f64, f64) = (120_000.0, 487_000.0);

/// A 300 m square site with two trees and a building, its origin at a round RD New point.
fn site() -> Scene {
    Scene {
        area: SiteArea::Circle {
            center: LonLat {
                lon: 4.8837,
                lat: 52.3747,
            },
            radius_m: 150.0,
        },
        crs: "EPSG:28992".to_string(),
        origin: ORIGIN,
        boundary: geo::Rect::new((-150.0, -150.0), (150.0, 150.0)).to_polygon(),
        features: vec![
            tree("G0363.oak", 10.0, 10.0),
            tree("G0363.elm", -20.0, 5.0),
            building("0363100012164990"),
        ],
        credits: Vec::new(),
        fetched_at: String::new(),
        warnings: Vec::new(),
        measured: Default::default(),
        walk: None,
        editions: Default::default(),
        context: Vec::new(),
        terrain: None,
        water_bed: None,
        water_levels: Vec::new(),
        solar: None,
        wind: None,
    }
}

fn tree(id: &str, x: f64, y: f64) -> Feature {
    Feature {
        layer: Layer::Tree,
        geometry: Geometry::Point(Point::new(x, y)),
        attributes: Attributes {
            source_id: Some(id.to_string()),
            kind: Some("boom".to_string()),
            ..Default::default()
        },
    }
}

fn building(id: &str) -> Feature {
    Feature {
        layer: Layer::Building,
        geometry: Geometry::Polygon(geo::Rect::new((30.0, 30.0), (40.0, 45.0)).to_polygon()),
        attributes: Attributes {
            source_id: Some(id.to_string()),
            height_m: Some(12.0),
            ..Default::default()
        },
    }
}

fn ids(scene: &Scene) -> Vec<&str> {
    scene
        .features
        .iter()
        .filter_map(|feature| feature.attributes.source_id.as_deref())
        .collect()
}

fn point_of<'a>(scene: &'a Scene, id: &str) -> &'a Geometry<f64> {
    &scene
        .features
        .iter()
        .find(|feature| feature.attributes.source_id.as_deref() == Some(id))
        .unwrap()
        .geometry
}

#[test]
fn without_edits_the_scene_is_the_same_scene() {
    let scene = site();

    assert!(matches!(
        edited(&scene, &SiteEdits::default()),
        Cow::Borrowed(_)
    ));
}

#[test]
fn a_removed_tree_and_building_leave_the_scene() {
    let scene = site();
    let edits = SiteEdits::default()
        .with_removed("G0363.oak")
        .with_removed("0363100012164990");

    assert_eq!(ids(&edited(&scene, &edits)), vec!["G0363.elm"]);
}

#[test]
fn a_moved_tree_stands_at_its_new_point_in_the_local_frame() {
    let scene = site();
    let edits = SiteEdits::default().with_moved("G0363.oak", ORIGIN.0 + 50.0, ORIGIN.1 - 25.0);

    let moved = edited(&scene, &edits);

    assert_eq!(
        point_of(&moved, "G0363.oak"),
        &Geometry::Point(Point::new(50.0, -25.0))
    );
}

#[test]
fn a_planted_tree_joins_the_trees_and_removing_it_takes_it_away() {
    let scene = site();
    let (edits, id) = SiteEdits::default().with_planted(ORIGIN.0, ORIGIN.1);

    let planted = edited(&scene, &edits);
    assert_eq!(id, "planted-1");
    assert_eq!(
        point_of(&planted, "planted-1"),
        &Geometry::Point(Point::new(0.0, 0.0))
    );

    let removed = edits.with_removed(&id);
    assert!(removed.is_empty());
}

#[test]
fn restoring_brings_a_tree_back_where_the_survey_has_it() {
    let edits = SiteEdits::default()
        .with_moved("G0363.oak", ORIGIN.0, ORIGIN.1)
        .with_removed("G0363.oak");

    assert!(edits.with_restored("G0363.oak").is_empty());
}

#[test]
fn moving_a_moved_tree_again_keeps_one_edit() {
    let edits = SiteEdits::default()
        .with_moved("G0363.oak", ORIGIN.0, ORIGIN.1)
        .with_moved("G0363.oak", ORIGIN.0 + 1.0, ORIGIN.1);

    assert_eq!(edits.moved.len(), 1);
    assert_eq!(edits.moved[0].x, ORIGIN.0 + 1.0);
}

#[test]
fn the_next_planted_id_skips_ids_in_use() {
    let (edits, _) = SiteEdits::default().with_planted(ORIGIN.0, ORIGIN.1);
    let (edits, _) = edits.with_planted(ORIGIN.0, ORIGIN.1);
    let edits = edits.with_removed("planted-1");

    let (_, id) = edits.with_planted(ORIGIN.0, ORIGIN.1);

    assert_eq!(id, "planted-3");
}

#[test]
fn model_picks_are_replaced_on_a_new_place_but_hand_removals_stay() {
    let edits = SiteEdits::default()
        .with_removed("G0363.elm")
        .with_model_picks(&["G0363.oak".to_string(), "G0363.elm".to_string()]);
    assert_eq!(edits.removed.len(), 2);
    assert!(!edits.removed[0].is_under_model);

    let moved_away = edits.with_model_picks(&[]);
    assert_eq!(moved_away.removed.len(), 1);
    assert_eq!(moved_away.removed[0].id, "G0363.elm");
    assert_eq!(edits.without_model_picks(), moved_away);
}

#[test]
fn edits_of_trees_gone_after_a_new_fetch_are_dropped() {
    let mut scene = site();
    let (edits, _) = SiteEdits::default()
        .with_removed("G0363.oak")
        .with_moved("G0363.elm", ORIGIN.0, ORIGIN.1)
        .with_planted(ORIGIN.0 + 500.0, ORIGIN.1);
    scene
        .features
        .retain(|feature| feature.layer == Layer::Building);

    // The planted tree is outside the 300 m square, so it goes too.
    assert!(edits.kept_in(&scene).is_empty());
    assert_eq!(edits.kept_in(&site()).removed.len(), 1);
}

#[test]
fn a_tree_outside_the_site_is_refused() {
    let scene = site();
    let (outside, _) = SiteEdits::default().with_planted(ORIGIN.0 + 500.0, ORIGIN.1);
    let (inside, _) = SiteEdits::default().with_planted(ORIGIN.0 + 5.0, ORIGIN.1);

    assert!(outside.check(&scene).is_err());
    assert!(inside.check(&scene).is_ok());
    assert!(SiteEdits::default()
        .with_moved("G0363.oak", f64::NAN, ORIGIN.1)
        .check(&scene)
        .is_err());
}

#[test]
fn standing_trees_follow_moves_and_plantings() {
    let scene = site();
    let (edits, _) = SiteEdits::default()
        .with_moved("G0363.oak", ORIGIN.0 + 1.0, ORIGIN.1 + 2.0)
        .with_planted(ORIGIN.0, ORIGIN.1);

    let trees = standing_trees(&scene, &edits);

    assert_eq!(trees.len(), 3);
    assert_eq!(trees[0].at, Point::new(1.0, 2.0));
    assert!(trees[2].is_planted);
}

#[test]
fn only_trees_and_buildings_count_as_surveyed() {
    let mut scene = site();
    scene.features[0].layer = Layer::Road;

    assert_eq!(surveyed(&scene, "G0363.oak"), None);
    assert_eq!(surveyed(&scene, "0363100012164990"), Some(Layer::Building));
}

#[test]
fn a_point_near_a_tree_picks_the_tree_and_inside_a_roof_the_building() {
    let scene = site();
    let edits = SiteEdits::default();

    let tree = editable_at(&scene, &edits, ORIGIN.0 + 11.0, ORIGIN.1 + 10.0, 3.0).unwrap();
    let building = editable_at(&scene, &edits, ORIGIN.0 + 35.0, ORIGIN.1 + 40.0, 3.0).unwrap();
    let nothing = editable_at(&scene, &edits, ORIGIN.0 - 100.0, ORIGIN.1, 3.0);

    assert_eq!(tree.id, "G0363.oak");
    assert_eq!(tree.at, Some([ORIGIN.0 + 10.0, ORIGIN.1 + 10.0]));
    assert_eq!(tree.kind.as_deref(), Some("boom"));
    assert_eq!(building.id, "0363100012164990");
    assert_eq!(building.outline[0].len(), 5);
    assert!(nothing.is_none());
}

#[test]
fn a_removed_tree_can_still_be_picked_to_restore_it() {
    let scene = site();
    let edits = SiteEdits::default().with_removed("G0363.oak");

    let tree = editable_at(&scene, &edits, ORIGIN.0 + 10.0, ORIGIN.1 + 10.0, 3.0).unwrap();

    assert!(tree.is_removed);
}

#[test]
fn every_edit_is_listed_with_where_a_moved_tree_stood() {
    let scene = site();
    let (edits, _) = SiteEdits::default()
        .with_removed("0363100012164990")
        .with_removed("gone-after-a-fetch")
        .with_moved("G0363.elm", ORIGIN.0, ORIGIN.1)
        .with_planted(ORIGIN.0 + 1.0, ORIGIN.1);

    let things = edited_things(&scene, &edits);

    let ids: Vec<&str> = things.iter().map(|thing| thing.id.as_str()).collect();
    assert_eq!(ids, vec!["0363100012164990", "G0363.elm", "planted-1"]);
    assert_eq!(
        things[1].surveyed_at,
        Some([ORIGIN.0 - 20.0, ORIGIN.1 + 5.0])
    );
    assert_eq!(things[1].at, Some([ORIGIN.0, ORIGIN.1]));
    assert!(things[2].is_planted);
}

#[test]
fn ticking_a_box_takes_away_under_the_model_and_unticking_keeps_a_tree_moved() {
    let edits = SiteEdits::default()
        .with_moved("G0363.oak", ORIGIN.0, ORIGIN.1)
        .with_ticked("G0363.oak", true);
    assert!(edits.removed[0].is_under_model);

    let kept = edits.with_ticked("G0363.oak", false);
    assert!(kept.removed.is_empty());
    assert_eq!(kept.moved.len(), 1);
}
