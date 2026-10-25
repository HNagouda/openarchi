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
//!    dashes and shadows are dropped, so the layers fuse into one solid figure;
//!    for layers in `blank`: fill becomes the paper tinted towards ink and the
//!    edge a fine `@ink` line, so they stand back from a coloured ground without matching a class;
//!    for layers in `outline` on a plan: no fill and a fine `@ink` edge, blank in a tilted view,
//! 4. for the classified layer: fill comes from the theme's ramp or categories,
//!    and features with no value get `@unknown`,
//! 5. the user's layer overrides.
//!
//! A diagram can have variants that colour another layer or measure for the
//! same question. The renderer first turns it into the plain diagram for the
//! look's variant, with `drawn_as`, so nothing past that point knows about them.

use serde::{Deserialize, Serialize};

use crate::scene::{Layer, Scene};

/// Stands in a legend note for the year of the data.
const YEAR_PLACEHOLDER: &str = "{year}";
/// Stands in a legend note for the look's sun date: "21 March".
pub const DATE_PLACEHOLDER: &str = "{date}";
/// Stands in a legend note for the wind the look draws: "from the SW, most common all year (12% of hours)".
pub const WIND_PLACEHOLDER: &str = "{wind}";
/// What a note says when the scene does not know the year of its data.
const UNKNOWN_YEAR: &str = "year unknown";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagram {
    /// Lowercase, dash-separated, unique: "figure-ground".
    pub id: String,
    pub name: String,
    /// The heading the picker lists it under: "Environment". Diagrams of one
    /// group sit next to each other in the catalog.
    pub group: String,
    /// One sentence on the question it answers, shown in the picker.
    pub description: String,
    /// Layers drawn, bottom to top. Empty in a diagram with variants, where
    /// each variant names its own.
    #[serde(default)]
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
    /// Layers drawn blank, near the paper colour with a fine ink edge: the
    /// buildings over hours of sun, where any fill would read as a class.
    #[serde(default)]
    pub blank: Vec<Layer>,
    /// Layers drawn as a fine ink edge without a fill on a plan, so a layer
    /// under them that covers roofs too shows through: the buildings over the
    /// solar radiation. In a tilted view they stand up blank instead.
    #[serde(default)]
    pub outline: Vec<Layer>,
    pub classify: Option<Classify>,
    /// A computed sun drawing on top of the layers, which needs no data beyond
    /// building heights and the site's position. `SunHours` draws nothing
    /// here: its bands come from the sun hours layer, and the app reads it
    /// to show the date without the time.
    #[serde(default)]
    pub solar: Option<Solar>,
    /// Zooms a plan out to the walk band of this many minutes, draws the walk
    /// bands up to it, and shows only the places within it. Needs a scene
    /// with a walk; without one the diagram stays at the site.
    #[serde(default)]
    pub walk_minutes: Option<u32>,
    /// Drawn in plan only: the axonometric and explode views are refused,
    /// and an old look that asks for one draws the plan. The wind is solved
    /// in two dimensions, so a tilted view would add height it never had.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub plan_only: bool,
    /// Ways to draw the same question with another layer or measure, such as
    /// NO₂, PM10 and PM2.5 for air quality. The first is the default.
    #[serde(default)]
    pub variants: Vec<Variant>,
}

/// One way to draw a diagram. It replaces the diagram's name, description,
/// layers, emphasis and classification; everything else is shared.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variant {
    /// Lowercase, unique within its diagram: "pm10".
    pub id: String,
    /// The short label on the switch: "PM10".
    pub name: String,
    /// The sheet title when this variant is drawn: "Air quality, PM10".
    pub title: String,
    pub description: String,
    pub layers: Vec<Layer>,
    pub emphasis: Option<Layer>,
    pub classify: Option<Classify>,
}

impl Diagram {
    /// The variant `id` names, or the first when `id` is `None` or unknown.
    /// `None` only for a diagram without variants.
    pub fn variant(&self, id: Option<&str>) -> Option<&Variant> {
        let named = id.and_then(|id| self.variants.iter().find(|variant| variant.id == id));
        named.or(self.variants.first())
    }

    /// The diagram as the renderer draws it for one variant: a plain diagram
    /// with that variant's fields filled in. A diagram without variants comes
    /// back unchanged.
    pub fn drawn_as(&self, variant: Option<&str>) -> Diagram {
        let Some(chosen) = self.variant(variant) else {
            return self.clone();
        };

        Diagram {
            name: chosen.title.clone(),
            description: chosen.description.clone(),
            layers: chosen.layers.clone(),
            emphasis: chosen.emphasis,
            classify: chosen.classify.clone(),
            variants: Vec::new(),
            ..self.clone()
        }
    }

    /// Every sheet this diagram makes, with the variant each one is drawn for:
    /// one per variant, or the diagram itself.
    pub fn drawings(&self) -> Vec<(Option<String>, Diagram)> {
        if self.variants.is_empty() {
            return vec![(None, self.clone())];
        }

        self.variants
            .iter()
            .map(|variant| (Some(variant.id.clone()), self.drawn_as(Some(&variant.id))))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Solar {
    /// The sun's daily arcs on the solstices and equinox, over the plan.
    SunPath,
    /// Building shadows at the look's date and time.
    Shadow,
    /// Hours of direct sun on the ground over the look's date. The time of
    /// day is not used.
    SunHours,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Classify {
    pub layer: Layer,
    pub attribute: Attribute,
    pub scheme: Scheme,
    /// Legend title, for example "Year built".
    pub title: String,
    /// A small line under the legend title: the unit, the source and its year.
    /// Write the year as `{year}`; `note_for` fills in the year of the data
    /// the scene holds, which depends on when the site was fetched.
    #[serde(default)]
    pub note: Option<String>,
    /// Which theme ramp a sequential scheme takes its colours from.
    #[serde(default)]
    pub ramp: Ramp,
    /// Run the ramp from dark to light, whichever way the theme runs it, for
    /// a measure where more should read brighter, such as hours of sun.
    /// Otherwise more reads stronger against the paper: darker on a light
    /// theme, lighter on a dark one.
    #[serde(default)]
    pub is_brighter_more: bool,
    /// The legend label for features without a value, when "Unknown" would
    /// mislead: CBS keeps some cells secret on purpose.
    #[serde(default)]
    pub unknown_label: Option<String>,
}

impl Classify {
    /// The note with `{year}` replaced by the year of the classified layer's
    /// data in `scene`: "Lden in dB. RIVM, 2022."
    pub fn note_for(&self, scene: &Scene) -> Option<String> {
        let year = match scene.year_of(self.layer) {
            Some(year) => year.to_string(),
            None => UNKNOWN_YEAR.to_string(),
        };
        self.note
            .as_ref()
            .map(|note| note.replace(YEAR_PLACEHOLDER, &year))
    }
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
    /// The group of a business, facility or transit stop: "food_drink", "tram".
    Group,
    /// Storeys estimated from roof height, see `floors.rs`.
    Floors,
    /// Footprint times estimated storeys, in m².
    GrossFloorAreaM2,
    /// A city block's building coverage, 0 to 1.
    Coverage,
    /// A city block's gross floor area over its area.
    FloorSpaceIndex,
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

/// The class of `value` under sequential `breaks`: a value on a break falls in
/// the class above it, so n breaks make n + 1 classes, 0 below the first.
/// The map bands, the legend and the ratio bins all split here, so they agree.
pub fn class_index(value: f64, breaks: &[f64]) -> usize {
    breaks.iter().filter(|&&limit| value >= limit).count()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Category {
    pub value: String,
    pub label: String,
}
