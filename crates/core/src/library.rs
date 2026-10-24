//! The folder that holds the user's projects, backups, deleted projects and themes.
//!
//! ```text
//! <root>/projects/<id>.openarchi        current projects
//! <root>/backups/<id>/<timestamp>.openarchi   the last BACKUPS_KEPT versions of each
//! <root>/trash/<id>.openarchi           Recently Deleted, restorable
//! <root>/themes/<id>.json               the user's own themes
//! ```
//!
//! `OPENARCHI_LIBRARY_DIR` points the app and the MCP server at a throwaway
//! folder in tests, so they never touch real projects.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::area::Area;
use crate::project::Project;
use crate::Result;

/// How many earlier versions of each project we keep.
pub const BACKUPS_KEPT: usize = 20;

pub const LIBRARY_DIR_ENV: &str = "OPENARCHI_LIBRARY_DIR";

pub struct Library {
    root: PathBuf,
}

/// A project as the project list shows it, without loading the scene.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub modified: String,
    pub has_scene: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backup {
    /// The timestamp in the file name, also how `restore_backup` finds it.
    pub stamp: String,
    pub modified: String,
}

impl Library {
    /// `OPENARCHI_LIBRARY_DIR` if set, otherwise the platform data folder
    /// (`~/Library/Application Support/openarchi`, `%APPDATA%\openarchi`).
    pub fn open_default() -> Result<Library> {
        todo!("library")
    }

    /// Opens or creates a library at `root`.
    pub fn at(root: &Path) -> Result<Library> {
        let _ = root;
        todo!("library")
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Where the user's own themes live.
    pub fn themes_dir(&self) -> PathBuf {
        self.root.join("themes")
    }

    /// Current projects, most recently modified first.
    pub fn list(&self) -> Result<Vec<ProjectSummary>> {
        todo!("library")
    }

    /// A new, saved project with no scene yet.
    pub fn create(&self, name: &str, area: Area) -> Result<Project> {
        let _ = (name, area);
        todo!("library")
    }

    /// Loads a project, migrating older format versions.
    pub fn load(&self, id: &str) -> Result<Project> {
        let _ = id;
        todo!("library")
    }

    /// Writes atomically, backs up the previous version, stamps `modified`.
    pub fn save(&self, project: &mut Project) -> Result<()> {
        let _ = project;
        todo!("library")
    }

    /// Moves the project to Recently Deleted.
    pub fn delete(&self, id: &str) -> Result<()> {
        let _ = id;
        todo!("library")
    }

    /// Recently Deleted, most recently deleted first.
    pub fn trash(&self) -> Result<Vec<ProjectSummary>> {
        todo!("library")
    }

    /// Moves a project back out of Recently Deleted.
    pub fn restore(&self, id: &str) -> Result<()> {
        let _ = id;
        todo!("library")
    }

    /// Earlier versions of a project, newest first.
    pub fn backups(&self, id: &str) -> Result<Vec<Backup>> {
        let _ = id;
        todo!("library")
    }

    /// Replaces the current project with a backup. The current version is backed up first.
    pub fn restore_backup(&self, id: &str, stamp: &str) -> Result<Project> {
        let _ = (id, stamp);
        todo!("library")
    }
}
