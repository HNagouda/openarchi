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
use openarchi_core::{
    analysis, catalog, environment, geocode, ratios, render, selection, site, walk,
};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use serde::Serialize;
use serde_json::{json, Value};

use crate::inputs::{
    CreateProject, CreateTheme, EditText, Export, GetAnalysis, GetEnvironment, GetRatios,
    ListThemes, ProjectId, RenderPreview, SearchPlaces, SetLook,
};
use crate::look::updated_look;
use crate::palettes::{self, ImportPalette};
use crate::reports::{self, ExportReport};
use crate::resolve::{check_plate, chosen_diagram, find_theme, with_render_input};
use crate::series::export_series;
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
add_sheet for more sheet tabs (each with its own look), add_highlight to mark the plot \
or another area, open_highlight_as_sheet for a sheet of just that area, add_pin to mark landmarks on every sheet, render_preview to look at the sheet, get_environment for noise, air, flood and population, get_ratios for the \
numbers behind it (land use, building use, coverage, floor area ratio), get_analysis \
for the location, climate and design notes, export to \
write a file, export_laser for laser cutting files of a physical site model, and list_reports and export_report for the multi-page reports the \
user laid out in the app. Projects live in the same library as the openarchi app, so the user sees every \
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
            tool_router: Self::tool_router()
                + Self::sheet_tools()
                + Self::laser_tools()
                + Self::section_tools()
                + Self::highlight_tools(),
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
        description = "List the diagrams a project can show, in the order and groups the app shows them. Each diagram answers one question about the site (what is built, how old, how tall, what use) and lists the layers it draws. Some diagrams have `variants` that colour another layer or measure for the same question, such as NO2, PM10 and PM2.5 for air quality; a diagram with variants takes its layers and classes from the chosen one, the first by default. Pass an id to set_look as diagram_id, and a variant id as variant."
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
        description = "Change how a project looks and save it. Pass only what should change: any of diagram_id and its variant, theme_id, sheet fields, per-layer overrides, the view (plan, axonometric with buildings standing up, or explode with several diagrams stacked as plates) and the sun's date and time for the sun path and shadow diagrams. Returns the whole new look. Earlier versions stay in the project's backups."
    )]
    async fn set_look(&self, Parameters(input): Parameters<SetLook>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                if let Some(theme_id) = &input.theme_id {
                    find_theme(&library.themes_dir(), theme_id)?;
                }
                let plates = input.view.as_ref().and_then(|view| view.plates.as_ref());
                for plate in plates.into_iter().flatten() {
                    check_plate(plate)?;
                }

                let mut project = library.load(&input.id).map_err(text)?;
                let look = project.sheet_mut(input.tab.unwrap_or(1)).map_err(text)?;
                let diagram = chosen_diagram(look, input.diagram_id, input.variant)?;
                *look = updated_look(
                    look,
                    Some(diagram),
                    input.theme_id,
                    input.sheet,
                    input.overrides,
                    input.view,
                    input.sun,
                )?;
                let look = look.clone();
                library.save(&mut project).map_err(text)?;
                Ok(look)
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
                let look = project.sheet_mut(input.tab.unwrap_or(1)).map_err(text)?;
                let (sheet, text_id) = edited_sheet(&look.sheet, input)?;
                look.sheet = sheet.clone();
                library.save(&mut project).map_err(text)?;

                let sheet = &sheet;
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
                with_render_input(library, &input.id, input.tab.unwrap_or(1), |render| {
                    let missing = render::missing_data(render)?;
                    Ok((export::render_png(render, width_px)?, missing))
                })
            })
            .await;

        match result {
            // The same notes the app shows over the sheet, so an assistant knows why a diagram looks empty.
            Ok((png, missing)) => {
                let mut content = vec![ContentBlock::image(
                    BASE64_STANDARD.encode(png),
                    "image/png",
                )];
                content.extend(missing.into_iter().map(ContentBlock::text));
                CallToolResult::success(content)
            }
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
                let look = project.sheet(input.tab.unwrap_or(1)).map_err(text)?;
                let theme_id = input.theme_id.unwrap_or(look.theme_id.clone());
                let theme = find_theme(&library.themes_dir(), &theme_id)?;
                let scene =
                    selection::framed_or_whole(scene, look.selection.as_ref()).map_err(text)?;
                ratios::compute(&scene, &theme).map_err(text)
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Get the site's 15 minute neighbourhood: for walks of 5, 10 and 15 minutes from the site centre at 1.4 m/s over the OpenStreetMap footpath network, how many businesses, facilities, car and bicycle parking places and transit stops lie within reach, by group (food_drink, shop, education, health, bicycle_parking, tram, bus and so on). Parking is counted apart from the facilities. Counts are cumulative, so the 10 minute counts include the 5 minute ones. The site must have been fetched first, with OpenStreetMap reachable."
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
        description = "Get the site analysis: location (address, municipality, province, coordinates), a site summary (area, buildings, coverage, floor area ratio, mean and tallest height, median year built, most common use, shares of green, water and paved ground, and facilities, shops and transit stops within a walk when the walk was fetched), the climate from a PVGIS typical year (monthly temperature, humidity, global and direct sun, outdoor comfort as the UTCI (Universal Thermal Climate Index) with daytime hours per stress category for a person in the sun and in the shade, named in `comfort_categories` and explained in `comfort_method`, wind roses by season, direct sun by direction), and 30 years of monthly rain from the nearest KNMI station. Each section comes with design notes: short rule-based advice for the architect. Climate and rain need the network the first time; if one cannot be fetched it is left out and named in `warnings`. With `tab`, a sheet opened from a highlight gives the site summary of that smaller area, with `summary.is_sheet_area` true; location, climate and rain stay on the whole site. The site must have been fetched first."
    )]
    async fn get_analysis(&self, Parameters(input): Parameters<GetAnalysis>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let project = library.load(&input.id).map_err(text)?;
                let Some(scene) = &project.scene else {
                    return Err(format!(
                        "project '{}' has no site data yet; call fetch_site first",
                        input.id
                    ));
                };
                let look = project.sheet(input.tab.unwrap_or(1)).map_err(text)?;
                let theme = find_theme(&library.themes_dir(), &look.theme_id)?;
                let sheet_area = match &look.selection {
                    Some(selection) => Some(selection::framed(scene, selection).map_err(text)?),
                    None => None,
                };
                analysis::report(scene, sheet_area.as_ref(), &theme, &library.tile_cache())
                    .map_err(text)
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Get the site's environment in numbers: road traffic noise (mean and highest Lden in dB) and all noise sources together, annual mean NO2, PM10 and PM2.5 in µg/m³, the deepest flood water and the share of the site it reaches in LIWO's extremely-small-chance scenario (under 1 in 30,000 a year), the steepest slope in percent, CBS residents and households (each 100 m square counted by its share inside the site, plus how many squares CBS keeps secret), and BAG dwellings and floor area. Each source's year is included. A figure is null when its source was not reached. With `tab`, a sheet opened from a highlight gives the figures of that smaller area; `whole_site` lists the layers whose figures still describe the whole site there (slope always, and noise, air and flood for sites fetched before sheet areas measured their own). The site must have been fetched first."
    )]
    async fn get_environment(
        &self,
        Parameters(input): Parameters<GetEnvironment>,
    ) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let project = library.load(&input.id).map_err(text)?;
                let Some(scene) = &project.scene else {
                    return Err(format!(
                        "project '{}' has no site data yet; call fetch_site first",
                        input.id
                    ));
                };
                let look = project.sheet(input.tab.unwrap_or(1)).map_err(text)?;
                let scene =
                    selection::framed_or_whole(scene, look.selection.as_ref()).map_err(text)?;
                Ok(environment::summarise(&scene))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Write the project's sheet to a file: svg or pdf for print, png for slides, dxf for CAD. A plan's DXF is in RD New metres with a layer per data layer, per legend class of the diagram, for the diagram's computed layers (city blocks, hours of sun), highlights (solid, dashed, dotted), the section line and pins; a section sheet's DXF is the section in metres along the line and metres NAP. series=true writes one file per diagram and variant into folder instead, as the app's \"whole series\" export does. Refuses to replace an existing file unless overwrite is true. The site must have been fetched first."
    )]
    async fn export(&self, Parameters(input): Parameters<Export>) -> CallToolResult {
        let format = ExportFormat::from(input.format);
        let dpi = input.dpi.unwrap_or(DEFAULT_EXPORT_DPI);
        if input.series.unwrap_or(false) {
            let Some(folder) = input.folder.clone().map(PathBuf::from) else {
                return failure("folder is needed when series is true".to_string());
            };
            if !folder.is_absolute() || !folder.is_dir() {
                return failure(format!(
                    "folder must be an absolute path to an existing folder, got \"{}\"",
                    folder.display()
                ));
            }
            let result = self
                .with_library(move |library| {
                    export_series(
                        library,
                        &input.id,
                        input.tab.unwrap_or(1),
                        format,
                        dpi,
                        &folder,
                        input.overwrite.unwrap_or(false),
                    )
                })
                .await;
            return reply(result);
        }
        let Some(path_text) = input.path.clone() else {
            return failure("path is needed unless series is true".to_string());
        };
        let path = PathBuf::from(&path_text);
        if !path.is_absolute() {
            return failure(format!("path must be absolute, got \"{path_text}\""));
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

        let result = self
            .with_library(move |library| {
                with_render_input(library, &input.id, input.tab.unwrap_or(1), |render| {
                    export::export(render, format, dpi, &path)
                })?;
                Ok(json!({ "path": path, "format": format }))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "List a project's reports: pages the user laid out in the app, each with its paper size and the blocks placed on it (ratios charts, analysis figures, facts and notes, sheets in another diagram, and text), positioned in millimetres. Pass a report id to export_report."
    )]
    async fn list_reports(&self, Parameters(input): Parameters<ProjectId>) -> CallToolResult {
        reply(
            self.with_library(move |library| reports::list(library, &input.id))
                .await,
        )
    }

    #[tool(
        description = "Write one of a project's reports, drawn from the project's current data in the report's palette: a PDF with a page per report page (the default), or an SVG or PNG per page. Refuses to replace existing files unless overwrite is true. The site must have been fetched first."
    )]
    async fn export_report(&self, Parameters(input): Parameters<ExportReport>) -> CallToolResult {
        reply(
            self.with_library(move |library| reports::export(library, input))
                .await,
        )
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
        description = "Import a palette from what a palette generator exports or copies (hex codes, JSON, an SVG of swatches, an XML palette, or a link ending in dash-joined codes) and save it as a theme. The colours fill the roles by a fixed rule: the lightest quiet swatch is the paper (a colourful one is softened to a pale tint), the darkest is the ink, the darkest that stands out from the paper is the buildings, the most colourful is the accent, and the swatches nearest blue and green are water and green. Returns the id, each role with the swatch it came from and why, and warnings where text or buildings fall below the WCAG contrast ratios. A palette whose text misses its ratio is not saved, as in the app. save=false only checks."
    )]
    async fn import_palette(&self, Parameters(input): Parameters<ImportPalette>) -> CallToolResult {
        reply(
            self.with_library(move |library| palettes::import(library, input))
                .await,
        )
    }

    #[tool(
        description = "List the palettes the user made or imported, with their swatches. Built-in themes are in list_themes."
    )]
    async fn list_user_palettes(&self) -> CallToolResult {
        reply(
            self.with_library(|library| Ok(palettes::list_user(library)))
                .await,
        )
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
pub(crate) fn reply<T: Serialize>(result: Result<T, String>) -> CallToolResult {
    let json = result.and_then(|value| serde_json::to_string_pretty(&value).map_err(text));
    match json {
        Ok(json) => CallToolResult::success(vec![ContentBlock::text(json)]),
        Err(message) => failure(message),
    }
}

fn failure(message: String) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message)])
}

pub(crate) fn text(error: impl std::fmt::Display) -> String {
    error.to_string()
}
