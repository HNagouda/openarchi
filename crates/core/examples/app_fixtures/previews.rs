use std::fs;
use std::path::Path;

use openarchi_core::catalog::builtin_diagrams;
use openarchi_core::export::laser::{self, LaserFormat, LaserSettings};
use openarchi_core::project::Look;
use openarchi_core::ratios;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::scene::Scene;
use openarchi_core::section::{SectionLine, DEFAULT_DEPTH_M};
use openarchi_core::sheet::Sheet;
use openarchi_core::view::{View, ViewMode};

use crate::palettes::fixture_themes;

/// Half the section line the mock's section preview is cut along.
const SECTION_HALF_LENGTH_M: f64 = 100.0;

/// The ratios in every theme, as `get_ratios` returns them, "ratios/paper.json".
pub(crate) fn write_ratios(scene: &Scene, out_dir: &Path) {
    let folder = out_dir.join("ratios");
    fs::create_dir_all(&folder).expect("create the ratios folder");
    for theme in fixture_themes() {
        let ratios = ratios::compute(scene, &theme).expect("work out the ratios");
        fs::write(
            folder.join(format!("{}.json", theme.id)),
            serde_json::to_string(&ratios).expect("serialise the ratios"),
        )
        .expect("write the ratios");
    }
}

/// What the laser export would write at each preset scale, with the dialog's
/// other defaults, "laser/1000.json"; the mock answers every setting with these.
pub(crate) fn write_laser_summaries(scene: &Scene, out_dir: &Path) {
    let folder = out_dir.join("laser");
    fs::create_dir_all(&folder).expect("create the laser folder");
    for scale in [500, 1000, 2500] {
        let settings = LaserSettings {
            scale,
            thickness_mm: laser::DEFAULT_THICKNESS_MM,
            bed_width_mm: laser::DEFAULT_BED_MM.0,
            bed_height_mm: laser::DEFAULT_BED_MM.1,
            has_base_plate: true,
            has_buildings: true,
            has_contours: true,
            format: LaserFormat::Svg,
        };
        let summary = laser::summarize(scene, None, &settings).expect("plan the laser files");
        let text = serde_json::to_string(&summary).expect("serialise the summary");
        fs::write(folder.join(format!("{scale}.json")), text).expect("write the summary");
    }
}

/// The explode view with its default plates in every theme, "explode--paper",
/// so the mock can show plates to drag. It cannot reorder them; the app can.
pub(crate) fn write_explode_previews(scene: &Scene, sheet: &Sheet, out_dir: &Path) {
    let diagrams = builtin_diagrams();
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == "context")
        .expect("the context diagram");
    for theme in fixture_themes() {
        let look = Look {
            diagram_id: diagram.id.clone(),
            theme_id: theme.id.clone(),
            sheet: sheet.clone(),
            view: View {
                mode: ViewMode::Explode,
                ..View::default()
            },
            ..Look::default()
        };
        let input = RenderInput {
            scene,
            diagram,
            theme: &theme,
            look: &look,
            pins: &[],
            highlights: &[],
        };
        let svg = render_svg(&input).expect("render the explode preview");
        fs::write(
            out_dir.join(format!("previews/explode--{}.svg", theme.id)),
            svg,
        )
        .expect("write the explode preview");
    }
}

/// The section view in every theme, "section--paper", along one line west to
/// east through the site centre, which the mock shows whatever line is drawn.
/// The fixture predates kept terrain, so its ground is flat.
pub(crate) fn write_section_previews(scene: &Scene, sheet: &Sheet, out_dir: &Path) {
    let diagrams = builtin_diagrams();
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == "context")
        .expect("the context diagram");
    let (x, y) = scene.origin;
    let line = SectionLine {
        start: (x - SECTION_HALF_LENGTH_M, y),
        end: (x + SECTION_HALF_LENGTH_M, y),
        depth_m: DEFAULT_DEPTH_M,
        scale: None,
        exaggeration: 1,
    };
    for theme in fixture_themes() {
        let look = Look {
            diagram_id: diagram.id.clone(),
            theme_id: theme.id.clone(),
            sheet: sheet.clone(),
            view: View {
                mode: ViewMode::Section,
                ..View::default()
            },
            section: Some(line.clone()),
            ..Look::default()
        };
        let input = RenderInput {
            scene,
            diagram,
            theme: &theme,
            look: &look,
            pins: &[],
            highlights: &[],
        };
        let svg = render_svg(&input).expect("render the section preview");
        fs::write(
            out_dir.join(format!("previews/section--{}.svg", theme.id)),
            svg,
        )
        .expect("write the section preview");
    }
}
