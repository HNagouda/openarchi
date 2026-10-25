//! What the user marks on a plan, in a plan DXF: highlights, the section line
//! and pins, each on layers of their own in real RD New metres, so a CAD user
//! can switch the marks off apart from the data.
//!
//! Paper sizes (a dash, a tick, a label) are the sheet's own millimetres
//! turned into ground metres at the sheet's scale, so the DXF plotted at that
//! scale looks like the sheet. A CAD user who plots at another scale changes
//! LTSCALE and the text height as usual.

use crate::highlight::{Highlight, Outline};
use crate::pin::{is_on_site, number_of, Pin, PinIcon};
use crate::scene::Scene;
use crate::section::SectionLine;

use super::dxf_write::{write_point, write_points, write_text, Drawing, Linetype};

/// Highlight outlines, one layer per kind of line, so each takes its line type by layer.
const HIGHLIGHT_LAYER: &str = "OA-HIGHLIGHT";
const HIGHLIGHT_DASHED_LAYER: &str = "OA-HIGHLIGHT-DASHED";
const HIGHLIGHT_DOTTED_LAYER: &str = "OA-HIGHLIGHT-DOTTED";
const HIGHLIGHT_LABEL_LAYER: &str = "OA-HIGHLIGHT-LABEL";
/// AutoCAD colour index for highlights: cyan, apart from the red site and magenta pins.
// #! NOTE[dxf, highlights] : Every highlight is cyan by layer; its own palette colour is not carried over to the nearest AutoCAD colour.
const HIGHLIGHT_COLOUR: u8 = 4;
/// The cut line itself in a centre-line pattern, and its ticks, arrows and labels solid.
const SECTION_LINE_LAYER: &str = "OA-SECTION-LINE";
const SECTION_MARK_LAYER: &str = "OA-SECTION-MARK";
/// AutoCAD colour index for the section line: yellow, the usual colour of a cut on screen.
const SECTION_COLOUR: u8 = 2;
/// Pins get their own layer, so a CAD user can switch the landmarks off in one go.
const PINS_LAYER: &str = "OA-PINS";
/// AutoCAD colour index for pins: magenta, apart from every data layer.
const PINS_COLOUR: u8 = 6;

pub(super) const DASHED: &str = "DASHED";
/// The far end of a section. The sheet sets a prime, A′; CAD gets the plain
/// apostrophe drafters type, since not every reader turns `\U+2032` back into one.
pub(super) const END_LABEL: &str = "A'";
const DOTTED: &str = "DOTTED";
const DASHDOT: &str = "DASHDOT";
// Paper millimetres, as the sheet draws them (render/highlights.rs, render/section_marker.rs).
const HIGHLIGHT_DASH_MM: f64 = 1.8;
const HIGHLIGHT_GAP_MM: f64 = 1.1;
const HIGHLIGHT_DOT_GAP_MM: f64 = 1.5;
/// Long dash, gap, short dash, gap: the centre line of a cut.
const SECTION_DASHES_MM: [f64; 4] = [6.0, 1.2, 0.8, 1.2];
const TICK_MM: f64 = 5.0;
const ARROW_LENGTH_MM: f64 = 4.4;
const ARROW_HALF_WIDTH_MM: f64 = 1.9;
const SECTION_LABEL_GAP_MM: f64 = 3.2;
/// Labels stand this tall on paper, the size of a pin label at A3.
const LABEL_MM: f64 = 2.5;
/// Room between a highlight's top edge and its label's baseline.
const HIGHLIGHT_LABEL_GAP_MM: f64 = 1.2;
/// A pin's label starts this far east of it.
const PIN_LABEL_OFFSET_MM: f64 = 2.5;

/// Ground metres for `paper_mm` on a sheet at 1:`scale`.
pub(super) fn metres(paper_mm: f64, scale: u32) -> f64 {
    paper_mm * f64::from(scale.max(1)) / 1000.0
}

/// The dashed and dotted patterns the marks use, sized for the sheet's scale.
pub fn linetypes(scale: u32) -> Vec<Linetype> {
    let dash = metres(HIGHLIGHT_DASH_MM, scale);
    let gap = metres(HIGHLIGHT_GAP_MM, scale);
    let dot_gap = metres(HIGHLIGHT_DOT_GAP_MM, scale);
    let [long, first_gap, short, second_gap] = SECTION_DASHES_MM.map(|mm| metres(mm, scale));
    vec![
        Linetype {
            name: DASHED,
            description: "Dashed __ __ __",
            pattern: vec![dash, -gap],
        },
        Linetype {
            name: DOTTED,
            description: "Dotted . . . .",
            pattern: vec![0.0, -dot_gap],
        },
        Linetype {
            name: DASHDOT,
            description: "Dash dot ____ _ ____",
            pattern: vec![long, -first_gap, short, -second_gap],
        },
    ]
}

/// Each highlight's outline on the layer of its line, and its caption centred just above it.
pub fn write_highlights(drawing: &mut Drawing, highlights: &[&Highlight], scale: u32) {
    for highlight in highlights {
        let layer = match highlight.outline {
            Outline::Solid => HIGHLIGHT_LAYER,
            Outline::Dashed => HIGHLIGHT_DASHED_LAYER,
            Outline::Dotted => HIGHLIGHT_DOTTED_LAYER,
        };
        let linetype = match highlight.outline {
            Outline::Solid => "CONTINUOUS",
            Outline::Dashed => DASHED,
            Outline::Dotted => DOTTED,
        };
        drawing.layer_with_line(layer, HIGHLIGHT_COLOUR, linetype);
        let ring = highlight.shape.ring();
        write_points(&mut drawing.entities, &ring, layer, true);

        let caption = highlight.caption();
        if caption.is_empty() {
            continue;
        }
        let (middle_x, _) = highlight.shape.middle();
        let top = ring.iter().map(|point| point.1).fold(f64::MIN, f64::max);
        drawing.layer(HIGHLIGHT_LABEL_LAYER, HIGHLIGHT_COLOUR);
        write_text(
            &mut drawing.entities,
            (middle_x, top + metres(HIGHLIGHT_LABEL_GAP_MM, scale)),
            metres(LABEL_MM, scale),
            &caption,
            HIGHLIGHT_LABEL_LAYER,
            true,
        );
    }
}

/// The cut marker of the plan: the line from A to A', a tick running inwards
/// from each end, an arrow at each end pointing where the section looks, and
/// the labels A and A' just past the ends.
pub fn write_section_line(drawing: &mut Drawing, line: &SectionLine, scale: u32) {
    drawing.layer_with_line(SECTION_LINE_LAYER, SECTION_COLOUR, DASHDOT);
    drawing.layer(SECTION_MARK_LAYER, SECTION_COLOUR);
    let axis = line.axis((0.0, 0.0));
    let along = axis.along;
    let looking = axis.looking;
    write_points(
        &mut drawing.entities,
        &[line.start, line.end],
        SECTION_LINE_LAYER,
        false,
    );

    let tick = metres(TICK_MM, scale);
    let arrow = metres(ARROW_LENGTH_MM, scale);
    let half = metres(ARROW_HALF_WIDTH_MM, scale);
    for (at, inwards) in [(line.start, along), (line.end, (-along.0, -along.1))] {
        let tick_end = (at.0 + inwards.0 * tick, at.1 + inwards.1 * tick);
        write_points(
            &mut drawing.entities,
            &[at, tick_end],
            SECTION_MARK_LAYER,
            false,
        );
        let arrow_points = [
            (at.0 + inwards.0 * half, at.1 + inwards.1 * half),
            (at.0 + looking.0 * arrow, at.1 + looking.1 * arrow),
            (at.0 - inwards.0 * half, at.1 - inwards.1 * half),
        ];
        write_points(
            &mut drawing.entities,
            &arrow_points,
            SECTION_MARK_LAYER,
            true,
        );
    }

    let gap = metres(SECTION_LABEL_GAP_MM, scale);
    let height = metres(LABEL_MM, scale);
    // Lowered by half the text height, so each letter sits centred on the line's direction.
    let a_at = (
        line.start.0 - along.0 * gap,
        line.start.1 - along.1 * gap - height / 2.0,
    );
    let b_at = (
        line.end.0 + along.0 * gap,
        line.end.1 + along.1 * gap - height / 2.0,
    );
    write_text(
        &mut drawing.entities,
        a_at,
        height,
        "A",
        SECTION_MARK_LAYER,
        true,
    );
    write_text(
        &mut drawing.entities,
        b_at,
        height,
        END_LABEL,
        SECTION_MARK_LAYER,
        true,
    );
}

/// Each pin on the site as a point with its label beside it; a numbered pin's
/// label starts with its number. Pins are in RD New metres already.
pub fn write_pins(drawing: &mut Drawing, scene: &Scene, pins: &[Pin], scale: u32) {
    drawing.layer(PINS_LAYER, PINS_COLOUR);
    for pin in pins {
        if !is_on_site(scene, pin.x, pin.y) {
            continue;
        }
        write_point(&mut drawing.entities, pin.x, pin.y, PINS_LAYER);

        let label = match (pin.icon, number_of(pins, &pin.id)) {
            (PinIcon::Number, Some(number)) if pin.label.is_empty() => number.to_string(),
            (PinIcon::Number, Some(number)) => format!("{number} {}", pin.label),
            _ => pin.label.clone(),
        };
        if label.is_empty() {
            continue;
        }
        write_text(
            &mut drawing.entities,
            (pin.x + metres(PIN_LABEL_OFFSET_MM, scale), pin.y),
            metres(LABEL_MM, scale),
            &label,
            PINS_LAYER,
            false,
        );
    }
}
