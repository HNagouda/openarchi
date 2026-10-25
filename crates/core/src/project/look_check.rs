//! Every rule a sheet's look must keep, in one place. The app checks a look
//! when it saves the sheets, the assistant when set_look changes one, and the
//! renderer before it draws, so all three refuse the same looks with the
//! same words.

use super::Look;
use crate::render::is_colour;
use crate::scene::Layer;
use crate::theme::LayerOverride;
use crate::view::{ViewMode, MAX_TILT_DEG, MIN_TILT_DEG};
use crate::{Error, Result};

impl Look {
    /// The look can be drawn as it is, or an error naming the bad value.
    pub fn check(&self) -> Result<()> {
        let sheet = &self.sheet;
        if sheet.scale == 0 {
            return invalid(
                "scale is the denominator, like 2500 for 1:2500, so it cannot be 0".to_string(),
            );
        }
        if !(0.0..=1.0).contains(&sheet.context_opacity) {
            return invalid(format!(
                "context_opacity runs from 0 to 1, like 0.4, got {}",
                sheet.context_opacity
            ));
        }
        // Named as the renderer names them, so the user can find the text.
        sheet
            .title_style
            .check()
            .map_err(|error| named("title", error))?;
        sheet
            .subtitle_style
            .check()
            .map_err(|error| named("subtitle", error))?;
        for text in &sheet.texts {
            let name = format!("text box {}", text.id);
            text.style.check().map_err(|error| named(&name, error))?;
        }

        let tilt_deg = self.view.tilt_deg;
        if !(MIN_TILT_DEG..=MAX_TILT_DEG).contains(&tilt_deg) {
            return invalid(format!(
                "tilt_deg must be between {MIN_TILT_DEG} and {MAX_TILT_DEG}, got {tilt_deg}"
            ));
        }
        // As in the app, where the Section view is greyed out until a line is drawn.
        match &self.section {
            Some(line) => line.check()?,
            None if self.view.mode == ViewMode::Section => {
                return invalid(
                    "this sheet has no section line yet; draw one before choosing the section view"
                        .to_string(),
                )
            }
            None => {}
        }
        if let Some(problem) = self.sun.problem() {
            return invalid(format!("sun: {problem}"));
        }

        for (layer, layer_override) in &self.overrides {
            check_override(*layer, layer_override)?;
        }

        Ok(())
    }
}

fn check_override(layer: Layer, layer_override: &LayerOverride) -> Result<()> {
    let name = layer.label();
    for colour in [&layer_override.fill, &layer_override.stroke]
        .into_iter()
        .flatten()
    {
        if !is_colour(colour) {
            return invalid(format!(
                "{name}: colour \"{colour}\" should be \"#RRGGBB\" or a palette role like \"@accent\""
            ));
        }
    }
    if let Some(opacity) = layer_override.opacity {
        if !(0.0..=1.0).contains(&opacity) {
            return invalid(format!(
                "{name}: opacity must be between 0 and 1, got {opacity}"
            ));
        }
    }
    if layer_override.stroke_mm.is_some_and(|width| width < 0.0) {
        return invalid(format!("{name}: stroke_mm cannot be negative"));
    }

    Ok(())
}

/// "title" and "size_pt must be ..." -> "title: size_pt must be ...".
fn named(name: &str, error: Error) -> Error {
    Error::Invalid(format!("{name}: {error}"))
}

fn invalid(message: String) -> Result<()> {
    Err(Error::Invalid(message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sheet::TextStyle;

    #[test]
    fn the_default_look_passes() {
        assert!(Look::default().check().is_ok());
    }

    #[test]
    fn a_zero_scale_and_a_steep_tilt_are_refused() {
        let mut flat = Look::default();
        flat.sheet.scale = 0;
        let mut steep = Look::default();
        steep.view.tilt_deg = 90.0;

        assert!(flat.check().unwrap_err().to_string().contains("scale"));
        assert!(steep.check().unwrap_err().to_string().contains("tilt_deg"));
    }

    #[test]
    fn the_section_view_needs_a_line() {
        let mut look = Look::default();
        look.view.mode = ViewMode::Section;

        assert!(look
            .check()
            .unwrap_err()
            .to_string()
            .contains("section line"));
    }

    #[test]
    fn a_misspelt_role_and_a_bad_opacity_name_their_layer() {
        let mut role = Look::default();
        role.overrides.insert(
            Layer::Water,
            LayerOverride {
                fill: Some("@accentt".to_string()),
                ..LayerOverride::default()
            },
        );
        let mut opacity = Look::default();
        opacity.overrides.insert(
            Layer::Water,
            LayerOverride {
                opacity: Some(1.5),
                ..LayerOverride::default()
            },
        );

        assert!(role.check().unwrap_err().to_string().contains("Water"));
        assert!(opacity.check().unwrap_err().to_string().contains("opacity"));
    }

    #[test]
    fn a_text_box_too_small_to_print_is_refused() {
        let mut look = Look::default();
        look.sheet.title_style = TextStyle {
            size_pt: Some(1.0),
            ..TextStyle::default()
        };

        assert!(look.check().unwrap_err().to_string().contains("size_pt"));
    }
}
