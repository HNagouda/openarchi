//! Sheet text as edit_text changes it: the title, the subtitle and text boxes.
//!
//! The title and subtitle are always there and can only be emptied; text boxes
//! come and go. All three share one way of changing words, style and position.

use openarchi_core::sheet::{Align, Position, Sheet, TextBox, TextStyle, SUBTITLE_ID, TITLE_ID};

use crate::text_inputs::{AlignInput, EditText, PositionInput, TextStyleInput};

/// The text id that adds a text box instead of changing one.
const NEW_TEXT_ID: &str = "new";

/// The sheet with the edit applied, and the id of the text it touched.
pub fn edited_sheet(sheet: &Sheet, edit: EditText) -> Result<(Sheet, String), String> {
    let mut sheet = sheet.clone();
    let (width, height) = sheet.size_mm();
    let position = edit
        .position
        .map(|position| checked_position(position, width, height))
        .transpose()?;
    let is_removal = edit.remove == Some(true);

    match edit.text_id.as_str() {
        TITLE_ID | SUBTITLE_ID if is_removal => Err(format!(
            "the {} cannot be removed; set its text to \"\" instead",
            edit.text_id
        )),
        TITLE_ID => {
            apply(
                &mut sheet.title,
                &mut sheet.title_style,
                &mut sheet.title_position,
                &edit,
                position,
            )?;
            Ok((sheet, edit.text_id))
        }
        SUBTITLE_ID => {
            apply(
                &mut sheet.subtitle,
                &mut sheet.subtitle_style,
                &mut sheet.subtitle_position,
                &edit,
                position,
            )?;
            Ok((sheet, edit.text_id))
        }
        NEW_TEXT_ID => {
            let Some(text) = edit.text.clone().filter(|text| !text.trim().is_empty()) else {
                return Err("a new text box needs some text".to_string());
            };
            let id = sheet.next_text_id();
            sheet.texts.push(TextBox {
                id: id.clone(),
                text,
                style: updated_style(&TextStyle::default(), false, edit.style)?,
                position,
            });
            Ok((sheet, id))
        }
        id => {
            let Some(index) = sheet.texts.iter().position(|text| text.id == id) else {
                let ids: Vec<&str> = sheet.texts.iter().map(|text| text.id.as_str()).collect();
                return Err(format!(
                    "no text \"{id}\"; use \"title\", \"subtitle\", \"new\" or one of [{}]",
                    ids.join(", ")
                ));
            };
            if is_removal {
                sheet.texts.remove(index);
            } else {
                let text_box = &mut sheet.texts[index];
                apply(
                    &mut text_box.text,
                    &mut text_box.style,
                    &mut text_box.position,
                    &edit,
                    position,
                )?;
            }
            Ok((sheet, edit.text_id))
        }
    }
}

/// Changes one text's words, style and position in place, as the edit asks.
fn apply(
    text: &mut String,
    style: &mut TextStyle,
    position: &mut Option<Position>,
    edit: &EditText,
    new_position: Option<Position>,
) -> Result<(), String> {
    if let Some(new_text) = &edit.text {
        *text = new_text.clone();
    }
    *style = updated_style(style, edit.reset_style == Some(true), edit.style.clone())?;
    if edit.auto_position == Some(true) {
        *position = None;
    }
    if new_position.is_some() {
        *position = new_position;
    }

    Ok(())
}

fn updated_style(
    style: &TextStyle,
    is_reset: bool,
    change: Option<TextStyleInput>,
) -> Result<TextStyle, String> {
    let base = if is_reset {
        TextStyle::default()
    } else {
        style.clone()
    };
    let change = change.unwrap_or_default();
    let updated = TextStyle {
        family: change.family.or(base.family),
        bold: change.bold.or(base.bold),
        italic: change.italic.or(base.italic),
        size_pt: change.size_pt.or(base.size_pt),
        align: change.align.map(Align::from).or(base.align),
        colour: change.colour.or(base.colour),
        opacity: change.opacity.or(base.opacity),
    };
    updated.check().map_err(|error| error.to_string())?;

    Ok(updated)
}

/// A position on the paper, or an error giving the paper's size.
fn checked_position(position: PositionInput, width: f64, height: f64) -> Result<Position, String> {
    let is_on_paper =
        (0.0..=width).contains(&position.x_mm) && (0.0..=height).contains(&position.y_mm);
    if !is_on_paper {
        return Err(format!(
            "position must lie on the {width} × {height} mm sheet, got x_mm {} and y_mm {}",
            position.x_mm, position.y_mm
        ));
    }

    Ok(Position {
        x_mm: position.x_mm,
        y_mm: position.y_mm,
    })
}

impl From<AlignInput> for Align {
    fn from(align: AlignInput) -> Self {
        match align {
            AlignInput::Left => Align::Left,
            AlignInput::Centre => Align::Centre,
            AlignInput::Right => Align::Right,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(text_id: &str) -> EditText {
        EditText {
            id: "project".to_string(),
            text_id: text_id.to_string(),
            text: None,
            style: None,
            reset_style: None,
            position: None,
            auto_position: None,
            remove: None,
            tab: None,
        }
    }

    #[test]
    fn new_text_box_gets_the_next_id_and_its_style() {
        let request = EditText {
            text: Some("Westerkerk".to_string()),
            style: Some(TextStyleInput {
                italic: Some(true),
                ..TextStyleInput::default()
            }),
            position: Some(PositionInput {
                x_mm: 30.0,
                y_mm: 40.0,
            }),
            ..edit("new")
        };

        let (sheet, id) = edited_sheet(&Sheet::default(), request).unwrap();

        assert_eq!(id, "text-1");
        assert_eq!(sheet.texts[0].text, "Westerkerk");
        assert_eq!(sheet.texts[0].style.italic, Some(true));
        assert_eq!(sheet.texts[0].position.unwrap().x_mm, 30.0);
    }

    #[test]
    fn moving_the_title_keeps_its_style_and_auto_position_brings_it_back() {
        let mut sheet = Sheet::default();
        sheet.title_style.bold = Some(true);
        let moved = EditText {
            position: Some(PositionInput {
                x_mm: 20.0,
                y_mm: 20.0,
            }),
            ..edit("title")
        };

        let (sheet, _) = edited_sheet(&sheet, moved).unwrap();
        assert!(sheet.title_position.is_some());
        assert_eq!(sheet.title_style.bold, Some(true));

        let back = EditText {
            auto_position: Some(true),
            ..edit("title")
        };
        let (sheet, _) = edited_sheet(&sheet, back).unwrap();
        assert_eq!(sheet.title_position, None);
    }

    #[test]
    fn reset_style_clears_the_old_fields_before_the_new_ones() {
        let mut sheet = Sheet::default();
        sheet.subtitle_style.italic = Some(true);
        let request = EditText {
            reset_style: Some(true),
            style: Some(TextStyleInput {
                size_pt: Some(12.0),
                ..TextStyleInput::default()
            }),
            ..edit("subtitle")
        };

        let (sheet, _) = edited_sheet(&sheet, request).unwrap();

        assert_eq!(sheet.subtitle_style.italic, None);
        assert_eq!(sheet.subtitle_style.size_pt, Some(12.0));
    }

    #[test]
    fn bad_family_size_and_position_are_refused() {
        let family = EditText {
            style: Some(TextStyleInput {
                family: Some("Comic Sans".to_string()),
                ..TextStyleInput::default()
            }),
            ..edit("title")
        };
        let size = EditText {
            style: Some(TextStyleInput {
                size_pt: Some(1000.0),
                ..TextStyleInput::default()
            }),
            ..edit("title")
        };
        let position = EditText {
            position: Some(PositionInput {
                x_mm: 900.0,
                y_mm: 10.0,
            }),
            ..edit("title")
        };

        assert!(edited_sheet(&Sheet::default(), family).is_err());
        assert!(edited_sheet(&Sheet::default(), size).is_err());
        assert!(edited_sheet(&Sheet::default(), position).is_err());
    }

    #[test]
    fn removing_a_box_works_and_removing_the_title_is_refused() {
        let mut sheet = Sheet::default();
        sheet.texts.push(TextBox {
            id: "text-1".to_string(),
            text: "Note".to_string(),
            style: TextStyle::default(),
            position: None,
        });
        let remove = |id: &str| EditText {
            remove: Some(true),
            ..edit(id)
        };

        let (emptied, _) = edited_sheet(&sheet, remove("text-1")).unwrap();

        assert!(emptied.texts.is_empty());
        assert!(edited_sheet(&sheet, remove("title")).is_err());
        assert!(edited_sheet(&sheet, remove("text-9")).is_err());
    }
}
