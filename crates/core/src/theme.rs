//! A theme is how a set of diagrams looks: palette, line weights, patterns, type.
//!
//! Layers point at palette roles ("@figure") rather than raw colours, so
//! changing one role restyles every diagram in the set at once and they keep
//! reading as a series. A layer may still use a raw "#RRGGBB" when it must.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::palette::PaletteSource;
use crate::scene::Layer;

/// Bump when a saved theme can no longer be read as-is, and migrate old ones on load.
pub const THEME_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    pub format_version: u32,
    /// Lowercase, dash-separated, unique: "paper", "ink".
    pub id: String,
    pub name: String,
    pub author: String,
    /// One line on the mood, shown under the name.
    pub description: String,
    pub palette: Palette,
    /// Colours for sequential classifications (age, height), light to strong. At least 3.
    pub ramp: Vec<String>,
    /// Shallow to deep water, for flood depth. Empty means the main `ramp`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub water_ramp: Vec<String>,
    /// Colours for categories (building use), in order. At least 6.
    pub categories: Vec<String>,
    pub typography: Typography,
    /// Styles for each layer. A layer missing here is not drawn.
    pub layers: BTreeMap<Layer, LayerStyle>,
    /// The swatches a palette made in the app was built from, so the palette
    /// editor can open it again. Built-in and hand-written themes have none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<PaletteSource>,
}

/// Named colour roles, all "#RRGGBB".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Palette {
    /// The sheet behind everything.
    pub paper: String,
    /// Open ground inside the site boundary.
    pub ground: String,
    /// The main subject: buildings in a figure-ground.
    pub figure: String,
    /// One accent only, for the site outline and highlights.
    pub accent: String,
    pub water: String,
    pub green: String,
    /// Fine linework and text.
    pub ink: String,
    /// Everything a diagram pushes into the background.
    pub muted: String,
    /// Features whose attribute is unknown in a classified diagram.
    pub unknown: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Typography {
    /// A family bundled with the app: "Inter", "Source Serif 4", "IBM Plex Mono".
    pub family: String,
    /// True for spaced capitals in titles and labels.
    pub uppercase_labels: bool,
}

/// How one layer is drawn. Colours are "@role" or "#RRGGBB"; `None` means no fill or no stroke.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LayerStyle {
    pub fill: Option<String>,
    pub stroke: Option<String>,
    /// Stroke width on paper, in millimetres.
    #[serde(default)]
    pub stroke_mm: f64,
    /// Dash and gap lengths on paper, in millimetres. Empty for a solid line.
    #[serde(default)]
    pub dash_mm: Vec<f64>,
    /// 0 to 1. Missing means fully opaque.
    #[serde(default = "opaque")]
    pub opacity: f64,
    pub pattern: Option<Pattern>,
    /// Point layers (trees) only: the symbol radius on paper, in millimetres.
    #[serde(default)]
    pub radius_mm: f64,
    /// Buildings only: a drop shadow offset to the south-east, in millimetres. 0 for none.
    #[serde(default)]
    pub shadow_mm: f64,
}

fn opaque() -> f64 {
    1.0
}

/// A fill drawn as lines or dots over (or instead of) the flat fill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Pattern {
    Hatch {
        angle_deg: f64,
        spacing_mm: f64,
        stroke_mm: f64,
        color: String,
    },
    CrossHatch {
        angle_deg: f64,
        spacing_mm: f64,
        stroke_mm: f64,
        color: String,
    },
    Dots {
        spacing_mm: f64,
        radius_mm: f64,
        color: String,
    },
}

/// The user's edits to one layer on top of the theme. Only set fields win.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LayerOverride {
    pub hidden: Option<bool>,
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub stroke_mm: Option<f64>,
    pub opacity: Option<f64>,
}
