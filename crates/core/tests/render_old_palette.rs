mod render_support;

use openarchi_core::catalog::{all_themes, builtin_themes};
use openarchi_core::render::{render_svg, RenderInput};
use render_support::{diagram, look, scene};

/// The Midnight palette as a user saved it before the block and sun-hours layers existed.
fn old_palette_json() -> serde_json::Value {
    let midnight = builtin_themes()
        .into_iter()
        .find(|theme| theme.id == "midnight")
        .expect("midnight");
    let mut value = serde_json::to_value(midnight).expect("theme as JSON");
    value["id"] = "my-midnight".into();
    let layers = value["layers"].as_object_mut().expect("layers");
    layers.remove("block");
    layers.remove("sun_hours");
    value
}

#[test]
fn a_palette_saved_before_computed_layers_draws_them_filled() {
    let folder = tempfile::tempdir().expect("temp folder");
    let json = serde_json::to_string_pretty(&old_palette_json()).expect("json");
    std::fs::write(folder.path().join("my-midnight.json"), json).expect("write");

    let (themes, problems) = all_themes(folder.path());
    assert!(problems.is_empty(), "{problems:?}");
    let theme = themes
        .iter()
        .find(|theme| theme.id == "my-midnight")
        .expect("the user palette loads");
    let scene = scene();
    let look = look();
    let svg_of = |id: &str| {
        render_svg(&RenderInput {
            scene: &scene,
            diagram: &diagram(id),
            theme,
            look: &look,
            pins: &[],
            highlights: &[],
        })
        .expect("render")
    };

    // Each class group carries its fill, so a drawn class means a filled layer.
    assert!(svg_of("city-block").contains(r#"id="layer-block""#));
    assert!(svg_of("city-block:coverage").contains(r#"id="block-class-"#));
    assert!(svg_of("sun-hours").contains(r#"id="sun-hours-class-"#));
}
