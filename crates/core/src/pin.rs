//! Landmarks the user pins on the site: a small marker and a label at a place.
//!
//! Pins belong to the project rather than to one sheet, and sit in real RD New
//! metres, the frame the scene is fetched in. So every sheet shows a pin where
//! it belongs, whatever area the sheet frames and however its view is tilted.
//! A sheet can hide them all.

use geo::{Contains, Point};
use serde::{Deserialize, Serialize};

use crate::scene::Scene;
use crate::{Error, Result};

/// A label longer than this crowds the drawing; a sentence belongs in a text box.
pub const MAX_LABEL_CHARS: usize = 80;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pin {
    /// Unique within the project: "pin-1".
    pub id: String,
    pub icon: PinIcon,
    /// May be empty: a marker on its own is a fine landmark.
    #[serde(default)]
    pub label: String,
    /// RD New metres, not the scene's local frame, so a pin stays put when the site is fetched again.
    pub x: f64,
    pub y: f64,
}

/// The markers a pin can be, in the order the app's palette offers them. The
/// app no longer offers `Number`, which read as an error badge, but projects
/// and assistants that already use it still draw it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PinIcon {
    #[default]
    Pin,
    Star,
    Flag,
    Dot,
    /// A disc with a number: its place among the project's numbered pins, from 1.
    Number,
}

impl Pin {
    /// A pin the renderer can place, or an error naming what is wrong.
    pub fn check(&self) -> Result<()> {
        if !self.x.is_finite() || !self.y.is_finite() {
            return Err(Error::Invalid(format!(
                "pin {} needs a real position, got x {} and y {}",
                self.id, self.x, self.y
            )));
        }
        let length = self.label.chars().count();
        if length > MAX_LABEL_CHARS {
            return Err(Error::Invalid(format!(
                "pin {} has a label of {length} characters; keep it to {MAX_LABEL_CHARS}",
                self.id
            )));
        }

        Ok(())
    }
}

/// True when RD New point (`x`, `y`) lies inside the fetched site, where a pin can be drawn.
pub fn is_on_site(scene: &Scene, x: f64, y: f64) -> bool {
    let local = Point::new(x - scene.origin.0, y - scene.origin.1);
    scene.boundary.contains(&local)
}

/// The first "pin-N" id no pin uses yet.
pub fn next_pin_id(pins: &[Pin]) -> String {
    let mut number = pins.len() + 1;
    loop {
        let id = format!("pin-{number}");
        if pins.iter().all(|pin| pin.id != id) {
            return id;
        }
        number += 1;
    }
}

/// The number a numbered pin shows, counting only numbered pins, in project order.
pub fn number_of(pins: &[Pin], id: &str) -> Option<usize> {
    pins.iter()
        .filter(|pin| pin.icon == PinIcon::Number)
        .position(|pin| pin.id == id)
        .map(|index| index + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin(id: &str, icon: PinIcon) -> Pin {
        Pin {
            id: id.to_string(),
            icon,
            label: String::new(),
            x: 120_000.0,
            y: 487_000.0,
        }
    }

    #[test]
    fn numbered_pins_count_only_each_other() {
        let pins = [
            pin("pin-1", PinIcon::Number),
            pin("pin-2", PinIcon::Star),
            pin("pin-3", PinIcon::Number),
        ];
        assert_eq!(number_of(&pins, "pin-3"), Some(2));
        assert_eq!(number_of(&pins, "pin-2"), None);
    }

    #[test]
    fn the_next_id_skips_ids_in_use() {
        let pins = [pin("pin-2", PinIcon::Pin)];
        assert_eq!(next_pin_id(&pins), "pin-3");
        assert_eq!(next_pin_id(&[]), "pin-1");
    }

    #[test]
    fn a_long_label_is_refused_with_the_limit() {
        let mut long = pin("pin-1", PinIcon::Flag);
        long.label = "x".repeat(MAX_LABEL_CHARS + 1);
        assert!(long.check().unwrap_err().to_string().contains("80"));
    }
}
