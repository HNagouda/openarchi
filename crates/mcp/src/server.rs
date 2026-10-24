//! The MCP tools. Each one is a thin shell over the core crate: take the input,
//! run the core call on tokio's blocking pool, and reply with JSON or a plain
//! error the assistant can act on.

use std::any::Any;
use std::path::PathBuf;

use base64::prelude::{Engine, BASE64_STANDARD};
use openarchi_core::area::Area;
use openarchi_core::export::{self, ExportFormat};
use openarchi_core::library::Library;
use openarchi_core::project::Project;
use openarchi_core::{catalog, geocode, ratios, site, walk};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use serde::Serialize;
use serde_json::{json, Value};

use crate::inputs::{
    CreateProject, CreateTheme, EditText, Export, GetRatios, ListThemes, ProjectId, RenderPreview,
    SearchPlaces, SetLook,
};
use crate::look::updated_look;
use crate::resolve::{find_diagram, find_theme, with_render_input};
use crate::summary::summarize;
use crate::text::edited_sheet;
use crate::user_themes::save_user_theme;

const DEFAULT_PLACES: u32 = 5;
const MAX_PLACES: u32 = 20;

const DEFAULT_PREVIEW_WIDTH_PX: u32 = 1200;
/// Narrower than this the labels are unreadable.
const MIN_PREVIEW_WIDTH_PX: u32 = 200;
/// Wider than this the image is more than an assistant can take in at once.
const MAX_PREVIEW_WIDTH_PX: u32 = 4000;

const DEFAULT_EXPORT_DPI: u32 = 300;

/// The message a `todo!()` panics with, which marks a part still being built.
const NOT_BUILT_PANIC: &str = "not yet implemented";

/// What MCP clients show as the server's name.
const SERVER_NAME: &str = "openarchi";

const INSTRUCTIONS: &str = "openarchi turns a site in the Netherlands into styled \
site-analysis diagrams (figure-ground, building age, heights, uses, green and water). \
A typical session: search_places to find the site, create_project with an area around \
it, fetch_site to download the site data, list_diagrams and list_themes, set_look to \
choose and tune the style, edit_text to style, move or add text on the sheet, \
render_preview to look at the sheet, get_ratios for the \
numbers behind it (land use, building use, coverage, floor area ratio), and export to \
write a file. Projects live in the same library as the openarchi app, so the user sees every \
project you make there. delete_project moves a project to Recently Deleted and \
restore_project brings it back.";

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
            tool_router: Self::tool_router(),
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
        description = "List the diagrams a project can show. Each diagram answers one question about the site (what is built, how old, how tall, what use) and lists the layers it draws. Pass an id to set_look as diagram_id."
    )]
    async fn list_diagrams(&self) -> CallToolResult {
        reply(blocking(|| Ok(catalog::builtin_diagrams())).await)
    }

    #[tool(
        description = "List the themes: built-in ones first, then the user's own. A theme is how diagrams look (palette, line weights, patterns, type). By default returns id, name, description and palette; full=true returns whole themes, the shape create_theme takes. `problems` lists user theme files that could not be read."
    )]
    async fn list_themes(&self, Parameters(input): Parameters<ListThemes>) -> CallToolResult {
        let is_full = input.full.unwrap_or(false);
        let result = self
            .with_library(move |library| {
                let (themes, problems) = catalog::all_themes(&library.themes_dir());
                let themes: Vec<Value> = themes
                    .into_iter()
                    .map(|theme| {
                        if is_full {
                            json!(theme)
                        } else {
                            json!({
                                "id": theme.id,
                                "name": theme.name,
                                "description": theme.description,
                                "palette": theme.palette,
                            })
                        }
                    })
                    .collect();
                Ok(json!({ "themes": themes, "problems": problems }))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Change how a project looks and save it. Pass only what should change: any of diagram_id, theme_id, sheet fields, per-layer overrides, the view (plan, axonometric with buildings standing up, or explode with several diagrams stacked as plates) and the sun's date and time for the sun path and shadow diagrams. Returns the whole new look. Earlier versions stay in the project's backups."
    )]
    async fn set_look(&self, Parameters(input): Parameters<SetLook>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                if let Some(diagram_id) = &input.diagram_id {
                    find_diagram(diagram_id)?;
                }
                if let Some(theme_id) = &input.theme_id {
                    find_theme(&library.themes_dir(), theme_id)?;
                }
                let plates = input.view.as_ref().and_then(|view| view.plates.as_ref());
                for plate in plates.into_iter().flatten() {
                    find_diagram(plate)?;
                }

                let mut project = library.load(&input.id).map_err(text)?;
                project.look = updated_look(
                    &project.look,
                    input.diagram_id,
                    input.theme_id,
                    input.sheet,
                    input.overrides,
                    input.view,
                    input.sun,
                )?;
                library.save(&mut project).map_err(text)?;
                Ok(project.look)
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Add, change, move or delete text on the sheet and save it. text_id is \"title\", \"subtitle\", an existing text box id, or \"new\" to add a text box. Set the words, the style (family, bold, italic, size_pt, align, colour, opacity) and the position in millimetres from the paper's top-left corner. Returns the id of the text, the paper size in millimetres and every text on the sheet. Check the result with render_preview."
    )]
    async fn edit_text(&self, Parameters(input): Parameters<EditText>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                let (sheet, text_id) = edited_sheet(&project.look.sheet, input)?;
                project.look.sheet = sheet;
                library.save(&mut project).map_err(text)?;

                let sheet = &project.look.sheet;
                let (width, height) = sheet.size_mm();
                Ok(json!({
                    "text_id": text_id,
                    "paper_mm": { "width": width, "height": height },
                    "title": { "text": sheet.title, "style": sheet.title_style, "position": sheet.title_position },
                    "subtitle": { "text": sheet.subtitle, "style": sheet.subtitle_style, "position": sheet.subtitle_position },
                    "texts": sheet.texts,
                }))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Render the project's sheet as a PNG image so you can see it: the same drawing the app shows and export writes. The site must have been fetched first."
    )]
    async fn render_preview(&self, Parameters(input): Parameters<RenderPreview>) -> CallToolResult {
        let width_px = input
            .width_px
            .unwrap_or(DEFAULT_PREVIEW_WIDTH_PX)
            .clamp(MIN_PREVIEW_WIDTH_PX, MAX_PREVIEW_WIDTH_PX);
        let result = self
            .with_library(move |library| {
                with_render_input(library, &input.id, |render| {
                    export::render_png(render, width_px)
                })
            })
            .await;

        match result {
            Ok(png) => CallToolResult::success(vec![ContentBlock::image(
                BASE64_STANDARD.encode(png),
                "image/png",
            )]),
            Err(message) => CallToolResult::error(vec![ContentBlock::text(message)]),
        }
    }

    #[tool(
        description = "Get the site's ratios: how it divides up, as groups of shares that each add up to 100 %. Land use and transport are by area in m² inside the site; building use, floors, height, year built and footprint by number of buildings; building coverage and floor area ratio by number of parcels; green and water by area. Each row has label, value, share (0 to 1) and fill, the theme colour the diagrams use for it. Also returns the site area, building count, and overall coverage and floor area ratio. Floors are estimated from roof height at 3 m per floor. The site must have been fetched first."
    )]
    async fn get_ratios(&self, Parameters(input): Parameters<GetRatios>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let project = library.load(&input.id).map_err(text)?;
                let Some(scene) = &project.scene else {
                    return Err(format!(
                        "project '{}' has no site data yet; call fetch_site first",
                        input.id
                    ));
                };
                let theme_id = input.theme_id.unwrap_or(project.look.theme_id.clone());
                let theme = find_theme(&library.themes_dir(), &theme_id)?;
                ratios::compute(scene, &theme).map_err(text)
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Get the site's 15 minute neighbourhood: for walks of 5, 10 and 15 minutes from the site centre at 1.4 m/s over the OpenStreetMap footpath network, how many businesses, facilities and transit stops lie within reach, by group (food_drink, shop, education, health, tram, bus and so on). Counts are cumulative, so the 10 minute counts include the 5 minute ones. The site must have been fetched first, with OpenStreetMap reachable."
    )]
    async fn get_neighbourhood(&self, Parameters(input): Parameters<ProjectId>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let project = library.load(&input.id).map_err(text)?;
                let Some(scene) = &project.scene else {
                    return Err(format!(
                        "project '{}' has no site data yet; call fetch_site first",
                        input.id
                    ));
                };
                walk::neighbourhood(scene).ok_or_else(|| {
                    format!(
                        "project '{}' has no walk data: OpenStreetMap was not reached when the site was fetched; call fetch_site again",
                        input.id
                    )
                })
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Write the project's sheet to a file: svg or pdf for print, png for slides, dxf for CAD. Refuses to replace an existing file unless overwrite is true. The site must have been fetched first."
    )]
    async fn export(&self, Parameters(input): Parameters<Export>) -> CallToolResult {
        let path = PathBuf::from(&input.path);
        if !path.is_absolute() {
            return failure(format!("path must be absolute, got \"{}\"", input.path));
        }
        if path.exists() && !input.overwrite.unwrap_or(false) {
            return failure(format!(
                "{} already exists; pass overwrite: true to replace it",
                path.display()
            ));
        }
        if !path.parent().is_some_and(|folder| folder.is_dir()) {
            return failure(format!("the folder for {} does not exist", path.display()));
        }

        let format = ExportFormat::from(input.format);
        let dpi = input.dpi.unwrap_or(DEFAULT_EXPORT_DPI);
        let result = self
            .with_library(move |library| {
                with_render_input(library, &input.id, |render| {
                    export::export(render, format, dpi, &path)
                })?;
                Ok(json!({ "path": path, "format": format }))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Save a new theme to the user's themes, where the app picks it up too. Get a starting point from list_themes with full=true, change the id, name and colours, and pass the whole theme. It is checked with the same rules the app uses. Saving an existing user theme id replaces it and moves the old one to Recently Deleted; built-in theme ids are refused."
    )]
    async fn create_theme(&self, Parameters(input): Parameters<CreateTheme>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let theme = save_user_theme(library, input.theme_json)?;
                Ok(json!({
                    "id": theme.id,
                    "name": theme.name,
                    "next": "Pass this id to set_look as theme_id to use it.",
                }))
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
    async fn with_library<T: Send + 'static>(
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
/// pool. A panic there, such as a `todo!()` in a part still being built, comes
/// back as an error instead of taking the server down.
async fn blocking<T: Send + 'static>(
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

    if detail.starts_with(NOT_BUILT_PANIC) {
        format!("this part of openarchi is not built yet ({detail})")
    } else {
        format!("openarchi hit an internal error: {detail}")
    }
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
    })
}

/// Pretty JSON on success, the plain message on failure. Tool failures are
/// results with `is_error` set rather than protocol errors, because clients
/// show those to the assistant, which can then fix its call.
fn reply<T: Serialize>(result: Result<T, String>) -> CallToolResult {
    let json = result.and_then(|value| serde_json::to_string_pretty(&value).map_err(text));
    match json {
        Ok(json) => CallToolResult::success(vec![ContentBlock::text(json)]),
        Err(message) => failure(message),
    }
}

fn failure(message: String) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message)])
}

fn text(error: impl std::fmt::Display) -> String {
    error.to_string()
}
