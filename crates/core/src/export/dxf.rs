//! A plan sheet as a DXF drawing: one CAD layer per scene layer, in real RD
//! New metres. The neighbourhood around the site goes on layers of its own,
//! "OA-CONTEXT-BUILDING" and so on, in one grey, so it can be switched off or
//! locked as a whole.
//!
//! Every stored layer is written whatever the diagram shows, because a CAD
//! user switches layers off more easily than they fetch data again. The
//! diagram adds what exists only when it is drawn: its computed layers (city
//! blocks, hours of sun, whatever `render/derived.rs` works out) and, for a
//! diagram that classifies a layer, one CAD layer per legend class, so a
//! building-height sheet arrives as "OA-BUILDING-6-10-M" and so on. Highlights,
//! the section line and pins each get layers of their own (`dxf_marks.rs`).
//! No styling beyond a layer colour and line type: CAD users bring their own.

use crate::diagram::Classify;
use crate::render::{
    build_classes, class_of, computed_features, layer_slug, shown_highlights, Classes, RenderInput,
};
use crate::scene::{Feature, Layer, Scene};
use crate::sources::bands::BANDED;
use crate::view::ViewMode;
use crate::Result;

use super::dxf_marks;
use super::dxf_write::{cad_name, write_geometry, write_polygon, Drawing};

// The laser files are written from the same R12 pieces.
pub(super) use super::dxf_write::{coordinate, group, section, write_polyline};

const SITE_LAYER: &str = "OA-SITE";
/// AutoCAD colour index for the site outline: red.
const SITE_COLOUR: u8 = 1;
/// AutoCAD colour index for every neighbourhood layer: grey, the usual colour of context.
const CONTEXT_COLOUR: u8 = 8;
/// AutoCAD colour index for a layer without a colour of its own yet: white, or black on paper.
const PLAIN_COLOUR: u8 = 7;
/// AutoCAD colour indexes for a classified layer's classes in legend order,
/// round the colour wheel so neighbouring classes never look alike. More
/// classes than this start the wheel again.
const CLASS_COLOURS: [u8; 9] = [10, 30, 50, 70, 90, 130, 150, 170, 210];
/// A feature the diagram cannot class, such as a building of unknown age: grey.
const UNKNOWN_CLASS_COLOUR: u8 = 9;

/// The sheet `input` draws, as a plan in RD New metres. `input.scene` is
/// already cut to the sheet's area and `input.diagram` drawn as its variant.
// #! NOTE[dxf, layers] : Every stored layer is written whatever the diagram shows; should a CAD export follow the diagram's visible layers instead?
pub fn write(input: &RenderInput) -> Result<String> {
    let scene = input.scene;
    let look = input.look;
    let mut drawing = Drawing::new();
    for &layer in Layer::ALL.iter() {
        drawing.layer(&cad_layer(layer), colour_index(layer));
    }
    let classes = match &input.diagram.classify {
        Some(classify) => Some((classify, build_classes(classify, input.theme)?)),
        None => None,
    };

    let computed = computed_features(scene, input.diagram, look);
    for feature in scene.features.iter().chain(&computed) {
        let layer = match &classes {
            Some((classify, classes)) if classify.layer == feature.layer => {
                class_layer(&mut drawing, feature, classify, classes)
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
    if look.sheet.show_context {
        write_context(&mut drawing, scene);
    }
    drawing.layer(SITE_LAYER, SITE_COLOUR);
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
        );
    }
    if let Some(line) = look
        .section
        .as_ref()
        .filter(|_| look.view.mode != ViewMode::Section)
    {
        dxf_marks::write_section_line(&mut drawing, line, scale);
    }
    if !look.hide_pins {
        dxf_marks::write_pins(&mut drawing, scene, input.pins, scale);
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

/// The CAD layer of the legend class `feature` falls in, declared on first use:
/// "OA-BUILDING-6-10-M", or "OA-BUILDING-UNKNOWN" for a building without a height.
fn class_layer(
    drawing: &mut Drawing,
    feature: &Feature,
    classify: &Classify,
    classes: &Classes,
) -> String {
    let (label, colour) = match class_of(feature, classify) {
        Some(index) => (
            classes
                .classes
                .get(index)
                .map(|class| class.label.as_str())
                .unwrap_or(classes.unknown_label.as_str()),
            CLASS_COLOURS[index % CLASS_COLOURS.len()],
        ),
        None => (classes.unknown_label.as_str(), UNKNOWN_CLASS_COLOUR),
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

/// AutoCAD colour index per layer, so the drawing reads before anyone restyles
/// it. A layer added later and not listed here comes out plain.
fn colour_index(layer: Layer) -> u8 {
    match layer {
        Layer::Building => 7,
        Layer::Road | Layer::Parking | Layer::Bridge => 8,
        Layer::Footpath | Layer::Paved | Layer::Unpaved | Layer::Yard => 9,
        Layer::Cycleway => 11,
        Layer::RailArea | Layer::RailTrack => 252,
        Layer::Water | Layer::WaterBank => 5,
        Layer::Green | Layer::Verge | Layer::Tree => 3,
        Layer::Parcel => 6,
        Layer::Contour => 30,
        Layer::RoadNoise | Layer::AllNoise => 40,
        Layer::No2 | Layer::Pm10 | Layer::Pm25 => 50,
        Layer::Flood => 150,
        Layer::Slope => 32,
        Layer::Population => 200,
        Layer::Business | Layer::Facility | Layer::TransitStop => 1,
        Layer::WalkPath | Layer::WalkBand => 40,
        Layer::Block => 6,
        Layer::SunHours => 50,
        Layer::SkyView => 140,
        Layer::SolarRadiation => 40,
        Layer::TreeCanopy | Layer::GreenCover | Layer::VegetationIndex => 3,
        Layer::HeatIsland => 30,
        Layer::Viewshed => 50,
        Layer::Wind => 150,
        Layer::WindLine => 7,
        #[allow(unreachable_patterns)]
        _ => PLAIN_COLOUR,
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
