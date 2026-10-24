//! Where things go on the sheet: the drawing zone and the furniture zone.
//!
//! Landscape sheets get a furniture column on the right, portrait and square
//! sheets a band along the bottom. The two zones never overlap, and the map is
//! clipped to its zone, so furniture cannot collide with the drawing. Sizes are
//! set for A3 and scale with the paper's short side.

use crate::sheet::{Paper, Sheet};

/// The paper whose furniture sizes the constants below are tuned for (A3).
const REFERENCE_SHORT_SIDE_MM: f64 = 297.0;
const MARGIN_MM: f64 = 14.0;
const COLUMN_WIDTH_MM: f64 = 88.0;
const BAND_HEIGHT_MM: f64 = 62.0;
/// Space between the drawing and the furniture.
const GUTTER_MM: f64 = 12.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn centre(&self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// Furniture stacked in a column right of the drawing.
    Column,
    /// Furniture side by side in a band under the drawing.
    Band,
}

#[derive(Debug, Clone, Copy)]
pub struct Layout {
    pub width: f64,
    pub height: f64,
    /// Furniture scale: 1 on A3, about 0.7 on A4, 2 on A1.
    pub unit: f64,
    pub drawing: Rect,
    pub furniture: Rect,
    pub placement: Placement,
}

impl Layout {
    pub fn new(sheet: &Sheet) -> Layout {
        let is_landscape = sheet.landscape && sheet.paper != Paper::Square;
        let (width, height) = sheet.size_mm();
        let unit = width.min(height) / REFERENCE_SHORT_SIDE_MM;
        let margin = MARGIN_MM * unit;
        let gutter = GUTTER_MM * unit;
        let inner = Rect {
            x: margin,
            y: margin,
            width: width - 2.0 * margin,
            height: height - 2.0 * margin,
        };

        if is_landscape {
            let column_width = COLUMN_WIDTH_MM * unit;
            let drawing = Rect {
                width: inner.width - column_width - gutter,
                ..inner
            };
            let furniture = Rect {
                x: drawing.right() + gutter,
                width: column_width,
                ..inner
            };
            Layout {
                width,
                height,
                unit,
                drawing,
                furniture,
                placement: Placement::Column,
            }
        } else {
            let band_height = BAND_HEIGHT_MM * unit;
            let drawing = Rect {
                height: inner.height - band_height - gutter,
                ..inner
            };
            let furniture = Rect {
                y: drawing.bottom() + gutter,
                height: band_height,
                ..inner
            };
            Layout {
                width,
                height,
                unit,
                drawing,
                furniture,
                placement: Placement::Band,
            }
        }
    }
}
