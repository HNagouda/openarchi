mod render_support;

use openarchi_core::edits::{edited, SiteEdits};
use openarchi_core::project::Look;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::scene::{Layer, Scene};
use openarchi_core::view::{SunMoment, ViewMode};
use render_support::{diagram, look, scene, theme, ORIGIN};

/// The test site with its two trees and the office named, as BGT and BAG name them.
fn site() -> Scene {
    let mut scene = scene();
    let mut trees = 0;
    for feature in &mut scene.features {
        if feature.layer == Layer::Tree {
            trees += 1;
            feature.attributes.source_id = Some(format!("tree-{trees}"));
        }
        if feature.attributes.height_m == Some(32.0) {
            feature.attributes.source_id = Some("office".to_string());
        }
    }
    scene
}

fn look_in(mode: ViewMode) -> Look {
    let mut look = look();
    look.view.mode = mode;
    look.sun = SunMoment {
        hour: 16,
        ..SunMoment::default()
    };
    look
}

fn svg(scene: &Scene, diagram_id: &str, look: &Look) -> String {
    let (diagram, theme) = (diagram(diagram_id), theme("paper"));
    render_svg(&RenderInput {
        scene,
        diagram: &diagram,
        theme: &theme,
        look,
        pins: &[],
        highlights: &[],
        model: None,
    })
    .expect("render")
}

/// What the group labelled `label` draws.
fn group<'a>(svg: &'a str, label: &str) -> &'a str {
    let Some(start) = svg.find(&format!(r#"inkscape:label="{label}""#)) else {
        return "";
    };
    let group = &svg[start..];
    &group[..group.find("</g>").unwrap()]
}

fn circles(svg: &str, label: &str) -> usize {
    group(svg, label).matches("<circle").count()
}

#[test]
fn a_sheet_without_edits_draws_the_scene_as_fetched() {
    let scene = site();
    let look = look_in(ViewMode::Plan);

    let plain = svg(&scene, "context", &look);
    let unedited = svg(&edited(&scene, &SiteEdits::default()), "context", &look);

    assert_eq!(plain, unedited);
}

#[test]
fn a_removed_tree_and_building_leave_the_plan() {
    let scene = site();
    let look = look_in(ViewMode::Plan);
    let edits = SiteEdits::default()
        .with_removed("tree-1")
        .with_removed("office");

    let before = svg(&scene, "context", &look);
    let after = svg(&edited(&scene, &edits), "context", &look);

    assert_eq!(circles(&before, "Trees"), 2);
    assert_eq!(circles(&after, "Trees"), 1);
    assert!(group(&after, "Buildings").len() < group(&before, "Buildings").len());
}

#[test]
fn a_moved_tree_is_drawn_where_it_now_stands() {
    let scene = site();
    let look = look_in(ViewMode::Plan);
    // Tree 2 stands at (70, 50); 40 m further east is 16 mm on this sheet.
    let edits = SiteEdits::default().with_moved("tree-2", ORIGIN.0 + 110.0, ORIGIN.1 + 50.0);

    let before = svg(&scene, "context", &look);
    let after = svg(&edited(&scene, &edits), "context", &look);

    assert!(group(&before, "Trees").contains(r#"cx="188""#));
    assert!(group(&after, "Trees").contains(r#"cx="204""#));
    assert_eq!(circles(&after, "Trees"), 2);
}

#[test]
fn a_planted_tree_casts_a_shadow_and_stands_up_in_the_axonometric() {
    let scene = site();
    let (edits, _) = SiteEdits::default().with_planted(ORIGIN.0 - 150.0, ORIGIN.1 - 150.0);
    let planted = edited(&scene, &edits);

    let shadow = look_in(ViewMode::Plan);
    let before = svg(&scene, "shadow", &shadow);
    let after = svg(&planted, "shadow", &shadow);
    assert_eq!(circles(&after, "Trees"), 3);
    assert!(group(&after, "Shadows").len() > group(&before, "Shadows").len());

    let tilted = look_in(ViewMode::Axonometric);
    let before = svg(&scene, "context", &tilted);
    let after = svg(&planted, "context", &tilted);
    assert!(after.len() > before.len());
}
