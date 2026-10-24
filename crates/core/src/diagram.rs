//! A diagram is the question a sheet answers: which layers show, which one
//! carries the theme colour, and how features are classified.
//!
//! Diagrams know nothing about colour. The renderer resolves each layer like
//! this, later steps winning:
//!
//! 1. the theme's style for the layer,
//! 2. if `mute_others` is set and the layer is neither `emphasis` nor the
//!    classified layer: fill and stroke become `@muted`, patterns are dropped,
//! 3. for the classified layer: fill comes from the theme's ramp or categories,
//!    and features with no value get `@unknown`,
//! 4. the user's layer overrides.

use serde::{Deserialize, Serialize};

use crate::scene::Layer;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagram {
    /// Lowercase, dash-separated, unique: "figure-ground".
    pub id: String,
    pub name: String,
    /// One sentence on the question it answers, shown in the picker.
    pub description: String,
    /// Layers drawn, bottom to top.
    pub layers: Vec<Layer>,
    /// The layer the eye should land on.
    pub emphasis: Option<Layer>,
    pub mute_others: bool,
    pub classify: Option<Classify>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Classify {
    pub layer: Layer,
    pub attribute: Attribute,
    pub scheme: Scheme,
    /// Legend title, for example "Year built".
    pub title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Attribute {
    YearBuilt,
    UseClass,
    HeightM,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Scheme {
    /// Classes split at `breaks`: n breaks make n + 1 classes, each taking the
    /// next ramp colour. `labels` has one entry per class.
    Sequential { breaks: Vec<f64>, labels: Vec<String> },
    /// One class per value, matched exactly, in order. A feature with several
    /// values (mixed use) takes the first that matches. Anything unmatched falls
    /// in "Other" with the last category colour.
    Categorical { categories: Vec<Category> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Category {
    pub value: String,
    pub label: String,
}
