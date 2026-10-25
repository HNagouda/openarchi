//! The whole series: one file per diagram, and per variant, in one sheet's
//! palette and paper, named after the project. The app and the MCP server both
//! plan it here, so an assistant's series has the same files with the same
//! names as the app's.

use std::path::{Path, PathBuf};

use crate::diagram::Diagram;
use crate::export::ExportFormat;
use crate::project::Look;

/// One file of the series: where it goes, the diagram it draws and the look to draw it in.
pub struct SeriesFile<'a> {
    pub path: PathBuf,
    pub diagram: &'a Diagram,
    pub look: Look,
}

/// "Westermarkt, Amsterdam" -> "westermarkt-amsterdam".
pub fn slug(name: &str) -> String {
    let mut slug = String::new();
    for letter in name.to_lowercase().chars() {
        if letter.is_alphanumeric() {
            slug.push(letter);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }

    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "site".to_string()
    } else {
        slug
    }
}

/// The file name an export of a diagram, or one of its variants, gets:
/// "westermarkt-figure-ground.pdf", "westermarkt-air-quality-pm10.pdf".
pub fn file_name(
    project_name: &str,
    diagram_id: &str,
    variant: Option<&str>,
    format: ExportFormat,
) -> String {
    let drawing = match variant {
        Some(variant) => format!("{diagram_id}-{variant}"),
        None => diagram_id.to_string(),
    };
    format!("{}-{drawing}.{}", slug(project_name), format.extension())
}

/// Every file of the series in `folder`, each drawn in `look` with its own
/// diagram and variant. A diagram with variants gets one file per variant: the
/// series is everything the site shows, and a variant left out would be a
/// sheet missing without a word.
pub fn files<'a>(
    diagrams: &'a [Diagram],
    project_name: &str,
    look: &Look,
    format: ExportFormat,
    folder: &Path,
) -> Vec<SeriesFile<'a>> {
    let mut files = Vec::new();
    for diagram in diagrams {
        for (variant, _) in diagram.drawings() {
            let name = file_name(project_name, &diagram.id, variant.as_deref(), format);
            files.push(SeriesFile {
                path: folder.join(name),
                diagram,
                look: Look {
                    diagram_id: diagram.id.clone(),
                    variant,
                    ..look.clone()
                },
            });
        }
    }

    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_keeps_letters_and_digits_and_joins_the_rest_with_one_dash() {
        assert_eq!(slug("Westermarkt, Amsterdam"), "westermarkt-amsterdam");
        assert_eq!(slug("  Plot 3 / B  "), "plot-3-b");
        assert_eq!(slug("!!!"), "site");
    }

    #[test]
    fn a_variant_is_named_after_its_diagram() {
        assert_eq!(
            file_name(
                "Westermarkt",
                "air-quality",
                Some("pm10"),
                ExportFormat::Pdf
            ),
            "westermarkt-air-quality-pm10.pdf"
        );
        assert_eq!(
            file_name("Westermarkt", "figure-ground", None, ExportFormat::Svg),
            "westermarkt-figure-ground.svg"
        );
    }
}
