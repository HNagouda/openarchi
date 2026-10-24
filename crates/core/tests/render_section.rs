mod render_support;

use openarchi_core::area::Area;
use openarchi_core::crs::rd_to_wgs84;
use openarchi_core::export::{export, ExportFormat};
use openarchi_core::project::Look;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::scene::Scene;
use openarchi_core::section::SectionLine;
use openarchi_core::sources::raster::Grid;
use openarchi_core::view::ViewMode;
use render_support::{diagram, look, scene, scene_with_neighbourhood, theme, ORIGIN};

/// West to east through the row of five test buildings, which stand at y 80 to 100.
fn through_the_row() -> SectionLine {
    SectionLine {
        start: (ORIGIN.0 - 150.0, ORIGIN.1 + 90.0),
        end: (ORIGIN.0 + 150.0, ORIGIN.1 + 90.0),
        depth_m: 60.0,
    }
}

fn look_in(mode: ViewMode) -> Look {
    let mut look = look();
    look.view.mode = mode;
    look.section = Some(through_the_row());
    look
}

fn svg(scene: &Scene, look: &Look, theme_id: &str) -> String {
    let diagram = diagram(&look.diagram_id);
    let theme = theme(theme_id);
    let input = RenderInput {
        scene,
        diagram: &diagram,
        theme: &theme,
        look,
        pins: &[],
    };
    render_svg(&input).expect("render the section")
}

/// Ground rising 1 m every 10 m eastwards, in 10 m cells over the whole test site.
fn sloping_terrain() -> Grid {
    let size = 60;
    let mut values = Vec::with_capacity(size * size);
    for _ in 0..size {
        for column in 0..size {
            values.push(column as f32);
        }
    }
    Grid {
        west: -300.0,
        north: 300.0,
        cell_m: 10.0,
        width: size,
        height: size,
        values,
    }
}

#[test]
fn every_building_the_line_crosses_is_drawn_cut_in_every_theme() {
    let scene = scene();
    for theme_id in [
        "paper",
        "ink",
        "midnight",
        "mono",
        "blueprint",
        "pastel",
        "polder",
        "terracotta",
    ] {
        let svg = svg(&scene, &look_in(ViewMode::Section), theme_id);
        let cut = svg.split(r#"id="section-cut""#).nth(1).unwrap();
        let cut = &cut[..cut.find("</g>").unwrap()];
        assert_eq!(cut.matches("<rect").count(), 5, "{theme_id}");
        // Some palettes set headings in capitals.
        let lower = svg.to_lowercase();
        assert!(lower.contains("section a\u{2013}a\u{2032}"), "{theme_id}");
    }
}

#[test]
fn a_scene_without_terrain_draws_flat_ground_and_says_so() {
    let svg = svg(&scene(), &look_in(ViewMode::Section), "paper");
    assert!(svg.contains("Ground drawn flat"));
    assert!(!svg.contains(">NAP<"));
}

#[test]
fn kept_terrain_gives_the_profile_and_levels_above_nap() {
    let scene = Scene {
        terrain: Some(sloping_terrain()),
        ..scene()
    };
    let svg = svg(&scene, &look_in(ViewMode::Section), "paper");
    assert!(svg.contains("Heights in metres above NAP"));
    assert!(svg.contains("m NAP"));
    // A measured building height is labelled over its cut.
    assert!(svg.contains("32.0 m"));
}

#[test]
fn a_section_has_no_pins_north_arrow_or_ground_points() {
    let svg = svg(&scene(), &look_in(ViewMode::Section), "paper");
    assert!(svg.contains(r#"data-ground-points="""#));
    assert!(!svg.contains("north-arrow"));
    assert!(!svg.contains(r#"id="section-line""#));
}

#[test]
fn the_plan_and_the_axonometric_show_the_cut_marker() {
    for mode in [ViewMode::Plan, ViewMode::Axonometric] {
        let svg = svg(&scene(), &look_in(mode), "paper");
        assert!(svg.contains(r#"id="section-line""#), "{mode:?}");
        assert!(svg.contains("A\u{2032}"), "{mode:?}");
    }
}

#[test]
fn the_section_view_without_a_line_is_a_readable_error() {
    let mut look = look_in(ViewMode::Section);
    look.section = None;
    let diagram = diagram(&look.diagram_id);
    let theme = theme("paper");
    let scene = scene();
    let input = RenderInput {
        scene: &scene,
        diagram: &diagram,
        theme: &theme,
        look: &look,
        pins: &[],
    };
    let message = render_svg(&input).unwrap_err().to_string();
    assert!(message.contains("section line"), "{message}");
}

#[test]
fn a_section_on_a_sheet_area_cuts_what_lies_outside_it_as_neighbourhood() {
    let mut look = look_in(ViewMode::Section);
    look.sheet.show_context = true;
    look.selection = Some(Area::Circle {
        center: rd_to_wgs84(ORIGIN.0 - 50.0, ORIGIN.1 + 90.0),
        radius_m: 40.0,
    });
    let svg = svg(&scene_with_neighbourhood(), &look, "paper");
    assert!(svg.contains("Neighbourhood beyond"));
    let cut = svg.split(r#"id="section-cut""#).nth(1).unwrap();
    // The area's own buildings, and the neighbourhood's whole buildings the line also crosses.
    assert!(cut[..cut.find("</g>").unwrap()].matches("<rect").count() >= 5);
}

#[test]
fn a_section_exports_as_pdf_and_png() {
    let look = look_in(ViewMode::Section);
    let diagram = diagram(&look.diagram_id);
    let theme = theme("paper");
    let scene = scene();
    let input = RenderInput {
        scene: &scene,
        diagram: &diagram,
        theme: &theme,
        look: &look,
        pins: &[],
    };
    let folder = tempfile::TempDir::new().unwrap();
    for (format, name) in [
        (ExportFormat::Pdf, "section.pdf"),
        (ExportFormat::Png, "section.png"),
    ] {
        let path = folder.path().join(name);
        export(&input, format, 72, &path).expect("export the section");
        assert!(std::fs::metadata(&path).unwrap().len() > 1000, "{name}");
    }
}
