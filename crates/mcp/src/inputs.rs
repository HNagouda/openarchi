//! What each tool accepts, with the JSON schema an assistant sees.
//!
//! Core types do not derive `JsonSchema`, so the ones a tool takes are mirrored
//! here with descriptions written for a model, and converted on the way in.

use std::collections::BTreeMap;

use openarchi_core::area::{Area, LonLat};
use openarchi_core::export::ExportFormat;
use openarchi_core::sheet::{Frame, Paper};
use openarchi_core::view::ViewMode;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProjectId {
    /// The project id from list_projects or create_project, like "2026-10-24-westermarkt-3f9a".
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchPlaces {
    /// An address, street, place name or postcode in the Netherlands, like "Westermarkt 20 Amsterdam".
    pub query: String,
    /// How many places to return, best first. Defaults to 5, at most 20.
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateProject {
    /// What the user calls the site, like "Westermarkt". Also the start of the project id.
    pub name: String,
    pub area: AreaInput,
}

/// A point in WGS84 degrees, as search_places returns it.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
pub struct LonLatInput {
    pub lon: f64,
    pub lat: f64,
}

/// The site to fetch, in longitude and latitude. Must lie in the Netherlands
/// and cover at most 4 km². A circle of 100 to 300 m radius around a
/// search_places location suits most site analyses.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AreaInput {
    Circle {
        center: LonLatInput,
        radius_m: f64,
    },
    Rectangle {
        south_west: LonLatInput,
        north_east: LonLatInput,
    },
    /// Outer ring only, at least 3 points, without repeating the first point at the end.
    Polygon {
        points: Vec<LonLatInput>,
    },
}

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
    /// Plan, axonometric (buildings standing up) or explode (several diagrams
    /// stacked as plates). Only the fields you pass change.
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
/// buildings to their height. "explode" stacks the chosen plates.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ViewModeInput {
    Plan,
    Axonometric,
    Explode,
}

impl From<ViewModeInput> for ViewMode {
    fn from(mode: ViewModeInput) -> Self {
        match mode {
            ViewModeInput::Plan => ViewMode::Plan,
            ViewModeInput::Axonometric => ViewMode::Axonometric,
            ViewModeInput::Explode => ViewMode::Explode,
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
pub struct EditText {
    /// The project id.
    pub id: String,
    /// Which text to change: "title", "subtitle", a text box id like "text-1"
    /// (get_project lists them under look.sheet.texts), or "new" to add a text box.
    pub text_id: String,
    /// The words. Required for "new". Line breaks start new lines in a text box.
    /// An empty title shows the diagram name instead; an empty subtitle hides it.
    pub text: Option<String>,
    /// Only the style fields you pass change, unless reset_style is true.
    pub style: Option<TextStyleInput>,
    /// Clears every style field back to the theme before `style` is applied.
    pub reset_style: Option<bool>,
    /// Where the text's anchor goes. The anchor is the top of the first line, at
    /// its left edge, centre or right edge depending on the alignment. A moved
    /// title or subtitle leaves the automatic layout and the legend takes its space.
    pub position: Option<PositionInput>,
    /// True puts the title or subtitle back in the automatic layout.
    pub auto_position: Option<bool>,
    /// True deletes this text box. The title and subtitle cannot be deleted; give them empty text instead.
    pub remove: Option<bool>,
    /// Which sheet tab, counting from 1 for the first, which is the default. list_sheets shows them.
    pub tab: Option<usize>,
}

/// How a text is set. Fields left out follow the theme.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct TextStyleInput {
    /// "Inter", "Source Serif 4" or "IBM Plex Mono".
    pub family: Option<String>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    /// Size on paper in points, 2 to 400. Titles default to about 21 pt on A3, text boxes to about 9 pt.
    pub size_pt: Option<f64>,
    pub align: Option<AlignInput>,
    /// "#RRGGBB" or a palette role like "@accent". Roles follow theme changes.
    pub colour: Option<String>,
    /// 0 to 1.
    pub opacity: Option<f64>,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AlignInput {
    Left,
    Centre,
    Right,
}

/// Millimetres from the top-left corner of the paper. edit_text returns the sheet's size.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
pub struct PositionInput {
    pub x_mm: f64,
    pub y_mm: f64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenderPreview {
    /// The project id.
    pub id: String,
    /// Width of the PNG in pixels. Defaults to 1200, between 200 and 4000.
    pub width_px: Option<u32>,
    /// Which sheet tab, counting from 1 for the first, which is the default. list_sheets shows them.
    pub tab: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetRatios {
    /// The project id.
    pub id: String,
    /// Theme whose colours the rows take, like "paper". Defaults to the sheet's own theme.
    pub theme_id: Option<String>,
    /// Which sheet tab, counting from 1. A sheet with a sub-selection gives the ratios of that smaller area.
    pub tab: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetEnvironment {
    /// The project id.
    pub id: String,
    /// Which sheet tab, counting from 1. A sheet with a sub-selection gives the figures of that smaller area.
    pub tab: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Export {
    /// The project id.
    pub id: String,
    pub format: FormatInput,
    /// Absolute path of the file to write, including the extension. The folder must exist.
    pub path: String,
    /// Resolution for png. Defaults to 300.
    pub dpi: Option<u32>,
    /// Replace a file that is already at `path`. Defaults to false.
    pub overwrite: Option<bool>,
    /// Which sheet tab, counting from 1 for the first, which is the default. list_sheets shows them.
    pub tab: Option<usize>,
}

/// svg and pdf for print and layout tools, png for slides and the web, dxf for
/// CAD with real layers in RD New coordinates.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FormatInput {
    Svg,
    Pdf,
    Png,
    Dxf,
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

impl From<LonLatInput> for LonLat {
    fn from(point: LonLatInput) -> Self {
        LonLat {
            lon: point.lon,
            lat: point.lat,
        }
    }
}

impl From<AreaInput> for Area {
    fn from(area: AreaInput) -> Self {
        match area {
            AreaInput::Circle { center, radius_m } => Area::Circle {
                center: center.into(),
                radius_m,
            },
            AreaInput::Rectangle {
                south_west,
                north_east,
            } => Area::Rectangle {
                south_west: south_west.into(),
                north_east: north_east.into(),
            },
            AreaInput::Polygon { points } => Area::Polygon {
                points: points.into_iter().map(LonLat::from).collect(),
            },
        }
    }
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

impl From<FormatInput> for ExportFormat {
    fn from(format: FormatInput) -> Self {
        match format {
            FormatInput::Svg => ExportFormat::Svg,
            FormatInput::Pdf => ExportFormat::Pdf,
            FormatInput::Png => ExportFormat::Png,
            FormatInput::Dxf => ExportFormat::Dxf,
        }
    }
}
