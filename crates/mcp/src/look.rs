//! Partial updates to a project's look, as set_look applies them.
//!
//! Diagram, variant and theme ids are checked against the catalog by the
//! caller; this module only merges what was passed into what was there.

use std::collections::BTreeMap;

use openarchi_core::project::Look;
use openarchi_core::scene::Layer;
use openarchi_core::sheet::Sheet;
use openarchi_core::theme::LayerOverride;
use openarchi_core::view::{SunMoment, View, MAX_TILT_DEG, MIN_TILT_DEG};

use crate::inputs::{OverrideInput, SheetInput, SunInput, ViewInput};

/// The look with every passed field replaced and the rest kept.
/// `diagram` is the diagram id and variant, already checked.
pub fn updated_look(
    look: &Look,
    diagram: Option<(String, Option<String>)>,
    theme_id: Option<String>,
    sheet: Option<SheetInput>,
    overrides: Option<BTreeMap<String, Option<OverrideInput>>>,
    view: Option<ViewInput>,
    sun: Option<SunInput>,
) -> Result<Look, String> {
    let (diagram_id, variant) =
        diagram.unwrap_or_else(|| (look.diagram_id.clone(), look.variant.clone()));

    Ok(Look {
        diagram_id,
        variant,
        theme_id: theme_id.unwrap_or_else(|| look.theme_id.clone()),
        sheet: updated_sheet(&look.sheet, sheet.unwrap_or_default())?,
        overrides: updated_overrides(&look.overrides, overrides.unwrap_or_default())?,
        view: updated_view(&look.view, view.unwrap_or_default())?,
        sun: updated_sun(&look.sun, sun.unwrap_or_default())?,
        // The tab's name, sub-selection and pin toggle are not set_look's to change.
        ..look.clone()
    })
}

/// Plate ids are checked against the catalog by the caller.
fn updated_view(view: &View, changes: ViewInput) -> Result<View, String> {
    let tilt_deg = changes.tilt_deg.unwrap_or(view.tilt_deg);
    if !(MIN_TILT_DEG..=MAX_TILT_DEG).contains(&tilt_deg) {
        return Err(format!(
            "tilt_deg must be between {MIN_TILT_DEG} and {MAX_TILT_DEG}, got {tilt_deg}"
        ));
    }
    if changes.plates.as_ref().is_some_and(Vec::is_empty) {
        return Err("plates needs at least one diagram id".to_string());
    }

    Ok(View {
        mode: changes.mode.map(Into::into).unwrap_or(view.mode),
        rotation_deg: changes
            .rotation_deg
            .map_or(view.rotation_deg, |degrees| degrees.rem_euclid(360.0)),
        tilt_deg,
        plates: changes.plates.unwrap_or_else(|| view.plates.clone()),
        stacking: changes.stacking.unwrap_or(view.stacking),
    })
}

fn updated_sun(sun: &SunMoment, changes: SunInput) -> Result<SunMoment, String> {
    let moment = SunMoment {
        month: changes.month.unwrap_or(sun.month),
        day: changes.day.unwrap_or(sun.day),
        hour: changes.hour.unwrap_or(sun.hour),
        minute: changes.minute.unwrap_or(sun.minute),
    };
    match moment.problem() {
        Some(problem) => Err(format!("sun: {problem}")),
        None => Ok(moment),
    }
}

fn updated_sheet(sheet: &Sheet, changes: SheetInput) -> Result<Sheet, String> {
    if let Some(opacity) = changes.context_opacity {
        if !(0.0..=1.0).contains(&opacity) {
            return Err(format!(
                "context_opacity runs from 0 to 1, like 0.4, got {opacity}"
            ));
        }
    }
    if changes.scale == Some(0) {
        return Err(
            "scale is the denominator, like 2500 for 1:2500, so it cannot be 0".to_string(),
        );
    }

    Ok(Sheet {
        paper: changes.paper.map(Into::into).unwrap_or(sheet.paper),
        landscape: changes.landscape.unwrap_or(sheet.landscape),
        scale: changes.scale.unwrap_or(sheet.scale),
        frame: changes.frame.map(Into::into).unwrap_or(sheet.frame),
        title: changes.title.unwrap_or_else(|| sheet.title.clone()),
        subtitle: changes.subtitle.unwrap_or_else(|| sheet.subtitle.clone()),
        show_legend: changes.show_legend.unwrap_or(sheet.show_legend),
        show_north_arrow: changes.show_north_arrow.unwrap_or(sheet.show_north_arrow),
        show_scale_bar: changes.show_scale_bar.unwrap_or(sheet.show_scale_bar),
        show_credits: changes.show_credits.unwrap_or(sheet.show_credits),
        show_context: changes.show_context.unwrap_or(sheet.show_context),
        context_opacity: changes.context_opacity.unwrap_or(sheet.context_opacity),
        // Text style, position and text boxes change through edit_text.
        title_style: sheet.title_style.clone(),
        subtitle_style: sheet.subtitle_style.clone(),
        title_position: sheet.title_position,
        subtitle_position: sheet.subtitle_position,
        texts: sheet.texts.clone(),
    })
}

fn updated_overrides(
    overrides: &BTreeMap<Layer, LayerOverride>,
    changes: BTreeMap<String, Option<OverrideInput>>,
) -> Result<BTreeMap<Layer, LayerOverride>, String> {
    let mut updated = overrides.clone();
    for (name, change) in changes {
        let layer = parse_layer(&name)?;
        match change {
            Some(change) => {
                updated.insert(layer, layer_override(&name, change)?);
            }
            None => {
                updated.remove(&layer);
            }
        }
    }

    Ok(updated)
}

/// "building" -> `Layer::Building`, using the same names saved files use.
fn parse_layer(name: &str) -> Result<Layer, String> {
    serde_json::from_value(serde_json::Value::from(name)).map_err(|_| {
        let names: Vec<String> = Layer::ALL
            .iter()
            .map(|layer| serde_json::to_string(layer).unwrap_or_default())
            .collect();
        format!("unknown layer \"{name}\"; layers are {}", names.join(", "))
    })
}

fn layer_override(layer: &str, change: OverrideInput) -> Result<LayerOverride, String> {
    for colour in [&change.fill, &change.stroke].into_iter().flatten() {
        if !is_colour(colour) {
            return Err(format!(
                "{layer}: colour \"{colour}\" should be \"#RRGGBB\" or a palette role like \"@accent\""
            ));
        }
    }
    if let Some(opacity) = change.opacity {
        if !(0.0..=1.0).contains(&opacity) {
            return Err(format!(
                "{layer}: opacity must be between 0 and 1, got {opacity}"
            ));
        }
    }
    if change.stroke_mm.is_some_and(|width| width < 0.0) {
        return Err(format!("{layer}: stroke_mm cannot be negative"));
    }

    Ok(LayerOverride {
        hidden: change.hidden,
        fill: change.fill,
        stroke: change.stroke,
        stroke_mm: change.stroke_mm,
        opacity: change.opacity,
    })
}

/// "#1a2b3c" or "@accent". Which roles exist is the renderer's call.
fn is_colour(value: &str) -> bool {
    if let Some(role) = value.strip_prefix('@') {
        return !role.is_empty();
    }
    match value.strip_prefix('#') {
        Some(hex) => hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use openarchi_core::sheet::{Frame, Paper};
    use openarchi_core::view::ViewMode;

    use super::*;
    use crate::inputs::{FrameInput, PaperInput, ViewModeInput};

    #[test]
    fn only_passed_sheet_fields_change() {
        let look = Look::default();
        let sheet = SheetInput {
            paper: Some(PaperInput::A1),
            title: Some("Westermarkt".to_string()),
            ..SheetInput::default()
        };

        let updated = updated_look(&look, None, None, Some(sheet), None, None, None).unwrap();

        assert_eq!(updated.sheet.paper, Paper::A1);
        assert_eq!(updated.sheet.title, "Westermarkt");
        assert_eq!(updated.sheet.frame, Frame::Circle);
        assert_eq!(updated.sheet.scale, look.sheet.scale);
        assert_eq!(updated.theme_id, look.theme_id);
    }

    #[test]
    fn diagram_and_theme_ids_are_replaced_when_passed() {
        let look = Look::default();
        let updated = updated_look(
            &look,
            Some(("figure-ground".into(), None)),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(updated.diagram_id, "figure-ground");
        assert_eq!(updated.theme_id, look.theme_id);
    }

    #[test]
    fn null_override_removes_it_and_others_stay() {
        let mut look = Look::default();
        look.overrides
            .insert(Layer::Water, LayerOverride::default());
        look.overrides.insert(Layer::Tree, LayerOverride::default());
        let changes = BTreeMap::from([
            ("water".to_string(), None),
            (
                "building".to_string(),
                Some(OverrideInput {
                    fill: Some("#112233".to_string()),
                    ..OverrideInput::default()
                }),
            ),
        ]);

        let updated = updated_look(&look, None, None, None, Some(changes), None, None).unwrap();

        assert!(!updated.overrides.contains_key(&Layer::Water));
        assert!(updated.overrides.contains_key(&Layer::Tree));
        assert_eq!(
            updated.overrides[&Layer::Building].fill.as_deref(),
            Some("#112233")
        );
    }

    #[test]
    fn unknown_layer_names_the_valid_ones() {
        let changes = BTreeMap::from([("buildings".to_string(), None)]);
        let error = updated_look(
            &Look::default(),
            None,
            None,
            None,
            Some(changes),
            None,
            None,
        )
        .unwrap_err();
        assert!(error.contains("\"building\""), "{error}");
    }

    #[test]
    fn bad_colour_and_opacity_are_refused() {
        let colour = OverrideInput {
            fill: Some("red".to_string()),
            ..OverrideInput::default()
        };
        let opacity = OverrideInput {
            opacity: Some(1.5),
            ..OverrideInput::default()
        };

        assert!(layer_override("water", colour).is_err());
        assert!(layer_override("water", opacity).is_err());
    }

    #[test]
    fn view_fields_change_one_at_a_time() {
        let view = ViewInput {
            mode: Some(ViewModeInput::Axonometric),
            rotation_deg: Some(-30.0),
            ..ViewInput::default()
        };

        let updated = updated_view(&View::default(), view).unwrap();

        assert_eq!(updated.mode, ViewMode::Axonometric);
        assert_eq!(updated.rotation_deg, 330.0);
        assert_eq!(updated.plates, View::default().plates);
    }

    #[test]
    fn a_tilt_out_of_range_and_an_empty_stack_are_refused() {
        let steep = ViewInput {
            tilt_deg: Some(90.0),
            ..ViewInput::default()
        };
        let empty = ViewInput {
            plates: Some(Vec::new()),
            ..ViewInput::default()
        };

        assert!(updated_view(&View::default(), steep).is_err());
        assert!(updated_view(&View::default(), empty).is_err());
    }

    #[test]
    fn sun_keeps_the_fields_not_passed_and_refuses_31_june() {
        let afternoon = SunInput {
            hour: Some(16),
            ..SunInput::default()
        };
        let impossible = SunInput {
            month: Some(6),
            day: Some(31),
            ..SunInput::default()
        };

        let updated = updated_sun(&SunMoment::default(), afternoon).unwrap();

        assert_eq!((updated.month, updated.day, updated.hour), (3, 21, 16));
        assert!(updated_sun(&SunMoment::default(), impossible).is_err());
    }

    #[test]
    fn frame_input_maps_to_the_core_frame() {
        let sheet = SheetInput {
            frame: Some(FrameInput::Area),
            ..SheetInput::default()
        };
        let updated = updated_sheet(&Sheet::default(), sheet).unwrap();
        assert_eq!(updated.frame, Frame::Area);
    }
}
