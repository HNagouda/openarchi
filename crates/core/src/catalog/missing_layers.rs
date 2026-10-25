//! Styles for the layers a saved theme is older than.
//!
//! A theme without a style for a layer does not draw it, so a palette saved
//! before a layer existed would draw that layer's diagrams empty. On load, each
//! missing layer gets the style the template theme gives it: through the
//! palette rules for a palette made from swatches, as it is for the built-in
//! Paper otherwise. Paper styles its newer layers with palette roles, so they
//! follow the theme's own colours either way. Nothing is written back to disk.

use crate::palette::{layer_style, template_theme};
use crate::theme::Theme;
use crate::Result;

/// The theme with a style for every layer the template styles.
pub fn filled(mut theme: Theme) -> Result<Theme> {
    for (layer, style) in template_theme()?.layers {
        if theme.layers.contains_key(&layer) {
            continue;
        }
        let style = match theme.source {
            Some(_) => layer_style(layer, &style, &theme.palette)?,
            None => style,
        };
        theme.layers.insert(layer, style);
    }

    Ok(theme)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::builtin_themes;
    use crate::palette::PaletteSource;
    use crate::scene::Layer;

    /// The Ink theme saved before the block, sun-hours, sky view, solar radiation and viewshed layers existed.
    fn old_ink() -> Theme {
        let mut theme = builtin_themes()
            .into_iter()
            .find(|theme| theme.id == "ink")
            .unwrap();
        theme.id = "my-ink".to_string();
        theme.layers.remove(&Layer::Block);
        theme.layers.remove(&Layer::SunHours);
        theme.layers.remove(&Layer::SkyView);
        theme.layers.remove(&Layer::SolarRadiation);
        theme.layers.remove(&Layer::Viewshed);
        theme
    }

    #[test]
    fn a_theme_missing_a_layer_gets_the_paper_style_in_its_own_roles() {
        let filled = filled(old_ink()).unwrap();
        let paper = builtin_themes().into_iter().next().unwrap();

        assert_eq!(filled.layers[&Layer::Block], paper.layers[&Layer::Block]);
        assert_eq!(
            filled.layers[&Layer::SunHours].fill.as_deref(),
            Some("@muted")
        );
        assert!(filled.layers.contains_key(&Layer::SkyView));
        assert!(filled.layers.contains_key(&Layer::SolarRadiation));
        assert_eq!(
            filled.layers[&Layer::Viewshed].fill.as_deref(),
            Some("@accent")
        );
    }

    #[test]
    fn a_palette_made_from_swatches_gets_its_own_rule_for_the_layer() {
        let mut theme = old_ink();
        theme.layers.remove(&Layer::Road);
        theme.source = Some(PaletteSource {
            swatches: Vec::new(),
            roles: Default::default(),
        });
        let filled = filled(theme.clone()).unwrap();

        // Paper paints roads in a raw colour; a swatch palette mixes its own.
        let road = filled.layers[&Layer::Road].fill.clone().unwrap();
        let expected = layer_style(
            Layer::Road,
            &builtin_themes()[0].layers[&Layer::Road],
            &theme.palette,
        )
        .unwrap();
        assert_eq!(Some(road), expected.fill);
        assert!(filled.layers.contains_key(&Layer::Block));
    }

    #[test]
    fn a_theme_saved_before_the_green_and_heat_layers_gets_their_styles() {
        let mut theme = old_ink();
        for layer in [
            Layer::TreeCanopy,
            Layer::GreenCover,
            Layer::VegetationIndex,
            Layer::HeatIsland,
        ] {
            theme.layers.remove(&layer);
        }
        let filled = filled(theme).unwrap();

        assert_eq!(
            filled.layers[&Layer::HeatIsland].fill.as_deref(),
            Some("@muted")
        );
        assert_eq!(
            filled.layers[&Layer::GreenCover].stroke.as_deref(),
            Some("@ink")
        );
    }

    #[test]
    fn a_theme_saved_before_the_wind_layers_gets_their_styles() {
        let mut theme = old_ink();
        theme.layers.remove(&Layer::Wind);
        theme.layers.remove(&Layer::WindLine);
        let filled = filled(theme).unwrap();

        assert_eq!(filled.layers[&Layer::Wind].fill.as_deref(), Some("@muted"));
        assert_eq!(
            filled.layers[&Layer::WindLine].stroke.as_deref(),
            Some("@paper")
        );
    }

    #[test]
    fn styles_the_theme_has_are_kept() {
        let mut theme = old_ink();
        let mut road = theme.layers[&Layer::Road].clone();
        road.stroke_mm = 1.5;
        theme.layers.insert(Layer::Road, road.clone());

        assert_eq!(filled(theme).unwrap().layers[&Layer::Road], road);
    }
}
