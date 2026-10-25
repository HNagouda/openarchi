//! How different two colours look: the CIEDE2000 colour difference, ΔE00
//! (CIE 142-2001, also ISO/CIE 11664-6), between colours turned from sRGB
//! (IEC 61966-2-1) into CIELAB under the D65 white.
//!
//! CIEDE2000 rather than the plain distance in CIELAB (CIE 1976), because the
//! plain distance weighs lightness too heavily for pale colours: it sent the
//! light end of a palette ramp to olive rather than to the nearest light
//! orange. The formula and its test pairs follow Sharma, Wu and Dalal, "The
//! CIEDE2000 color-difference formula: implementation notes, supplementary
//! test data, and mathematical observations", Color Research & Application
//! 30(1), 2005.

/// The D65 white in CIE XYZ, Y scaled to 1 (CIE 15:2004).
const WHITE_X: f64 = 0.950_47;
const WHITE_Y: f64 = 1.0;
const WHITE_Z: f64 = 1.088_83;
/// CIELAB's switch from the cube root to a straight line near black, (6/29)³, and its slope.
const LAB_EPSILON: f64 = 216.0 / 24_389.0;
const LAB_KAPPA: f64 = 24_389.0 / 27.0;
/// sRGB's switch from its straight segment to its power curve, and that segment's slope.
const SRGB_KNEE: f64 = 0.040_45;
const SRGB_SLOPE: f64 = 12.92;
/// 25⁷, where CIEDE2000's chroma corrections level off.
const CHROMA_SCALE: f64 = 6_103_515_625.0;

/// A colour in CIELAB: lightness L* and the two opponent axes a* and b*.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lab {
    pub lightness: f64,
    pub a: f64,
    pub b: f64,
}

/// sRGB channels of 0 to 255 -> CIELAB.
pub fn lab((red, green, blue): (f64, f64, f64)) -> Lab {
    let (red, green, blue) = (linear(red), linear(green), linear(blue));
    // Linear sRGB to CIE XYZ under D65: the matrix of IEC 61966-2-1 to seven
    // places, whose rows add up to the D65 white above, so white has no tint.
    let x = 0.412_456_4 * red + 0.357_576_1 * green + 0.180_437_5 * blue;
    let y = 0.212_672_9 * red + 0.715_152_2 * green + 0.072_175_0 * blue;
    let z = 0.019_333_9 * red + 0.119_192_0 * green + 0.950_304_1 * blue;
    let fx = lab_curve(x / WHITE_X);
    let fy = lab_curve(y / WHITE_Y);
    let fz = lab_curve(z / WHITE_Z);

    Lab {
        lightness: 116.0 * fy - 16.0,
        a: 500.0 * (fx - fy),
        b: 200.0 * (fy - fz),
    }
}

/// ΔE00 between two colours, with the weights kL, kC and kH all 1, as for surface colours.
pub fn ciede2000(first: Lab, second: Lab) -> f64 {
    let mean_chroma = (first.a.hypot(first.b) + second.a.hypot(second.b)) / 2.0;
    let g = 0.5 * (1.0 - chroma_share(mean_chroma));
    let first_a = (1.0 + g) * first.a;
    let second_a = (1.0 + g) * second.a;
    let first_chroma = first_a.hypot(first.b);
    let second_chroma = second_a.hypot(second.b);
    let first_hue = hue_deg(first.b, first_a);
    let second_hue = hue_deg(second.b, second_a);
    let is_grey = first_chroma * second_chroma == 0.0;

    let hue_step = if is_grey {
        0.0
    } else {
        wrapped_deg(second_hue - first_hue)
    };
    let lightness_difference = second.lightness - first.lightness;
    let chroma_difference = second_chroma - first_chroma;
    let hue_difference =
        2.0 * (first_chroma * second_chroma).sqrt() * (hue_step / 2.0).to_radians().sin();

    let mean_lightness = (first.lightness + second.lightness) / 2.0;
    let mean_chroma = (first_chroma + second_chroma) / 2.0;
    let mean_hue = mean_hue_deg(first_hue, second_hue, is_grey);
    let t = 1.0 - 0.17 * (mean_hue - 30.0).to_radians().cos()
        + 0.24 * (2.0 * mean_hue).to_radians().cos()
        + 0.32 * (3.0 * mean_hue + 6.0).to_radians().cos()
        - 0.20 * (4.0 * mean_hue - 63.0).to_radians().cos();
    let rotation_deg = 30.0 * (-((mean_hue - 275.0) / 25.0).powi(2)).exp();
    let rotation = -(2.0 * rotation_deg).to_radians().sin() * 2.0 * chroma_share(mean_chroma);
    let lightness_shift = (mean_lightness - 50.0).powi(2);
    let lightness_weight = 1.0 + 0.015 * lightness_shift / (20.0 + lightness_shift).sqrt();
    let chroma_weight = 1.0 + 0.045 * mean_chroma;
    let hue_weight = 1.0 + 0.015 * mean_chroma * t;

    let lightness_term = lightness_difference / lightness_weight;
    let chroma_term = chroma_difference / chroma_weight;
    let hue_term = hue_difference / hue_weight;
    (lightness_term.powi(2)
        + chroma_term.powi(2)
        + hue_term.powi(2)
        + rotation * chroma_term * hue_term)
        .sqrt()
}

/// C⁷ / (C⁷ + 25⁷): near 0 for greys, near 1 for strong colours.
fn chroma_share(chroma: f64) -> f64 {
    let power = chroma.powi(7);
    (power / (power + CHROMA_SCALE)).sqrt()
}

/// The hue angle in degrees, 0 to 360; 0 for a grey.
fn hue_deg(b: f64, a: f64) -> f64 {
    if a == 0.0 && b == 0.0 {
        return 0.0;
    }
    b.atan2(a).to_degrees().rem_euclid(360.0)
}

/// A difference of hue angles brought into -180 to 180.
fn wrapped_deg(step: f64) -> f64 {
    if step > 180.0 {
        step - 360.0
    } else if step < -180.0 {
        step + 360.0
    } else {
        step
    }
}

/// The mean of two hue angles the short way round the circle.
fn mean_hue_deg(first: f64, second: f64, is_grey: bool) -> f64 {
    if is_grey {
        first + second
    } else if (first - second).abs() <= 180.0 {
        (first + second) / 2.0
    } else if first + second < 360.0 {
        (first + second + 360.0) / 2.0
    } else {
        (first + second - 360.0) / 2.0
    }
}

/// An sRGB channel of 0 to 255 -> linear light, 0 to 1.
fn linear(channel: f64) -> f64 {
    let value = channel / 255.0;
    if value <= SRGB_KNEE {
        value / SRGB_SLOPE
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn lab_curve(ratio: f64) -> f64 {
    if ratio > LAB_EPSILON {
        ratio.cbrt()
    } else {
        (LAB_KAPPA * ratio + 16.0) / 116.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(lightness: f64, a: f64, b: f64) -> Lab {
        Lab { lightness, a, b }
    }

    #[test]
    fn sharmas_test_pairs_come_out_to_four_decimals() {
        // Pairs 1, 7, 17 and 25 of Sharma, Wu and Dalal's table 1.
        let pairs = [
            (at(50.0, 2.6772, -79.7751), at(50.0, 0.0, -82.7485), 2.0425),
            (at(50.0, 0.0, 0.0), at(50.0, -1.0, 2.0), 2.3669),
            (at(50.0, 2.5, 0.0), at(73.0, 25.0, -18.0), 27.1492),
            (
                at(60.2574, -34.0099, 36.2677),
                at(60.4626, -34.1751, 39.4387),
                1.2644,
            ),
        ];
        for (first, second, expected) in pairs {
            let difference = ciede2000(first, second);
            assert!(
                (difference - expected).abs() < 0.000_1,
                "{difference} vs {expected}"
            );
            assert!((ciede2000(second, first) - expected).abs() < 0.000_1);
        }
    }

    #[test]
    fn white_and_black_sit_at_the_ends_of_lightness() {
        let white = lab((255.0, 255.0, 255.0));
        let black = lab((0.0, 0.0, 0.0));
        assert!((white.lightness - 100.0).abs() < 0.01);
        assert!(white.a.abs() < 0.01 && white.b.abs() < 0.01);
        assert!(black.lightness.abs() < 0.01);
    }
}
