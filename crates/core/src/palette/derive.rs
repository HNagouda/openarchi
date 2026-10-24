//! Everything a theme needs beyond its main roles, mixed from those roles.
//!
//! The built-in "paper" theme is the template: its line weights, opacities,
//! symbol sizes and the choice of which layers use which role. Where it paints
//! a layer in a colour of its own, such as roads, the colour is mixed here
//! from the new palette with about the share paper.json uses, so roads stay a
//! little darker than the ground on any palette, light or dark.

use super::colour::lch;
use super::roles::RoleColour;
use super::{PaletteDraft, PaletteSource};
use crate::catalog::builtin_themes;
use crate::render::{is_hex, mix, sample_ramp};
use crate::scene::Layer;
use crate::theme::{Palette, Theme, Typography, THEME_FORMAT_VERSION};
use crate::{Error, Result};

const TEMPLATE_THEME: &str = "paper";
const AUTHOR: &str = "You";
const DEFAULT_FAMILY: &str = "Inter";

/// Built-in themes have six ramp stops and twelve categories; so do these.
const RAMP_STOPS: usize = 6;
const CATEGORY_COUNT: usize = 12;
/// Swatches this close to the paper's lightness would vanish as a ramp stop.
const RAMP_MIN_LIGHTNESS_GAP: f64 = 0.04;

/// Ground, muted and unknown, from paper towards ink, as paper.json has them.
const GROUND_SHARE: f64 = 0.06;
const MUTED_SHARE: f64 = 0.1;
const UNKNOWN_SHARE: f64 = 0.22;
/// Extra categories: tints towards the paper, then shades towards the ink.
const CATEGORY_TINT: f64 = 0.45;
const CATEGORY_SHADE: f64 = 0.35;

pub fn theme(draft: &PaletteDraft, id: &str, roles: &[RoleColour]) -> Result<Theme> {
    let hex_of = |role: &str| -> String {
        roles
            .iter()
            .find(|colour| colour.role == role)
            .map(|colour| colour.hex.clone())
            .unwrap_or_default()
    };
    let paper = hex_of("paper");
    let ink = hex_of("ink");
    let palette = Palette {
        ground: mix(&paper, &ink, GROUND_SHARE),
        muted: mix(&paper, &ink, MUTED_SHARE),
        unknown: mix(&paper, &ink, UNKNOWN_SHARE),
        figure: hex_of("figure"),
        accent: hex_of("accent"),
        water: hex_of("water"),
        green: hex_of("green"),
        paper,
        ink,
    };

    let template = builtin_themes()
        .into_iter()
        .find(|theme| theme.id == TEMPLATE_THEME)
        .ok_or_else(|| Error::Invalid(format!("built-in theme {TEMPLATE_THEME:?} is missing")))?;
    let mut layers = template.layers.clone();
    // Fill and stroke of one layer are the same mixed colour, as in the template.
    for (layer, style) in layers.iter_mut() {
        if style.fill.as_deref().is_some_and(is_hex) {
            style.fill = Some(mixed_colour(*layer, &palette)?);
        }
        if style.stroke.as_deref().is_some_and(is_hex) {
            style.stroke = Some(mixed_colour(*layer, &palette)?);
        }
    }

    let water_ramp = sample_ramp(
        &[
            mix(&palette.paper, &palette.water, 0.5),
            palette.water.clone(),
            mix(&palette.water, &palette.ink, 0.35),
            mix(&palette.water, &palette.ink, 0.65),
        ],
        RAMP_STOPS,
    );
    let swatches: Vec<String> = draft
        .swatches
        .iter()
        .map(|swatch| swatch.hex.clone())
        .collect();
    Ok(Theme {
        format_version: THEME_FORMAT_VERSION,
        id: id.to_string(),
        name: draft.name.trim().to_string(),
        author: AUTHOR.to_string(),
        description: format!("Your own palette of {} colours.", draft.swatches.len()),
        ramp: ramp(&swatches, &palette),
        water_ramp,
        categories: categories(&swatches, &palette),
        typography: draft.typography.clone().unwrap_or(Typography {
            family: DEFAULT_FAMILY.to_string(),
            uppercase_labels: false,
        }),
        palette,
        layers,
        source: Some(PaletteSource {
            swatches: draft.swatches.clone(),
            roles: draft.roles.clone(),
        }),
    })
}

/// The colour of a layer that the template paints in a raw colour, mixed
/// from the palette. Every such layer is in this one list; a template layer
/// missing here is an error, so a new layer cannot slip in with a colour
/// from the wrong palette.
fn mixed_colour(layer: Layer, palette: &Palette) -> Result<String> {
    let Palette {
        paper,
        ground,
        ink,
        accent,
        water,
        green,
        ..
    } = palette;
    let colour = match layer {
        Layer::Unpaved => mix(ground, ink, 0.06),
        Layer::Paved => mix(paper, ground, 0.5),
        Layer::Yard => mix(ground, green, 0.2),
        Layer::Verge => mix(green, paper, 0.3),
        Layer::WaterBank => mix(ground, water, 0.3),
        Layer::RailArea => mix(ground, ink, 0.1),
        Layer::Road => mix(ground, ink, 0.2),
        Layer::Parking => mix(ground, ink, 0.06),
        Layer::Cycleway => mix(ground, accent, 0.4),
        Layer::Footpath => mix(paper, ground, 0.15),
        Layer::Bridge => mix(ground, ink, 0.2),
        Layer::RailTrack => mix(ink, paper, 0.12),
        Layer::Contour => mix(ground, ink, 0.15),
        Layer::Tree => mix(green, ink, 0.3),
        other => {
            return Err(Error::Invalid(format!(
                "the {other:?} layer has a raw colour in the template theme but no mixed colour"
            )))
        }
    };

    Ok(colour)
}

/// Every swatch darker than the paper, light to dark, sampled to six stops.
/// Sorting by lightness keeps the ramp reading as "more" from start to end,
/// whatever the hues.
fn ramp(swatches: &[String], palette: &Palette) -> Vec<String> {
    let paper_lightness = lch(&palette.paper).lightness;
    let mut stops: Vec<String> = swatches
        .iter()
        .filter(|hex| lch(hex).lightness < paper_lightness - RAMP_MIN_LIGHTNESS_GAP)
        .cloned()
        .collect();
    stops.sort_by(|a, b| lch(b).lightness.total_cmp(&lch(a).lightness));
    stops.dedup();
    if stops.len() < 2 {
        stops = vec![
            mix(&palette.paper, &palette.accent, 0.4),
            palette.accent.clone(),
            palette.ink.clone(),
        ];
    }

    sample_ramp(&stops, RAMP_STOPS)
}

/// The accent first, then the other swatches by how colourful they are, then
/// their tints and shades, twelve in all.
fn categories(swatches: &[String], palette: &Palette) -> Vec<String> {
    let mut base: Vec<String> = vec![palette.accent.clone()];
    let mut others: Vec<String> = swatches
        .iter()
        .filter(|hex| **hex != palette.paper && **hex != palette.ink)
        .cloned()
        .collect();
    others.sort_by(|a, b| lch(b).chroma.total_cmp(&lch(a).chroma));
    base.extend(others);
    base.push(palette.figure.clone());
    let mut unique: Vec<String> = Vec::new();
    for hex in base {
        if !unique.contains(&hex) {
            unique.push(hex);
        }
    }

    let tints = unique
        .iter()
        .map(|hex| mix(hex, &palette.paper, CATEGORY_TINT));
    let shades = unique
        .iter()
        .map(|hex| mix(hex, &palette.ink, CATEGORY_SHADE));
    let mut categories: Vec<String> = unique.clone();
    for hex in tints.chain(shades) {
        if categories.len() >= CATEGORY_COUNT {
            break;
        }
        if !categories.contains(&hex) {
            categories.push(hex);
        }
    }
    // A two-colour palette still needs twelve classes; greys fill the rest.
    let mut step = 1;
    while categories.len() < CATEGORY_COUNT {
        let share = step as f64 / (CATEGORY_COUNT + 1) as f64;
        categories.push(mix(&palette.paper, &palette.ink, share));
        step += 1;
    }
    categories.truncate(CATEGORY_COUNT);

    categories
}
