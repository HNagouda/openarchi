//! Turns the ids a project stores into the diagram and theme the renderer needs.

use std::path::Path;

use openarchi_core::catalog;
use openarchi_core::diagram::Diagram;
use openarchi_core::library::Library;
use openarchi_core::project::Look;
use openarchi_core::render::RenderInput;
use openarchi_core::theme::Theme;

/// A built-in diagram, following the ids of diagrams folded into another.
pub fn find_diagram(id: &str) -> Result<Diagram, String> {
    let (id, _) = catalog::current_id(id, None);
    catalog::builtin_diagrams()
        .into_iter()
        .find(|diagram| diagram.id == id)
        .ok_or_else(|| format!("no diagram with id '{id}'; list_diagrams shows the ids"))
}

/// An explode plate: a diagram id, or "diagram:variant" for one variant.
pub fn check_plate(plate: &str) -> Result<(), String> {
    match catalog::find_plate(&catalog::builtin_diagrams(), plate) {
        Some(_) => Ok(()),
        None => Err(format!(
            "no diagram for plate '{plate}'; use an id from list_diagrams, or 'id:variant' for one variant"
        )),
    }
}

/// The diagram and variant set_look ends on. The variant stays while the
/// diagram does; a new diagram without a variant starts on its first. An old
/// id like "air-quality-pm10" lands on the variant it became.
pub fn chosen_diagram(
    look: &Look,
    diagram_id: Option<String>,
    variant: Option<String>,
) -> Result<(String, Option<String>), String> {
    let id = diagram_id.unwrap_or_else(|| look.diagram_id.clone());
    let variant = variant.or_else(|| {
        if id == look.diagram_id {
            look.variant.clone()
        } else {
            None
        }
    });
    let (id, variant) = catalog::current_id(&id, variant.as_deref());
    let diagram = find_diagram(&id)?;

    let ids: Vec<&str> = diagram.variants.iter().map(|v| v.id.as_str()).collect();
    match &variant {
        Some(chosen) if ids.is_empty() => Err(format!(
            "diagram '{id}' has no variants, so variant '{chosen}' does not apply"
        )),
        Some(chosen) if !ids.contains(&chosen.as_str()) => Err(format!(
            "diagram '{id}' has no variant '{chosen}'; its variants are {}",
            ids.join(", ")
        )),
        _ => Ok((id, variant)),
    }
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
