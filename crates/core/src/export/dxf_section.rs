//! A section sheet as a DXF drawing, in section coordinates: x is metres
//! along the line from A, y is metres above NAP (or above a flat 0 for a
//! scene kept without terrain). Heights are always true, whatever the sheet
//! exaggerates them by, because a CAD user scales a drawing rather than its data.
//!
//! The pieces are the sheet's own, from `render/section_pieces.rs`: the
//! ground profile, lowered over water nobody surveyed the bed of; buildings
//! the line cuts; buildings and trees beyond it within the line's depth;
//! water from its level down to its bed.
//! Each goes on a layer of its own. Elevations are whole outlines, so a
//! building behind another is not trimmed where it is hidden.

use crate::render::{
    section_pieces, section_water_body, tree_size, RenderInput, SectionBuilding, SectionTree,
    SECTION_AIR_M, SECTION_EARTH_M,
};
use crate::section::cut::Stretch;
use crate::section::profile::Profile;
use crate::section::water::Water;
use crate::section::Axis;
use crate::Result;

use super::dxf_marks::{linetypes, metres, DASHED, END_LABEL};
use super::dxf_write::{write_circle, write_points, write_text, Drawing};

const GROUND_LAYER: &str = "OA-SECTION-GROUND";
const CUT_LAYER: &str = "OA-SECTION-CUT";
const ELEVATION_LAYER: &str = "OA-SECTION-ELEVATION";
const CONTEXT_LAYER: &str = "OA-SECTION-CONTEXT-ELEVATION";
const TREE_LAYER: &str = "OA-SECTION-TREE";
const WATER_LAYER: &str = "OA-SECTION-WATER";
/// The NAP datum and the A and A' ends.
const DATUM_LAYER: &str = "OA-SECTION-NAP";
const TEXT_LAYER: &str = "OA-SECTION-TEXT";
// AutoCAD colour indexes: the cut white (black on paper), what stands beyond
// in greys, earth brown, trees green, water blue, the datum red.
// #! NOTE[dxf, colours] : A section DXF keeps these fixed colours; the plan DXF takes the sheet's colours.
const GROUND_COLOUR: u8 = 32;
const CUT_COLOUR: u8 = 7;
const ELEVATION_COLOUR: u8 = 8;
const CONTEXT_COLOUR: u8 = 9;
const TREE_COLOUR: u8 = 3;
const WATER_COLOUR: u8 = 5;
const DATUM_COLOUR: u8 = 1;
const TEXT_COLOUR: u8 = 7;
/// The datum and the drawing reach this far past A and A', in metres.
const OVERHANG_M: f64 = 5.0;
/// The scale text is sized for when the section picks its own to fit the sheet.
// #! NOTE[dxf, section] : A fitted section has no scale of its own in the core, so its text is sized for 1:500.
const FITTED_SCALE: u32 = 500;
const LABEL_MM: f64 = 2.5;

/// The section the sheet `input` draws. `input.scene` is already cut to the
/// sheet's area, as the sheet's own section is.
pub fn write(input: &RenderInput) -> Result<String> {
    let pieces = section_pieces(input.scene, input.look)?;
    let profile = &pieces.profile;
    let scale = pieces.line.scale.unwrap_or(FITTED_SCALE);
    let text_height = metres(LABEL_MM, scale);

    let mut drawing = Drawing::new();
    let mut highest = profile.highest();
    highest = highest.max(write_buildings(
        &mut drawing,
        &pieces.buildings,
        ELEVATION_LAYER,
        ELEVATION_COLOUR,
    ));
    highest = highest.max(write_buildings(
        &mut drawing,
        &pieces.neighbours,
        CONTEXT_LAYER,
        CONTEXT_COLOUR,
    ));
    write_trees(&mut drawing, &pieces.trees);
    write_waters(&mut drawing, &pieces.waters, profile, text_height);

    drawing.layer(GROUND_LAYER, GROUND_COLOUR);
    write_points(&mut drawing.entities, &profile.samples, GROUND_LAYER, false);
    write_datum(&mut drawing, &pieces.axis, profile, text_height, highest);

    // Earth below the lowest ground and air above the tallest roof, as on the sheet.
    let extent = (
        -OVERHANG_M,
        pieces.lowest().min(0.0) - SECTION_EARTH_M,
        pieces.axis.length + OVERHANG_M,
        highest + SECTION_AIR_M,
    );

    Ok(drawing.finish(extent, &linetypes(scale)))
}

/// Each building as its outline beyond the line on `layer`, and where the
/// line passes through it as a cut. Returns the tallest roof drawn.
fn write_buildings(
    drawing: &mut Drawing,
    buildings: &[SectionBuilding],
    layer: &str,
    colour: u8,
) -> f64 {
    let mut highest = f64::MIN;
    for building in buildings {
        highest = highest.max(building.top);
        drawing.layer(layer, colour);
        write_box(
            drawing,
            building.stretch,
            building.base,
            building.top,
            layer,
        );
        for cut in &building.cuts {
            drawing.layer(CUT_LAYER, CUT_COLOUR);
            write_box(drawing, cut.stretch, cut.bottom, cut.top, CUT_LAYER);
        }
    }

    highest
}

fn write_box(drawing: &mut Drawing, stretch: Stretch, bottom: f64, top: f64, layer: &str) {
    let corners = [
        (stretch.from, bottom),
        (stretch.to, bottom),
        (stretch.to, top),
        (stretch.from, top),
    ];
    write_points(&mut drawing.entities, &corners, layer, true);
}

/// Each tree as a trunk and a round crown, as the sheet draws it.
fn write_trees(drawing: &mut Drawing, trees: &[SectionTree]) {
    let (height, crown) = tree_size();
    for tree in trees {
        let x = (tree.stretch.from + tree.stretch.to) / 2.0;
        let crown_centre = tree.base + height - crown;
        drawing.layer(TREE_LAYER, TREE_COLOUR);
        write_points(
            &mut drawing.entities,
            &[(x, tree.base), (x, crown_centre - crown)],
            TREE_LAYER,
            false,
        );
        write_circle(&mut drawing.entities, x, crown_centre, crown, TREE_LAYER);
    }
}

/// A water with a surveyed bed as its whole body, surface and bed; one
/// without as its surface only, the ground below it unknown. A published
/// target level is written beside the surface.
fn write_waters(drawing: &mut Drawing, waters: &[Water], profile: &Profile, text_height: f64) {
    for water in waters {
        drawing.layer(WATER_LAYER, WATER_COLOUR);
        let (from, to, level) = (water.stretch.from, water.stretch.to, water.level_m);
        if water.has_bed {
            let body = section_water_body(water, profile);
            write_points(&mut drawing.entities, &body, WATER_LAYER, true);
        } else {
            write_points(
                &mut drawing.entities,
                &[(from, level), (to, level)],
                WATER_LAYER,
                false,
            );
        }

        if let Some(name) = &water.level_name {
            drawing.layer(TEXT_LAYER, TEXT_COLOUR);
            write_text(
                &mut drawing.entities,
                ((from + to) / 2.0, level + text_height / 2.0),
                text_height,
                &format!("{name} NAP {level:+.2} m"),
                TEXT_LAYER,
                true,
            );
        }
    }
}

/// A dashed line at 0 across the whole section, named NAP when the ground is
/// measured, and the A and A' ends as vertical lines up to the tallest roof.
fn write_datum(
    drawing: &mut Drawing,
    axis: &Axis,
    profile: &Profile,
    text_height: f64,
    highest: f64,
) {
    drawing.layer_with_line(DATUM_LAYER, DATUM_COLOUR, DASHED);
    let (left, right) = (-OVERHANG_M, axis.length + OVERHANG_M);
    write_points(
        &mut drawing.entities,
        &[(left, 0.0), (right, 0.0)],
        DATUM_LAYER,
        false,
    );
    let datum = if profile.is_measured {
        "NAP 0.00 m"
    } else {
        "0 m, flat: the site was kept without terrain"
    };
    drawing.layer(TEXT_LAYER, TEXT_COLOUR);
    write_text(
        &mut drawing.entities,
        (left, text_height / 2.0),
        text_height,
        datum,
        TEXT_LAYER,
        false,
    );

    let bottom = profile.lowest().min(0.0);
    let top = highest.max(profile.highest()) + text_height;
    for (x, label) in [(0.0, "A"), (axis.length, END_LABEL)] {
        write_points(
            &mut drawing.entities,
            &[(x, bottom), (x, top)],
            DATUM_LAYER,
            false,
        );
        write_text(
            &mut drawing.entities,
            (x, top + text_height / 2.0),
            text_height,
            label,
            TEXT_LAYER,
            true,
        );
    }
}
