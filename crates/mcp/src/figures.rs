//! The figure tools: the numbers behind a fetched site, its ratios, its walk,
//! its analysis and its environment. They join the router in `server.rs`,
//! like the sheet and pin tools.

use openarchi_core::{analysis, environment, ratios, selection, walk};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router};

use crate::figure_inputs::{GetAnalysis, GetEnvironment, GetRatios};
use crate::inputs::ProjectId;
use crate::resolve::find_theme;
use crate::server::{reply, text, OpenArchi};

#[tool_router(router = figure_tools, vis = "pub(crate)")]
impl OpenArchi {
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
        description = "Get the site's environment in numbers: road traffic noise (mean and highest Lden in dB) and all noise sources together, annual mean NO2, PM10 and PM2.5 in µg/m³, the deepest flood water and the share of the site it reaches in LIWO's extremely-small-chance scenario (under 1 in 30,000 a year), the steepest slope in percent, CBS residents and households (each 100 m square counted by its share inside the site, plus how many squares CBS keeps secret), BAG dwellings and floor area, the share of the site under tree canopy (RIVM tree map), the mean approximate vegetation index (NDVI from the infrared aerial photo, -1 to 1) and the mean urban heat island effect in °C (summer air temperature above the countryside, RIVM). Each source's year is included. A figure is null when its source was not reached. With `tab`, a sheet opened from a highlight gives the figures of that smaller area; `whole_site` lists the layers whose figures still describe the whole site there (slope always, and noise, air and flood for sites fetched before sheet areas measured their own). The site must have been fetched first."
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
}
