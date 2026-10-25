//! What each tool accepts, with the JSON schema an assistant sees.
//!
//! Core types do not derive `JsonSchema`, so the ones a tool takes are mirrored
//! here with descriptions written for a model, and converted on the way in.
//! This module holds the project and area inputs the other `*_inputs` modules
//! share. A tool area with several tools keeps the rest in its own `*_inputs`
//! module; a module with a single tool keeps its input beside the tool.

use openarchi_core::area::{Area, LonLat};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProjectId {
    /// The project id from list_projects or create_project, like "2026-10-24-westermarkt-3f9a".
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchPlaces {
    /// An address, street, place name or postcode in the Netherlands, like "Westermarkt 20 Amsterdam".
    pub query: String,
    /// How many places to return, best first. Defaults to 5, at most 20.
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateProject {
    /// What the user calls the site, like "Westermarkt". Also the start of the project id.
    pub name: String,
    pub area: AreaInput,
}

/// A point in WGS84 degrees, as search_places returns it.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
pub struct LonLatInput {
    pub lon: f64,
    pub lat: f64,
}

/// The site to fetch, in longitude and latitude. Must lie in the Netherlands
/// and cover at most 4 km². A circle of 100 to 300 m radius around a
/// search_places location suits most site analyses.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AreaInput {
    Circle {
        center: LonLatInput,
        radius_m: f64,
    },
    Rectangle {
        south_west: LonLatInput,
        north_east: LonLatInput,
    },
    /// Outer ring only, at least 3 points, without repeating the first point at the end.
    Polygon {
        points: Vec<LonLatInput>,
    },
}

impl From<LonLatInput> for LonLat {
    fn from(point: LonLatInput) -> Self {
        LonLat {
            lon: point.lon,
            lat: point.lat,
        }
    }
}

impl From<AreaInput> for Area {
    fn from(area: AreaInput) -> Self {
        match area {
            AreaInput::Circle { center, radius_m } => Area::Circle {
                center: center.into(),
                radius_m,
            },
            AreaInput::Rectangle {
                south_west,
                north_east,
            } => Area::Rectangle {
                south_west: south_west.into(),
                north_east: north_east.into(),
            },
            AreaInput::Polygon { points } => Area::Polygon {
                points: points.into_iter().map(LonLat::from).collect(),
            },
        }
    }
}
