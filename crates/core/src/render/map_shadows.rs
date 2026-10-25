//! The shadows a map casts: plain ones in the shadow diagram, soft cool ones
//! that only light a tilted view, and the shadow diagram's legend row.

use std::collections::BTreeMap;
use std::fmt::Write;

use super::color;
use super::drawing::{shadow_colour, Canvas};
use super::legend::{Row, Swatch};
use super::map::layer_features;
use super::shadow;
use super::style::ResolvedLayer;
use super::svg::layer_attributes;
use crate::model::PlacedModel;
use crate::scene::{Feature, Layer};
use crate::sun::SunPosition;
use crate::theme::Theme;

/// Shadows in the shadow diagram: clearly there, but the ground still reads through.
pub(super) const DIAGRAM_SHADOW_OPACITY: f64 = 0.32;
/// Shadows that only light a tilted view: soft, so they model without shouting.
pub(super) const AMBIENT_SHADOW_OPACITY: f64 = 0.2;
/// A cool blue-grey the ambient shadows lean towards, the colour of shade outdoors.
const COOL_SHADE: &str = "#4A5878";
const COOL_SHARE: f64 = 0.35;

/// What casts a map's shadows: the buildings and trees the diagram draws,
/// and the imported model standing among them.
pub(super) struct Casters<'a> {
    pub features: Vec<&'a Feature>,
    pub model: Option<&'a PlacedModel>,
}

/// Buildings and trees the diagram draws, which are the ones that cast shadows.
pub(super) fn shadow_casters<'a>(
    layers: &[ResolvedLayer],
    features: &BTreeMap<Layer, Vec<&'a Feature>>,
    model: Option<&'a PlacedModel>,
) -> Casters<'a> {
    let features = layers
        .iter()
        .filter(|resolved| matches!(resolved.layer, Layer::Building | Layer::Tree))
        .flat_map(|resolved| layer_features(features, resolved.layer).iter().copied())
        .collect();
    Casters { features, model }
}

pub(super) fn write_shadows(
    out: &mut String,
    casters: &Casters,
    sun: &SunPosition,
    canvas: &Canvas,
    theme: &Theme,
    opacity: f64,
) {
    let colour = color_for_shadows(theme, opacity);
    let _ = write!(
        out,
        r#"<g {} clip-path="url(#{})" stroke="none" fill="{colour}">"#,
        layer_attributes(&format!("{}shadows", canvas.scope), "Shadows"),
        canvas.clip_id
    );
    shadow::write(
        out,
        &casters.features,
        casters.model,
        sun,
        &canvas.projection,
        opacity,
    );
    out.push_str("</g>\n");
}

/// Diagram shadows in the theme's shadow colour; the softer ambient ones lean cool.
fn color_for_shadows(theme: &Theme, opacity: f64) -> String {
    let base = shadow_colour(theme);
    if opacity < DIAGRAM_SHADOW_OPACITY {
        color::mix(&base, COOL_SHADE, COOL_SHARE)
    } else {
        base
    }
}

pub(super) fn shadow_row(sun: &SunPosition, look: &crate::project::Look, theme: &Theme) -> Row {
    let label = if sun.altitude_deg < shadow::MIN_SUN_ALTITUDE_DEG {
        format!("No sun on {}", look.sun.label())
    } else {
        format!("Shadow on {}", look.sun.label())
    };
    Row {
        label,
        swatch: Swatch::Area {
            fill: shadow_colour(theme),
            stroke: None,
            stroke_mm: 0.0,
            opacity: DIAGRAM_SHADOW_OPACITY,
        },
    }
}
