mod render_support;

use openarchi_core::export::{export, render_png, ExportFormat};
use openarchi_core::render::RenderInput;
use render_support::{diagram, look, scene, theme, ORIGIN, SITE_RADIUS_M};

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
/// A3 landscape in PDF points: 420 mm and 297 mm at 72 points to the inch.
const A3_LANDSCAPE_PT: (f64, f64) = (1190.55, 841.89);

fn export_to_bytes(format: ExportFormat, dpi: u32) -> Vec<u8> {
    let scene = scene();
    let (diagram, theme, look) = (diagram("context"), theme("paper"), look());
    let input = RenderInput {
        scene: &scene,
        diagram: &diagram,
        theme: &theme,
        look: &look,
        pins: &[],
        highlights: &[],
        model: None,
    };
    let folder = tempfile::tempdir().expect("temp folder");
    let path = folder.path().join(format!("sheet.{}", format.extension()));

    export(&input, format, dpi, &path).expect("export");
    std::fs::read(&path).expect("read the export back")
}

/// Width and height from the PNG header.
fn png_size(bytes: &[u8]) -> (u32, u32) {
    let number =
        |start: usize| u32::from_be_bytes(bytes[start..start + 4].try_into().expect("four bytes"));
    (number(16), number(20))
}

/// DXF as (group code, value) pairs.
fn dxf_pairs(text: &str) -> Vec<(i32, String)> {
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len() % 2, 0, "DXF is pairs of lines");
    lines
        .chunks(2)
        .map(|pair| {
            (
                pair[0].trim().parse().expect("group code"),
                pair[1].to_string(),
            )
        })
        .collect()
}

#[test]
fn png_thumbnail_has_the_requested_width_and_sheet_proportions() {
    let scene = scene();
    let (diagram, theme, look) = (diagram("context"), theme("paper"), look());
    let input = RenderInput {
        scene: &scene,
        diagram: &diagram,
        theme: &theme,
        look: &look,
        pins: &[],
        highlights: &[],
        model: None,
    };

    let png = render_png(&input, 840).expect("png");

    assert!(png.starts_with(PNG_SIGNATURE));
    assert_eq!(png_size(&png), (840, 594));
}

#[test]
fn png_export_is_rendered_at_the_requested_dpi() {
    // 420 mm at 100 dpi is 1654 px.
    let png = export_to_bytes(ExportFormat::Png, 100);

    assert_eq!(png_size(&png), (1654, 1169));
}

#[test]
fn svg_export_is_the_rendered_sheet() {
    let svg = String::from_utf8(export_to_bytes(ExportFormat::Svg, 0)).expect("utf-8");

    assert!(svg.starts_with("<?xml") && svg.trim_end().ends_with("</svg>"));
}

#[test]
fn pdf_page_is_the_paper_size_and_embeds_the_bundled_font() {
    let pdf = export_to_bytes(ExportFormat::Pdf, 0);
    let text = String::from_utf8_lossy(&pdf);

    assert!(pdf.starts_with(b"%PDF-"));
    assert!(text.trim_end().ends_with("%%EOF"));
    let media_box = text
        .split("/MediaBox [")
        .nth(1)
        .expect("a page size")
        .split(']')
        .next()
        .unwrap_or_default();
    let numbers: Vec<f64> = media_box
        .split_whitespace()
        .map(|n| n.parse().expect("number"))
        .collect();
    assert!(
        (numbers[2] - A3_LANDSCAPE_PT.0).abs() < 0.5
            && (numbers[3] - A3_LANDSCAPE_PT.1).abs() < 0.5,
        "{numbers:?}"
    );
    assert!(
        text.contains("SourceSerif4"),
        "the theme's serif is embedded"
    );
}

#[test]
fn dxf_has_one_cad_layer_per_scene_layer() {
    let dxf = String::from_utf8(export_to_bytes(ExportFormat::Dxf, 0)).expect("utf-8");
    let pairs = dxf_pairs(&dxf);
    let entity_layers: Vec<&str> = pairs
        .windows(2)
        .filter(|window| {
            window[0] == (0, "POLYLINE".to_string()) || window[0] == (0, "CIRCLE".to_string())
        })
        .map(|window| window[1].1.as_str())
        .collect();

    for layer in [
        "OA-BUILDING",
        "OA-WATER",
        "OA-ROAD",
        "OA-RAIL-TRACK",
        "OA-TREE",
        "OA-SITE",
    ] {
        assert!(entity_layers.contains(&layer), "no entities on {layer}");
    }
    // Parcels are fetched but the context diagram does not draw them.
    assert!(!entity_layers.contains(&"OA-PARCEL"));
    assert_eq!(pairs.last(), Some(&(0, "EOF".to_string())));
    let sections = pairs.iter().filter(|pair| pair.1 == "SECTION").count();
    let ends = pairs.iter().filter(|pair| pair.1 == "ENDSEC").count();
    assert_eq!(sections, ends);
}

#[test]
fn dxf_coordinates_are_real_rd_new_metres() {
    let dxf = String::from_utf8(export_to_bytes(ExportFormat::Dxf, 0)).expect("utf-8");
    let pairs = dxf_pairs(&dxf);
    let xs: Vec<f64> = pairs
        .iter()
        .filter(|pair| pair.0 == 10)
        .filter_map(|pair| pair.1.parse().ok())
        .filter(|&x| x != 0.0)
        .collect();

    assert!(!xs.is_empty());
    for x in xs {
        assert!(
            (x - ORIGIN.0).abs() <= SITE_RADIUS_M + 1.0,
            "x {x} is not near the site"
        );
    }
}

#[test]
fn dxf_polygon_holes_are_their_own_closed_polylines() {
    let dxf = String::from_utf8(export_to_bytes(ExportFormat::Dxf, 0)).expect("utf-8");
    let pairs = dxf_pairs(&dxf);
    let water_polylines = pairs
        .windows(2)
        .filter(|window| window[0].1 == "POLYLINE" && window[1].1 == "OA-WATER")
        .count();

    assert_eq!(water_polylines, 2, "canal and island");
}

#[test]
fn an_export_refuses_a_path_that_does_not_end_in_its_format() {
    let scene = scene();
    let (diagram, theme, look) = (diagram("context"), theme("paper"), look());
    let input = RenderInput {
        scene: &scene,
        diagram: &diagram,
        theme: &theme,
        look: &look,
        pins: &[],
        highlights: &[],
        model: None,
    };
    let folder = tempfile::tempdir().expect("temp folder");
    let path = folder.path().join(".zshrc");

    assert!(export(&input, ExportFormat::Pdf, 150, &path).is_err());
    assert!(!path.exists());
}
