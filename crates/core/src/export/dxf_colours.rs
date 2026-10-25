//! The colour each CAD layer of a plan DXF takes: the one the layer has on the
//! sheet, as its nearest AutoCAD colour (`aci.rs`). A drawing that keeps the
//! sheet's colours is easier to read and to rebuild the sheet from in CAD.
//!
//! A layer takes its fill, then the colour of its pattern, then its line,
//! since a filled area is what the eye takes its colour from. Opacity is left out: a CAD
//! layer has no paper under it to fade towards.

use crate::diagram::Diagram;
use crate::project::Look;
use crate::render::{resolve_colour, ResolvedLayer};
use crate::scene::Layer;
use crate::theme::{Pattern, Theme};
use crate::Result;

use super::aci;

/// AutoCAD colour index for a layer the palette has no style for: 7, black on paper.
const PLAIN_COLOUR: u8 = 7;

/// The colour of a layer the sheet draws, as `render/style.rs` resolved it.
/// A blank layer's fill is the paper itself, so it takes its ink edge instead.
pub fn shown(resolved: &ResolvedLayer, diagram: &Diagram) -> u8 {
    let style = &resolved.style;
    let colour = if diagram.blank.contains(&resolved.layer) {
        style.stroke.as_ref().or(style.fill.as_ref())
    } else {
        style
            .fill
            .as_ref()
            .or(style.pattern.as_ref().map(pattern_colour))
            .or(style.stroke.as_ref())
    };

    colour.map_or(PLAIN_COLOUR, |hex| aci::nearest(hex))
}

/// The colour of a layer this sheet does not draw: the palette's own style for
/// it, with the user's colours for the layer on top, as another diagram would
/// show it unmuted.
pub fn unshown(layer: Layer, theme: &Theme, look: &Look) -> Result<u8> {
    let overrides = look.overrides.get(&layer);
    let style = theme.layers.get(&layer);
    let fill = overrides
        .and_then(|own| own.fill.clone())
        .or_else(|| style.and_then(|style| style.fill.clone()))
        .or_else(|| {
            style
                .and_then(|style| style.pattern.as_ref())
                .map(|pattern| pattern_colour(pattern).clone())
        });
    let stroke = overrides
        .and_then(|own| own.stroke.clone())
        .or_else(|| style.and_then(|style| style.stroke.clone()));

    match fill.or(stroke) {
        Some(value) => Ok(aci::nearest(&resolve_colour(&theme.palette, &value)?)),
        None => Ok(PLAIN_COLOUR),
    }
}

fn pattern_colour(pattern: &Pattern) -> &String {
    match pattern {
        Pattern::Hatch { color, .. }
        | Pattern::CrossHatch { color, .. }
        | Pattern::Dots { color, .. } => color,
    }
}
