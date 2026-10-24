use serde_json::Value;

use crate::project::{packed_scene, Project, PROJECT_FORMAT_VERSION};
use crate::scene::Scene;
use crate::{Error, Result};

/// Lifts a project file of any earlier format version to the current one.
///
/// Each format change adds one step here, oldest first, so a file from any
/// version walks up through every step.
pub fn migrate(mut value: Value) -> Result<Project> {
    let Some(version) = value.get("format_version").and_then(Value::as_u64) else {
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
    }

    serde_json::from_value(value)
        .map_err(|e| Error::Invalid(format!("not a valid openarchi project: {e}")))
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
