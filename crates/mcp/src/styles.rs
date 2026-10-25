//! The style tools: the diagrams and themes to choose from, set_look and
//! edit_text to change a sheet, and create_theme and the palette tools for new
//! themes. They join the router in `server.rs`, like the sheet and pin tools.

use openarchi_core::catalog;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router};
use serde_json::{json, Value};

use crate::look::updated_look;
use crate::palettes::{self, ImportPalette};
use crate::resolve::{check_plate, chosen_diagram, find_theme, kept_in_plan};
use crate::server::{blocking, reply, text, OpenArchi};
use crate::style_inputs::{CreateTheme, ListThemes, SetLook};
use crate::text::edited_sheet;
use crate::text_inputs::EditText;
use crate::user_themes::save_user_theme;

#[tool_router(router = style_tools, vis = "pub(crate)")]
impl OpenArchi {
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
        description = "Change how a project looks and save it. Pass only what should change: any of diagram_id and its variant, theme_id, sheet fields, per-layer overrides, the view (plan, axonometric with buildings standing up, or explode with several diagrams stacked as plates) and the sun's date and time for the sun path and shadow diagrams. A plan-only diagram (Wind) refuses the axonometric view, and choosing it on an axonometric sheet turns the sheet to its plan, as in the app; it may still be an explode plate. Returns the whole new look. Earlier versions stay in the project's backups."
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

                let asked_mode = input
                    .view
                    .as_ref()
                    .and_then(|view| view.mode)
                    .map(Into::into);

                let mut project = library.load(&input.id).map_err(text)?;
                let look = project.sheet_mut(input.tab.unwrap_or(1)).map_err(text)?;
                let diagram = chosen_diagram(look, input.diagram_id, input.variant)?;
                let updated = updated_look(
                    look,
                    diagram,
                    input.theme_id,
                    input.sheet,
                    input.overrides,
                    input.view,
                    input.sun,
                )?;
                *look = kept_in_plan(updated, asked_mode)?;
                let look = look.clone();
                library.save(&mut project).map_err(text)?;
                Ok(look)
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
}
