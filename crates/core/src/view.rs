//! How a sheet looks at the site: straight down, tilted with buildings
//! standing up, as a stack of diagrams pulled apart, or in section. Also the
//! moment the sun and shadow diagrams are drawn for.
//!
//! Both live in the look, and every field has a default, so projects saved
//! before these existed open as plain plans at the default moment.

use serde::{Deserialize, Serialize};

const DEFAULT_ROTATION_DEG: f64 = 0.0;
const DEFAULT_TILT_DEG: f64 = 35.0;
/// The tilt is kept off the extremes: at 90 the view is a plan, near 0 the ground vanishes.
/// The app's Tilt slider uses the same limits (app/src/sheet/inspector/ViewSection.tsx).
pub const MIN_TILT_DEG: f64 = 15.0;
pub const MAX_TILT_DEG: f64 = 80.0;
/// The plates a new explode view stacks, top first. Each one is a built-in diagram id.
const DEFAULT_PLATES: [&str; 5] = [
    "building-use",
    "building-age",
    "landscape",
    "movement",
    "context",
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewMode {
    /// Straight down, the classic site plan.
    #[default]
    Plan,
    /// Tilted, with buildings extruded to their height and trees as volumes.
    Axonometric,
    /// Several diagrams of the site as flat tilted plates, stacked and labelled.
    Explode,
    /// An elevation along the sheet's section line: ground, buildings cut and beyond.
    Section,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct View {
    pub mode: ViewMode,
    /// Turns the site counterclockwise on the paper before tilting, in degrees.
    pub rotation_deg: f64,
    /// How steeply the viewer looks down: 90 would be a plan.
    pub tilt_deg: f64,
    /// Diagram ids stacked in the explode view, top plate first.
    pub plates: Vec<String>,
    /// Plates sit close together, overlapping like a stack of sheets.
    pub stacking: bool,
}

impl Default for View {
    fn default() -> Self {
        View {
            mode: ViewMode::Plan,
            rotation_deg: DEFAULT_ROTATION_DEG,
            tilt_deg: DEFAULT_TILT_DEG,
            plates: DEFAULT_PLATES.iter().map(|id| id.to_string()).collect(),
            stacking: false,
        }
    }
}

/// A day and a clock time, in the site's local time (CET, or CEST in summer).
///
/// The default is 21 March at 13:00. The spring equinox is the usual reference
/// day for Dutch sun studies, because the sun then stands at its average
/// height of the year, and 13:00 winter time is close to solar noon across the
/// Netherlands, so the default shadow is the short midday one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SunMoment {
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
}

impl Default for SunMoment {
    fn default() -> Self {
        SunMoment {
            month: 3,
            day: 21,
            hour: 13,
            minute: 0,
        }
    }
}

impl SunMoment {
    /// Plain words for a moment that cannot exist, or `None` when it is valid.
    pub fn problem(&self) -> Option<String> {
        // The reference year has no 29 February, see `sun::REFERENCE_YEAR`.
        let days_in_month = match self.month {
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            1..=12 => 31,
            _ => return Some(format!("month must be 1 to 12, got {}", self.month)),
        };
        if self.day == 0 || self.day > days_in_month {
            return Some(format!(
                "day must be 1 to {days_in_month} in month {}, got {}",
                self.month, self.day
            ));
        }
        if self.hour > 23 || self.minute > 59 {
            return Some(format!(
                "time must be between 00:00 and 23:59, got {:02}:{:02}",
                self.hour, self.minute
            ));
        }

        None
    }

    /// "21 March, 13:00".
    pub fn label(&self) -> String {
        format!("{}, {:02}:{:02}", self.date_label(), self.hour, self.minute)
    }

    /// "21 March".
    pub fn date_label(&self) -> String {
        format!("{} {}", self.day, month_name(self.month))
    }
}

/// The months' names, January first, in one place.
const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// 1 -> "January", 12 -> "December"; 0 and 13 wrap round to December and January.
pub(crate) fn month_name(month: u32) -> &'static str {
    MONTH_NAMES[(month as usize + 11) % 12]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_look_without_view_fields_reads_as_the_default() {
        let view: View = serde_json::from_str("{}").unwrap();
        assert_eq!(view, View::default());
        let sun: SunMoment = serde_json::from_str(r#"{"hour": 9}"#).unwrap();
        assert_eq!(sun.hour, 9);
        assert_eq!(sun.month, 3);
    }

    #[test]
    fn impossible_moments_are_refused() {
        let leap_day = SunMoment {
            month: 2,
            day: 29,
            ..SunMoment::default()
        };
        assert!(leap_day.problem().is_some());
        assert!(SunMoment::default().problem().is_none());
    }
}
