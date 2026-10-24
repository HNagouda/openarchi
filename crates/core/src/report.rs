//! Reports: pages the user lays out themselves, with charts, analysis, sheets
//! and text placed in boxes measured in millimetres.
//!
//! A block stores what it shows by reference (a ratios group id, an analysis
//! section id, a diagram id), never a picture of it. Every render draws the
//! block again from the project's data in the report's palette, so changing
//! the palette or fetching the site again restyles the whole report. Each page
//! is one SVG composed from its blocks, and PDF and PNG are converted from
//! those SVGs, the same way a sheet is exported.

mod analysis_blocks;
mod compose;
mod data;
mod draw;
mod files;
mod ratio_blocks;
mod text_block;

use serde::{Deserialize, Serialize};

use crate::sheet::{Paper, TextStyle};
use crate::{Error, Result};

pub use compose::{backdrop_svg, block_svg, page_svg, pages_svg};
pub use data::{parts_needed, ReportData};
pub use files::{export, output_paths, ReportFormat};

/// What a page shows behind its blocks when nothing is chosen: the palette's paper.
pub const DEFAULT_BACKGROUND: &str = "@paper";
/// Below this a block cannot be grabbed or read, on screen or on paper.
pub const MIN_BLOCK_MM: f64 = 5.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    /// Unique within the project: "report-1".
    pub id: String,
    pub name: String,
    /// The palette every block is drawn in. `None` follows the project's sheet.
    #[serde(default)]
    pub theme_id: Option<String>,
    /// "@paper", another palette role, or "#RRGGBB".
    #[serde(default = "default_background")]
    pub background: String,
    #[serde(default)]
    pub show_page_numbers: bool,
    pub pages: Vec<Page>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    /// Unique within the report: "page-1".
    pub id: String,
    pub paper: Paper,
    pub landscape: bool,
    /// Drawn in this order, so the last block lies on top.
    pub blocks: Vec<Block>,
}

/// A box on the page and what it shows. Position and size are in millimetres
/// from the page's top-left corner.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    /// Unique within the report: "block-7".
    pub id: String,
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
    pub content: Content,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Content {
    /// One ratios group, such as "land_use", drawn as bars, a pie or a table.
    Ratios { group: String, style: RatiosStyle },
    /// One figure of an analysis section: section "wind", figure "wind-roses".
    AnalysisFigure { section: String, figure: String },
    /// The labelled values at the top of an analysis section.
    AnalysisFacts { section: String },
    /// The design notes of an analysis section, with where its numbers come from.
    AnalysisNotes { section: String },
    /// A whole sheet of the project in another diagram, with the project's
    /// paper, view, sun and text. `None` draws the diagram's first variant.
    Sheet {
        diagram_id: String,
        #[serde(default)]
        variant: Option<String>,
    },
    /// Line breaks in `text` are kept; long lines wrap at the box's width.
    Text {
        text: String,
        #[serde(default)]
        style: TextStyle,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RatiosStyle {
    Bars,
    Pie,
    Table,
}

fn default_background() -> String {
    DEFAULT_BACKGROUND.to_string()
}

impl Page {
    /// Width and height in millimetres, turned for landscape. Square paper never turns.
    pub fn size_mm(&self) -> (f64, f64) {
        let (width, height) = self.paper.size_mm();
        if self.landscape && self.paper != Paper::Square {
            (height, width)
        } else {
            (width, height)
        }
    }
}

impl Report {
    /// Every page and block is one the renderer can draw, or an error naming it.
    pub fn check(&self) -> Result<()> {
        if self.pages.is_empty() {
            return Err(Error::Invalid(format!("report {} has no pages", self.id)));
        }
        for page in &self.pages {
            for block in &page.blocks {
                block.check()?;
            }
        }

        Ok(())
    }
}

impl Block {
    fn check(&self) -> Result<()> {
        let is_finite = [self.x_mm, self.y_mm, self.width_mm, self.height_mm]
            .iter()
            .all(|value| value.is_finite());
        if !is_finite || self.width_mm < MIN_BLOCK_MM || self.height_mm < MIN_BLOCK_MM {
            return Err(Error::Invalid(format!(
                "block {} must be at least {MIN_BLOCK_MM} mm wide and tall, got {} × {} mm",
                self.id, self.width_mm, self.height_mm
            )));
        }
        if let Content::Text { style, .. } = &self.content {
            style
                .check()
                .map_err(|error| Error::Invalid(format!("block {}: {error}", self.id)))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
