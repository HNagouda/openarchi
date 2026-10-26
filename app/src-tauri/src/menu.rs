//! The macOS menu bar. Every custom item sends its id to the window as a
//! "menu" event, and the window runs the same command a key would.
//!
//! Windows and Linux get no menu bar; the window handles Ctrl shortcuts itself.

use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Runtime};

pub const MENU_EVENT: &str = "menu";

fn item<R: Runtime>(
    app: &AppHandle<R>,
    id: &str,
    label: &str,
    keys: Option<&str>,
) -> tauri::Result<MenuItem<R>> {
    MenuItem::with_id(app, id, label, true, keys)
}

/// What the About panel says besides the name and icon.
fn about() -> AboutMetadata<'static> {
    AboutMetadata {
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
        comments: Some("Site-analysis diagrams from a drawn site.".to_string()),
        copyright: Some("Free software under the GPL, version 3 or later.".to_string()),
        license: Some(env!("CARGO_PKG_LICENSE").to_string()),
        website: Some(env!("CARGO_PKG_REPOSITORY").to_string()),
        website_label: Some("openarchi on GitHub".to_string()),
        credits: Some(
            "Built on open data; every sheet credits the sources it draws from.".to_string(),
        ),
        ..AboutMetadata::default()
    }
}

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let separator = || PredefinedMenuItem::separator(app);

    let app_menu = Submenu::with_items(
        app,
        "openarchi",
        true,
        &[
            &PredefinedMenuItem::about(app, Some("About openarchi"), Some(about()))?,
            &separator()?,
            &item(app, "settings", "Settings…", Some("CmdOrCtrl+,"))?,
            &separator()?,
            &PredefinedMenuItem::services(app, None)?,
            &separator()?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &separator()?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;
    let file_menu = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &item(app, "new", "New Project", Some("CmdOrCtrl+N"))?,
            &item(app, "open", "Open", Some("CmdOrCtrl+O"))?,
            &separator()?,
            &item(app, "export", "Export…", Some("CmdOrCtrl+E"))?,
            &separator()?,
            &item(app, "home", "All Projects", Some("CmdOrCtrl+Shift+H"))?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;
    // Undo and redo are ours, not the predefined ones, because they undo
    // changes to the site and the look. In a text field the window passes them on.
    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &item(app, "undo", "Undo", Some("CmdOrCtrl+Z"))?,
            &item(app, "redo", "Redo", Some("CmdOrCtrl+Shift+Z"))?,
            &separator()?,
            // A plain key in the label rather than an accelerator, so T still types in text fields.
            &item(app, "add-text", "Add Text  (T)", None)?,
            &separator()?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;
    let view_menu = Submenu::with_items(
        app,
        "View",
        true,
        &[
            &item(app, "zoom-in", "Zoom In", Some("CmdOrCtrl+="))?,
            &item(app, "zoom-out", "Zoom Out", Some("CmdOrCtrl+-"))?,
            &item(app, "zoom-fit", "Zoom to Fit", Some("CmdOrCtrl+0"))?,
            &separator()?,
            &item(app, "diagrams", "Toggle Diagrams  (D)", None)?,
            &item(app, "inspector", "Toggle Inspector  (I)", None)?,
            &separator()?,
            &PredefinedMenuItem::fullscreen(app, None)?,
        ],
    )?;
    let window_menu = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
        ],
    )?;
    let help_menu = Submenu::with_items(
        app,
        "Help",
        true,
        &[
            &item(app, "tour", "Show Tour", None)?,
            &item(app, "assistant", "Connect an Assistant…", None)?,
            &item(app, "shortcuts", "Keyboard Shortcuts", Some("CmdOrCtrl+/"))?,
        ],
    )?;

    Menu::with_items(
        app,
        &[
            &app_menu,
            &file_menu,
            &edit_menu,
            &view_menu,
            &window_menu,
            &help_menu,
        ],
    )
}

pub fn forward(app: &AppHandle, id: &str) {
    let _ = app.emit(MENU_EVENT, id.to_string());
}
