//! What the highlight tools accept, with the schema an assistant sees.

use openarchi_core::highlight::Outline;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::inputs::AreaInput;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddHighlight {
    /// The project id.
    pub id: String,
    /// The area to highlight, in longitude and latitude: a circle, a rectangle
    /// or a polygon. Its middle must lie inside the fetched site, and it must
    /// cover at least 4 m².
    pub shape: AreaInput,
    /// A short name drawn above it and listed in the legend, like "Plot" or
    /// "New house". Up to 80 characters. Without one the legend says "Highlighted area".
    pub label: Option<String>,
    /// "solid" (the default) for what exists, like the plot; "dashed" for what
    /// is proposed; "dotted" for what is studied or uncertain.
    pub outline: Option<OutlineInput>,
    /// The colour of the outline and tint: a palette role like "@accent" (the
    /// default), "@water" or "@green", or "#RRGGBB".
    pub colour: Option<String>,
    /// True fades everything outside the highlight a little. Defaults to false.
    pub dims_rest: Option<bool>,
    /// True adds the area in m² to the label. Defaults to false.
    pub shows_area: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EditHighlight {
    /// The project id.
    pub id: String,
    /// The highlight's id from list_highlights, like "highlight-1".
    pub highlight_id: String,
    /// A new shape, to move or reshape it. Its middle must lie inside the fetched site.
    pub shape: Option<AreaInput>,
    /// A new label; an empty string leaves it unnamed.
    pub label: Option<String>,
    pub outline: Option<OutlineInput>,
    /// A palette role like "@accent" or "#RRGGBB".
    pub colour: Option<String>,
    pub dims_rest: Option<bool>,
    pub shows_area: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct HighlightId {
    /// The project id.
    pub id: String,
    /// The highlight's id from list_highlights, like "highlight-1".
    pub highlight_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OpenHighlight {
    /// The project id.
    pub id: String,
    /// The highlight's id from list_highlights, like "highlight-1".
    pub highlight_id: String,
    /// The tab to copy the look from, counting from 1. Defaults to 1.
    pub copy_of: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShowMarks {
    /// The project id.
    pub id: String,
    /// Which sheet tab, counting from 1. Defaults to 1.
    pub tab: Option<usize>,
    /// True shows the project's pins on this sheet, false hides them.
    pub pins: Option<bool>,
    /// True shows the project's highlights on this sheet, false hides them.
    pub highlights: Option<bool>,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OutlineInput {
    Solid,
    Dashed,
    Dotted,
}

impl From<OutlineInput> for Outline {
    fn from(outline: OutlineInput) -> Self {
        match outline {
            OutlineInput::Solid => Outline::Solid,
            OutlineInput::Dashed => Outline::Dashed,
            OutlineInput::Dotted => Outline::Dotted,
        }
    }
}
