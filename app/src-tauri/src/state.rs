//! What the app keeps in memory between commands: the library, the open
//! project and the catalog.
//!
//! The open project's scene sits behind an `Arc`, so the preview and the
//! filmstrip can render at the same time without copying a large scene.

use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use openarchi_core::diagram::Diagram;
use openarchi_core::library::Library;
use openarchi_core::project::Project;
use openarchi_core::scene::Scene;
use openarchi_core::theme::Theme;
use serde::Serialize;

pub type Shared = Arc<AppState>;

pub struct AppState {
    /// Opened once at start. An error here is shown in the gallery, calmly.
    pub library: Result<Library, String>,
    pub open: Mutex<Option<OpenProject>>,
    pub catalog: Mutex<Option<Catalog>>,
    /// Gallery thumbnails as data URLs, keyed by id, modified time and width.
    pub thumbnails: Mutex<HashMap<String, String>>,
    /// When the app itself last wrote each project, so the watcher can tell
    /// our own saves from an assistant's.
    pub own_saves: Mutex<HashMap<String, Instant>>,
    /// Bumped by every fetch and cancel; a fetch whose number is stale is dropped.
    pub fetch_generation: AtomicU64,
}

pub struct OpenProject {
    /// The project with `scene` taken out; the scene lives next to it.
    pub project: Project,
    pub scene: Option<Arc<Scene>>,
}

#[derive(Clone, Serialize)]
pub struct Catalog {
    pub diagrams: Vec<Diagram>,
    pub themes: Vec<Theme>,
    /// Theme files in the library that could not be read.
    pub warnings: Vec<String>,
    pub layers: Vec<LayerInfo>,
}

#[derive(Clone, Serialize)]
pub struct LayerInfo {
    pub id: openarchi_core::scene::Layer,
    pub label: String,
}

impl AppState {
    pub fn new(library: Result<Library, String>) -> AppState {
        AppState {
            library,
            open: Mutex::new(None),
            catalog: Mutex::new(None),
            thumbnails: Mutex::new(HashMap::new()),
            own_saves: Mutex::new(HashMap::new()),
            fetch_generation: AtomicU64::new(0),
        }
    }

    pub fn library(&self) -> Result<&Library, String> {
        self.library.as_ref().map_err(|error| error.clone())
    }
}

impl OpenProject {
    pub fn from_project(mut project: Project) -> OpenProject {
        let scene = project.scene.take().map(Arc::new);
        OpenProject { project, scene }
    }

    /// The full project, scene included, as `Library::save` wants it.
    pub fn to_project(&self) -> Project {
        let mut project = self.project.clone();
        project.scene = self.scene.as_ref().map(|scene| Scene::clone(scene));
        project
    }
}

/// A poisoned lock only means an earlier command panicked; the data is still fine.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
