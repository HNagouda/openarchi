//! The fonts bundled with the app, and how wide a line of text will be.
//!
//! Every sheet is set in one of three OFL families shipped in assets/fonts.
//! PNG and PDF exports load exactly these files, so measuring with them here
//! means wrapped titles and legends never run into the drawing.

pub const INTER: &str = "Inter";
pub const SERIF: &str = "Source Serif 4";
pub const MONO: &str = "IBM Plex Mono";

/// Every font file the app ships, in one place: family, weight, italic, bytes.
pub const BUNDLED: [(&str, u16, bool, &[u8]); 8] = [
    (
        INTER,
        400,
        false,
        include_bytes!("../../assets/fonts/Inter-Regular.ttf"),
    ),
    (
        INTER,
        500,
        false,
        include_bytes!("../../assets/fonts/Inter-Medium.ttf"),
    ),
    (
        INTER,
        600,
        false,
        include_bytes!("../../assets/fonts/Inter-SemiBold.ttf"),
    ),
    (
        SERIF,
        400,
        false,
        include_bytes!("../../assets/fonts/SourceSerif4-Regular.ttf"),
    ),
    (
        SERIF,
        400,
        true,
        include_bytes!("../../assets/fonts/SourceSerif4-It.ttf"),
    ),
    (
        SERIF,
        600,
        false,
        include_bytes!("../../assets/fonts/SourceSerif4-Semibold.ttf"),
    ),
    (
        MONO,
        400,
        false,
        include_bytes!("../../assets/fonts/IBMPlexMono-Regular.ttf"),
    ),
    (
        MONO,
        500,
        false,
        include_bytes!("../../assets/fonts/IBMPlexMono-Medium.ttf"),
    ),
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
pub fn text_width(text: &str, face: &Face, weight: u16, size_mm: f64, tracking_em: f64) -> f64 {
    let characters = text.chars().count() as f64;
    let tracking = characters * tracking_em * size_mm;

    let Some(parsed) = parsed_face(face.family, weight) else {
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
        let fits = text_width(&candidate, face, weight, size_mm, tracking_em) <= max_width_mm;
        if fits || current.is_empty() {
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

/// The upright file closest to `weight` in `family`.
fn parsed_face(family: &str, weight: u16) -> Option<ttf_parser::Face<'static>> {
    let (_, _, _, bytes) = BUNDLED
        .iter()
        .filter(|(name, _, italic, _)| *name == family && !italic)
        .min_by_key(|(_, file_weight, _, _)| file_weight.abs_diff(weight))?;
    ttf_parser::Face::parse(bytes, 0).ok()
}
