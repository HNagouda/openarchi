//! A project is one site plus the sheets the user made of it, saved as one file.
//!
//! The fetched scene is stored inside, so a project opens offline and never
//! changes under the user because a data service updated. It is stored
//! compressed, see `packed_scene`.

mod look_check;
pub mod packed_scene;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::area::Area;
use crate::edits::SiteEdits;
use crate::highlight::Highlight;
use crate::model::Model;
use crate::pin::Pin;
use crate::report::Report;
use crate::scene::{Layer, Scene};
use crate::section::SectionLine;
use crate::sheet::Sheet;
use crate::theme::LayerOverride;
use crate::view::{SunMoment, View};
use crate::wind::WindSeason;
use crate::{Error, Result};

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
    /// The first sheet. Projects from before sheets had tabs have only this one.
    pub look: Look,
    /// Pages the user laid out from the project's charts, analysis and sheets.
    /// Older projects have none.
    #[serde(default)]
    pub reports: Vec<Report>,
    /// Every sheet after the first, in tab order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub more_sheets: Vec<Look>,
    /// Landmarks the user pinned, shown on every sheet that does not hide them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pins: Vec<Pin>,
    /// Areas the user highlighted inside the site, shown on every sheet that does not hide them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub highlights: Vec<Highlight>,
    /// Trees and buildings the user took away, moved or planted, seen on
    /// every sheet. Older projects have none; a model's replaced buildings
    /// from before edits existed become edits under the model (`library::migrate`).
    #[serde(default, skip_serializing_if = "SiteEdits::is_empty")]
    pub edits: SiteEdits,
    /// `None` until the site has been fetched.
    /// Left out of the file while `None`, so `Library::save_with_packed_scene`
    /// can write the packed text in its place. Last, because it is nearly the
    /// whole file: the project list reads every other field before it.
    #[serde(
        with = "packed_scene",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub scene: Option<Scene>,
    /// The building model the user imported, placed on the site. After the
    /// scene, so the project list never reads its mesh. Older projects have none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<Model>,
}

impl Project {
    /// Every sheet in tab order, the first one being `look`.
    pub fn sheets(&self) -> Vec<Look> {
        let mut sheets = vec![self.look.clone()];
        sheets.extend(self.more_sheets.iter().cloned());
        sheets
    }

    /// Replaces every sheet, each kept to `Look::check`. A project always keeps at least one.
    pub fn set_sheets(&mut self, sheets: Vec<Look>) -> Result<()> {
        for look in &sheets {
            look.check()?;
        }
        let mut sheets = sheets.into_iter();
        let Some(first) = sheets.next() else {
            return Err(Error::Invalid(
                "a project needs at least one sheet".to_string(),
            ));
        };
        self.look = first;
        self.more_sheets = sheets.collect();
        Ok(())
    }

    pub fn sheet_count(&self) -> usize {
        1 + self.more_sheets.len()
    }

    /// The sheet at `number` in tab order, counting from 1 as the tabs do.
    pub fn sheet(&self, number: usize) -> Result<&Look> {
        let count = self.sheet_count();
        match number {
            1 => Ok(&self.look),
            _ if number >= 2 && number <= count => Ok(&self.more_sheets[number - 2]),
            _ => Err(Error::Invalid(format!(
                "sheet must be 1 to {count}, got {number}"
            ))),
        }
    }

    /// Sheets whose eye stood on a pin that is gone go back to the site
    /// centre, so a new pin that reuses the id does not take the eye.
    pub fn untie_lost_view_pins(&mut self) {
        let pins = &self.pins;
        for look in std::iter::once(&mut self.look).chain(&mut self.more_sheets) {
            let is_lost = look
                .view_pin
                .as_ref()
                .is_some_and(|id| !pins.iter().any(|pin| pin.id == *id));
            if is_lost {
                look.view_pin = None;
            }
        }
    }

    pub fn sheet_mut(&mut self, number: usize) -> Result<&mut Look> {
        self.sheet(number)?;
        if number == 1 {
            Ok(&mut self.look)
        } else {
            Ok(&mut self.more_sheets[number - 2])
        }
    }
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
    /// The sheet's tab name. Empty in projects from before sheets had tabs,
    /// where the app shows "Sheet 1".
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// A smaller area inside the fetched site that this sheet frames and
    /// clips to. `None` draws the whole site. See `selection`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<Area>,
    /// Leaves the project's pins off this sheet.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hide_pins: bool,
    /// Leaves the project's highlights off this sheet.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hide_highlights: bool,
    /// The line this sheet is cut along in the section view, drawn on its
    /// plan as a cut marker. `None` until the user draws one; older projects have none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<SectionLine>,
    /// Where the view diagram's eye stands, in RD New metres, when it was
    /// placed on the sheet. `None` stands it at the site centre; older
    /// projects have none. See `viewshed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view_point: Option<(f64, f64)>,
    /// The months the wind diagram takes its prevailing wind from. Older
    /// projects have none and draw the wind of the whole year.
    #[serde(default, skip_serializing_if = "WindSeason::is_year")]
    pub wind_season: WindSeason,
    /// The pin the view diagram's eye stands on, by id, so the eye follows
    /// the pin when it moves. Wins over `view_point`. Older projects have none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view_pin: Option<String>,
}

impl Look {
    /// Where the view diagram's eye stands, in RD New metres: on its pin, at
    /// the placed point, or `None` for the site centre. A pin that is gone
    /// stands the eye back at the site centre rather than at a stale place.
    pub fn eye_among(&self, pins: &[Pin]) -> Option<(f64, f64)> {
        match &self.view_pin {
            Some(id) => pins
                .iter()
                .find(|pin| pin.id == *id)
                .map(|pin| (pin.x, pin.y)),
            None => self.view_point,
        }
    }
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
            name: String::new(),
            selection: None,
            hide_pins: false,
            hide_highlights: false,
            section: None,
            view_point: None,
            wind_season: WindSeason::Year,
            view_pin: None,
        }
    }
}
