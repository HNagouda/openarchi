//! The wind tool: which months a sheet's Wind diagram takes its prevailing
//! wind from, as the app's season switch sets it. It joins the router in
//! `server.rs`, like the viewpoint tool.

use openarchi_core::wind::{self, WindSeason};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::server::{reply, text, OpenArchi};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetWindSeason {
    /// The project id.
    pub id: String,
    /// Which sheet tab, counting from 1 for the first, which is the default.
    pub tab: Option<usize>,
    pub season: SeasonInput,
}

/// The whole year, or a meteorological season: winter is December to February.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SeasonInput {
    Year,
    Winter,
    Spring,
    Summer,
    Autumn,
}

impl From<SeasonInput> for WindSeason {
    fn from(season: SeasonInput) -> Self {
        match season {
            SeasonInput::Year => WindSeason::Year,
            SeasonInput::Winter => WindSeason::Winter,
            SeasonInput::Spring => WindSeason::Spring,
            SeasonInput::Summer => WindSeason::Summer,
            SeasonInput::Autumn => WindSeason::Autumn,
        }
    }
}

#[tool_router(router = wind_tools, vis = "pub(crate)")]
impl OpenArchi {
    #[tool(
        description = "Choose which months a sheet's Wind diagram takes its wind from: the whole year (the default) or one season. The diagram blows the most common wind direction of those months, from the site's typical year, through the site in plan. Then set_look with diagram_id \"wind\" shows it. Returns the season and the wind it gives, such as \"from the SW, most common all year (12% of hours)\"."
    )]
    async fn set_wind_season(
        &self,
        Parameters(input): Parameters<SetWindSeason>,
    ) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                let season = WindSeason::from(input.season);
                let wind = project
                    .scene
                    .as_ref()
                    .map(|scene| wind::described(scene, season));
                let look = project.sheet_mut(input.tab.unwrap_or(1)).map_err(text)?;
                look.wind_season = season;
                library.save(&mut project).map_err(text)?;
                Ok(json!({ "wind_season": season, "wind": wind }))
            })
            .await;
        reply(result)
    }
}
