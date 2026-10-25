//! What the sheet and pin tools accept, with the schema an assistant sees.

use openarchi_core::pin::{PinIcon, PinSize};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::inputs::{AreaInput, LonLatInput};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddSheet {
    /// The project id.
    pub id: String,
    /// The tab's name, like "Square detail". Defaults to "Sheet N".
    pub name: Option<String>,
    /// The tab to copy the look from (diagram, theme, sheet, view), counting
    /// from 1. Defaults to 1. Change the copy afterwards with set_look and its `tab`.
    pub copy_of: Option<usize>,
    /// A smaller area inside the fetched site that this sheet frames, clips
    /// and zooms to, in longitude and latitude. Its ratios describe that area.
    /// Leave out to show the whole site.
    pub selection: Option<AreaInput>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddPin {
    /// The project id.
    pub id: String,
    /// Where the landmark is, as search_places returns locations. Must lie inside the fetched site.
    pub location: LonLatInput,
    /// A short name shown beside the marker, like "Westerkerk". Up to 80 characters.
    pub label: Option<String>,
    /// The marker. Defaults to "pin".
    pub icon: Option<PinIconInput>,
    /// How large the marker and its label draw. Defaults to "medium".
    pub size: Option<PinSizeInput>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EditPin {
    /// The project id.
    pub id: String,
    /// The pin's id from list_pins, like "pin-1".
    pub pin_id: String,
    /// A new place for the pin, inside the fetched site.
    pub location: Option<LonLatInput>,
    /// A new label; an empty string leaves only the marker.
    pub label: Option<String>,
    pub icon: Option<PinIconInput>,
    pub size: Option<PinSizeInput>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PinId {
    /// The project id.
    pub id: String,
    /// The pin's id from list_pins, like "pin-1".
    pub pin_id: String,
}

/// "pin" is a map pin; "star", "flag" and "dot" are what they say. The app's
/// numbered disc is left out, because the app no longer offers it either and
/// an assistant does only what the user can do.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PinIconInput {
    Pin,
    Star,
    Flag,
    Dot,
}

impl From<PinIconInput> for PinIcon {
    fn from(icon: PinIconInput) -> Self {
        match icon {
            PinIconInput::Pin => PinIcon::Pin,
            PinIconInput::Star => PinIcon::Star,
            PinIconInput::Flag => PinIcon::Flag,
            PinIconInput::Dot => PinIcon::Dot,
        }
    }
}

/// The four steps the app's pin bar offers, smallest first.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PinSizeInput {
    Small,
    Medium,
    Large,
    ExtraLarge,
}

impl From<PinSizeInput> for PinSize {
    fn from(size: PinSizeInput) -> Self {
        match size {
            PinSizeInput::Small => PinSize::Small,
            PinSizeInput::Medium => PinSize::Medium,
            PinSizeInput::Large => PinSize::Large,
            PinSizeInput::ExtraLarge => PinSize::ExtraLarge,
        }
    }
}
