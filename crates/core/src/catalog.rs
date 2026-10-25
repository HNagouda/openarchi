//! The diagrams and themes that ship with the app, plus the user's own themes.
//!
//! Built-ins are JSON files in assets/, compiled into the binary, so a theme
//! author can copy one as a starting point and the app never depends on files
//! next to it.

mod lookup;
mod missing_layers;

use std::path::Path;

pub use self::lookup::{current_id, current_look, find_drawing, find_plate, plate_id};
use crate::diagram::Diagram;
use crate::render::{is_bundled_font, is_hex, role, ROLES};
use crate::theme::{LayerStyle, Pattern, Theme, THEME_FORMAT_VERSION};
use crate::{Error, Result};

const MIN_RAMP_COLOURS: usize = 3;
const MIN_CATEGORY_COLOURS: usize = 6;
const THEME_EXTENSION: &str = "json";

/// Built-in diagrams in picker order, in one place. The picker shows them under
/// their `group` headings, so the diagrams of one group stay together here.
const BUILTIN_DIAGRAMS: [(&str, &str); 26] = [
    // Context and built form.
    ("context", include_str!("../assets/diagrams/context.json")),
    (
        "figure-ground",
        include_str!("../assets/diagrams/figure-ground.json"),
    ),
    (
        "open-space",
        include_str!("../assets/diagrams/open-space.json"),
    ),
    (
        "building-use",
        include_str!("../assets/diagrams/building-use.json"),
    ),
    (
        "building-age",
        include_str!("../assets/diagrams/building-age.json"),
    ),
    (
        "building-height",
        include_str!("../assets/diagrams/building-height.json"),
    ),
    (
        "building-floors",
        include_str!("../assets/diagrams/building-floors.json"),
    ),
    ("parcels", include_str!("../assets/diagrams/parcels.json")),
    (
        "city-block",
        include_str!("../assets/diagrams/city-block.json"),
    ),
    // Movement and places.
    ("movement", include_str!("../assets/diagrams/movement.json")),
    ("transit", include_str!("../assets/diagrams/transit.json")),
    (
        "walkability",
        include_str!("../assets/diagrams/walkability.json"),
    ),
    ("business", include_str!("../assets/diagrams/business.json")),
    ("facility", include_str!("../assets/diagrams/facility.json")),
    // Environment.
    (
        "landscape",
        include_str!("../assets/diagrams/landscape.json"),
    ),
    ("slope", include_str!("../assets/diagrams/slope.json")),
    ("flood", include_str!("../assets/diagrams/flood.json")),
    ("noise", include_str!("../assets/diagrams/noise.json")),
    (
        "air-quality",
        include_str!("../assets/diagrams/air-quality.json"),
    ),
    // People.
    (
        "population",
        include_str!("../assets/diagrams/population.json"),
    ),
    (
        "households",
        include_str!("../assets/diagrams/households.json"),
    ),
    // Sun and shade.
    ("sun-path", include_str!("../assets/diagrams/sun-path.json")),
    ("shadow", include_str!("../assets/diagrams/shadow.json")),
    (
        "sun-hours",
        include_str!("../assets/diagrams/sun-hours.json"),
    ),
    ("sky-view", include_str!("../assets/diagrams/sky-view.json")),
    (
        "solar-radiation",
        include_str!("../assets/diagrams/solar-radiation.json"),
    ),
];

/// Built-in themes in picker order, in one place. "paper" is the default look.
const BUILTIN_THEMES: [(&str, &str); 8] = [
    ("paper", include_str!("../assets/themes/paper.json")),
    ("ink", include_str!("../assets/themes/ink.json")),
    ("mono", include_str!("../assets/themes/mono.json")),
    ("polder", include_str!("../assets/themes/polder.json")),
    ("blueprint", include_str!("../assets/themes/blueprint.json")),
    (
        "terracotta",
        include_str!("../assets/themes/terracotta.json"),
    ),
    ("midnight", include_str!("../assets/themes/midnight.json")),
    ("pastel", include_str!("../assets/themes/pastel.json")),
];

/// Built-in diagrams, in the order the picker shows them.
pub fn builtin_diagrams() -> Vec<Diagram> {
    BUILTIN_DIAGRAMS
        .iter()
        .map(|(name, json)| {
            serde_json::from_str(json)
                .unwrap_or_else(|error| panic!("built-in diagram {name} is broken: {error}"))
        })
        .collect()
}

/// Built-in themes, in the order the picker shows them.
pub fn builtin_themes() -> Vec<Theme> {
    BUILTIN_THEMES
        .iter()
        .map(|(name, json)| {
            serde_json::from_str(json)
                .unwrap_or_else(|error| panic!("built-in theme {name} is broken: {error}"))
        })
        .collect()
}

/// Built-in themes followed by every valid `*.json` theme in `user_dir`.
/// A broken file is skipped and reported in the second value, never fatal.
pub fn all_themes(user_dir: &Path) -> (Vec<Theme>, Vec<String>) {
    let mut themes = builtin_themes();
    let mut problems = Vec::new();
    let Ok(entries) = std::fs::read_dir(user_dir) else {
        // No user folder yet is the normal first-run state, not a problem.
        return (themes, problems);
    };

    let mut paths: Vec<_> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == THEME_EXTENSION)
        })
        .collect();
    paths.sort();
    for path in paths {
        match read_theme(&path) {
            Ok(theme) if themes.iter().any(|existing| existing.id == theme.id) => {
                problems.push(format!(
                    "{}: theme id {:?} is already taken",
                    path.display(),
                    theme.id
                ));
            }
            Ok(theme) => themes.push(theme),
            Err(error) => problems.push(error.to_string()),
        }
    }

    (themes, problems)
}

/// Reads and validates one theme file, migrating older format versions.
pub fn read_theme(path: &Path) -> Result<Theme> {
    let format_error = |detail: String| Error::Format {
        path: path.display().to_string(),
        detail,
    };
    let text = std::fs::read_to_string(path).map_err(|source| Error::io(path, source))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| format_error(error.to_string()))?;

    let version = value
        .get("format_version")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    if version == 0 || version > THEME_FORMAT_VERSION as u64 {
        return Err(format_error(format!(
            "format_version must be between 1 and {THEME_FORMAT_VERSION}, got {}",
            value
                .get("format_version")
                .map_or("nothing".to_string(), ToString::to_string)
        )));
    }
    // Version 1 is current. A version 2 adds its migration step here.
    let theme: Theme =
        serde_json::from_value(value).map_err(|error| format_error(error.to_string()))?;

    let problems = theme_problems(&theme);
    if !problems.is_empty() {
        return Err(format_error(problems.join("; ")));
    }

    missing_layers::filled(theme)
}

/// Everything wrong with a theme, in plain words. Empty means valid.
pub(crate) fn theme_problems(theme: &Theme) -> Vec<String> {
    let mut problems = Vec::new();
    let is_id = !theme.id.is_empty()
        && theme
            .id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !is_id {
        problems.push(format!(
            "id must be lowercase and dash-separated, got {:?}",
            theme.id
        ));
    }

    let palette = &theme.palette;
    for name in ROLES {
        let value = role(palette, name).unwrap_or_default();
        if !is_hex(&value) {
            problems.push(format!(
                "palette.{name} must look like '#RRGGBB', got {value:?}"
            ));
        }
    }

    if theme.ramp.len() < MIN_RAMP_COLOURS {
        problems.push(format!(
            "ramp needs at least {MIN_RAMP_COLOURS} colours, got {}",
            theme.ramp.len()
        ));
    }
    if theme.categories.len() < MIN_CATEGORY_COLOURS {
        problems.push(format!(
            "categories needs at least {MIN_CATEGORY_COLOURS} colours, got {}",
            theme.categories.len()
        ));
    }
    for (index, value) in theme.ramp.iter().enumerate() {
        check_colour(&format!("ramp[{index}]"), value, &mut problems);
    }
    // Optional, but a water ramp that is there must be usable on its own.
    if !theme.water_ramp.is_empty() && theme.water_ramp.len() < MIN_RAMP_COLOURS {
        problems.push(format!(
            "water_ramp needs at least {MIN_RAMP_COLOURS} colours, got {}",
            theme.water_ramp.len()
        ));
    }
    for (index, value) in theme.water_ramp.iter().enumerate() {
        check_colour(&format!("water_ramp[{index}]"), value, &mut problems);
    }
    for (index, value) in theme.categories.iter().enumerate() {
        check_colour(&format!("categories[{index}]"), value, &mut problems);
    }

    if !is_bundled_font(&theme.typography.family) {
        problems.push(format!(
            "typography.family must be a bundled family (Inter, Source Serif 4, IBM Plex Mono), got {:?}",
            theme.typography.family
        ));
    }

    for (layer, style) in &theme.layers {
        let name = serde_json::to_value(layer)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();
        check_style(&format!("layers.{name}"), style, &mut problems);
    }

    problems
}

fn check_style(path: &str, style: &LayerStyle, problems: &mut Vec<String>) {
    if let Some(fill) = &style.fill {
        check_colour(&format!("{path}.fill"), fill, problems);
    }
    if let Some(stroke) = &style.stroke {
        check_colour(&format!("{path}.stroke"), stroke, problems);
    }
    if !(0.0..=1.0).contains(&style.opacity) {
        problems.push(format!(
            "{path}.opacity must be between 0 and 1, got {}",
            style.opacity
        ));
    }
    let lengths = [style.stroke_mm, style.radius_mm, style.shadow_mm];
    if lengths
        .iter()
        .chain(&style.dash_mm)
        .any(|&length| length < 0.0 || !length.is_finite())
    {
        problems.push(format!("{path} has a negative or invalid length"));
    }
    match &style.pattern {
        Some(Pattern::Hatch {
            spacing_mm, color, ..
        })
        | Some(Pattern::CrossHatch {
            spacing_mm, color, ..
        })
        | Some(Pattern::Dots {
            spacing_mm, color, ..
        }) => {
            check_colour(&format!("{path}.pattern.color"), color, problems);
            if *spacing_mm <= 0.0 {
                problems.push(format!(
                    "{path}.pattern.spacing_mm must be above 0, got {spacing_mm}"
                ));
            }
        }
        None => {}
    }
}

/// A colour is "#RRGGBB" or "@role" with a role the palette has.
fn check_colour(path: &str, value: &str, problems: &mut Vec<String>) {
    let is_valid = match value.strip_prefix('@') {
        Some(name) => ROLES.contains(&name),
        None => is_hex(value),
    };
    if !is_valid {
        problems.push(format!(
            "{path} must be '#RRGGBB' or one of @{}, got {value:?}",
            ROLES.join(", @")
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_theme_is_valid() {
        for theme in builtin_themes() {
            assert_eq!(
                theme_problems(&theme),
                Vec::<String>::new(),
                "theme {}",
                theme.id
            );
        }
    }

    #[test]
    fn builtin_ids_match_their_file_names() {
        for ((name, _), theme) in BUILTIN_THEMES.iter().zip(builtin_themes()) {
            assert_eq!(*name, theme.id);
        }
        for ((name, _), diagram) in BUILTIN_DIAGRAMS.iter().zip(builtin_diagrams()) {
            assert_eq!(*name, diagram.id);
        }
    }

    #[test]
    fn diagrams_of_one_group_sit_together() {
        let groups: Vec<String> = builtin_diagrams().into_iter().map(|d| d.group).collect();
        let mut seen: Vec<&String> = Vec::new();
        for (index, group) in groups.iter().enumerate() {
            let is_new = index == 0 || groups[index - 1] != *group;
            if is_new {
                assert!(!seen.contains(&group), "group {group:?} is split up");
                seen.push(group);
            }
        }
    }

    #[test]
    fn a_diagram_with_variants_takes_its_layers_only_from_them() {
        for diagram in builtin_diagrams() {
            if diagram.variants.is_empty() {
                assert!(!diagram.layers.is_empty(), "{} draws nothing", diagram.id);
                continue;
            }
            assert!(diagram.layers.is_empty(), "{} has layers", diagram.id);
            assert!(diagram.emphasis.is_none(), "{} has an emphasis", diagram.id);
            assert!(diagram.classify.is_none(), "{} classifies", diagram.id);

            let mut ids: Vec<&str> = diagram.variants.iter().map(|v| v.id.as_str()).collect();
            ids.sort_unstable();
            ids.dedup();
            assert_eq!(
                ids.len(),
                diagram.variants.len(),
                "{} repeats a variant",
                diagram.id
            );
            for variant in &diagram.variants {
                assert!(!variant.layers.is_empty(), "{}:{}", diagram.id, variant.id);
                assert!(!variant.id.contains(':'), "{}:{}", diagram.id, variant.id);
            }
        }
    }
}
