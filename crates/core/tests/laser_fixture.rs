//! Laser cutting files for the recorded Westermarkt site, read back and checked
//! the way a laser shop would: right colours, real millimetres, nothing overlapping.

use std::path::Path;

use openarchi_core::area::Area;
use openarchi_core::crs::rd_to_wgs84;
use openarchi_core::export::laser::{export, summarize, LaserFormat, LaserSettings, LaserSummary};
use openarchi_core::scene::Scene;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/westermarkt.scene.json"
);
/// The fixture site is a circle 300 m across each way from its centre.
const SITE_RADIUS_M: f64 = 300.0;
const BED: (f64, f64) = (600.0, 400.0);
/// Must match `MARGIN_MM` in the laser module.
const MARGIN_MM: f64 = 5.0;

fn scene() -> Scene {
    let text = std::fs::read_to_string(FIXTURE).expect("read the fixture");
    serde_json::from_str(&text).expect("parse the fixture")
}

fn settings(scale: u32) -> LaserSettings {
    LaserSettings {
        scale,
        thickness_mm: 2.0,
        bed_width_mm: BED.0,
        bed_height_mm: BED.1,
        has_base_plate: true,
        has_buildings: true,
        has_contours: true,
        format: LaserFormat::Svg,
    }
}

/// A 60 m circle around the site centre, a small block for quick tests.
fn block(scene: &Scene) -> Area {
    Area::Circle {
        center: rd_to_wgs84(scene.origin.0, scene.origin.1),
        radius_m: 60.0,
    }
}

/// Every path's colour, width and bounding box from one bed's SVG.
struct DrawnPath {
    stroke: String,
    width: String,
    bounds: (f64, f64, f64, f64),
}

fn paths(svg: &str) -> Vec<DrawnPath> {
    let mut found = Vec::new();
    for element in svg.split("<path ").skip(1) {
        let attribute = |name: &str| {
            let start = element.find(&format!("{name}=\"")).expect(name) + name.len() + 2;
            let end = start + element[start..].find('"').expect("closing quote");
            element[start..end].to_string()
        };
        let numbers: Vec<f64> = attribute("d")
            .split(['M', 'L', 'Z', ' '])
            .filter(|part| !part.is_empty())
            .map(|part| part.parse().expect("a number"))
            .collect();
        let mut bounds = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for point in numbers.chunks(2) {
            bounds = (
                bounds.0.min(point[0]),
                bounds.1.min(point[1]),
                bounds.2.max(point[0]),
                bounds.3.max(point[1]),
            );
        }
        found.push(DrawnPath {
            stroke: attribute("stroke"),
            width: attribute("stroke-width"),
            bounds,
        });
    }

    found
}

fn export_to(
    folder: &Path,
    settings: &LaserSettings,
    selection: Option<&Area>,
) -> (Vec<String>, LaserSummary) {
    let written = export(&scene(), selection, settings, folder, "westermarkt").expect("export");
    let texts = written
        .files
        .iter()
        .map(|file| std::fs::read_to_string(file).expect("read a bed back"))
        .collect();
    (texts, written.summary)
}

#[test]
fn every_bed_uses_only_red_hairline_cuts_and_blue_hairline_scores_in_millimetres() {
    let folder = tempfile::tempdir().unwrap();
    let scene = scene();
    let (beds, summary) = export_to(folder.path(), &settings(1000), Some(&block(&scene)));
    assert_eq!(beds.len(), summary.sheet_count);
    for bed in &beds {
        assert!(bed.contains("width=\"600mm\" height=\"400mm\" viewBox=\"0 0 600 400\""));
        assert!(!bed.contains("transform"));
        assert!(!bed.contains("<text"));
        for path in paths(bed) {
            assert!(
                path.stroke == "#FF0000" || path.stroke == "#0000FF",
                "{}",
                path.stroke
            );
            assert_eq!(path.width, "0.01");
        }
        assert!(!bed.contains("fill=\"#"), "nothing is filled");
    }
}

#[test]
fn pieces_stay_inside_the_bed_margin_and_never_overlap() {
    let folder = tempfile::tempdir().unwrap();
    let (beds, _) = export_to(folder.path(), &settings(1000), None);
    for bed in &beds {
        // Each piece writes a score path and a cut path; its cut outline bounds it.
        let cuts: Vec<(f64, f64, f64, f64)> = paths(bed)
            .into_iter()
            .filter(|path| path.stroke == "#FF0000")
            .map(|path| path.bounds)
            .collect();
        for (index, a) in cuts.iter().enumerate() {
            assert!(a.0 >= MARGIN_MM - 0.01 && a.1 >= MARGIN_MM - 0.01, "{a:?}");
            assert!(
                a.2 <= BED.0 - MARGIN_MM + 0.01 && a.3 <= BED.1 - MARGIN_MM + 0.01,
                "{a:?}"
            );
            for b in &cuts[index + 1..] {
                let apart = a.2 <= b.0 || b.2 <= a.0 || a.3 <= b.1 || b.3 <= a.1;
                assert!(apart, "{a:?} overlaps {b:?}");
            }
        }
    }
}

#[test]
fn every_building_layer_is_a_piece_and_the_plate_is_tiled_to_fit_the_bed() {
    let scene = scene();
    let summary = summarize(&scene, None, &settings(1000)).unwrap();
    // 600 m across at 1:1000 is 600 mm, too tall for a 400 mm bed.
    assert!((summary.plate_width_mm - 2.0 * SITE_RADIUS_M).abs() < 2.0);
    assert!(summary.plate_tiles >= 2, "{summary:?}");
    let buildings = summary.building_count + summary.dropped_buildings;
    assert!(buildings > 1000, "{buildings}");
    assert!(summary.piece_count >= summary.plate_tiles + summary.building_count);
    assert!(summary.has_contours);
    assert!(summary.material_m2 >= summary.pieces_m2);

    // At 1:2500 the whole plate fits one bed.
    let small = summarize(&scene, None, &settings(2500)).unwrap();
    assert_eq!(small.plate_tiles, 1);
    assert!(small.dropped_buildings > summary.dropped_buildings);
}

#[test]
fn a_sheet_area_makes_a_smaller_model() {
    let scene = scene();
    let whole = summarize(&scene, None, &settings(1000)).unwrap();
    let area = summarize(&scene, Some(&block(&scene)), &settings(1000)).unwrap();
    assert!(area.plate_width_mm < 125.0, "{area:?}");
    assert!(area.building_count < whole.building_count);
    assert_eq!(area.plate_tiles, 1);
}

#[test]
fn only_the_buildings_leaves_the_plate_out() {
    let scene = scene();
    let mut only_buildings = settings(1000);
    only_buildings.has_base_plate = false;
    let summary = summarize(&scene, Some(&block(&scene)), &only_buildings).unwrap();
    assert_eq!(summary.plate_tiles, 0);
    assert!(summary.piece_count >= summary.building_count);

    let mut nothing = only_buildings.clone();
    nothing.has_buildings = false;
    let error = summarize(&scene, None, &nothing).unwrap_err();
    assert!(error.to_string().contains("Nothing to cut"), "{error}");
}

#[test]
fn a_thicker_board_needs_fewer_pieces() {
    let scene = scene();
    let mut thick = settings(1000);
    thick.thickness_mm = 3.0;
    let thin = summarize(&scene, Some(&block(&scene)), &settings(1000)).unwrap();
    let thick = summarize(&scene, Some(&block(&scene)), &thick).unwrap();
    assert!(thick.piece_count < thin.piece_count);
}

#[test]
fn the_dxf_puts_cuts_and_scores_on_their_own_layers() {
    let folder = tempfile::tempdir().unwrap();
    let scene = scene();
    let mut dxf = settings(1000);
    dxf.format = LaserFormat::Dxf;
    let (beds, _) = export_to(folder.path(), &dxf, Some(&block(&scene)));
    let bed = &beds[0];
    assert!(bed.contains("\n  2\nCUT\n") && bed.contains("\n  2\nENGRAVE\n"));
    assert!(bed.contains("\n  8\nCUT\n") && bed.contains("\n  8\nENGRAVE\n"));
    assert!(folder.path().join("westermarkt-laser-1.dxf").exists());
}

#[test]
fn settings_out_of_range_are_refused_with_the_value() {
    let scene = scene();
    let mut wrong = settings(1000);
    wrong.thickness_mm = 0.0;
    let error = summarize(&scene, None, &wrong).unwrap_err().to_string();
    assert!(error.contains("0 mm"), "{error}");
    let error = summarize(&scene, None, &settings(20))
        .unwrap_err()
        .to_string();
    assert!(error.contains("1:20"), "{error}");
}
