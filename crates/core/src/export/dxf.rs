//! A plan sheet as a DXF drawing: one CAD layer per scene layer, in real RD
//! New metres. The neighbourhood around the site goes on layers of its own,
//! "OA-CONTEXT-BUILDING" and so on, in one grey, so it can be switched off or
//! locked as a whole.
//!
//! By default the DXF holds what the sheet draws: the diagram's layers less
//! those the user hid. `DxfLayers::All` writes every fetched layer instead,
//! for a CAD user who would rather switch layers off than fetch data again.
//! Either way the diagram adds what exists only when it is drawn: its
//! computed layers (city blocks, hours of sun, whatever `render/derived.rs`
//! works out) and, for a diagram that classifies a layer, one CAD layer per
//! legend class, so a building-height sheet arrives as "OA-BUILDING-6-10-M"
//! and so on. Highlights, the section line and pins each get layers of their
//! own (`dxf_marks.rs`). Each layer takes its colour on the sheet
//! (`dxf_colours.rs`); no styling beyond that and a line type, since CAD users
//! bring their own.

use crate::diagram::Classify;
use crate::render::{
    class_of, computed_features, layer_slug, resolve_layers, shown_highlights, Classes,
    RenderInput, ResolvedLayer,
};
use crate::scene::{Feature, Layer, Scene};
use crate::sources::bands::BANDED;
use crate::view::ViewMode;
use crate::Result;

use super::aci;
use super::dxf_colours;
use super::dxf_marks;
use super::dxf_write::{cad_name, write_geometry, write_polygon, Drawing};
use super::DxfLayers;

// The laser files are written from the same R12 pieces.
pub(super) use super::dxf_write::{coordinate, group, section, write_polyline};

const SITE_LAYER: &str = "OA-SITE";
/// AutoCAD colour index for every neighbourhood layer: grey, the usual colour of
/// context. The sheet fades each neighbourhood layer towards the paper, which
/// in CAD would be white on white.
// #! NOTE[dxf, colours] : The neighbourhood stays one grey rather than its faded sheet colours.
const CONTEXT_COLOUR: u8 = 8;

/// The sheet `input` draws, as a plan in RD New metres, with the layers
/// `dxf_layers` asks for. `input.scene` is already cut to the sheet's area
/// and `input.diagram` drawn as its variant.
pub fn write(input: &RenderInput, dxf_layers: DxfLayers) -> Result<String> {
    let scene = input.scene;
    let look = input.look;
    let palette = &input.theme.palette;
    let shown = resolve_layers(input.diagram, input.theme, look)?;
    let written: Vec<Layer> = match dxf_layers {
        DxfLayers::Shown => shown.iter().map(|resolved| resolved.layer).collect(),
        DxfLayers::All => Layer::ALL.to_vec(),
    };

    let mut drawing = Drawing::new();
    for &layer in &written {
        // A classified layer is split into its classes, declared as they are used.
        let resolved = shown.iter().find(|resolved| resolved.layer == layer);
        let colour = match resolved {
            Some(resolved) if resolved.classes.is_some() => continue,
            Some(resolved) => dxf_colours::shown(resolved, input.diagram),
            None => dxf_colours::unshown(layer, input.theme, look)?,
        };
        drawing.layer(&cad_layer(layer), colour);
    }

    let classified = input.diagram.classify.as_ref().and_then(|classify| {
        classes_of(&shown, classify.layer).map(|classes| Classified {
            classify,
            classes,
            colours: class_colours(classes),
        })
    });
    let computed = computed_features(scene, input.diagram, &look.sun);
    for feature in scene.features.iter().chain(&computed) {
        if !written.contains(&feature.layer) {
            continue;
        }
        let layer = match &classified {
            Some(classified) if classified.classify.layer == feature.layer => {
                class_layer(&mut drawing, feature, classified)
            }
            _ => cad_layer(feature.layer),
        };
        write_geometry(
            &mut drawing.entities,
            &feature.geometry,
            &layer,
            elevation(feature),
            scene.origin,
        );
    }
    let is_context_shown = look.sheet.show_context && look.sheet.context_opacity > 0.0;
    if dxf_layers == DxfLayers::All || is_context_shown {
        write_context(&mut drawing, scene);
    }
    // The sheet outlines the site in the accent.
    drawing.layer(SITE_LAYER, aci::nearest(&palette.accent));
    write_polygon(
        &mut drawing.entities,
        &scene.boundary,
        SITE_LAYER,
        0.0,
        scene.origin,
    );

    let scale = look.sheet.scale;
    if !look.hide_highlights {
        dxf_marks::write_highlights(
            &mut drawing,
            &shown_highlights(input.highlights, scene),
            scale,
            palette,
        );
    }
    if let Some(line) = look
        .section
        .as_ref()
        .filter(|_| look.view.mode != ViewMode::Section)
    {
        dxf_marks::write_section_line(&mut drawing, line, scale, palette);
    }
    if !look.hide_pins {
        dxf_marks::write_pins(&mut drawing, scene, input.pins, scale, palette);
    }

    Ok(drawing.finish(extent(scene), &dxf_marks::linetypes(scale)))
}

/// Layer::RailArea -> "OA-RAIL-AREA".
fn cad_layer(layer: Layer) -> String {
    format!("OA-{}", layer_slug(layer).to_uppercase())
}

/// Layer::Building -> "OA-CONTEXT-BUILDING".
fn context_cad_layer(layer: Layer) -> String {
    format!("OA-CONTEXT-{}", layer_slug(layer).to_uppercase())
}

/// The legend classes the sheet colours `layer` by. `None` when the sheet
/// does not draw it, or draws it flat because the user gave it a fill.
fn classes_of(shown: &[ResolvedLayer], layer: Layer) -> Option<&Classes> {
    shown
        .iter()
        .find(|resolved| resolved.layer == layer)
        .and_then(|resolved| resolved.classes.as_ref())
}

/// The layer the sheet colours by class, its classes and their AutoCAD colours.
struct Classified<'a> {
    classify: &'a Classify,
    classes: &'a Classes,
    /// One per class in legend order, then one for the unknown class.
    colours: Vec<u8>,
}

/// Each class's nearest AutoCAD colour that no earlier class took, in legend
/// order, then the unknown class's. A subtle ramp would otherwise put two
/// neighbouring classes on one colour, and the CAD layers would read as one.
fn class_colours(classes: &Classes) -> Vec<u8> {
    let mut colours = Vec::new();
    for class in &classes.classes {
        colours.push(aci::nearest_apart(&class.fill, &colours));
    }
    colours.push(aci::nearest_apart(&classes.unknown_fill, &colours));

    colours
}

/// The CAD layer of the legend class `feature` falls in, declared on first use
/// in the class's colour: "OA-BUILDING-6-10-M", or "OA-BUILDING-UNKNOWN" for a
/// building without a height.
fn class_layer(drawing: &mut Drawing, feature: &Feature, classified: &Classified) -> String {
    let classes = classified.classes;
    let index =
        class_of(feature, classified.classify).filter(|&index| index < classes.classes.len());
    let (label, colour) = match index {
        Some(index) => (
            classes.classes[index].label.as_str(),
            classified.colours[index],
        ),
        None => (
            classes.unknown_label.as_str(),
            classified.colours[classes.classes.len()],
        ),
    };
    let name = cad_name(&format!("{}-{label}", cad_layer(feature.layer)));
    drawing.layer(&name, colour);

    name
}

/// A band sits at the height of its lower edge, as a contour sits at its
/// level, so a CAD user can read 55 dB or 0.5 m off the outline.
fn elevation(feature: &Feature) -> f64 {
    if feature.layer == Layer::Contour {
        feature.attributes.ground_m.unwrap_or(0.0)
    } else if BANDED.contains(&feature.layer) {
        feature.attributes.value.unwrap_or(0.0)
    } else {
        0.0
    }
}

fn write_context(drawing: &mut Drawing, scene: &Scene) {
    for feature in &scene.context {
        let layer = context_cad_layer(feature.layer);
        drawing.layer(&layer, CONTEXT_COLOUR);
        write_geometry(
            &mut drawing.entities,
            &feature.geometry,
            &layer,
            0.0,
            scene.origin,
        );
    }
}

/// Real-world extent of the boundary, for the header.
fn extent(scene: &Scene) -> (f64, f64, f64, f64) {
    let (shift_x, shift_y) = scene.origin;
    let mut bounds = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for point in &scene.boundary.exterior().0 {
        bounds = (
            bounds.0.min(point.x),
            bounds.1.min(point.y),
            bounds.2.max(point.x),
            bounds.3.max(point.y),
        );
    }
    if bounds.0 > bounds.2 {
        return (shift_x, shift_y, shift_x, shift_y);
    }

    (
        bounds.0 + shift_x,
        bounds.1 + shift_y,
        bounds.2 + shift_x,
        bounds.3 + shift_y,
    )
}
