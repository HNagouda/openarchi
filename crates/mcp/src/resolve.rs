//! Turns the ids a project stores into the diagram and theme the renderer needs.

use std::path::Path;

use openarchi_core::catalog;
use openarchi_core::diagram::Diagram;
use openarchi_core::library::Library;
use openarchi_core::render::RenderInput;
use openarchi_core::theme::Theme;

pub fn find_diagram(id: &str) -> Result<Diagram, String> {
    catalog::builtin_diagrams()
        .into_iter()
        .find(|diagram| diagram.id == id)
        .ok_or_else(|| format!("no diagram with id '{id}'; list_diagrams shows the ids"))
}

/// Built-in or user theme. Broken user theme files are skipped, as the app does.
pub fn find_theme(themes_dir: &Path, id: &str) -> Result<Theme, String> {
    let (themes, _problems) = catalog::all_themes(themes_dir);
    themes
        .into_iter()
        .find(|theme| theme.id == id)
        .ok_or_else(|| format!("no theme with id '{id}'; list_themes shows the ids"))
}

/// Loads the project, resolves its look and hands the renderer input to `work`.
pub fn with_render_input<T>(
    library: &Library,
    id: &str,
    work: impl FnOnce(&RenderInput) -> openarchi_core::Result<T>,
) -> Result<T, String> {
    let project = library.load(id).map_err(|e| e.to_string())?;
    let Some(scene) = &project.scene else {
        return Err(format!(
            "project '{id}' has no site data yet; call fetch_site first"
        ));
    };
    let diagram = find_diagram(&project.look.diagram_id)?;
    let theme = find_theme(&library.themes_dir(), &project.look.theme_id)?;

    let input = RenderInput {
        scene,
        diagram: &diagram,
        theme: &theme,
        look: &project.look,
    };
    work(&input).map_err(|e| e.to_string())
}
