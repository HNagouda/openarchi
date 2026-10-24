//! How light, how colourful and which hue a swatch is, and how well two colours read on each other.
//!
//! Lightness, chroma and hue are OKLCH, from Björn Ottosson's OKLab (2020,
//! https://bottosson.github.io/posts/oklab/). Unlike HSL, its lightness
//! matches what the eye sees, so "the lightest swatch" means the one a person
//! would pick. Contrast is the WCAG 2.1 contrast ratio
//! (https://www.w3.org/TR/WCAG21/#dfn-contrast-ratio), the measure behind the
//! 4.5:1 rule for text and the 3:1 rule for graphics.

use crate::render::rgb;

/// One colour in OKLCH: lightness 0 to 1, chroma from 0 (grey) up to about 0.37, hue in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lch {
    pub lightness: f64,
    pub chroma: f64,
    pub hue_deg: f64,
}

/// "#F79F79" -> its OKLCH. Expects a value that passed `is_hex`.
pub fn lch(hex: &str) -> Lch {
    let (red, green, blue) = rgb(hex);
    let (red, green, blue) = (linear(red), linear(green), linear(blue));

    let l = (0.4122214708 * red + 0.5363325363 * green + 0.0514459929 * blue).cbrt();
    let m = (0.2119034982 * red + 0.6806995451 * green + 0.1073969566 * blue).cbrt();
    let s = (0.0883024619 * red + 0.2817188376 * green + 0.6299787005 * blue).cbrt();

    let lightness = 0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s;
    let a = 1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s;
    let b = 0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s;

    Lch {
        lightness,
        chroma: a.hypot(b),
        hue_deg: b.atan2(a).to_degrees().rem_euclid(360.0),
    }
}

/// WCAG 2.1 relative luminance, 0 for black to 1 for white.
pub fn relative_luminance(hex: &str) -> f64 {
    let (red, green, blue) = rgb(hex);
    0.2126 * linear(red) + 0.7152 * linear(green) + 0.0722 * linear(blue)
}

/// WCAG 2.1 contrast ratio, from 1 (the same colour) to 21 (black on white).
pub fn contrast_ratio(first: &str, second: &str) -> f64 {
    let a = relative_luminance(first);
    let b = relative_luminance(second);
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// The shorter way round the colour wheel between two hues, 0 to 180 degrees.
pub fn hue_distance(first_deg: f64, second_deg: f64) -> f64 {
    let difference = (first_deg - second_deg).rem_euclid(360.0);
    difference.min(360.0 - difference)
}

/// An sRGB channel, 0 to 255, to linear light, 0 to 1.
fn linear(channel: f64) -> f64 {
    let value = channel / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_on_white_is_twenty_one_to_one() {
        assert!((contrast_ratio("#000000", "#FFFFFF") - 21.0).abs() < 1e-9);
        assert!((contrast_ratio("#777777", "#777777") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn grey_has_no_chroma_and_white_is_fully_light() {
        let white = lch("#FFFFFF");
        assert!((white.lightness - 1.0).abs() < 1e-3);
        assert!(lch("#808080").chroma < 1e-3);
    }

    #[test]
    fn oklch_hues_land_where_the_reference_puts_them() {
        // Ottosson's reference: sRGB red sits near 29°, green near 142°, blue near 264°.
        assert!(hue_distance(lch("#FF0000").hue_deg, 29.0) < 2.0);
        assert!(hue_distance(lch("#00FF00").hue_deg, 142.0) < 2.0);
        assert!(hue_distance(lch("#0000FF").hue_deg, 264.0) < 2.0);
    }

    #[test]
    fn hue_distance_goes_the_short_way_round() {
        assert_eq!(hue_distance(350.0, 10.0), 20.0);
        assert_eq!(hue_distance(0.0, 180.0), 180.0);
    }
}
