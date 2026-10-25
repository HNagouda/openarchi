//! What edit_text accepts: which text, its words, style and position.

use openarchi_core::sheet::Align;
use schemars::JsonSchema;
use serde::Deserialize;

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

impl From<AlignInput> for Align {
    fn from(align: AlignInput) -> Self {
        match align {
            AlignInput::Left => Align::Left,
            AlignInput::Centre => Align::Centre,
            AlignInput::Right => Align::Right,
        }
    }
}

/// Millimetres from the top-left corner of the paper. edit_text returns the sheet's size.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
pub struct PositionInput {
    pub x_mm: f64,
    pub y_mm: f64,
}
