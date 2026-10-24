//! The paper a diagram is laid out on: size, scale, frame and furniture.

use serde::{Deserialize, Serialize};

use crate::render::{is_hex, FONT_FAMILIES, ROLES};
use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sheet {
    pub paper: Paper,
    pub landscape: bool,
    /// Scale denominator: 2500 means 1:2500.
    pub scale: u32,
    pub frame: Frame,
    pub title: String,
    pub subtitle: String,
    pub show_legend: bool,
    pub show_north_arrow: bool,
    pub show_scale_bar: bool,
    /// Data sources and licenses in small print. Required by CC BY sources, so on by default.
    pub show_credits: bool,
    /// The neighbourhood around the site, drawn muted outside the boundary.
    /// On for new sheets; projects from before it open without it.
    #[serde(default)]
    pub show_context: bool,
    /// How the title and subtitle are set. Every field left empty follows the theme.
    #[serde(default)]
    pub title_style: TextStyle,
    #[serde(default)]
    pub subtitle_style: TextStyle,
    /// Where the title and subtitle sit. `None` keeps them in the automatic layout.
    #[serde(default)]
    pub title_position: Option<Position>,
    #[serde(default)]
    pub subtitle_position: Option<Position>,
    /// Text boxes the user added, drawn above everything else in this order.
    #[serde(default)]
    pub texts: Vec<TextBox>,
}

/// How a piece of sheet text is set. A field left empty follows the theme.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextStyle {
    /// A bundled family: "Inter", "Source Serif 4" or "IBM Plex Mono".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    /// Points on paper, so 12 pt prints at 12 pt whatever the paper size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_pt: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
    /// "#RRGGBB" or a palette role like "@accent".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colour: Option<String>,
    /// 0 to 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f64>,
}

/// Text sizes a sheet accepts, in points. Below 2 pt nothing prints legibly,
/// and 400 pt already fills an A3 sheet with two letters.
pub const MIN_TEXT_SIZE_PT: f64 = 2.0;
pub const MAX_TEXT_SIZE_PT: f64 = 400.0;

impl TextStyle {
    /// Every set field is one the renderer can draw, or an error naming the bad value.
    pub fn check(&self) -> Result<()> {
        if let Some(family) = &self.family {
            if !FONT_FAMILIES.contains(&family.as_str()) {
                return Err(Error::Invalid(format!(
                    "family must be one of {}, got {family:?}",
                    FONT_FAMILIES.join(", ")
                )));
            }
        }
        if let Some(size) = self.size_pt {
            if !(MIN_TEXT_SIZE_PT..=MAX_TEXT_SIZE_PT).contains(&size) {
                return Err(Error::Invalid(format!(
                    "size_pt must be between {MIN_TEXT_SIZE_PT} and {MAX_TEXT_SIZE_PT}, got {size}"
                )));
            }
        }
        if let Some(opacity) = self.opacity {
            if !(0.0..=1.0).contains(&opacity) {
                return Err(Error::Invalid(format!(
                    "opacity must be between 0 and 1, got {opacity}"
                )));
            }
        }
        if let Some(colour) = &self.colour {
            let is_role = colour
                .strip_prefix('@')
                .is_some_and(|name| ROLES.contains(&name));
            if !is_role && !is_hex(colour) {
                return Err(Error::Invalid(format!(
                    "colour must look like '#RRGGBB' or a palette role like '@accent', got {colour:?}"
                )));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    Left,
    Centre,
    Right,
}

/// A text's anchor on the sheet, in millimetres from the paper's top-left
/// corner. It is the top of the first line, at the left edge, centre or right
/// edge depending on the alignment, so changing the alignment keeps the side
/// the user lined up.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x_mm: f64,
    pub y_mm: f64,
}

/// A text box the user added to the sheet. Line breaks in `text` are kept.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextBox {
    /// Unique within the sheet, never "title" or "subtitle": "text-1".
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub style: TextStyle,
    /// `None` puts the box at the top-left of the drawing.
    #[serde(default)]
    pub position: Option<Position>,
}

/// The ids the title and subtitle go by wherever sheet texts are addressed by id.
pub const TITLE_ID: &str = "title";
pub const SUBTITLE_ID: &str = "subtitle";

impl Sheet {
    /// Width and height in millimetres, turned for landscape. Square paper never turns.
    pub fn size_mm(&self) -> (f64, f64) {
        let (width, height) = self.paper.size_mm();
        if self.landscape && self.paper != Paper::Square {
            (height, width)
        } else {
            (width, height)
        }
    }

    /// The first "text-N" id no text box uses yet.
    pub fn next_text_id(&self) -> String {
        let mut number = self.texts.len() + 1;
        loop {
            let id = format!("text-{number}");
            if self.texts.iter().all(|text| text.id != id) {
                return id;
            }
            number += 1;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Paper {
    A4,
    A3,
    A2,
    A1,
    A0,
    /// 1:1 square for social posts and portfolios: 300 × 300 mm.
    Square,
}

impl Paper {
    /// Width and height in millimetres, portrait.
    pub fn size_mm(self) -> (f64, f64) {
        match self {
            Paper::A4 => (210.0, 297.0),
            Paper::A3 => (297.0, 420.0),
            Paper::A2 => (420.0, 594.0),
            Paper::A1 => (594.0, 841.0),
            Paper::A0 => (841.0, 1189.0),
            Paper::Square => (300.0, 300.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Frame {
    /// The drawing is cropped to a circle around the site, the classic site-analysis look.
    Circle,
    /// The drawing fills a square.
    Square,
    /// The drawing is cropped to the drawn area's own shape.
    Area,
}

impl Default for Sheet {
    fn default() -> Self {
        Sheet {
            paper: Paper::A3,
            landscape: true,
            scale: 2500,
            frame: Frame::Circle,
            title: String::new(),
            subtitle: String::new(),
            show_legend: true,
            show_north_arrow: true,
            show_scale_bar: true,
            show_credits: true,
            show_context: true,
            title_style: TextStyle::default(),
            subtitle_style: TextStyle::default(),
            title_position: None,
            subtitle_position: None,
            texts: Vec::new(),
        }
    }
}
