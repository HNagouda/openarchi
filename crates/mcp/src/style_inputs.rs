//! What the style tools accept: set_look and the sheet, view, sun and layer
//! fields it changes, list_themes and create_theme.

use std::collections::BTreeMap;

use openarchi_core::sheet::{Frame, Paper};
use openarchi_core::view::ViewMode;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListThemes {
    /// True returns every field of each theme (ramp, categories, typography,
    /// layer styles), which is the shape create_theme expects. Defaults to false.
    pub full: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetLook {
    /// The project id.
    pub id: String,
    /// A diagram id from list_diagrams.
    pub diagram_id: Option<String>,
    /// One of the diagram's variants from list_diagrams, like "pm25" for
    /// air-quality. Kept while the diagram stays the same; a new diagram
    /// without a variant starts on its first.
    pub variant: Option<String>,
    /// A theme id from list_themes.
    pub theme_id: Option<String>,
    /// Only the sheet fields you pass change.
    pub sheet: Option<SheetInput>,
    /// Per-layer edits on top of the theme, keyed by layer name ("building",
    /// "road", "water" and so on, as in list_diagrams). Passing a layer replaces
    /// its override; passing null for a layer removes it.
    pub overrides: Option<BTreeMap<String, Option<OverrideInput>>>,
    /// Plan, axonometric (buildings standing up), explode (several diagrams
    /// stacked as plates) or section (along the line set_section draws). Only
    /// the fields you pass change.
    pub view: Option<ViewInput>,
    /// The date and Dutch clock time the sun path and shadow diagrams, and the
    /// lighting of the axonometric, are drawn for. Only the fields you pass change.
    pub sun: Option<SunInput>,
    /// Which sheet tab, counting from 1 for the first, which is the default. list_sheets shows them.
    pub tab: Option<usize>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct ViewInput {
    pub mode: Option<ViewModeInput>,
    /// Turns the site counterclockwise on the paper, in degrees. 0 keeps north up.
    pub rotation_deg: Option<f64>,
    /// How steeply the viewer looks down, 15 to 80 degrees. Defaults to 35.
    pub tilt_deg: Option<f64>,
    /// Diagram ids from list_diagrams to stack in the explode view, top plate
    /// first, like ["building-use", "building-age", "landscape", "context"].
    /// A diagram with variants is drawn as its first; "air-quality:pm25" picks one.
    pub plates: Option<Vec<String>>,
    /// True packs the explode plates close together like a stack of sheets.
    pub stacking: Option<bool>,
}

/// "plan" looks straight down. "axonometric" tilts the site and extrudes
/// buildings to their height. "explode" stacks the chosen plates. "section"
/// draws the elevation along the sheet's section line, which set_section draws first.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ViewModeInput {
    Plan,
    Axonometric,
    Explode,
    Section,
}

impl From<ViewModeInput> for ViewMode {
    fn from(mode: ViewModeInput) -> Self {
        match mode {
            ViewModeInput::Plan => ViewMode::Plan,
            ViewModeInput::Axonometric => ViewMode::Axonometric,
            ViewModeInput::Explode => ViewMode::Explode,
            ViewModeInput::Section => ViewMode::Section,
        }
    }
}

/// Dutch clock time (summer time included), on any day of the year.
#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct SunInput {
    /// 1 to 12.
    pub month: Option<u32>,
    pub day: Option<u32>,
    /// 0 to 23.
    pub hour: Option<u32>,
    pub minute: Option<u32>,
}

/// The paper the diagram is laid out on.
#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct SheetInput {
    pub paper: Option<PaperInput>,
    pub landscape: Option<bool>,
    /// Scale denominator: 2500 means 1:2500.
    pub scale: Option<u32>,
    pub frame: Option<FrameInput>,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub show_legend: Option<bool>,
    pub show_north_arrow: Option<bool>,
    pub show_scale_bar: Option<bool>,
    /// Data sources and licenses in small print. Some sources require it, so leave it on for anything published.
    pub show_credits: Option<bool>,
    /// The neighbourhood around the site: buildings, roads, water and green
    /// outside the boundary, drawn muted in the palette. Not in the legend.
    pub show_context: Option<bool>,
    /// How much of the neighbourhood shows through, 0 to 1. 1 is the usual
    /// muted look; lower lets the ground show through it.
    pub context_opacity: Option<f64>,
}

/// "square" is 300 × 300 mm, for social posts and portfolios.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaperInput {
    A4,
    A3,
    A2,
    A1,
    A0,
    Square,
}

/// "circle" crops the drawing to a circle around the site, the classic
/// site-analysis look. "square" fills a square. "area" crops to the drawn area.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FrameInput {
    Circle,
    Square,
    Area,
}

/// Only the fields you set win over the theme. Colours are "#RRGGBB" or a
/// palette role like "@accent".
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct OverrideInput {
    pub hidden: Option<bool>,
    pub fill: Option<String>,
    pub stroke: Option<String>,
    /// Stroke width on paper, in millimetres.
    pub stroke_mm: Option<f64>,
    /// 0 to 1.
    pub opacity: Option<f64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateTheme {
    /// A complete theme in the shape list_themes returns with full=true:
    /// format_version (1), id (lowercase, dash-separated, new), name, author,
    /// description, palette (paper, ground, figure, accent, water, green, ink,
    /// muted, unknown as "#RRGGBB"), ramp (3+ colours, light to strong),
    /// categories (6+ colours), typography (family, uppercase_labels) and
    /// layers (style per layer name). Start from an existing theme and change it.
    pub theme_json: serde_json::Map<String, serde_json::Value>,
}

impl From<PaperInput> for Paper {
    fn from(paper: PaperInput) -> Self {
        match paper {
            PaperInput::A4 => Paper::A4,
            PaperInput::A3 => Paper::A3,
            PaperInput::A2 => Paper::A2,
            PaperInput::A1 => Paper::A1,
            PaperInput::A0 => Paper::A0,
            PaperInput::Square => Paper::Square,
        }
    }
}

impl From<FrameInput> for Frame {
    fn from(frame: FrameInput) -> Self {
        match frame {
            FrameInput::Circle => Frame::Circle,
            FrameInput::Square => Frame::Square,
            FrameInput::Area => Frame::Area,
        }
    }
}
