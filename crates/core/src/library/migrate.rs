use serde_json::{json, Value};

use crate::catalog;
use crate::project::{packed_scene, Project, PROJECT_FORMAT_VERSION};
use crate::scene::Scene;
use crate::{Error, Result};

/// Lifts a project file of any earlier format version to the current one.
///
/// Each format change adds one step here, oldest first, so a file from any
/// version walks up through every step.
pub fn migrate(mut value: Value) -> Result<Project> {
    let Some(mut version) = value.get("format_version").and_then(Value::as_u64) else {
        return Err(Error::Invalid(
            "not an openarchi project: format_version is missing".to_string(),
        ));
    };
    if version > u64::from(PROJECT_FORMAT_VERSION) {
        return Err(Error::Invalid(format!(
            "saved by a newer openarchi (format {version}); update openarchi to open it"
        )));
    }
    if version == 0 {
        return Err(Error::Invalid("unknown project format 0".to_string()));
    }

    if version == 1 {
        value = v1_to_v2(value)?;
        version = 2;
    }
    if version == 2 {
        value = v2_to_v3(value);
        version = 3;
    }
    debug_assert_eq!(version, u64::from(PROJECT_FORMAT_VERSION));

    let project: Project = serde_json::from_value(value)
        .map_err(|e| Error::Invalid(format!("not a valid openarchi project: {e}")))?;
    // Not a format change: built-in diagrams that were folded into another as
    // variants, in any version, open on the variant they became.
    Ok(Project {
        look: catalog::current_look(project.look),
        more_sheets: project
            .more_sheets
            .into_iter()
            .map(catalog::current_look)
            .collect(),
        ..project
    })
}

/// Version 1 stored the scene as plain JSON; version 2 stores it packed.
fn v1_to_v2(mut value: Value) -> Result<Value> {
    if let Some(slot) = value.get_mut("scene").filter(|scene| !scene.is_null()) {
        let scene: Scene = serde_json::from_value(slot.take())
            .map_err(|e| Error::Invalid(format!("not a valid openarchi scene: {e}")))?;
        *slot = Value::String(packed_scene::pack(&scene)?);
    }
    value["format_version"] = Value::from(2);
    Ok(value)
}

/// Version 3 lets the title and subtitle be styled and moved, and adds text
/// boxes. An older sheet keeps its automatic layout and theme type.
fn v2_to_v3(mut value: Value) -> Value {
    if let Some(sheet) = value
        .pointer_mut("/look/sheet")
        .and_then(Value::as_object_mut)
    {
        sheet.insert("title_style".to_string(), json!({}));
        sheet.insert("subtitle_style".to_string(), json!({}));
        sheet.insert("title_position".to_string(), Value::Null);
        sheet.insert("subtitle_position".to_string(), Value::Null);
        sheet.insert("texts".to_string(), json!([]));
    }
    value["format_version"] = Value::from(3);

    value
}
