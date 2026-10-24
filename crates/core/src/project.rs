//! A project is one site plus the look the user chose for it, saved as one file.
//!
//! The fetched scene is stored inside, so a project opens offline and never
//! changes under the user because a data service updated.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::area::Area;
use crate::scene::{Layer, Scene};
use crate::sheet::Sheet;
use crate::theme::LayerOverride;

/// Bump when the file shape changes, and add a step to `library::migrate`.
pub const PROJECT_FORMAT_VERSION: u32 = 1;

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
    pub scene: Option<Scene>,
    pub look: Look,
}

/// What the user chose, by reference. The renderer resolves ids through the catalog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Look {
    pub diagram_id: String,
    pub theme_id: String,
    pub sheet: Sheet,
    #[serde(default)]
    pub overrides: BTreeMap<Layer, LayerOverride>,
}

impl Default for Look {
    fn default() -> Self {
        Look {
            diagram_id: "context".to_string(),
            theme_id: "paper".to_string(),
            sheet: Sheet::default(),
            overrides: BTreeMap::new(),
        }
    }
}
