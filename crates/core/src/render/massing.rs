//! The colour buildings stand in when a tilted view extrudes them.
//!
//! Standing buildings in a dark figure colour turn into one heavy mass on a
//! light sheet, so unclassified ones become a light neutral instead and colour
//! stays for diagrams that classify.

use super::color;
use super::style::{ResolvedLayer, Role};
use crate::project::Look;
use crate::scene::Layer;
use crate::theme::Theme;

/// Sheets at least this light count as light; dark sheets already have light massing.
const LIGHT_SHEET_LUMINANCE: f64 = 0.5;
/// Building fills darker than this are lightened in a tilted view on a light sheet.
const DARK_MASSING_LUMINANCE: f64 = 0.4;
/// How far a dark building fill moves towards the paper colour.
const MASSING_LIGHTEN: f64 = 0.82;

/// Lightens the plain building layer for standing up. A fill the user set is kept.
pub fn lighten(layers: &mut [ResolvedLayer], look: &Look, theme: &Theme) {
    let paper = &theme.palette.paper;
    let has_user_fill = look
        .overrides
        .get(&Layer::Building)
        .is_some_and(|edit| edit.fill.is_some());
    if has_user_fill {
        return;
    }
    for resolved in layers.iter_mut() {
        let is_plain = matches!(resolved.role, Role::Theme | Role::Figure);
        if resolved.layer != Layer::Building || !is_plain {
            continue;
        }
        if let Some(fill) = &resolved.style.fill {
            resolved.style.fill = Some(lightened(fill, paper));
        }
    }
}

/// The theme's own building fill as it stands in a tilted view.
pub fn theme_fill(theme: &Theme) -> String {
    let palette = &theme.palette;
    let fill = theme
        .layers
        .get(&Layer::Building)
        .and_then(|style| style.fill.as_deref())
        .and_then(|fill| color::resolve(palette, fill).ok())
        .unwrap_or_else(|| palette.figure.clone());
    lightened(&fill, &palette.paper)
}

/// A dark building fill on a light sheet moved towards the paper; any other fill as it is.
fn lightened(fill: &str, paper: &str) -> String {
    let is_light_sheet = color::luminance(paper) >= LIGHT_SHEET_LUMINANCE;
    if is_light_sheet && color::luminance(fill) < DARK_MASSING_LUMINANCE {
        color::mix(fill, paper, MASSING_LIGHTEN)
    } else {
        fill.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dark_fill_on_a_light_sheet_turns_light() {
        let fill = lightened("#2D2A26", "#F4F1EA");
        assert!(color::luminance(&fill) > DARK_MASSING_LUMINANCE);
    }

    #[test]
    fn a_dark_sheet_keeps_its_fill() {
        assert_eq!(lightened("#1E2024", "#131417"), "#1E2024");
    }
}
