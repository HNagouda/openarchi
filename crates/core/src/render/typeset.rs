//! The sheet's type scale, and `<text>` elements set in it.
//!
//! Sizes are in millimetres for A3 and scale with the paper. Small labels are
//! always spaced capitals; titles and legend labels follow the theme's choice.

use std::fmt::Write;

use super::fonts::{text_width, wrap, Face};
use super::geometry::{decimal, mm};
use super::svg::escape;
use crate::theme::Typography;

/// Cap height as a share of the font size, close for all three bundled families.
const CAP_HEIGHT_EM: f64 = 0.72;
const ELLIPSIS: char = '\u{2026}';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The diagram name over the title, in the accent colour.
    Eyebrow,
    Title,
    Subtitle,
    /// Legend title and other small headings.
    Heading,
    /// Legend rows.
    Label,
    /// Scale bar numbers and the scale note.
    Small,
    /// Data credits, the smallest print on the sheet.
    Credit,
    /// The quiet openarchi mark.
    Mark,
}

struct Spec {
    size: f64,
    weight: u16,
    tracking_em: f64,
    is_uppercase: bool,
    is_italic: bool,
    is_accent: bool,
    opacity: f64,
    line_height: f64,
}

pub struct Typesetter {
    pub face: Face,
    pub unit: f64,
    is_uppercase_theme: bool,
    pub ink: String,
    accent: String,
}

impl Typesetter {
    pub fn new(typography: &Typography, unit: f64, ink: &str, accent: &str) -> Typesetter {
        Typesetter {
            face: Face::for_family(&typography.family),
            unit,
            is_uppercase_theme: typography.uppercase_labels,
            ink: ink.to_string(),
            accent: accent.to_string(),
        }
    }

    fn spec(&self, kind: Kind) -> Spec {
        let caps = self.is_uppercase_theme;
        let base = Spec {
            size: 2.7,
            weight: 400,
            tracking_em: 0.0,
            is_uppercase: false,
            is_italic: false,
            is_accent: false,
            opacity: 1.0,
            line_height: 1.35,
        };
        let spec = match kind {
            Kind::Eyebrow => Spec {
                size: 2.3,
                weight: 500,
                tracking_em: 0.14,
                is_uppercase: true,
                is_accent: true,
                ..base
            },
            Kind::Title if caps => Spec {
                size: 6.2,
                weight: self.face.title_weight,
                tracking_em: 0.06,
                is_uppercase: true,
                line_height: 1.18,
                ..base
            },
            Kind::Title => Spec {
                size: 7.4,
                weight: self.face.title_weight,
                tracking_em: -0.01,
                line_height: 1.1,
                ..base
            },
            Kind::Subtitle => Spec {
                size: 3.3,
                is_italic: self.face.italic_subtitle,
                opacity: 0.68,
                ..base
            },
            Kind::Heading => Spec {
                size: 2.2,
                weight: 500,
                tracking_em: 0.14,
                is_uppercase: true,
                opacity: 0.6,
                ..base
            },
            Kind::Label if caps => Spec {
                size: 2.25,
                tracking_em: 0.08,
                is_uppercase: true,
                ..base
            },
            Kind::Label => Spec { size: 2.7, ..base },
            Kind::Small => Spec {
                size: 2.0,
                tracking_em: 0.02,
                opacity: 0.75,
                ..base
            },
            Kind::Credit => Spec {
                size: 1.7,
                opacity: 0.55,
                line_height: 1.4,
                ..base
            },
            Kind::Mark => Spec {
                size: 2.2,
                weight: 500,
                tracking_em: 0.04,
                opacity: 0.45,
                ..base
            },
        };

        Spec {
            size: spec.size * self.unit,
            ..spec
        }
    }

    pub fn cap_height(&self, kind: Kind) -> f64 {
        self.spec(kind).size * CAP_HEIGHT_EM
    }

    /// Baseline to baseline.
    pub fn line_pitch(&self, kind: Kind) -> f64 {
        let spec = self.spec(kind);
        spec.size * spec.line_height
    }

    pub fn width(&self, text: &str, kind: Kind) -> f64 {
        let spec = self.spec(kind);
        text_width(
            &self.cased(text, &spec),
            &self.face,
            spec.weight,
            spec.size,
            spec.tracking_em,
        )
    }

    pub fn wrap(&self, text: &str, kind: Kind, max_width: f64) -> Vec<String> {
        let spec = self.spec(kind);
        wrap(
            &self.cased(text, &spec),
            &self.face,
            spec.weight,
            spec.size,
            spec.tracking_em,
            max_width,
        )
    }

    /// `text`, shortened with an ellipsis if it is wider than `max_width`.
    pub fn fit(&self, text: &str, kind: Kind, max_width: f64) -> String {
        if self.width(text, kind) <= max_width {
            return text.to_string();
        }
        let mut characters: Vec<char> = text.chars().collect();
        while !characters.is_empty() {
            characters.pop();
            let shortened: String = characters.iter().collect::<String>().trim_end().to_string()
                + &ELLIPSIS.to_string();
            if self.width(&shortened, kind) <= max_width {
                return shortened;
            }
        }

        String::new()
    }

    /// A `<text>` with its baseline at `y`. `anchor` is "middle" or "end"; `None` starts at `x`.
    pub fn write(
        &self,
        out: &mut String,
        x: f64,
        y: f64,
        text: &str,
        kind: Kind,
        anchor: Option<&str>,
    ) {
        if text.is_empty() {
            return;
        }
        let spec = self.spec(kind);
        let colour = if spec.is_accent {
            &self.accent
        } else {
            &self.ink
        };
        let _ = write!(
            out,
            r#"<text x="{}" y="{}" font-size="{}" font-weight="{}" fill="{colour}""#,
            mm(x),
            mm(y),
            mm(spec.size),
            spec.weight
        );
        if spec.is_italic {
            out.push_str(r#" font-style="italic""#);
        }
        if spec.tracking_em != 0.0 {
            let _ = write!(
                out,
                r#" letter-spacing="{}""#,
                mm(spec.tracking_em * spec.size)
            );
        }
        if spec.opacity < 1.0 {
            let _ = write!(out, r#" fill-opacity="{}""#, decimal(spec.opacity));
        }
        if let Some(anchor) = anchor {
            let _ = write!(out, r#" text-anchor="{anchor}""#);
        }
        let _ = write!(out, ">{}</text>", escape(&self.cased(text, &spec)));
    }

    fn cased(&self, text: &str, spec: &Spec) -> String {
        if spec.is_uppercase {
            text.to_uppercase()
        } else {
            text.to_string()
        }
    }
}
