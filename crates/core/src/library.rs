//! The folder that holds the user's projects, backups, deleted projects and themes.
//!
//! ```text
//! <root>/projects/<id>.openarchi        current projects
//! <root>/backups/<id>/<timestamp>.openarchi   the last BACKUPS_KEPT versions of each
//! <root>/trash/<id>.openarchi           Recently Deleted, restorable
//! <root>/themes/<id>.json               the user's own themes
//! <root>/cache/                         downloaded map tiles, see `sources::cache`
//! ```
//!
//! `OPENARCHI_LIBRARY_DIR` points the app and the MCP server at a throwaway
//! folder in tests, so they never touch real projects.
//!
//! The app and the MCP server may save the same project at the same time. That
//! is safe because every save backs up whatever is on disk before replacing it,
//! so the loser of a race is still one click away in the backups.

mod files;
mod migrate;
mod themes;

use std::cmp::Reverse;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::area::Area;
use crate::project::{Look, Project, PROJECT_FORMAT_VERSION};
use crate::sheet::Frame;
use crate::sources::cache::TileCache;
use crate::{Error, Result};

use self::files::{
    backup_stamps, check_file_stem, file_name, now_rfc3339, read_modified, read_project,
    read_summaries, rfc3339_from_stamp, saved_modified, slugify, stamp_from_rfc3339,
    write_atomically,
};
pub use self::migrate::migrate;

/// How many earlier versions of each project we keep.
pub const BACKUPS_KEPT: usize = 20;

pub const LIBRARY_DIR_ENV: &str = "OPENARCHI_LIBRARY_DIR";

/// The library's folder inside the platform data folder.
const APP_FOLDER: &str = "openarchi";

const PROJECTS_FOLDER: &str = "projects";
const BACKUPS_FOLDER: &str = "backups";
const TRASH_FOLDER: &str = "trash";
const THEMES_FOLDER: &str = "themes";
const CACHE_FOLDER: &str = "cache";

/// Tries at a free random id suffix. A clash needs two projects with the same
/// name on the same day, so running out means something else is wrong.
const ID_ATTEMPTS: usize = 100;

/// The slug of a project whose name has no ASCII letters or digits in it.
const SLUG_FALLBACK: &str = "project";

/// The frame that fits the shape the user drew, so a rectangle is not left
/// floating inside an empty circle.
fn frame_for(area: &Area) -> Frame {
    match area {
        Area::Circle { .. } => Frame::Circle,
        Area::Rectangle { .. } => Frame::Square,
        Area::Polygon { .. } => Frame::Area,
    }
}

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
    /// The diagram, theme and sheet, so a gallery card can print its scale and paper.
    #[serde(default)]
    pub look: Option<Look>,
    /// Set when the file cannot be read. The project still shows in the list,
    /// so the user can see it exists and restore a backup.
    #[serde(default)]
    pub error: Option<String>,
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
        if let Some(dir) = std::env::var_os(LIBRARY_DIR_ENV) {
            return Library::at(Path::new(&dir));
        }

        let Some(data_dir) = dirs::data_dir() else {
            return Err(Error::Invalid(format!(
                "could not find the app data folder on this computer; set {LIBRARY_DIR_ENV} to choose one"
            )));
        };
        Library::at(&data_dir.join(APP_FOLDER))
    }

    /// Opens or creates a library at `root`.
    pub fn at(root: &Path) -> Result<Library> {
        for folder in [PROJECTS_FOLDER, BACKUPS_FOLDER, TRASH_FOLDER, THEMES_FOLDER] {
            let path = root.join(folder);
            fs::create_dir_all(&path).map_err(|e| Error::io(&path, e))?;
        }

        Ok(Library {
            root: root.to_path_buf(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Where the user's own themes live.
    pub fn themes_dir(&self) -> PathBuf {
        self.root.join(THEMES_FOLDER)
    }

    /// Downloaded map tiles, shared by every project so neighbouring sites
    /// reuse each other's downloads.
    pub fn tile_cache(&self) -> TileCache {
        TileCache::at(&self.root.join(CACHE_FOLDER))
    }

    /// Current projects, most recently modified first.
    pub fn list(&self) -> Result<Vec<ProjectSummary>> {
        let mut projects: Vec<ProjectSummary> = read_summaries(&self.root.join(PROJECTS_FOLDER))?
            .into_iter()
            .map(|(summary, _)| summary)
            .collect();

        // Every `modified` comes from `now_rfc3339`, which always writes UTC
        // with nine fraction digits, so the strings sort like the times.
        projects.sort_by(|a, b| b.modified.cmp(&a.modified));
        Ok(projects)
    }

    /// A new, saved project with no scene yet.
    pub fn create(&self, name: &str, area: Area) -> Result<Project> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::Invalid("a project needs a name".to_string()));
        }

        let now = now_rfc3339();
        let mut project = Project {
            format_version: PROJECT_FORMAT_VERSION,
            id: self.new_id(name, &now)?,
            name: name.to_string(),
            created: now.clone(),
            modified: now,
            area,
            scene: None,
            look: Look::default(),
            reports: Vec::new(),
            more_sheets: Vec::new(),
            pins: Vec::new(),
            highlights: Vec::new(),
        };
        project.look.sheet.title = project.name.clone();
        project.look.sheet.frame = frame_for(&project.area);
        self.save(&mut project)?;
        Ok(project)
    }

    /// Loads a project, migrating older format versions.
    pub fn load(&self, id: &str) -> Result<Project> {
        let path = self.project_path(id)?;
        if !path.exists() {
            return Err(no_project(id));
        }
        read_project(&path)
    }

    /// When the project on disk was last saved, read from the start of the
    /// file, so the app can tell whether someone else wrote it since.
    pub fn modified(&self, id: &str) -> Result<String> {
        let path = self.project_path(id)?;
        if !path.exists() {
            return Err(no_project(id));
        }
        read_modified(&path)
    }

    /// Writes atomically, backs up the previous version, stamps `modified`.
    pub fn save(&self, project: &mut Project) -> Result<()> {
        self.write(project, serde_json::to_vec)
    }

    /// Like `save`, for a project whose `scene` is `None` and whose scene was
    /// already packed with `packed_scene::pack`. Packing takes most of a save
    /// and only a fetch changes the scene, so the app packs it once and keeps it.
    pub fn save_with_packed_scene(&self, project: &mut Project, packed_scene: &str) -> Result<()> {
        if project.scene.is_some() {
            return Err(Error::Invalid(
                "a project saved with a packed scene must not carry the scene itself".to_string(),
            ));
        }
        #[derive(Serialize)]
        struct Stored<'a> {
            #[serde(flatten)]
            project: &'a Project,
            scene: &'a str,
        }
        self.write(project, |project| {
            serde_json::to_vec(&Stored {
                project,
                scene: packed_scene,
            })
        })
    }

    fn write(
        &self,
        project: &mut Project,
        encode: impl FnOnce(&Project) -> serde_json::Result<Vec<u8>>,
    ) -> Result<()> {
        let path = self.project_path(&project.id)?;
        project.format_version = PROJECT_FORMAT_VERSION;
        project.modified = now_rfc3339();

        // Compact rather than pretty: a scene holds many thousands of
        // coordinates, and indentation roughly doubles the file.
        let bytes = encode(project).map_err(|e| Error::Format {
            path: path.display().to_string(),
            detail: e.to_string(),
        })?;

        if path.exists() {
            self.back_up(&project.id, &path)?;
        }
        write_atomically(&path, &bytes)
    }

    /// Moves the project to Recently Deleted.
    pub fn delete(&self, id: &str) -> Result<()> {
        let path = self.project_path(id)?;
        if !path.exists() {
            return Err(no_project(id));
        }
        let target = self.trash_path(id)?;
        if target.exists() {
            return Err(Error::Invalid(format!(
                "Recently Deleted already holds a project with id '{id}'"
            )));
        }

        // A rename keeps the modified time, so stamp the deletion time first:
        // Recently Deleted sorts by it.
        let file = fs::File::options()
            .write(true)
            .open(&path)
            .map_err(|e| Error::io(&path, e))?;
        file.set_modified(SystemTime::now())
            .map_err(|e| Error::io(&path, e))?;
        drop(file);

        fs::rename(&path, &target).map_err(|e| Error::io(&path, e))
    }

    /// Recently Deleted, most recently deleted first.
    pub fn trash(&self) -> Result<Vec<ProjectSummary>> {
        let mut deleted = read_summaries(&self.root.join(TRASH_FOLDER))?;
        deleted.sort_by_key(|entry| Reverse(entry.1));
        Ok(deleted.into_iter().map(|(summary, _)| summary).collect())
    }

    /// Moves a project back out of Recently Deleted.
    pub fn restore(&self, id: &str) -> Result<()> {
        let source = self.trash_path(id)?;
        if !source.exists() {
            return Err(Error::Invalid(format!(
                "Recently Deleted has no project with id '{id}'"
            )));
        }
        let target = self.project_path(id)?;
        if target.exists() {
            return Err(Error::Invalid(format!(
                "a project with id '{id}' already exists, so the deleted one cannot be put back"
            )));
        }

        fs::rename(&source, &target).map_err(|e| Error::io(&source, e))
    }

    /// Earlier versions of a project, newest first.
    pub fn backups(&self, id: &str) -> Result<Vec<Backup>> {
        let folder = self.backups_dir(id)?;
        let mut backups: Vec<Backup> = backup_stamps(&folder)?
            .into_iter()
            .map(|stamp| Backup {
                modified: rfc3339_from_stamp(&stamp),
                stamp,
            })
            .collect();

        backups.sort_by(|a, b| b.stamp.cmp(&a.stamp));
        Ok(backups)
    }

    /// Replaces the current project with a backup. The current version is backed up first.
    pub fn restore_backup(&self, id: &str, stamp: &str) -> Result<Project> {
        let path = self.backups_dir(id)?.join(file_name(stamp)?);
        if !path.exists() {
            return Err(Error::Invalid(format!(
                "project '{id}' has no backup '{stamp}'"
            )));
        }

        let mut project = read_project(&path)?;
        // The file name is the id everywhere else, so it wins over the content.
        project.id = id.to_string();
        self.save(&mut project)?;
        Ok(project)
    }

    /// "2026-10-24-westermarkt-3f9a": the date, the name, and a random suffix
    /// so two projects with the same name on the same day do not clash.
    fn new_id(&self, name: &str, now: &str) -> Result<String> {
        let date = &now[..10];
        let slug = slugify(name, SLUG_FALLBACK);
        for _ in 0..ID_ATTEMPTS {
            let id = format!("{date}-{slug}-{:04x}", fastrand::u16(..));
            if !self.project_path(&id)?.exists() && !self.trash_path(&id)?.exists() {
                return Ok(id);
            }
        }

        Err(Error::Invalid(format!(
            "could not find a free project id for '{name}'"
        )))
    }

    /// Copies the file on disk into the backups and prunes the oldest.
    fn back_up(&self, id: &str, current: &Path) -> Result<()> {
        let folder = self.backups_dir(id)?;
        fs::create_dir_all(&folder).map_err(|e| Error::io(&folder, e))?;

        let stamp = stamp_from_rfc3339(&saved_modified(current));
        let target = folder.join(file_name(&stamp)?);
        // The stamp is the version's own save time, so an existing backup with
        // this stamp already holds this exact version.
        if !target.exists() {
            fs::copy(current, &target).map_err(|e| Error::io(current, e))?;
        }

        self.prune_backups(&folder)
    }

    fn prune_backups(&self, folder: &Path) -> Result<()> {
        let mut stamps = backup_stamps(folder)?;
        stamps.sort_by(|a, b| b.cmp(a));

        for stamp in stamps.iter().skip(BACKUPS_KEPT) {
            let path = folder.join(file_name(stamp)?);
            fs::remove_file(&path).map_err(|e| Error::io(&path, e))?;
        }
        Ok(())
    }

    fn project_path(&self, id: &str) -> Result<PathBuf> {
        Ok(self.root.join(PROJECTS_FOLDER).join(file_name(id)?))
    }

    fn trash_path(&self, id: &str) -> Result<PathBuf> {
        Ok(self.root.join(TRASH_FOLDER).join(file_name(id)?))
    }

    fn backups_dir(&self, id: &str) -> Result<PathBuf> {
        check_file_stem(id)?;
        Ok(self.root.join(BACKUPS_FOLDER).join(id))
    }
}

fn no_project(id: &str) -> Error {
    Error::Invalid(format!("no project with id '{id}'"))
}

#[cfg(test)]
mod format_tests;
#[cfg(test)]
mod packed_save_tests;
#[cfg(test)]
mod summary_tests;
#[cfg(test)]
mod tests;
