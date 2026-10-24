//! The site step: address search, measuring a drawn area, and fetching it.

use std::f64::consts::PI;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use geo::{GeodesicArea, LineString, Polygon};
use openarchi_core::area::{Area, LonLat, MAX_AREA_M2};
use openarchi_core::geocode::{self, Place};
use openarchi_core::site::{build_scene, Progress};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::commands::library::{save_open_project, view_of, ProjectView};
use crate::guard::{blocking, calm, text};
use crate::state::{lock, AppState, Shared};

const SEARCH_LIMIT: usize = 6;

// A loose box around the Netherlands, only for a gentle hint while drawing.
// The core's fetch is the real judge and refuses areas it cannot serve.
const NL_WEST: f64 = 3.2;
const NL_EAST: f64 = 7.3;
const NL_SOUTH: f64 = 50.7;
const NL_NORTH: f64 = 53.7;

pub const FETCH_PROGRESS_EVENT: &str = "fetch-progress";
pub const FETCH_FINISHED_EVENT: &str = "fetch-finished";

#[derive(Serialize)]
pub struct AreaSize {
    pub m2: f64,
    pub max_m2: f64,
    pub in_netherlands: bool,
}

#[derive(Clone, Serialize)]
struct FetchProgress {
    generation: u64,
    progress: Progress,
}

#[derive(Serialize, Clone)]
struct FetchFinished {
    generation: u64,
    error: Option<String>,
}

#[tauri::command]
pub async fn search_places(query: String) -> Result<Vec<Place>, String> {
    blocking(move || geocode::search(&query, SEARCH_LIMIT).map_err(text)).await
}

fn ring(points: &[LonLat]) -> Polygon<f64> {
    let coords: Vec<(f64, f64)> = points.iter().map(|point| (point.lon, point.lat)).collect();
    Polygon::new(LineString::from(coords), vec![])
}

fn corner_points(area: &Area) -> Vec<LonLat> {
    match area {
        Area::Circle { center, .. } => vec![*center],
        Area::Rectangle {
            south_west,
            north_east,
        } => vec![*south_west, *north_east],
        Area::Polygon { points } => points.clone(),
    }
}

/// "2.6 ha, inside the Netherlands": shown live while the user draws.
#[tauri::command]
pub fn measure_area(area: Area) -> AreaSize {
    let m2 = match &area {
        Area::Circle { radius_m, .. } => PI * radius_m * radius_m,
        Area::Rectangle {
            south_west,
            north_east,
        } => ring(&[
            *south_west,
            LonLat {
                lon: north_east.lon,
                lat: south_west.lat,
            },
            *north_east,
            LonLat {
                lon: south_west.lon,
                lat: north_east.lat,
            },
        ])
        .geodesic_area_unsigned(),
        Area::Polygon { points } if points.len() >= 3 => ring(points).geodesic_area_unsigned(),
        Area::Polygon { .. } => 0.0,
    };
    let in_netherlands = corner_points(&area).iter().all(|point| {
        (NL_WEST..=NL_EAST).contains(&point.lon) && (NL_SOUTH..=NL_NORTH).contains(&point.lat)
    });

    AreaSize {
        m2,
        max_m2: MAX_AREA_M2,
        in_netherlands,
    }
}

fn fetch_and_store(
    app: &AppHandle,
    state: &AppState,
    area: Area,
    generation: u64,
) -> Result<(), String> {
    let on_progress = |progress: Progress| {
        let _ = app.emit(
            FETCH_PROGRESS_EVENT,
            FetchProgress {
                generation,
                progress,
            },
        );
    };
    let cache = state.library()?.tile_cache();
    let scene = build_scene(&area, &cache, &on_progress).map_err(text)?;
    if state.fetch_generation.load(Ordering::SeqCst) != generation {
        return Ok(());
    }

    match lock(&state.open).as_mut() {
        Some(open) => {
            open.project.area = area;
            open.scene = Some(Arc::new(scene));
            open.packed_scene = None;
        }
        None => return Err("The project was closed during the fetch.".to_string()),
    }
    save_open_project(state).map(|_| ())
}

/// Starts fetching `area` for the open project on a background thread.
/// Progress and the result arrive as events; the returned number names this fetch.
#[tauri::command]
pub fn fetch_site(app: AppHandle, state: State<'_, Shared>, area: Area) -> u64 {
    let state = state.inner().clone();
    let generation = state.fetch_generation.fetch_add(1, Ordering::SeqCst) + 1;

    std::thread::spawn(move || {
        let result = calm(|| fetch_and_store(&app, &state, area, generation));
        let _ = app.emit(
            FETCH_FINISHED_EVENT,
            FetchFinished {
                generation,
                error: result.err(),
            },
        );
    });
    generation
}

/// Stops listening to the running fetch. The data services still finish
/// their requests, but the result is dropped.
#[tauri::command]
pub fn cancel_fetch(state: State<'_, Shared>) {
    state.fetch_generation.fetch_add(1, Ordering::SeqCst);
}

/// The open project after a fetch finished.
#[tauri::command]
pub async fn current_project(state: State<'_, Shared>) -> Result<ProjectView, String> {
    match lock(&state.open).as_ref() {
        Some(open) => Ok(view_of(open)),
        None => Err("No project is open.".to_string()),
    }
}

