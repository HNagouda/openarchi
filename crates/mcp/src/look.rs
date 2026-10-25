//! Partial updates to a project's look, as set_look applies them.
//!
//! Diagram, variant and theme ids are checked against the catalog by the
//! caller; this module merges what was passed into what was there, and the
//! merged look is held to the core's `Look::check`, as the app's saves are.

use std::collections::BTreeMap;

use openarchi_core::project::Look;
use openarchi_core::scene::Layer;
use openarchi_core::sheet::Sheet;
use openarchi_core::theme::LayerOverride;
use openarchi_core::view::{SunMoment, View};

use crate::style_inputs::{OverrideInput, SheetInput, SunInput, ViewInput};

/// The look with every passed field replaced and the rest kept.
/// `diagram` is the diagram id and variant, already checked.
pub fn updated_look(
    look: &Look,
    diagram: (String, Option<String>),
    theme_id: Option<String>,
    sheet: Option<SheetInput>,
    overrides: Option<BTreeMap<String, Option<OverrideInput>>>,
    view: Option<ViewInput>,
    sun: Option<SunInput>,
) -> Result<Look, String> {
    let (diagram_id, variant) = diagram;
    let updated = Look {
        diagram_id,
        variant,
        theme_id: theme_id.unwrap_or_else(|| look.theme_id.clone()),
        sheet: updated_sheet(&look.sheet, sheet.unwrap_or_default()),
        overrides: updated_overrides(&look.overrides, overrides.unwrap_or_default())?,
        view: updated_view(&look.view, view.unwrap_or_default())?,
        sun: updated_sun(&look.sun, sun.unwrap_or_default()),
        // Everything else on the sheet (name, area, marks, section, viewpoint,
        // wind) has its own tool.
        ..look.clone()
    };
    updated.check().map_err(|error| error.to_string())?;

    Ok(updated)
}

/// Plate ids are checked against the catalog by the caller.
fn updated_view(view: &View, changes: ViewInput) -> Result<View, String> {
    // The app never empties the stack; the renderer would fall back to the sheet's own diagram.
    if changes.plates.as_ref().is_some_and(Vec::is_empty) {
        return Err("plates needs at least one diagram id".to_string());
    }

    Ok(View {
        mode: changes.mode.map(Into::into).unwrap_or(view.mode),
        rotation_deg: changes
            .rotation_deg
            .map_or(view.rotation_deg, |degrees| degrees.rem_euclid(360.0)),
        tilt_deg: changes.tilt_deg.unwrap_or(view.tilt_deg),
        plates: changes.plates.unwrap_or_else(|| view.plates.clone()),
        stacking: changes.stacking.unwrap_or(view.stacking),
    })
}

fn updated_sun(sun: &SunMoment, changes: SunInput) -> SunMoment {
    SunMoment {
        month: changes.month.unwrap_or(sun.month),
        day: changes.day.unwrap_or(sun.day),
        hour: changes.hour.unwrap_or(sun.hour),
        minute: changes.minute.unwrap_or(sun.minute),
    }
}

fn updated_sheet(sheet: &Sheet, changes: SheetInput) -> Sheet {
    Sheet {
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
    }
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
                updated.insert(layer, layer_override(change));
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

fn layer_override(change: OverrideInput) -> LayerOverride {
    LayerOverride {
        hidden: change.hidden,
        fill: change.fill,
        stroke: change.stroke,
        stroke_mm: change.stroke_mm,
        opacity: change.opacity,
    }
}

#[cfg(test)]
mod tests {
    use openarchi_core::sheet::{Frame, Paper};
    use openarchi_core::view::ViewMode;

    use super::*;
    use crate::style_inputs::{FrameInput, PaperInput, ViewModeInput};

    #[test]
    fn only_passed_sheet_fields_change() {
        let look = Look::default();
        let sheet = SheetInput {
            paper: Some(PaperInput::A1),
            title: Some("Westermarkt".to_string()),
            ..SheetInput::default()
        };

        let diagram = (look.diagram_id.clone(), None);
        let updated = updated_look(&look, diagram, None, Some(sheet), None, None, None).unwrap();

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
            ("figure-ground".into(), None),
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

        let diagram = (look.diagram_id.clone(), None);
        let updated = updated_look(&look, diagram, None, None, Some(changes), None, None).unwrap();

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
        let look = Look::default();
        let error = updated_look(
            &look,
            (look.diagram_id.clone(), None),
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
    fn bad_colour_and_opacity_are_refused_by_the_core_check() {
        let changes =
            |change: OverrideInput| Some(BTreeMap::from([("water".to_string(), Some(change))]));
        let misspelt = OverrideInput {
            fill: Some("@accentt".to_string()),
            ..OverrideInput::default()
        };
        let opacity = OverrideInput {
            opacity: Some(1.5),
            ..OverrideInput::default()
        };
        let known = OverrideInput {
            fill: Some("@accent".to_string()),
            ..OverrideInput::default()
        };
        let look = Look::default();
        let update = |overrides| {
            updated_look(
                &look,
                (look.diagram_id.clone(), None),
                None,
                None,
                overrides,
                None,
                None,
            )
        };

        assert!(update(changes(misspelt)).is_err());
        assert!(update(changes(opacity)).is_err());
        assert!(update(changes(known)).is_ok());
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
        let look = Look::default();
        let update = |view| {
            updated_look(
                &look,
                (look.diagram_id.clone(), None),
                None,
                None,
                None,
                Some(view),
                None,
            )
        };
        let steep = ViewInput {
            tilt_deg: Some(90.0),
            ..ViewInput::default()
        };
        let empty = ViewInput {
            plates: Some(Vec::new()),
            ..ViewInput::default()
        };

        assert!(update(steep).is_err());
        assert!(update(empty).is_err());
    }

    #[test]
    fn sun_keeps_the_fields_not_passed_and_refuses_31_june() {
        let look = Look::default();
        let update = |sun| {
            updated_look(
                &look,
                (look.diagram_id.clone(), None),
                None,
                None,
                None,
                None,
                Some(sun),
            )
        };
        let afternoon = SunInput {
            hour: Some(16),
            ..SunInput::default()
        };
        let impossible = SunInput {
            month: Some(6),
            day: Some(31),
            ..SunInput::default()
        };

        let updated = update(afternoon).unwrap().sun;

        assert_eq!((updated.month, updated.day, updated.hour), (3, 21, 16));
        assert!(update(impossible).is_err());
    }

    #[test]
    fn frame_input_maps_to_the_core_frame() {
        let sheet = SheetInput {
            frame: Some(FrameInput::Area),
            ..SheetInput::default()
        };
        let updated = updated_sheet(&Sheet::default(), sheet);
        assert_eq!(updated.frame, Frame::Area);
    }
}
