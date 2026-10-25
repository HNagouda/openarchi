//! The DXF of a plan sheet read back for its two options: only the layers the
//! sheet shows or every fetched layer, and each CAD layer in its sheet colour
//! as the nearest AutoCAD colour.

mod dxf_support;
mod render_support;

use dxf_support::{dxf_of, dxf_with, Dxf, Sheet};
use openarchi_core::export::{aci, DxfLayers};
use openarchi_core::highlight::{Highlight, HighlightShape, Outline};
use openarchi_core::project::Look;
use openarchi_core::scene::Layer;
use openarchi_core::theme::{LayerOverride, Palette, Pattern};
use render_support::{look, scene, theme, ORIGIN};

/// Two palettes far apart: warm paper and a dark blue one.
const PALETTES: [&str; 2] = ["paper", "blueprint"];

fn sheet_dxf(diagram_id: &str, theme_id: &str, look: &Look, dxf_layers: DxfLayers) -> Dxf {
    let sheet = Sheet {
        diagram_id,
        theme_id,
        look,
        dxf_layers,
    };
    dxf_with(&scene(), &sheet, &[], &[])
}

/// "@figure" or "#RRGGBB" as the palette draws it.
fn on_sheet(palette: &Palette, value: &str) -> String {
    match value {
        "@figure" => palette.figure.clone(),
        "@water" => palette.water.clone(),
        "@green" => palette.green.clone(),
        "@accent" => palette.accent.clone(),
        "@ink" => palette.ink.clone(),
        "@muted" => palette.muted.clone(),
        "@ground" => palette.ground.clone(),
        _ => value.to_string(),
    }
}

fn colour_of(dxf: &Dxf, layer: &str) -> i32 {
    dxf.layer(layer)
        .unwrap_or_else(|| panic!("{layer} is declared"))
        .1
}

fn highlight(id: &str, outline: Outline, colour: &str, x: f64) -> Highlight {
    Highlight {
        id: id.to_string(),
        label: id.to_string(),
        shape: HighlightShape::Rectangle {
            min: [ORIGIN.0 + x, ORIGIN.1 - 20.0],
            max: [ORIGIN.0 + x + 30.0, ORIGIN.1 + 10.0],
        },
        outline,
        colour: colour.to_string(),
        dims_rest: false,
        shows_area: false,
    }
}

#[test]
fn shown_layers_leave_out_what_the_diagram_does_not_draw() {
    let look = Look {
        diagram_id: "building-height".to_string(),
        ..look()
    };
    let shown = sheet_dxf("building-height", "paper", &look, DxfLayers::Shown);
    let all = sheet_dxf("building-height", "paper", &look, DxfLayers::All);

    assert!(shown.layer("OA-ROAD").is_none());
    assert!(shown.on("OA-ROAD").is_empty());
    assert_eq!(all.on("OA-ROAD").len(), 1);
    // The classes come along either way.
    assert_eq!(shown.on("OA-BUILDING-UNKNOWN").len(), 1);
    assert_eq!(all.on("OA-BUILDING-UNKNOWN").len(), 1);
}

#[test]
fn a_layer_the_user_hid_is_left_out_unless_every_layer_is_asked_for() {
    let mut hidden = look();
    hidden.overrides.insert(
        Layer::Building,
        LayerOverride {
            hidden: Some(true),
            ..LayerOverride::default()
        },
    );

    let shown = sheet_dxf("context", "paper", &hidden, DxfLayers::Shown);
    assert!(shown.layer("OA-BUILDING").is_none());
    assert!(shown.on("OA-BUILDING").is_empty());
    assert!(!shown.on("OA-ROAD").is_empty());
    let all = sheet_dxf("context", "paper", &hidden, DxfLayers::All);
    assert!(!all.on("OA-BUILDING").is_empty());
}

#[test]
fn layers_take_their_sheet_colour_in_every_palette() {
    for theme_id in PALETTES {
        let palette_theme = theme(theme_id);
        let palette = &palette_theme.palette;
        let dxf = sheet_dxf("context", theme_id, &look(), DxfLayers::Shown);

        for (layer, name) in [(Layer::Building, "OA-BUILDING"), (Layer::Water, "OA-WATER")] {
            let style = &palette_theme.layers[&layer];
            // Blueprint draws water as a hatch only, so its hatch is its colour.
            let hatch = match &style.pattern {
                Some(Pattern::Hatch { color, .. }) => Some(color.as_str()),
                _ => None,
            };
            let fill = style.fill.as_deref().or(hatch).expect("a filled layer");
            let expected = aci::nearest(&on_sheet(palette, fill));
            assert_eq!(
                colour_of(&dxf, name),
                i32::from(expected),
                "{theme_id} {name}"
            );
        }
        let site = aci::nearest(&palette.accent);
        assert_eq!(colour_of(&dxf, "OA-SITE"), i32::from(site), "{theme_id}");
    }
}

fn filled(colour: &str) -> LayerOverride {
    LayerOverride {
        fill: Some(colour.to_string()),
        ..LayerOverride::default()
    }
}

#[test]
fn the_users_own_layer_colours_win() {
    // Building height draws no roads, and a fill of their own turns the buildings flat.
    let mut own = Look {
        diagram_id: "building-height".to_string(),
        ..look()
    };
    own.overrides.insert(Layer::Building, filled("#FF0000"));
    own.overrides.insert(Layer::Road, filled("#0000FF"));

    let shown = sheet_dxf("building-height", "paper", &own, DxfLayers::Shown);
    assert_eq!(colour_of(&shown, "OA-BUILDING"), 1);
    assert!(shown.layer("OA-BUILDING-UNKNOWN").is_none());
    let all = sheet_dxf("building-height", "paper", &own, DxfLayers::All);
    assert_eq!(colour_of(&all, "OA-ROAD"), 5);
}

#[test]
fn legend_classes_each_keep_a_colour_of_their_own() {
    let look = Look {
        diagram_id: "building-height".to_string(),
        ..look()
    };
    let dxf = sheet_dxf("building-height", "paper", &look, DxfLayers::Shown);

    let mut colours: Vec<i32> = dxf
        .layers
        .iter()
        .filter(|layer| layer.0.starts_with("OA-BUILDING-") && layer.0 != "OA-BUILDING-UNKNOWN")
        .map(|layer| layer.1)
        .collect();
    let count = colours.len();
    colours.sort();
    colours.dedup();
    assert!(count >= 3, "{count} classes");
    assert_eq!(colours.len(), count, "every class has a colour of its own");
}

#[test]
fn highlights_of_different_colours_on_one_line_get_layers_of_their_own() {
    let highlights = [
        highlight("plot", Outline::Dashed, "@accent", -100.0),
        highlight("house", Outline::Dashed, "#2E7D32", 0.0),
        highlight("square", Outline::Solid, "@accent", 100.0),
    ];
    let palette = theme("paper").palette;
    let dxf = dxf_of(&scene(), "context", &look(), &[], &highlights);

    let accent = i32::from(aci::nearest(&palette.accent));
    assert_eq!(colour_of(&dxf, "OA-HIGHLIGHT-DASHED-ACCENT"), accent);
    assert_eq!(colour_of(&dxf, "OA-HIGHLIGHT-ACCENT"), accent);
    let green = dxf
        .layer("OA-HIGHLIGHT-DASHED-2E7D32")
        .expect("the green highlight's layer");
    assert_eq!(green.1, i32::from(aci::nearest("#2E7D32")));
    assert_eq!(green.2, "DASHED");
    assert_ne!(green.1, accent);
    assert_eq!(dxf.on("OA-HIGHLIGHT-DASHED-2E7D32").len(), 1);
    assert_eq!(
        colour_of(&dxf, "OA-HIGHLIGHT-LABEL"),
        i32::from(aci::nearest(&palette.ink))
    );
}

#[test]
fn every_fetched_layer_includes_the_neighbourhood_the_sheet_hides() {
    let mut quiet = look();
    quiet.sheet.show_context = false;
    let scene = render_support::scene_with_neighbourhood();
    let sheet = |dxf_layers| Sheet {
        diagram_id: "context",
        theme_id: "paper",
        look: &quiet,
        dxf_layers,
    };

    assert!(dxf_with(&scene, &sheet(DxfLayers::Shown), &[], &[])
        .on("OA-CONTEXT-ROAD")
        .is_empty());
    assert!(!dxf_with(&scene, &sheet(DxfLayers::All), &[], &[])
        .on("OA-CONTEXT-ROAD")
        .is_empty());
}
