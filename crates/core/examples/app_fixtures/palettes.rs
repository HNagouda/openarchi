use std::fs;
use std::path::Path;

use openarchi_core::catalog::builtin_themes;
use openarchi_core::palette::{self, PaletteCheck, PaletteDraft, RoleChoice};
use openarchi_core::theme::Theme;

/// The user's own palette in the mock: five colours from a palette generator.
const SAMPLE_PALETTE: &str = "f79f79, f7d08a, e3f09b, 87b6a7, 5b5941";
const SAMPLE_PALETTE_ID: &str = "westermarkt-sample";
const SAMPLE_PALETTE_NAME: &str = "Westermarkt sample";

/// The built-in themes, then the sample palette, as the app lists the user's own after them.
pub(crate) fn fixture_themes() -> Vec<Theme> {
    let mut themes = builtin_themes();
    themes.push(sample_palette().theme);
    themes
}

/// The sample palette as the editor checks it, with the rule filling every role.
pub(crate) fn sample_palette() -> PaletteCheck {
    let draft = PaletteDraft {
        name: SAMPLE_PALETTE_NAME.to_string(),
        swatches: palette::parse(SAMPLE_PALETTE).expect("read the sample palette"),
        roles: RoleChoice::default(),
        typography: None,
        based_on: None,
    };
    palette::check(&draft, SAMPLE_PALETTE_ID).expect("build the sample palette")
}

/// What the palette editor asks for: the swatches an import finds, the check
/// of the sample, and of every palette a draft of it and its unchanged check,
/// "palette/draft--paper.json" and "palette/check--paper.json".
pub(crate) fn write_palette(out_dir: &Path) {
    let folder = out_dir.join("palette");
    fs::create_dir_all(&folder).expect("create the palette folder");
    let swatches = palette::parse(SAMPLE_PALETTE).expect("read the sample palette");
    fs::write(
        folder.join("swatches.json"),
        serde_json::to_string(&swatches).expect("serialise the swatches"),
    )
    .expect("write the swatches");
    fs::write(
        folder.join("check.json"),
        serde_json::to_string(&sample_palette()).expect("serialise the check"),
    )
    .expect("write the check");
    for theme in fixture_themes() {
        let draft = palette::draft_from(&theme, &theme.name);
        let unchanged = palette::check_copy(&draft, &theme.id, &theme).expect("check the draft");
        fs::write(
            folder.join(format!("check--{}.json", theme.id)),
            serde_json::to_string(&unchanged).expect("serialise the check"),
        )
        .expect("write the check");
        fs::write(
            folder.join(format!("draft--{}.json", theme.id)),
            serde_json::to_string(&draft).expect("serialise the draft"),
        )
        .expect("write the draft");
    }
}
