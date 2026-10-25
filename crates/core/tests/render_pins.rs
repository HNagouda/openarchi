mod render_support;

use openarchi_core::area::Area;
use openarchi_core::crs::rd_to_wgs84;
use openarchi_core::export::{export, ExportFormat};
use openarchi_core::pin::{Pin, PinIcon, PinSize};
use openarchi_core::project::Look;
use openarchi_core::ratios;
use openarchi_core::render::{fitting_scale, render_svg, RenderInput};
use openarchi_core::selection;
use openarchi_core::view::ViewMode;
use render_support::{diagram, look, scene, theme, ORIGIN};

/// A pin `x`, `y` metres from the site centre.
fn pin(id: &str, icon: PinIcon, label: &str, x: f64, y: f64) -> Pin {
    Pin {
        id: id.to_string(),
        icon,
        label: label.to_string(),
        x: ORIGIN.0 + x,
        y: ORIGIN.1 + y,
        size: PinSize::Medium,
    }
}

fn pins() -> Vec<Pin> {
    vec![
        pin("pin-1", PinIcon::Star, "Old church", -90.0, 90.0),
        pin("pin-2", PinIcon::Number, "Entrance", 10.0, -40.0),
        pin("pin-3", PinIcon::Number, "", 50.0, 20.0),
        // Far outside the 300 m site.
        pin("pin-4", PinIcon::Flag, "Station", 2000.0, 0.0),
    ]
}

fn svg_with(look: &Look, pins: &[Pin]) -> String {
    let scene = scene();
    let (diagram, theme) = (diagram("context"), theme("paper"));
    render_svg(&RenderInput {
        scene: &scene,
        diagram: &diagram,
        theme: &theme,
        look,
        pins,
        highlights: &[],
    })
    .expect("render")
}

/// A circle `radius_m` around a point `x`, `y` metres from the site centre.
fn selection_at(x: f64, y: f64, radius_m: f64) -> Area {
    Area::Circle {
        center: rd_to_wgs84(ORIGIN.0 + x, ORIGIN.1 + y),
        radius_m,
    }
}

#[test]
fn pins_inside_the_site_are_drawn_as_groups_the_app_can_find() {
    let svg = svg_with(&look(), &pins());

    assert!(svg.contains(r#"inkscape:label="Pins""#));
    assert!(svg.contains(r#"data-pin="pin-1" data-icon="star" data-label="Old church""#));
    assert!(svg.contains(">Old church</text>"));
    assert!(
        !svg.contains(r#"data-pin="pin-4""#),
        "a pin outside the site is left off"
    );
}

#[test]
fn numbered_pins_count_from_one_in_project_order() {
    let svg = svg_with(&look(), &pins());
    let second = svg.find(r#"data-pin="pin-3""#).expect("pin-3 drawn");
    let group = &svg[second..second + svg[second..].find("</g>").unwrap()];

    assert!(group.contains(">2</text>"), "{group}");
}

/// The radius of the first circle in pin `id`'s group, for a dot.
fn dot_radius(svg: &str, id: &str) -> f64 {
    let start = svg.find(&format!(r#"data-pin="{id}""#)).unwrap();
    let at = start + svg[start..].find(r#" r=""#).unwrap() + 4;
    svg[at..at + svg[at..].find('"').unwrap()].parse().unwrap()
}

#[test]
fn a_larger_pin_draws_a_larger_marker_and_scales_its_label_less() {
    let mut small = pin("pin-1", PinIcon::Dot, "Kiosk", 0.0, 0.0);
    small.size = PinSize::Small;
    let mut large = pin("pin-2", PinIcon::Dot, "Hall", 40.0, 0.0);
    large.size = PinSize::ExtraLarge;
    let medium = pin("pin-3", PinIcon::Dot, "", -40.0, 0.0);

    let svg = svg_with(&look(), &[small, large, medium]);

    let ratio = dot_radius(&svg, "pin-2") / dot_radius(&svg, "pin-3");
    assert!(
        (ratio - PinSize::ExtraLarge.scale()).abs() < 0.01,
        "{ratio}"
    );
    assert!(dot_radius(&svg, "pin-1") < dot_radius(&svg, "pin-3"));
    // The label grows by the square root of the marker's scale, 1.41 for twice the size.
    assert!(
        svg.contains("scale(1.41"),
        "the label scales less than the marker"
    );
}

#[test]
fn a_sheet_that_hides_pins_draws_none() {
    let hidden = Look {
        hide_pins: true,
        ..look()
    };

    assert!(!svg_with(&hidden, &pins()).contains("data-pin="));
}

#[test]
fn pins_are_drawn_in_the_tilted_views_too() {
    for mode in [ViewMode::Axonometric, ViewMode::Explode] {
        let mut tilted = look();
        tilted.view.mode = mode;
        tilted.view.plates = vec!["context".to_string(), "figure-ground".to_string()];

        let svg = svg_with(&tilted, &pins());

        assert!(svg.contains(r#"data-pin="pin-2""#), "{mode:?}");
    }
}

#[test]
fn the_ground_points_turn_paper_back_into_rd_metres() {
    let svg = svg_with(&look(), &pins());
    let start = svg.find(r#"data-ground-points=""#).unwrap() + r#"data-ground-points=""#.len();
    let numbers: Vec<f64> = svg[start..start + svg[start..].find('"').unwrap()]
        .split(' ')
        .map(|number| number.parse().unwrap())
        .collect();
    let star = svg.find(r#"data-pin="pin-1""#).unwrap();
    let read = |name: &str| -> f64 {
        let at = star + svg[star..].find(&format!(r#"{name}=""#)).unwrap() + name.len() + 2;
        svg[at..at + svg[at..].find('"').unwrap()].parse().unwrap()
    };
    let (paper_x, paper_y) = (read("data-x"), read("data-y"));

    // In a plan, x grows east and y grows south, both at the sheet's scale.
    let metres_per_mm = (numbers[6] - numbers[2]) / (numbers[4] - numbers[0]);
    let rd_x = numbers[2] + (paper_x - numbers[0]) * metres_per_mm;
    let rd_y = numbers[3] - (paper_y - numbers[1]) * metres_per_mm;
    assert!((rd_x - (ORIGIN.0 - 90.0)).abs() < 0.05, "{rd_x}");
    assert!((rd_y - (ORIGIN.1 + 90.0)).abs() < 0.05, "{rd_y}");
}

#[test]
fn a_sub_selection_zooms_the_same_renderer_in_on_its_area() {
    let mut framed = Look {
        selection: Some(selection_at(-80.0, 80.0, 60.0)),
        ..look()
    };
    framed.sheet.scale = fitting_scale(&scene(), &framed).unwrap();
    let zoomed = svg_with(&framed, &pins());

    // A 120 m circle fills the drawing zone of an A3 at 1:500, the largest round scale it fits.
    assert_eq!(framed.sheet.scale, 500);
    assert!(zoomed.contains(r#"id="layer-building""#));
    assert!(zoomed.contains(r#"data-pin="pin-1""#));
    assert!(
        !zoomed.contains(r#"data-pin="pin-2""#),
        "a pin outside the selection is left off"
    );
}

#[test]
fn a_sub_selection_outside_the_site_fails_with_a_reason() {
    let outside = Look {
        selection: Some(selection_at(5000.0, 0.0, 60.0)),
        ..look()
    };
    let scene = scene();
    let (diagram, theme) = (diagram("context"), theme("paper"));
    let error = render_svg(&RenderInput {
        scene: &scene,
        diagram: &diagram,
        theme: &theme,
        look: &outside,
        pins: &[],
        highlights: &[],
    })
    .unwrap_err();

    assert!(error.to_string().contains("outside the fetched site"));
}

#[test]
fn ratios_of_a_sub_selection_describe_the_smaller_area() {
    let scene = scene();
    let theme = theme("paper");
    let selection = selection_at(-80.0, 80.0, 60.0);
    let framed = selection::framed(&scene, &selection).unwrap();

    let whole = ratios::compute(&scene, &theme).unwrap();
    let part = ratios::compute(&framed, &theme).unwrap();

    let circle = std::f64::consts::PI * 60.0 * 60.0;
    assert!((part.site_area_m2 - circle).abs() / circle < 0.01);
    assert!(part.building_count < whole.building_count);
}

#[test]
fn dxf_puts_pins_on_their_own_layer_with_their_labels() {
    let scene = scene();
    let (diagram, theme, look) = (diagram("context"), theme("paper"), look());
    let pins = pins();
    let input = RenderInput {
        scene: &scene,
        diagram: &diagram,
        theme: &theme,
        look: &look,
        pins: &pins,
        highlights: &[],
    };
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("sheet.dxf");
    export(&input, ExportFormat::Dxf, 0, &path).unwrap();
    let dxf = std::fs::read_to_string(&path).unwrap();

    assert!(dxf.contains("\nOA-PINS\n"));
    assert!(dxf.contains("\nOld church\n"));
    assert!(dxf.contains("\n1 Entrance\n"));
    assert!(
        !dxf.contains("\nStation\n"),
        "a pin outside the site is left out"
    );
}
