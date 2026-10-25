//! The user's own themes: saving, deleting and bringing one back.
//!
//! A theme that is replaced or deleted is never removed. It moves to
//! `<root>/trash/themes/<id>-<millis>.json`, the themes' corner of Recently
//! Deleted, so redesigning or deleting a palette is always one step from undone.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::files::{check_file_stem, slugify, write_atomically};
use super::{Library, TRASH_FOLDER};
use crate::catalog::{all_themes, builtin_themes, read_theme, theme_problems};
use crate::theme::Theme;
use crate::{Error, Result};

const THEMES_TRASH_FOLDER: &str = "themes";
const THEME_EXTENSION: &str = "json";
/// The id of a palette whose name has no letters or digits at all.
const ID_FALLBACK: &str = "palette";

impl Library {
    /// Writes `theme` to the user's themes under its id. Built-in ids are
    /// refused; a user theme with the same id moves to Recently Deleted first.
    pub fn save_theme(&self, theme: &Theme) -> Result<()> {
        if builtin_themes()
            .iter()
            .any(|builtin| builtin.id == theme.id)
        {
            return Err(Error::Invalid(format!(
                "'{}' is a built-in palette; save yours under a new id",
                theme.id
            )));
        }
        let problems = theme_problems(theme);
        if !problems.is_empty() {
            return Err(Error::Invalid(format!(
                "the palette is not valid: {}",
                problems.join("; ")
            )));
        }

        let path = self.theme_path(&theme.id)?;
        if path.exists() {
            self.set_theme_aside(&theme.id)?;
        }
        let text = serde_json::to_string_pretty(theme)
            .map_err(|error| Error::Invalid(error.to_string()))?;
        write_atomically(&path, text.as_bytes())
    }

    /// Moves a user theme to Recently Deleted. Returns the name it is kept
    /// under, which `restore_theme` takes to bring it back.
    pub fn delete_theme(&self, id: &str) -> Result<String> {
        if !self.theme_path(id)?.exists() {
            return Err(Error::Invalid(format!(
                "there is no palette of yours with id '{id}'"
            )));
        }
        self.set_theme_aside(id)
    }

    /// Puts a deleted theme back under its own id.
    pub fn restore_theme(&self, kept: &str) -> Result<Theme> {
        check_file_stem(kept)?;
        let source = self
            .themes_trash()
            .join(format!("{kept}.{THEME_EXTENSION}"));
        if !source.exists() {
            return Err(Error::Invalid(format!(
                "Recently Deleted has no palette '{kept}'"
            )));
        }
        let theme = read_theme(&source)?;
        let target = self.theme_path(&theme.id)?;
        if target.exists() {
            return Err(Error::Invalid(format!(
                "a palette with id '{}' already exists, so the deleted one cannot be put back",
                theme.id
            )));
        }

        fs::rename(&source, &target).map_err(|e| Error::io(&source, e))?;
        Ok(theme)
    }

    /// "Sea Glass" -> "sea-glass", or "sea-glass-2" when that id is taken by any theme.
    pub fn free_theme_id(&self, name: &str) -> String {
        let base = slugify(name, ID_FALLBACK);
        let (themes, _) = all_themes(&self.themes_dir());
        let is_taken = |id: &str| themes.iter().any(|theme| theme.id == id);
        if !is_taken(&base) {
            return base;
        }

        let mut number = 2;
        while is_taken(&format!("{base}-{number}")) {
            number += 1;
        }
        format!("{base}-{number}")
    }

    fn set_theme_aside(&self, id: &str) -> Result<String> {
        let folder = self.themes_trash();
        fs::create_dir_all(&folder).map_err(|e| Error::io(&folder, e))?;
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis())
            .unwrap_or_default();
        let kept = format!("{id}-{millis}");
        let source = self.theme_path(id)?;
        let target = folder.join(format!("{kept}.{THEME_EXTENSION}"));

        fs::rename(&source, &target).map_err(|e| Error::io(&source, e))?;
        Ok(kept)
    }

    fn theme_path(&self, id: &str) -> Result<PathBuf> {
        check_file_stem(id)?;
        Ok(self.themes_dir().join(format!("{id}.{THEME_EXTENSION}")))
    }

    fn themes_trash(&self) -> PathBuf {
        self.root().join(TRASH_FOLDER).join(THEMES_TRASH_FOLDER)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::{check, parse, PaletteDraft, RoleChoice};

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::at(dir.path()).unwrap();
        (dir, library)
    }

    fn palette(id: &str, name: &str) -> Theme {
        let draft = PaletteDraft {
            name: name.to_string(),
            swatches: parse("f79f79, f7d08a, e3f09b, 87b6a7, 5b5941").unwrap(),
            roles: RoleChoice::default(),
            typography: None,
            based_on: None,
        };
        check(&draft, id).unwrap().theme
    }

    fn user_ids(library: &Library) -> Vec<String> {
        let (themes, problems) = all_themes(&library.themes_dir());
        assert!(problems.is_empty(), "{problems:?}");
        themes
            .into_iter()
            .filter(|theme| theme.source.is_some())
            .map(|theme| theme.id)
            .collect()
    }

    #[test]
    fn a_saved_palette_loads_with_the_catalog() {
        let (_dir, library) = library();
        library
            .save_theme(&palette("sea-glass", "Sea Glass"))
            .unwrap();
        assert_eq!(user_ids(&library), ["sea-glass"]);
    }

    #[test]
    fn a_builtin_id_is_refused() {
        let (_dir, library) = library();
        let error = library.save_theme(&palette("paper", "Paper")).unwrap_err();
        assert!(error.to_string().contains("built-in"), "{error}");
    }

    #[test]
    fn deleting_and_restoring_brings_the_same_palette_back() {
        let (_dir, library) = library();
        let theme = palette("sea-glass", "Sea Glass");
        library.save_theme(&theme).unwrap();
        let kept = library.delete_theme("sea-glass").unwrap();
        assert!(user_ids(&library).is_empty());

        assert_eq!(library.restore_theme(&kept).unwrap(), theme);
        assert_eq!(user_ids(&library), ["sea-glass"]);
    }

    #[test]
    fn saving_over_a_palette_keeps_the_old_one_in_recently_deleted() {
        let (_dir, library) = library();
        library
            .save_theme(&palette("sea-glass", "Sea Glass"))
            .unwrap();
        library
            .save_theme(&palette("sea-glass", "Sea Glass, again"))
            .unwrap();
        let kept = fs::read_dir(library.themes_trash()).unwrap().count();
        assert_eq!(kept, 1);
    }

    #[test]
    fn a_free_id_skips_taken_ones_and_builtins() {
        let (_dir, library) = library();
        library
            .save_theme(&palette("sea-glass", "Sea Glass"))
            .unwrap();
        assert_eq!(library.free_theme_id("Sea Glass"), "sea-glass-2");
        assert_eq!(library.free_theme_id("Paper"), "paper-2");
        assert_eq!(library.free_theme_id("★★"), "palette");
    }

    #[test]
    fn a_kept_name_cannot_reach_outside_the_library() {
        let (_dir, library) = library();
        assert!(library.restore_theme("../../etc/passwd").is_err());
        assert!(library.delete_theme("../projects/x").is_err());
    }
}
