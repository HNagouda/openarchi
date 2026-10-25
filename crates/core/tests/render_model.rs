mod dxf_support;
mod render_support;

use std::path::Path;

use dxf_support::{dxf_with_model, Sheet};
use openarchi_core::export::DxfLayers;
use openarchi_core::model::{self, Model, Placement};
use openarchi_core::project::Look;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::scene::{Layer, Scene};
use openarchi_core::view::{SunMoment, ViewMode};
use render_support::{diagram, look, scene, theme, ORIGIN};

/// The 32 m office at (-20, 80), given an id so the model can replace it.
const OFFICE_ID: &str = "office";

fn site() -> Scene {
    let mut scene = scene();
    let office = scene
        .features
        .iter_mut()
        .find(|feature| {
            feature.layer == Layer::Building && feature.attributes.height_m == Some(32.0)
        })
        .expect("the office");
    office.attributes.source_id = Some(OFFICE_ID.to_string());
    scene
}

/// The gable house standing in the empty ground north of the canal.
fn house(scene: &Scene) -> Model {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/models/gable-house.glb");
    let placement = Placement {
        x: ORIGIN.0 + 100.0,
        y: ORIGIN.1 + 40.0,
        rotation_deg: 0.0,
        base_m: None,
    };
    model::import(&path, None, None, scene, Some(placement)).expect("import the house")
}

fn look_in(mode: ViewMode, hour: u32) -> Look {
    let mut look = look();
    look.view.mode = mode;
    look.sun = SunMoment {
        hour,
        ..SunMoment::default()
    };
    look
}

fn svg(scene: &Scene, diagram_id: &str, look: &Look, model: Option<&Model>) -> String {
    let (diagram, theme) = (diagram(diagram_id), theme("paper"));
    render_svg(&RenderInput {
        scene,
        diagram: &diagram,
        theme: &theme,
        look,
        pins: &[],
        highlights: &[],
        model,
    })
    .expect("render")
}

/// The `d` of the path inside the group with `id`.
fn group_path<'a>(svg: &'a str, id: &str) -> &'a str {
    let group = &svg[svg.find(&format!(r#"id="{id}""#)).expect("the group")..];
    let start = group.find(r#" d=""#).expect("a path") + 4;
    &group[start..start + group[start..].find('"').unwrap()]
}

fn paths_in_solids(svg: &str) -> usize {
    let solids = &svg[svg.find(r#"id="solids""#).expect("the solids")..];
    solids[..solids.find("</g>\n").unwrap()]
        .matches("<path")
        .count()
}

#[test]
fn the_plan_draws_the_model_footprint_and_names_it_in_the_legend() {
    let scene = site();
    let model = house(&scene);

    let svg = svg(
        &scene,
        "context",
        &look_in(ViewMode::Plan, 13),
        Some(&model),
    );

    assert!(svg.contains(r#"id="proposal""#));
    assert!(svg.contains(">Proposal<"));
    assert!(usvg::Tree::from_str(&svg, &usvg::Options::default()).is_ok());
}

#[test]
fn a_diagram_without_buildings_leaves_the_model_out() {
    let scene = site();
    let model = house(&scene);

    let svg = svg(
        &scene,
        "open-space",
        &look_in(ViewMode::Plan, 13),
        Some(&model),
    );

    assert!(!svg.contains(r#"id="proposal""#));
    assert!(!svg.contains(">Proposal<"));
}

#[test]
fn the_model_stands_among_the_buildings_in_a_tilted_view() {
    let scene = site();
    let model = house(&scene);
    let look = look_in(ViewMode::Axonometric, 13);

    let with = svg(&scene, "context", &look, Some(&model));
    let without = svg(&scene, "context", &look, None);

    // From the south and above: the floor and the north side are hidden, the
    // south gable, the east and west walls and both roof planes may show.
    let added = paths_in_solids(&with) - paths_in_solids(&without);
    assert!((3..=5).contains(&added), "{added} faces drawn");
}

#[test]
fn a_replaced_building_is_left_out_of_every_view() {
    let scene = site();
    let mut model = house(&scene);
    let look = look_in(ViewMode::Axonometric, 13);
    let kept = svg(&scene, "context", &look, Some(&model));

    model.replaced = vec![OFFICE_ID.to_string()];
    let replaced = svg(&scene, "context", &look, Some(&model));

    assert!(paths_in_solids(&replaced) < paths_in_solids(&kept));
}

#[test]
fn the_model_casts_a_shadow_in_the_shadow_diagram() {
    let scene = site();
    let model = house(&scene);
    let look = look_in(ViewMode::Plan, 16);

    let with = svg(&scene, "shadow", &look, Some(&model));
    let without = svg(&scene, "shadow", &look, None);

    assert!(group_path(&with, "shadows").len() > group_path(&without, "shadows").len());
}

#[test]
fn the_dxf_puts_the_model_on_its_own_layer_in_place_of_what_it_replaces() {
    let scene = site();
    let mut model = house(&scene);
    let look = look_in(ViewMode::Plan, 13);
    let sheet = Sheet {
        diagram_id: "context",
        theme_id: "paper",
        look: &look,
        dxf_layers: DxfLayers::Shown,
    };
    let buildings = |dxf: &dxf_support::Dxf| {
        let is_building = |entity: &&dxf_support::Entity| entity.layer.starts_with("OA-BUILDING");
        dxf.entities.iter().filter(is_building).count()
    };
    let kept = dxf_with_model(&scene, &sheet, &model);

    model.replaced = vec![OFFICE_ID.to_string()];
    let replaced = dxf_with_model(&scene, &sheet, &model);

    assert!(replaced.layer("OA-PROPOSAL").is_some());
    assert_eq!(replaced.on("OA-PROPOSAL").len(), 1);
    assert_eq!(buildings(&replaced), buildings(&kept) - 1);
}
