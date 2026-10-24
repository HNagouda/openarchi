//! How the sun lights the walls of a standing building in a tilted view: a
//! wall facing the sun turns lighter than its roof, a wall turned away darker.

use super::color;
use crate::sun::SunPosition;
use crate::theme::Theme;

/// The most a wall moves towards white in full sun, or towards shade facing away.
const SUNLIT_GAIN: f64 = 0.45;
const SHADE_GAIN: f64 = 0.4;
/// Shade every wall gets anyway, so walls never vanish into their roof.
const AMBIENT_SHADE: f64 = 0.08;
/// Roofs count as this much brighter than the sun alone makes them, so most
/// walls read darker than their roof, the way a massing model is read.
const ROOF_BIAS: f64 = 0.3;
/// Below this the sun counts as down and every wall is in shade.
const SUN_UP_DEG: f64 = 0.5;
/// Sheets darker than this sink shade into the paper rather than into black.
const DARK_SHEET_LUMINANCE: f64 = 0.5;

/// The light a sheet's buildings are drawn in, resolved once per sheet.
pub struct Light {
    /// (east, north, up) unit vector towards the sun, or `None` at night.
    pub sun: Option<(f64, f64, f64)>,
    pub shade: String,
    /// How strongly walls differ from their roof: 1 for the site, less for
    /// the muted neighbours around it.
    pub strength: f64,
}

impl Light {
    /// The same light with walls `strength` as far from their roof.
    pub fn softened(&self, strength: f64) -> Light {
        Light {
            sun: self.sun,
            shade: self.shade.clone(),
            strength,
        }
    }
}

pub fn light(sun: &SunPosition, theme: &Theme) -> Light {
    let is_dark_sheet = color::luminance(&theme.palette.paper) < DARK_SHEET_LUMINANCE;
    let shade = if is_dark_sheet {
        theme.palette.paper.clone()
    } else {
        "#000000".to_string()
    };
    let sun = (sun.altitude_deg > SUN_UP_DEG).then(|| {
        let (azimuth, altitude) = (sun.azimuth_deg.to_radians(), sun.altitude_deg.to_radians());
        (
            azimuth.sin() * altitude.cos(),
            azimuth.cos() * altitude.cos(),
            altitude.sin(),
        )
    });

    Light {
        sun,
        shade,
        strength: 1.0,
    }
}

/// Lighter than the roof where the wall catches more sun than the roof does, darker where less.
pub fn wall_fill(base: &str, normal: (f64, f64), light: &Light) -> String {
    let strength = light.strength;
    let Some((east, north, up)) = light.sun else {
        return color::mix(base, &light.shade, (AMBIENT_SHADE + SHADE_GAIN) * strength);
    };
    let wall_light = (normal.0 * east + normal.1 * north).max(0.0);
    let difference = wall_light - up - ROOF_BIAS;
    if difference > 0.0 {
        color::mix(base, "#FFFFFF", difference * SUNLIT_GAIN * strength)
    } else {
        let share = AMBIENT_SHADE - difference * SHADE_GAIN;
        color::mix(base, &light.shade, share * strength)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wall_facing_the_sun_is_lighter_than_one_facing_away() {
        let light = Light {
            sun: Some((0.0, -0.7, 0.7)),
            shade: "#000000".to_string(),
            strength: 1.0,
        };
        let south = wall_fill("#808080", (0.0, -1.0), &light);
        let north = wall_fill("#808080", (0.0, 1.0), &light);
        assert!(color::luminance(&south) > color::luminance(&north));
    }
}
