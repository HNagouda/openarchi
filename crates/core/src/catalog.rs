//! The diagrams and themes that ship with the app, plus the user's own themes.

use std::path::Path;

use crate::diagram::Diagram;
use crate::theme::Theme;
use crate::Result;

/// Built-in diagrams, in the order the picker shows them.
pub fn builtin_diagrams() -> Vec<Diagram> {
    todo!("render layer")
}

/// Built-in themes, in the order the picker shows them.
pub fn builtin_themes() -> Vec<Theme> {
    todo!("render layer")
}

/// Built-in themes followed by every valid `*.json` theme in `user_dir`.
/// A broken file is skipped and reported in the second value, never fatal.
pub fn all_themes(user_dir: &Path) -> (Vec<Theme>, Vec<String>) {
    let _ = user_dir;
    todo!("render layer")
}

/// Reads and validates one theme file, migrating older format versions.
pub fn read_theme(path: &Path) -> Result<Theme> {
    let _ = path;
    todo!("render layer")
}
