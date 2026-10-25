//! Colour values as themes write them: "#RRGGBB" or "@role".

use crate::theme::Palette;
use crate::{Error, Result};

/// Every palette role a theme may point at, in one place.
pub const ROLES: [&str; 9] = [
    "paper", "ground", "figure", "accent", "water", "green", "ink", "muted", "unknown",
];

pub fn is_hex(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// "#1a2b3c" or a palette role the renderer knows, "@accent". Anything else
/// would save and then fail every render of the sheet.
pub fn is_colour(value: &str) -> bool {
    let is_role = value
        .strip_prefix('@')
        .is_some_and(|name| ROLES.contains(&name));
    is_role || is_hex(value)
}

/// "@muted" -> the palette's muted colour. `None` for a role that does not exist.
pub fn role(palette: &Palette, name: &str) -> Option<String> {
    let value = match name {
        "paper" => &palette.paper,
        "ground" => &palette.ground,
        "figure" => &palette.figure,
        "accent" => &palette.accent,
        "water" => &palette.water,
        "green" => &palette.green,
        "ink" => &palette.ink,
        "muted" => &palette.muted,
        "unknown" => &palette.unknown,
        _ => return None,
    };
    Some(value.clone())
}

/// "@role" or "#RRGGBB" to "#RRGGBB", or an error naming the bad value.
pub fn resolve(palette: &Palette, value: &str) -> Result<String> {
    let resolved = match value.strip_prefix('@') {
        Some(name) => role(palette, name)
            .ok_or_else(|| Error::Invalid(format!("unknown colour role {value:?}")))?,
        None => value.to_string(),
    };
    if !is_hex(&resolved) {
        return Err(Error::Invalid(format!(
            "colour must look like '#RRGGBB' or '@role', got {value:?}"
        )));
    }

    Ok(resolved)
}

/// "#336699" -> (0x33, 0x66, 0x99). Expects a value that passed `is_hex`.
pub fn rgb(hex: &str) -> (f64, f64, f64) {
    let channel = |start: usize| u8::from_str_radix(&hex[start..start + 2], 16).unwrap_or(0) as f64;
    (channel(1), channel(3), channel(5))
}

pub fn to_hex((red, green, blue): (f64, f64, f64)) -> String {
    let channel = |value: f64| value.round().clamp(0.0, 255.0) as u8;
    format!(
        "#{:02X}{:02X}{:02X}",
        channel(red),
        channel(green),
        channel(blue)
    )
}

/// `share` of the way from `from` to `to`, in RGB.
pub fn mix(from: &str, to: &str, share: f64) -> String {
    let (from_red, from_green, from_blue) = rgb(from);
    let (to_red, to_green, to_blue) = rgb(to);
    to_hex((
        from_red + (to_red - from_red) * share,
        from_green + (to_green - from_green) * share,
        from_blue + (to_blue - from_blue) * share,
    ))
}

/// Relative luminance from 0 (black) to 1 (white), close enough to tell dark sheets from light.
pub fn luminance(hex: &str) -> f64 {
    let (red, green, blue) = rgb(hex);
    (0.2126 * red + 0.7152 * green + 0.0722 * blue) / 255.0
}

/// `count` colours spread evenly along `ramp`. A ramp as long as `count` is used as is.
pub fn sample_ramp(ramp: &[String], count: usize) -> Vec<String> {
    if ramp.len() == count || ramp.len() < 2 {
        return (0..count)
            .map(|index| ramp[index.min(ramp.len() - 1)].clone())
            .collect();
    }

    let last_stop = (ramp.len() - 1) as f64;
    (0..count)
        .map(|index| {
            let position = if count == 1 {
                last_stop
            } else {
                index as f64 / (count - 1) as f64 * last_stop
            };
            let lower = position.floor() as usize;
            let upper = (lower + 1).min(ramp.len() - 1);
            mix(&ramp[lower], &ramp[upper], position - lower as f64)
        })
        .collect()
}
