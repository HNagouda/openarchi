//! A project is one site plus the look the user chose for it, saved as one file.
//!
//! The fetched scene is stored inside, so a project opens offline and never
//! changes under the user because a data service updated. It is stored
//! compressed, see `packed_scene`.

pub mod packed_scene;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::area::Area;
use crate::report::Report;
use crate::scene::{Layer, Scene};
use crate::sheet::Sheet;
use crate::theme::LayerOverride;
use crate::view::{SunMoment, View};

/// Bump when the file shape changes, and add a step to `library::migrate`.
/// Version 2 stores the scene compressed. Version 3 adds styled, movable sheet text.
pub const PROJECT_FORMAT_VERSION: u32 = 3;

pub const PROJECT_EXTENSION: &str = "openarchi";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub format_version: u32,
    /// Stable id, also the file name: "2026-10-24-westermarkt-3f9a".
    pub id: String,
    pub name: String,
    /// RFC 3339.
    pub created: String,
    pub modified: String,
    pub area: Area,
    /// `None` until the site has been fetched.
    #[serde(with = "packed_scene")]
    pub scene: Option<Scene>,
    pub look: Look,
    /// Pages the user laid out from the project's charts, analysis and sheets.
    /// Older projects have none.
    #[serde(default)]
    pub reports: Vec<Report>,
}

/// What the user chose, by reference. The renderer resolves ids through the catalog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Look {
    pub diagram_id: String,
    /// One of the diagram's variants, such as "pm10" for air quality. `None`,
    /// or an id the diagram does not have, draws its first variant. Older
    /// projects have none.
    #[serde(default)]
    pub variant: Option<String>,
    pub theme_id: String,
    pub sheet: Sheet,
    #[serde(default)]
    pub overrides: BTreeMap<Layer, LayerOverride>,
    /// Plan, axonometric or explode. Older projects have none and open as a plan.
    #[serde(default)]
    pub view: View,
    /// The moment the sun path and shadow diagrams are drawn for.
    #[serde(default)]
    pub sun: SunMoment,
}

impl Default for Look {
    fn default() -> Self {
        Look {
            diagram_id: "context".to_string(),
            variant: None,
            theme_id: "paper".to_string(),
            sheet: Sheet::default(),
            overrides: BTreeMap::new(),
            view: View::default(),
            sun: SunMoment::default(),
        }
    }
}
