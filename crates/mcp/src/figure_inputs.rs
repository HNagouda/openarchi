//! What the figure tools accept.

use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetRatios {
    /// The project id.
    pub id: String,
    /// Theme whose colours the rows take, like "paper". Defaults to the sheet's own theme.
    pub theme_id: Option<String>,
    /// Which sheet tab, counting from 1. A sheet opened from a highlight gives the ratios of that smaller area.
    pub tab: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetEnvironment {
    /// The project id.
    pub id: String,
    /// Which sheet tab, counting from 1. A sheet opened from a highlight gives the figures of that smaller area.
    pub tab: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetAnalysis {
    /// The project id.
    pub id: String,
    /// Which sheet tab, counting from 1. On a sheet opened from a highlight the site summary describes
    /// that smaller area; location, climate and rain always describe the whole site.
    pub tab: Option<usize>,
}
