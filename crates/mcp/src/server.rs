//! The MCP server: the tool router every tool module joins, the project
//! tools, and the helpers they all share. Each tool is a thin shell over the
//! core crate: take the input, run the core call on tokio's blocking pool, and
//! reply with JSON or a plain error the assistant can act on.

use std::any::Any;
use std::path::PathBuf;

use openarchi_core::area::Area;
use openarchi_core::library::Library;
use openarchi_core::project::Project;
use openarchi_core::scene::Scene;
use openarchi_core::{geocode, site};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use serde::Serialize;
use serde_json::{json, Value};

use crate::inputs::{CreateProject, ProjectId, SearchPlaces};
use crate::model::model_json;
use crate::summary::summarize;

const DEFAULT_PLACES: u32 = 5;
const MAX_PLACES: u32 = 20;

/// What MCP clients show as the server's name.
const SERVER_NAME: &str = "openarchi";

const INSTRUCTIONS: &str = "openarchi turns a site in the Netherlands into \
styled site-analysis diagrams (figure-ground, building age, heights, uses, green \
and water). A typical session: search_places to find the site, create_project \
with an area around it, and fetch_site to download the site data. Then style the \
sheet: list_diagrams and list_themes, set_look to choose and tune the look, \
edit_text to style, move or add text, create_theme or import_palette for a \
palette of your own. add_sheet adds sheet tabs, each with its own look, and \
list_sheets lists them. add_pin marks landmarks and add_highlight marks the plot \
or another area on every sheet; open_highlight_as_sheet gives a sheet of just \
that area, and show_on_sheet shows or hides the marks per sheet. Each sheet also \
has set_section for its section line, set_view_point for the eye of the View \
diagram and set_wind_season for the Wind diagram. The numbers come from \
get_ratios (land use, coverage, floor area ratio), get_environment (noise, air, \
flood, population), get_neighbourhood (the 15 minute neighbourhood) and \
get_analysis (location, climate, design notes). render_preview shows the sheet, \
export writes it or the whole series to files, export_laser writes laser cutting \
files for a physical site model, and list_reports and export_report give the \
multi-page reports the user laid out in the app. Projects live in the same \
library as the openarchi app, so the user sees every project you make there. \
delete_project moves a project to Recently Deleted and restore_project brings it \
back.";

#[derive(Clone)]
pub struct OpenArchi {
    library_root: PathBuf,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl OpenArchi {
    pub fn new(library_root: PathBuf) -> Self {
        OpenArchi {
            library_root,
            tool_router: Self::tool_router()
                + Self::style_tools()
                + Self::figure_tools()
                + Self::export_tools()
                + Self::sheet_tools()
                + Self::laser_tools()
                + Self::section_tools()
                + Self::highlight_tools()
                + Self::view_point_tools()
                + Self::wind_tools()
                + Self::model_tools(),
        }
    }

    #[tool(
        description = "Find addresses and places in the Netherlands. Returns label, kind and location (lon, lat) for each match, best first. Use a location as the centre of the area in create_project."
    )]
    async fn search_places(&self, Parameters(input): Parameters<SearchPlaces>) -> CallToolResult {
        let limit = input.limit.unwrap_or(DEFAULT_PLACES).clamp(1, MAX_PLACES) as usize;
        reply(blocking(move || geocode::search(&input.query, limit).map_err(text)).await)
    }

    #[tool(
        description = "List the user's projects, most recently changed first, with id, name, modified time and whether the site has been fetched. A project whose file cannot be read has `error` set."
    )]
    async fn list_projects(&self) -> CallToolResult {
        reply(
            self.with_library(|library| library.list().map_err(text))
                .await,
        )
    }

    #[tool(
        description = "Create and save a new project for a site. Returns the project id and its starting look. The site data is not downloaded yet: call fetch_site next."
    )]
    async fn create_project(&self, Parameters(input): Parameters<CreateProject>) -> CallToolResult {
        let area = Area::from(input.area);
        let result = self
            .with_library(move |library| {
                let project = library.create(&input.name, area).map_err(text)?;
                Ok(json!({
                    "id": project.id,
                    "name": project.name,
                    "look": project.look,
                    "next": "Call fetch_site with this id to download the site data.",
                }))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Get a project: name, area, its look (diagram, theme, sheet, layer overrides) and, once fetched, a site summary with feature counts per layer, building heights (min, median, max), construction years and the most common building uses."
    )]
    async fn get_project(&self, Parameters(input): Parameters<ProjectId>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let project = library.load(&input.id).map_err(text)?;
                Ok(overview(&project))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Download the site data for a project (buildings with age, height and use, roads, paths, water, green, trees, parcels, terrain) and save it into the project. Takes from a few seconds to a minute. Returns the site summary; `warnings` lists sources that failed while the rest of the site is still usable. Running it again reuses data downloaded in the last 30 days, so a nudged or resized area is quick; the previous version stays in the project's backups."
    )]
    async fn fetch_site(&self, Parameters(input): Parameters<ProjectId>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let area = library.load(&input.id).map_err(text)?.area;
                let scene =
                    site::build_scene(&area, &library.tile_cache(), &|_| {}).map_err(text)?;
                // Load again after the slow fetch, so a look the user changed
                // in the app meanwhile is kept.
                let mut project = library.load(&input.id).map_err(text)?;
                let summary = summarize(&scene);
                project.scene = Some(scene);
                library.save(&mut project).map_err(text)?;
                Ok(summary)
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Move a project to Recently Deleted. Nothing is lost: restore_project brings it back with its backups."
    )]
    async fn delete_project(&self, Parameters(input): Parameters<ProjectId>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                library.delete(&input.id).map_err(text)?;
                Ok(json!({
                    "id": input.id,
                    "message": "Moved to Recently Deleted. restore_project brings it back.",
                }))
            })
            .await;
        reply(result)
    }

    #[tool(description = "Bring a project back from Recently Deleted.")]
    async fn restore_project(&self, Parameters(input): Parameters<ProjectId>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                library.restore(&input.id).map_err(text)?;
                Ok(json!({ "id": input.id, "message": "Restored." }))
            })
            .await;
        reply(result)
    }

    /// Opens the library on the blocking pool and runs `work` with it.
    pub(crate) async fn with_library<T: Send + 'static>(
        &self,
        work: impl FnOnce(&Library) -> Result<T, String> + Send + 'static,
    ) -> Result<T, String> {
        let root = self.library_root.clone();
        blocking(move || {
            let library = Library::at(&root).map_err(text)?;
            work(&library)
        })
        .await
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for OpenArchi {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(SERVER_NAME, env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }
}

/// Core calls block on files and the network, so they run on tokio's blocking
/// pool. A panic there comes back as an error instead of taking the server down.
pub(crate) async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    match tokio::task::spawn_blocking(work).await {
        Ok(result) => result,
        Err(error) if error.is_panic() => Err(panic_message(error.into_panic())),
        Err(error) => Err(format!("openarchi stopped the request: {error}")),
    }
}

fn panic_message(payload: Box<dyn Any + Send>) -> String {
    let detail = match payload.downcast::<String>() {
        Ok(message) => *message,
        Err(payload) => payload
            .downcast_ref::<&str>()
            .map(|message| message.to_string())
            .unwrap_or_default(),
    };

    format!("openarchi hit an internal error: {detail}")
}

/// The project's fetched site, or the error every tool that needs one gives.
pub(crate) fn fetched_scene(project: &Project) -> Result<&Scene, String> {
    project.scene.as_ref().ok_or_else(|| {
        format!(
            "project '{}' has no site data yet; call fetch_site first",
            project.id
        )
    })
}

fn overview(project: &Project) -> Value {
    json!({
        "id": project.id,
        "name": project.name,
        "created": project.created,
        "modified": project.modified,
        "area": project.area,
        "look": project.look,
        "site": project.scene.as_ref().map(summarize),
        "model": project.model.as_ref().map(|model| model_json(model, project.scene.as_ref())),
    })
}

/// Pretty JSON on success, the plain message on failure. Tool failures are
/// results with `is_error` set rather than protocol errors, because clients
/// show those to the assistant, which can then fix its call.
pub(crate) fn reply<T: Serialize>(result: Result<T, String>) -> CallToolResult {
    let json = result.and_then(|value| serde_json::to_string_pretty(&value).map_err(text));
    match json {
        Ok(json) => CallToolResult::success(vec![ContentBlock::text(json)]),
        Err(message) => failure(message),
    }
}

/// An absolute path ending in `.extension`, in a folder that exists.
pub(crate) fn checked_path(typed: &str, extension: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(typed);
    if !path.is_absolute() {
        return Err(format!("path must be absolute, got \"{typed}\""));
    }
    let has_extension = path
        .extension()
        .is_some_and(|found| found.eq_ignore_ascii_case(extension));
    if !has_extension {
        return Err(format!("path must end in .{extension}, got \"{typed}\""));
    }
    if !path.parent().is_some_and(|folder| folder.is_dir()) {
        return Err(format!("the folder for {} does not exist", path.display()));
    }

    Ok(path)
}

/// An absolute path to a folder that exists, for tools that write several files.
pub(crate) fn existing_folder(typed: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(typed);
    if !path.is_absolute() || !path.is_dir() {
        return Err(format!(
            "folder must be an absolute path to an existing folder, got \"{typed}\""
        ));
    }

    Ok(path)
}

pub(crate) fn failure(message: String) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message)])
}

pub(crate) fn text(error: impl std::fmt::Display) -> String {
    error.to_string()
}
