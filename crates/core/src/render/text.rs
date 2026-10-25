//! Sheet text the user can style and move: the title, the subtitle and their own text boxes.
//!
//! Each text starts from its kind's look in `typeset` and takes the user's
//! choices from its `TextStyle` on top. It is written as one group carrying its
//! id, anchor and resolved style as data attributes, so the app can find,
//! select and drag a text on the very SVG it shows instead of guessing where
//! the renderer put it.

use std::fmt::Write;

use super::color;
use super::fonts::{wrap, Face, BOLD_WEIGHT, REGULAR_WEIGHT};
use super::geometry::{decimal, mm};
use super::layout::Rect;
use super::svg::{escape, layer_attributes};
use super::typeset::{capitals, Kind, Typesetter};
use crate::sheet::{Align, Position, Sheet, TextBox, TextStyle};
use crate::theme::Palette;
use crate::{Error, Result};

/// Millimetres in a typographic point (1/72 inch).
const MM_PER_PT: f64 = 25.4 / 72.0;
/// Where a text box with no position goes, from the drawing's top-left, at A3.
const AUTO_BOX_INSET: f64 = 6.0;

/// Every user-styled text on a sheet, resolved up front so a bad colour or
/// size fails the render with a reason that names the text.
pub struct SheetTexts<'a> {
    pub title: StyledText,
    pub subtitle: StyledText,
    pub boxes: Vec<(&'a TextBox, StyledText)>,
}

impl SheetTexts<'_> {
    pub fn resolve<'a>(
        sheet: &'a Sheet,
        typesetter: &Typesetter,
        palette: &Palette,
    ) -> Result<SheetTexts<'a>> {
        let mut boxes = Vec::with_capacity(sheet.texts.len());
        for text_box in &sheet.texts {
            let styled = StyledText::new(typesetter, Kind::Note, &text_box.style, palette)
                .map_err(|error| Error::Invalid(format!("text box {}: {error}", text_box.id)))?;
            boxes.push((text_box, styled));
        }

        Ok(SheetTexts {
            title: StyledText::new(typesetter, Kind::Title, &sheet.title_style, palette)
                .map_err(|error| Error::Invalid(format!("title: {error}")))?,
            subtitle: StyledText::new(typesetter, Kind::Subtitle, &sheet.subtitle_style, palette)
                .map_err(|error| Error::Invalid(format!("subtitle: {error}")))?,
            boxes,
        })
    }
}

/// A text's look with everything resolved: the kind's defaults, then the user's style.
pub struct StyledText {
    pub face: Face,
    pub weight: u16,
    pub is_italic: bool,
    /// Font size in millimetres.
    pub size: f64,
    tracking_em: f64,
    is_uppercase: bool,
    colour: String,
    opacity: f64,
    line_height: f64,
    pub align: Align,
}

impl StyledText {
    pub fn new(
        typesetter: &Typesetter,
        kind: Kind,
        style: &TextStyle,
        palette: &Palette,
    ) -> Result<StyledText> {
        style.check()?;
        let spec = typesetter.spec(kind);
        let face = match &style.family {
            Some(family) => Face::for_family(family),
            None => typesetter.face,
        };
        // The theme's title weight belongs to the theme's family; another family gets its own.
        let default_weight = if kind == Kind::Title {
            face.title_weight
        } else {
            spec.weight
        };
        let weight = match style.bold {
            Some(true) => BOLD_WEIGHT,
            Some(false) => REGULAR_WEIGHT,
            None => default_weight,
        };
        let colour = match &style.colour {
            Some(value) => color::resolve(palette, value)?,
            None => palette.ink.clone(),
        };

        Ok(StyledText {
            face,
            weight,
            is_italic: style.italic.unwrap_or(spec.is_italic),
            size: style.size_pt.map_or(spec.size, |points| points * MM_PER_PT),
            tracking_em: spec.tracking_em,
            is_uppercase: spec.is_uppercase,
            colour,
            opacity: style.opacity.unwrap_or(spec.opacity),
            line_height: spec.line_height,
            align: style.align.unwrap_or(Align::Left),
        })
    }

    pub fn cap_height(&self) -> f64 {
        self.size * super::typeset::CAP_HEIGHT_EM
    }

    /// Baseline to baseline.
    pub fn line_pitch(&self) -> f64 {
        self.size * self.line_height
    }

    /// Word-wrapped to `max_width`, in the case it will be printed in.
    pub fn wrap(&self, text: &str, max_width: f64) -> Vec<String> {
        wrap(
            &self.cased(text),
            &self.face,
            self.weight,
            self.is_italic,
            self.size,
            self.tracking_em,
            max_width,
        )
    }

    /// One line per line break the user typed, in the case it will be printed in.
    pub fn lines(&self, text: &str) -> Vec<String> {
        self.cased(text).lines().map(str::to_string).collect()
    }

    /// The anchor x for a block that should sit within `area_x` .. `area_x + width`.
    pub fn anchor_x(&self, area_x: f64, width: f64) -> f64 {
        match self.align {
            Align::Left => area_x,
            Align::Centre => area_x + width / 2.0,
            Align::Right => area_x + width,
        }
    }

    fn cased(&self, text: &str) -> String {
        if self.is_uppercase {
            capitals(text)
        } else {
            text.to_string()
        }
    }
}

/// Opens the group for one text, anchored at (`x`, `top`). Lines inside inherit its style.
pub fn open_group(out: &mut String, styled: &StyledText, id: &str, x: f64, top: f64) {
    let anchor = match styled.align {
        Align::Left => "start",
        Align::Centre => "middle",
        Align::Right => "end",
    };
    let align = match styled.align {
        Align::Left => "left",
        Align::Centre => "centre",
        Align::Right => "right",
    };
    let _ = write!(
        out,
        r#"<g data-text="{id}" data-x="{x}" data-y="{y}" data-align="{align}" data-family="{family}" data-size-pt="{size_pt}" data-bold="{bold}" data-italic="{italic}" data-colour="{colour}" data-opacity="{opacity}" font-family="{stack}" font-size="{size}" font-weight="{weight}" fill="{colour}" text-anchor="{anchor}""#,
        id = escape(id),
        x = mm(x),
        y = mm(top),
        family = styled.face.family,
        size_pt = decimal(styled.size / MM_PER_PT),
        bold = styled.weight >= BOLD_WEIGHT,
        italic = styled.is_italic,
        colour = styled.colour,
        opacity = decimal(styled.opacity),
        stack = styled.face.css_stack().replace('\'', "&apos;"),
        size = mm(styled.size),
        weight = styled.weight,
    );
    if styled.is_italic {
        out.push_str(r#" font-style="italic""#);
    }
    if styled.tracking_em != 0.0 {
        let _ = write!(
            out,
            r#" letter-spacing="{}""#,
            mm(styled.tracking_em * styled.size)
        );
    }
    if styled.opacity < 1.0 {
        let _ = write!(out, r#" fill-opacity="{}""#, decimal(styled.opacity));
    }
    out.push('>');
}

pub fn close_group(out: &mut String) {
    out.push_str("</g>");
}

/// Writes `lines` from `first_baseline` down. Returns the last baseline.
/// An empty line keeps its space, so blank lines the user typed stay blank.
pub fn write_lines(
    out: &mut String,
    styled: &StyledText,
    lines: &[String],
    x: f64,
    first_baseline: f64,
) -> f64 {
    let mut baseline = first_baseline;
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            baseline += styled.line_pitch();
        }
        if !line.is_empty() {
            let _ = write!(
                out,
                r#"<text x="{}" y="{}">{}</text>"#,
                mm(x),
                mm(baseline),
                escape(line)
            );
        }
    }

    baseline
}

/// The user's text boxes, each in its own group, in the order they were added.
pub fn write_boxes(out: &mut String, boxes: &[(&TextBox, StyledText)], drawing: &Rect, unit: f64) {
    for (text_box, styled) in boxes {
        let lines = styled.lines(&text_box.text);
        if lines.iter().all(|line| line.trim().is_empty()) {
            continue;
        }
        let Position { x_mm, y_mm } = text_box.position.unwrap_or(Position {
            x_mm: drawing.x + AUTO_BOX_INSET * unit,
            y_mm: drawing.y + AUTO_BOX_INSET * unit,
        });
        open_group(out, styled, &text_box.id, x_mm, y_mm);
        write_lines(out, styled, &lines, x_mm, y_mm + styled.cap_height());
        close_group(out);
    }
}

/// Opens the top-level layer moved and added text lives in, named for layout tools.
pub fn open_layer(out: &mut String) {
    let _ = write!(out, "<g {}>", layer_attributes("text", "Text"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::builtin_themes;
    use crate::theme::Typography;

    #[test]
    fn an_uppercase_title_keeps_the_micro_sign() {
        let palette = builtin_themes()[0].palette.clone();
        let typography = Typography {
            family: "Inter".to_string(),
            uppercase_labels: true,
        };
        let typesetter = Typesetter::new(&typography, 1.0, &palette.ink, &palette.accent);
        let title = StyledText::new(&typesetter, Kind::Title, &TextStyle::default(), &palette)
            .expect("a plain title style");

        assert_eq!(title.lines("PM2.5 in µg/m³"), vec!["PM2.5 IN µG/M³"]);
    }
}
