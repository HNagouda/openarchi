//! The desktop shell over `openarchi-core`. Commands live in `commands/`,
//! one module per part of the window; this file wires them together.

mod commands;
mod guard;
mod menu;
mod state;
mod watcher;

use std::sync::{Arc, Mutex};

use openarchi_core::library::Library;
use tauri::Manager;

use crate::commands::{
    analysis, export, laser, library, model, palettes, ratios, render, report, sheets, site, view3d,
};
use crate::guard::{calm, text};
use crate::state::AppState;

/// Keeps the folder watcher alive for as long as the app runs.
struct WatcherHandle {
    _watcher: Mutex<Option<notify::RecommendedWatcher>>,
}

pub fn run() {
    let library = calm(|| Library::open_default().map_err(text));
    let state = Arc::new(AppState::new(library));

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state.clone())
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            {
                let menu = menu::build(app.handle())?;
                app.set_menu(menu)?;
                app.on_menu_event(|app, event| menu::forward(app, &event.id().0));
            }

            // Without a library there is nothing to watch; the gallery says why.
            let watcher = watcher::watch_projects(app.handle().clone(), state.clone()).ok();
            app.manage(WatcherHandle {
                _watcher: Mutex::new(watcher),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            library::library_path,
            library::list_projects,
            library::list_trash,
            library::create_project,
            library::open_project,
            library::reload_project,
            library::close_project,
            library::rename_project,
            library::delete_project,
            library::restore_project,
            render::get_catalog,
            render::render_preview,
            render::render_thumbnail,
            render::project_thumbnail,
            sheets::save_sheets,
            sheets::fitting_scale,
            sheets::highlight_area,
            site::search_places,
            site::measure_area,
            site::fetch_site,
            site::cancel_fetch,
            site::current_project,
            export::export_file_name,
            export::export_sheet,
            export::export_series,
            export::reveal_path,
            export::open_link,
            laser::laser_summary,
            laser::export_laser,
            ratios::get_ratios,
            ratios::ratios_file_name,
            ratios::export_ratios,
            analysis::get_analysis,
            analysis::analysis_file_name,
            analysis::export_analysis,
            report::save_reports,
            report::render_report_block,
            report::render_report_backdrop,
            report::report_file_name,
            report::export_report,
            palettes::import_palette,
            palettes::check_palette,
            palettes::render_palette_preview,
            palettes::palette_draft,
            palettes::save_palette,
            palettes::delete_palette,
            palettes::restore_palette,
            model::import_model,
            model::reread_model,
            model::set_model,
            model::covered_buildings,
            view3d::view_3d,
            view3d::aerial_photo,
            view3d::sun_day,
            view3d::save_view_png,
        ])
        .run(tauri::generate_context!())
        .expect("openarchi could not start its window");
}
