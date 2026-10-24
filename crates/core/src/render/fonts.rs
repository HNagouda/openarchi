//! The fonts bundled with the app, and how wide a line of text will be.
//!
//! Every sheet is set in one of three OFL families shipped in assets/fonts.
//! PNG and PDF exports load exactly these files, so measuring with them here
//! means wrapped titles and legends never run into the drawing.

pub const INTER: &str = "Inter";
pub const SERIF: &str = "Source Serif 4";
pub const MONO: &str = "IBM Plex Mono";
/// The families a theme or a piece of sheet text may name.
pub const FAMILIES: [&str; 3] = [INTER, SERIF, MONO];

/// The weight "bold" means on sheet text: the heaviest weight every family ships.
pub const BOLD_WEIGHT: u16 = 600;
pub const REGULAR_WEIGHT: u16 = 400;

/// Every font file the app ships, in one place: family, weight, italic, bytes.
/// Each family has 400 and 600 both upright and italic, so the bold and italic
/// choices on sheet text always find a real face rather than a faked slant.
#[rustfmt::skip]
pub const BUNDLED: [(&str, u16, bool, &[u8]); 16] = [
    (INTER, 400, false, include_bytes!("../../assets/fonts/Inter-Regular.ttf")),
    (INTER, 400, true, include_bytes!("../../assets/fonts/Inter-Italic.ttf")),
    (INTER, 500, false, include_bytes!("../../assets/fonts/Inter-Medium.ttf")),
    (INTER, 500, true, include_bytes!("../../assets/fonts/Inter-MediumItalic.ttf")),
    (INTER, 600, false, include_bytes!("../../assets/fonts/Inter-SemiBold.ttf")),
    (INTER, 600, true, include_bytes!("../../assets/fonts/Inter-SemiBoldItalic.ttf")),
    (SERIF, 400, false, include_bytes!("../../assets/fonts/SourceSerif4-Regular.ttf")),
    (SERIF, 400, true, include_bytes!("../../assets/fonts/SourceSerif4-It.ttf")),
    (SERIF, 600, false, include_bytes!("../../assets/fonts/SourceSerif4-Semibold.ttf")),
    (SERIF, 600, true, include_bytes!("../../assets/fonts/SourceSerif4-SemiboldIt.ttf")),
    (MONO, 400, false, include_bytes!("../../assets/fonts/IBMPlexMono-Regular.ttf")),
    (MONO, 400, true, include_bytes!("../../assets/fonts/IBMPlexMono-Italic.ttf")),
    (MONO, 500, false, include_bytes!("../../assets/fonts/IBMPlexMono-Medium.ttf")),
    (MONO, 500, true, include_bytes!("../../assets/fonts/IBMPlexMono-MediumItalic.ttf")),
    (MONO, 600, false, include_bytes!("../../assets/fonts/IBMPlexMono-SemiBold.ttf")),
    (MONO, 600, true, include_bytes!("../../assets/fonts/IBMPlexMono-SemiBoldItalic.ttf")),
];

/// Fallback when a width cannot be measured, as a share of the font size.
/// Close to Inter's average advance for mixed-case text.
const AVERAGE_ADVANCE_EM: f64 = 0.55;

/// A theme's family resolved to a bundled one, with the weights its sheet uses.
#[derive(Debug, Clone, Copy)]
pub struct Face {
    pub family: &'static str,
    pub title_weight: u16,
    /// The serif sets subtitles in italic; the sans and mono stay upright.
    pub italic_subtitle: bool,
}

impl Face {
    /// Unknown families fall back to Inter rather than failing the render.
    pub fn for_family(family: &str) -> Face {
        if family.eq_ignore_ascii_case(SERIF) || family.eq_ignore_ascii_case("Source Serif") {
            Face {
                family: SERIF,
                title_weight: 600,
                italic_subtitle: true,
            }
        } else if family.eq_ignore_ascii_case(MONO) {
            Face {
                family: MONO,
                title_weight: 500,
                italic_subtitle: false,
            }
        } else {
            Face {
                family: INTER,
                title_weight: 500,
                italic_subtitle: false,
            }
        }
    }

    /// The CSS font-family list written into the SVG. The fallbacks matter only
    /// when someone opens the SVG on a machine without the bundled fonts.
    pub fn css_stack(&self) -> &'static str {
        match self.family {
            SERIF => "'Source Serif 4', Georgia, serif",
            MONO => "'IBM Plex Mono', Menlo, monospace",
            _ => "Inter, 'Helvetica Neue', Arial, sans-serif",
        }
    }
}

pub fn is_bundled(family: &str) -> bool {
    BUNDLED
        .iter()
        .any(|(name, ..)| name.eq_ignore_ascii_case(family))
}

/// Width of `text` in millimetres when set at `size_mm` with `tracking_em`
/// extra space after every character. Kerning is ignored; it moves a line by
/// well under a percent.
pub fn text_width(
    text: &str,
    face: &Face,
    weight: u16,
    is_italic: bool,
    size_mm: f64,
    tracking_em: f64,
) -> f64 {
    let characters = text.chars().count() as f64;
    let tracking = characters * tracking_em * size_mm;

    let Some(parsed) = parsed_face(face.family, weight, is_italic) else {
        return characters * AVERAGE_ADVANCE_EM * size_mm + tracking;
    };
    let units_per_em = parsed.units_per_em() as f64;
    let mut advance_units = 0.0;
    for character in text.chars() {
        let advance = parsed
            .glyph_index(character)
            .and_then(|glyph| parsed.glyph_hor_advance(glyph))
            .map(f64::from)
            .unwrap_or(AVERAGE_ADVANCE_EM * units_per_em);
        advance_units += advance;
    }

    advance_units / units_per_em * size_mm + tracking
}

/// Greedy word wrap to `max_width_mm`. A single word wider than the line keeps its own line.
pub fn wrap(
    text: &str,
    face: &Face,
    weight: u16,
    is_italic: bool,
    size_mm: f64,
    tracking_em: f64,
    max_width_mm: f64,
) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        let width = text_width(&candidate, face, weight, is_italic, size_mm, tracking_em);
        if width <= max_width_mm || current.is_empty() {
            current = candidate;
        } else {
            lines.push(std::mem::take(&mut current));
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

/// The file closest to `weight` in `family` and style. On a tie the lighter
/// file wins, which is also the one the PDF and PNG font matching picks.
fn parsed_face(family: &str, weight: u16, is_italic: bool) -> Option<ttf_parser::Face<'static>> {
    let (_, _, _, bytes) = BUNDLED
        .iter()
        .filter(|(name, _, italic, _)| *name == family && *italic == is_italic)
        .min_by_key(|(_, file_weight, _, _)| file_weight.abs_diff(weight))?;
    ttf_parser::Face::parse(bytes, 0).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_family_ships_regular_and_bold_both_upright_and_italic() {
        for family in [INTER, SERIF, MONO] {
            for weight in [REGULAR_WEIGHT, BOLD_WEIGHT] {
                for is_italic in [false, true] {
                    let has_file = BUNDLED.iter().any(|(name, file_weight, italic, _)| {
                        *name == family && *file_weight == weight && *italic == is_italic
                    });
                    assert!(has_file, "{family} {weight} italic={is_italic} is missing");
                }
            }
        }
    }

    #[test]
    fn every_bundled_file_parses_with_the_style_it_is_listed_under() {
        for (family, weight, is_italic, bytes) in BUNDLED {
            let face = ttf_parser::Face::parse(bytes, 0).expect("font parses");
            assert_eq!(face.is_italic(), is_italic, "{family} {weight}");
            assert_eq!(face.weight().to_number(), weight, "{family} {weight}");
        }
    }

    #[test]
    fn italic_is_measured_with_the_italic_file() {
        let face = Face::for_family(SERIF);
        let upright = text_width("Westermarkt", &face, 400, false, 10.0, 0.0);
        let italic = text_width("Westermarkt", &face, 400, true, 10.0, 0.0);

        assert!((upright - italic).abs() > 0.01, "{upright} vs {italic}");
    }
}
