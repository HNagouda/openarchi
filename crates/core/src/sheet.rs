//! The paper a diagram is laid out on: size, scale, frame and furniture.

use serde::{Deserialize, Serialize};

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
        }
    }
}
