//! A diagram is the question a sheet answers: which layers show, which one
//! carries the theme colour, and how features are classified.
//!
//! Diagrams know nothing about colour. The renderer resolves each layer like
//! this, later steps winning:
//!
//! 1. the theme's style for the layer,
//! 2. if `mute_others` is set and the layer is not `emphasis`, not in
//!    `highlight`, not in `as_figure` and not the classified layer: fill and
//!    stroke become `@muted`, patterns and shadows are dropped,
//! 3. for layers in `as_figure`: fill and stroke become `@figure`, patterns,
//!    dashes and shadows are dropped, so the layers fuse into one solid figure,
//! 4. for the classified layer: fill comes from the theme's ramp or categories,
//!    and features with no value get `@unknown`,
//! 5. the user's layer overrides.

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
    /// More layers that keep their theme colours when `mute_others` is set,
    /// for diagrams about a group of layers (green and water, the road network).
    #[serde(default)]
    pub highlight: Vec<Layer>,
    /// Layers drawn as one solid figure, for an inverse figure-ground where the
    /// open space is the subject.
    #[serde(default)]
    pub as_figure: Vec<Layer>,
    pub classify: Option<Classify>,
    /// A computed sun drawing on top of the layers. Needs no data beyond
    /// building heights and the site's position.
    #[serde(default)]
    pub solar: Option<Solar>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Solar {
    /// The sun's daily arcs on the solstices and equinox, over the plan.
    SunPath,
    /// Building shadows at the look's date and time.
    Shadow,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Classify {
    pub layer: Layer,
    pub attribute: Attribute,
    pub scheme: Scheme,
    /// Legend title, for example "Year built".
    pub title: String,
    /// A small line under the legend title: the unit, the source and its year.
    #[serde(default)]
    pub note: Option<String>,
    /// Which theme ramp a sequential scheme takes its colours from.
    #[serde(default)]
    pub ramp: Ramp,
    /// The legend label for features without a value, when "Unknown" would
    /// mislead: CBS keeps some cells secret on purpose.
    #[serde(default)]
    pub unknown_label: Option<String>,
}

/// A theme's colour ramps. A theme without the asked-for ramp uses its main one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ramp {
    #[default]
    Main,
    /// Shallow to deep water, for flood depth.
    Water,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Attribute {
    YearBuilt,
    UseClass,
    HeightM,
    /// A band's lower edge, or the residents of a statistics cell.
    Value,
    Dwellings,
    FloorAreaM2,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Scheme {
    /// Classes split at `breaks`: n breaks make n + 1 classes, each taking the
    /// next ramp colour. `labels` has one entry per class.
    Sequential {
        breaks: Vec<f64>,
        labels: Vec<String>,
    },
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
