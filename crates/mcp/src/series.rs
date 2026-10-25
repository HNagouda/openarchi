//! The whole series export: one file per built-in diagram and per variant,
//! in one sheet's palette and paper, into a folder. The same files, with the
//! same names, as "The whole series" in the app's export dialog.

use std::path::Path;

use openarchi_core::catalog;
use openarchi_core::export::{self, series, DxfLayers, ExportFormat};
use openarchi_core::library::Library;
use openarchi_core::render::RenderInput;
use serde_json::{json, Value};

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
    let diagrams = catalog::builtin_diagrams();
    let files = series::files(&diagrams, &project.name, look, format, folder);

    for file in &files {
        if file.path.exists() && !overwrite {
            return Err(format!(
                "{} already exists; pass overwrite: true to replace it",
                file.path.display()
            ));
        }
    }

    let mut written = Vec::new();
    for file in &files {
        let input = RenderInput {
            scene,
            diagram: file.diagram,
            theme: &theme,
            look: &file.look,
            pins: &project.pins,
            highlights: &project.highlights,
        };
        export::export_with_dxf_layers(&input, format, dpi, dxf_layers, &file.path)
            .map_err(text)?;
        written.push(file.path.display().to_string());
    }

    Ok(json!({ "files": written, "format": format }))
}
