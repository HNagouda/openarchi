//! The whole series export: one file per built-in diagram and per variant,
//! in one sheet's palette and paper, into a folder. The same files, with the
//! same names, as "The whole series" in the app's export dialog.

use std::path::{Path, PathBuf};

use openarchi_core::catalog;
use openarchi_core::export::{self, DxfLayers, ExportFormat};
use openarchi_core::library::Library;
use openarchi_core::project::Look;
use openarchi_core::render::RenderInput;
use serde_json::{json, Value};

use crate::laser::slug;
use crate::resolve::find_theme;
use crate::server::text;

/// Writes every drawing of every diagram for sheet `tab` into `folder`, and
/// returns the paths. Checks every file name first, so nothing is written
/// when one would replace a file without `overwrite`.
#[allow(clippy::too_many_arguments)]
pub fn export_series(
    library: &Library,
    id: &str,
    tab: usize,
    format: ExportFormat,
    dpi: u32,
    dxf_layers: DxfLayers,
    folder: &Path,
    overwrite: bool,
) -> Result<Value, String> {
    let project = library.load(id).map_err(text)?;
    let Some(scene) = &project.scene else {
        return Err(format!(
            "project '{id}' has no site data yet; call fetch_site first"
        ));
    };
    let look = project.sheet(tab).map_err(text)?;
    let theme = find_theme(&library.themes_dir(), &look.theme_id)?;
    let stem = slug(&project.name);
    let diagrams = catalog::builtin_diagrams();

    let mut drawings: Vec<(PathBuf, Look)> = Vec::new();
    for diagram in &diagrams {
        for (variant, _) in diagram.drawings() {
            let drawing = match &variant {
                Some(variant) => format!("{}-{variant}", diagram.id),
                None => diagram.id.clone(),
            };
            let path = folder.join(format!("{stem}-{drawing}.{}", format.extension()));
            if path.exists() && !overwrite {
                return Err(format!(
                    "{} already exists; pass overwrite: true to replace it",
                    path.display()
                ));
            }
            let diagram_look = Look {
                diagram_id: diagram.id.clone(),
                variant,
                ..look.clone()
            };
            drawings.push((path, diagram_look));
        }
    }

    let mut written = Vec::new();
    for (path, diagram_look) in &drawings {
        let diagram = diagrams
            .iter()
            .find(|diagram| diagram.id == diagram_look.diagram_id)
            .expect("the diagram this loop came from");
        let input = RenderInput {
            scene,
            diagram,
            theme: &theme,
            look: diagram_look,
            pins: &project.pins,
            highlights: &project.highlights,
        };
        export::export_with_dxf_layers(&input, format, dpi, dxf_layers, path).map_err(text)?;
        written.push(path.display().to_string());
    }

    Ok(json!({ "files": written, "format": format }))
}
