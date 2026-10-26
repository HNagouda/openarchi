use std::path::Path;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use geo::{Geometry, Rect};

use super::*;
use crate::area::{Area, LonLat};
use crate::catalog::{builtin_diagrams, builtin_themes};
use crate::model::{Model, Placement};
use crate::project::Look;
use crate::render::luminance;
use crate::scene::Attributes;
use crate::theme::Theme;

fn building(id: &str, min: (f64, f64), max: (f64, f64), height_m: f64) -> Feature {
    Feature {
        layer: Layer::Building,
        geometry: Geometry::Polygon(Rect::new(min, max).to_polygon()),
        attributes: Attributes {
            source_id: Some(id.to_string()),
            height_m: Some(height_m),
            ..Default::default()
        },
    }
}

fn site(features: Vec<Feature>) -> Scene {
    Scene {
        area: Area::Circle {
            center: LonLat {
                lon: 4.8837,
                lat: 52.3747,
            },
            radius_m: 100.0,
        },
        crs: "EPSG:28992".to_string(),
        origin: (120_000.0, 487_000.0),
        boundary: Rect::new((-100.0, -100.0), (100.0, 100.0)).to_polygon(),
        features,
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

fn theme(id: &str) -> Theme {
    builtin_themes()
        .into_iter()
        .find(|theme| theme.id == id)
        .unwrap()
}

fn paper() -> Theme {
    builtin_themes()
        .into_iter()
        .find(|theme| theme.id == "paper")
        .unwrap()
}

/// The 3D view of `scene` opened from a sheet of `diagram_id` in the paper palette.
fn view_of(scene: &Scene, diagram_id: &str, model: Option<&Model>) -> View3d {
    view_in(scene, diagram_id, "paper", model)
}

fn view_in(scene: &Scene, diagram_id: &str, theme_id: &str, model: Option<&Model>) -> View3d {
    let diagram = builtin_diagrams()
        .into_iter()
        .find(|diagram| diagram.id == diagram_id)
        .unwrap();
    let theme = theme(theme_id);
    let look = Look {
        diagram_id: diagram_id.to_string(),
        ..Look::default()
    };
    let input = RenderInput {
        scene,
        diagram: &diagram,
        theme: &theme,
        look: &look,
        pins: &[],
        highlights: &[],
        model,
    };
    view(&input).unwrap()
}

fn built_in(mut feature: Feature, year: i32) -> Feature {
    feature.attributes.year_built = Some(year);
    feature
}

fn road(min: (f64, f64), max: (f64, f64)) -> Feature {
    Feature {
        layer: Layer::Road,
        geometry: Geometry::Polygon(Rect::new(min, max).to_polygon()),
        attributes: Attributes::default(),
    }
}

/// Every triangle's corners from a surface's packed text.
fn corners(surface: &Surface) -> Vec<[[f32; 3]; 3]> {
    let bytes = BASE64.decode(&surface.corners).unwrap();
    let values: Vec<f32> = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| f32::from_le_bytes(*chunk))
        .collect();
    values
        .as_chunks::<9>()
        .0
        .iter()
        .map(|t| [[t[0], t[1], t[2]], [t[3], t[4], t[5]], [t[6], t[7], t[8]]])
        .collect()
}

fn normal([a, b, c]: [[f32; 3]; 3]) -> [f32; 3] {
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ]
}

#[test]
fn a_building_stands_as_four_walls_facing_out_under_a_roof_facing_up() {
    let scene = site(vec![building("a", (0.0, 0.0), (10.0, 10.0), 12.0)]);

    let view = view_of(&scene, "context", None);
    let triangles = corners(&view.buildings[0]);

    assert_eq!(view.buildings.len(), 1);
    assert_eq!(view.buildings[0].triangle_count, 10);
    for triangle in triangles {
        let normal = normal(triangle);
        let middle = [0, 1, 2].map(|axis| triangle.iter().map(|c| c[axis]).sum::<f32>() / 3.0);
        let is_roof = triangle.iter().all(|c| c[2] == 12.0);
        if is_roof {
            assert!(normal[2] > 0.0);
        } else {
            let outward = (middle[0] - 5.0) * normal[0] + (middle[1] - 5.0) * normal[1];
            assert!(outward > 0.0, "a wall faces in: {triangle:?}");
        }
    }
}

#[test]
fn the_ground_faces_up_and_the_site_is_framed_by_its_corners() {
    let view = view_of(&site(Vec::new()), "context", None);

    assert!((view.radius_m - 100.0 * 2f64.sqrt()).abs() < 1e-9);
    for triangle in corners(&view.ground[0]) {
        assert!(normal(triangle)[2] > 0.0);
    }
}

#[test]
fn a_replaced_building_is_left_out_and_the_model_stands_in_its_place() {
    let scene = site(vec![
        building("old", (-4.0, -5.0), (4.0, 5.0), 9.0),
        building("kept", (40.0, 40.0), (50.0, 50.0), 9.0),
    ]);
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/models/gable-house.glb");
    let model = model::import(
        &path,
        None,
        None,
        &scene,
        Some(Placement::centre_of(&scene)),
    )
    .unwrap();

    let view = view_of(&scene, "context", Some(&model));

    assert_eq!(model.replaced, vec!["old".to_string()]);
    assert_eq!(view.buildings[0].triangle_count, 10);
    let shown = view.model.unwrap();
    assert_eq!(shown.triangle_count, model.mesh.triangles.len());
    assert_eq!(shown.colour, paper().palette.accent);
}

#[test]
fn an_age_sheet_colours_old_and_new_buildings_apart_as_its_roofs_do() {
    let scene = site(vec![
        built_in(building("old", (0.0, 0.0), (10.0, 10.0), 12.0), 1650),
        built_in(building("new", (30.0, 0.0), (40.0, 10.0), 12.0), 2010),
        built_in(building("newer", (60.0, 0.0), (70.0, 10.0), 12.0), 2015),
    ]);

    let plain = view_of(&scene, "context", None);
    let by_age = view_of(&scene, "building-age", None);

    assert_eq!(plain.buildings.len(), 1);
    assert_eq!(by_age.buildings.len(), 2);
    assert!(by_age
        .buildings
        .iter()
        .all(|surface| surface.pieces.iter().all(|&count| count == 10)));
    let triangles: usize = by_age
        .buildings
        .iter()
        .map(|surface| surface.triangle_count)
        .sum();
    assert_eq!(triangles, 30);
}

#[test]
fn a_building_of_unknown_height_is_greyed_like_on_the_sheet() {
    let mut guessed = building("guessed", (30.0, 0.0), (40.0, 10.0), 12.0);
    guessed.attributes.height_m = None;
    let scene = site(vec![
        building("measured", (0.0, 0.0), (10.0, 10.0), 12.0),
        guessed,
    ]);

    let view = view_of(&scene, "context", None);

    assert_eq!(view.buildings.len(), 2);
}

#[test]
fn the_neighbourhood_ground_is_muted_and_keeps_its_layer() {
    let mut scene = site(vec![road((-10.0, -10.0), (10.0, 10.0))]);
    scene.context = vec![road((150.0, -10.0), (170.0, 10.0))];

    let view = view_of(&scene, "context", None);

    let inside = view
        .ground
        .iter()
        .find(|surface| surface.layer == Some(Layer::Road))
        .unwrap();
    let around = &view.surroundings[0];
    assert_eq!(around.layer, Some(Layer::Road));
    assert_ne!(around.colour, inside.colour);
    assert_eq!(view.ground[0].layer, None);
}

#[test]
fn the_sun_through_the_day_is_the_sun_the_sheets_draw() {
    let scene = site(Vec::new());

    let day = sun_day(&scene, 6, 21);
    let noon = &day.steps[(13 * 60 / SUN_STEP_MINUTES) as usize];
    let sheet = sun::position_at(
        sun::site_location(&scene.area),
        &SunMoment {
            month: 6,
            day: 21,
            hour: 13,
            minute: 0,
        },
    );

    assert_eq!(day.steps.len(), 145);
    assert_eq!(noon.minute, 13 * 60);
    assert_eq!(
        (noon.azimuth_deg, noon.altitude_deg),
        (sheet.azimuth_deg, sheet.altitude_deg)
    );
    assert!(day.steps[0].altitude_deg < 0.0);
}

#[test]
fn in_a_palette_of_outlined_buildings_the_buildings_stand_in_the_outline_colour() {
    let mut scene = site(vec![building("a", (0.0, 0.0), (10.0, 10.0), 12.0)]);
    scene.context = vec![building("b", (150.0, 0.0), (160.0, 10.0), 12.0)];
    let paper_of = |id: &str| luminance(&theme(id).palette.paper);

    let ink = view_in(&scene, "context", "ink", None);

    let building = luminance(&ink.buildings[0].colour);
    let neighbour = luminance(&ink.neighbours.colour);
    assert!(
        building - paper_of("ink") > 0.5,
        "{}",
        ink.buildings[0].colour
    );
    assert!(
        neighbour - paper_of("ink") > 0.05,
        "{}",
        ink.neighbours.colour
    );
}
