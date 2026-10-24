use serde_json::Value;

use crate::project::{Project, PROJECT_FORMAT_VERSION};
use crate::{Error, Result};

/// Lifts a project file of any earlier format version to the current one.
///
/// Each format change adds one step here, oldest first, so a file from any
/// version walks up through every step:
///
/// ```text
/// if version == 1 { value = v1_to_v2(value); version = 2; }
/// ```
pub fn migrate(value: Value) -> Result<Project> {
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

    serde_json::from_value(value)
        .map_err(|e| Error::Invalid(format!("not a valid openarchi project: {e}")))
}
