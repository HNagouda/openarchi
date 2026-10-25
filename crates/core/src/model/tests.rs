use std::path::PathBuf;

use geo::{Area, Geometry, Polygon};

use super::*;
use crate::area::{Area as SiteArea, LonLat};
use crate::scene::{Attributes, Feature, Layer, Scene};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/models")
        .join(name)
}

/// An empty site with its origin at a round RD New point.
fn site(features: Vec<Feature>) -> Scene {
    Scene {
        area: SiteArea::Circle {
            center: LonLat {
                lon: 4.8837,
                lat: 52.3747,
            },
            radius_m: 150.0,
        },
        crs: "EPSG:28992".to_string(),
        origin: (120_000.0, 487_000.0),
        boundary: geo::Rect::new((-150.0, -150.0), (150.0, 150.0)).to_polygon(),
        features,
        credits: Vec::new(),
        fetched_at: String::new(),
        warnings: Vec::new(),
        measured: Default::default(),
        walk: None,
        editions: Default::default(),
        context: Vec::new(),
        terrain: None,
        water_bed: None,
        water_levels: Vec::new(),
        solar: None,
        wind: None,
    }
}

fn building(id: &str, min: (f64, f64), max: (f64, f64)) -> Feature {
    Feature {
        layer: Layer::Building,
        geometry: Geometry::Polygon(geo::Rect::new(min, max).to_polygon()),
        attributes: Attributes {
            source_id: Some(id.to_string()),
            height_m: Some(10.0),
            ..Default::default()
        },
    }
}

fn footprint_area(polygons: &[Polygon<f64>]) -> f64 {
    polygons.iter().map(|polygon| polygon.unsigned_area()).sum()
}

fn assert_size(size: [f64; 3], expected: [f64; 3]) {
    for axis in 0..3 {
        assert!(
            (size[axis] - expected[axis]).abs() < 1e-3,
            "size {size:?}, expected {expected:?}"
        );
    }
}

#[test]
fn an_obj_in_millimetres_is_read_in_metres_with_z_up() {
    let (mesh, unit, up) = read(&fixture("gable-house-mm.obj"), None, None).unwrap();

    assert_eq!(unit, Unit::Millimetre);
    assert_eq!(up, UpAxis::Z);
    assert_size(mesh.size(), [8.0, 10.0, 9.0]);
}

#[test]
fn a_glb_with_y_up_reads_as_the_same_house() {
    let (glb, unit, up) = read(&fixture("gable-house.glb"), None, None).unwrap();
    let (obj, _, _) = read(&fixture("gable-house-mm.obj"), None, None).unwrap();

    assert_eq!((unit, up), (Unit::Metre, UpAxis::Y));
    assert_size(glb.size(), obj.size());
    assert_eq!(glb.triangles.len(), obj.triangles.len());
    let glb_area = footprint_area(&faces::shape(&glb).footprint);
    let obj_area = footprint_area(&faces::shape(&obj).footprint);
    assert!((glb_area - 80.0).abs() < 1e-3 && (obj_area - 80.0).abs() < 1e-3);
}

#[test]
fn a_model_stands_on_zero_centred_on_its_plan() {
    let (mesh, _, _) = read(&fixture("gable-house-mm.obj"), None, None).unwrap();

    let lowest = mesh
        .positions
        .iter()
        .map(|p| p[2])
        .fold(f64::INFINITY, f64::min);
    let west = mesh
        .positions
        .iter()
        .map(|p| p[0])
        .fold(f64::INFINITY, f64::min);
    let south = mesh
        .positions
        .iter()
        .map(|p| p[1])
        .fold(f64::INFINITY, f64::min);
    assert!(lowest.abs() < 1e-9);
    assert!((west + 4.0).abs() < 1e-9 && (south + 5.0).abs() < 1e-9);
}

#[test]
fn a_y_up_obj_read_as_y_up_is_an_l_shaped_block() {
    let (mesh, unit, _) = read(&fixture("l-block-y-up.obj"), None, Some(UpAxis::Y)).unwrap();
    let shape = faces::shape(&mesh);

    assert_eq!(unit, Unit::Metre);
    assert_size(mesh.size(), [30.0, 30.0, 12.0]);
    assert!((footprint_area(&shape.footprint) - 500.0).abs() < 1e-3);
    // Floor and roof each join their two quads: two faces and six walls.
    assert_eq!(shape.faces.len(), 8);
}

#[test]
fn a_unit_chosen_by_the_user_wins_over_the_guess() {
    let (mesh, unit, _) =
        read(&fixture("gable-house-mm.obj"), Some(Unit::Centimetre), None).unwrap();

    assert_eq!(unit, Unit::Centimetre);
    assert_size(mesh.size(), [80.0, 100.0, 90.0]);
}

#[test]
fn a_model_read_again_matches_reading_the_file_that_way() {
    let path = fixture("l-block-y-up.obj");
    let scene = site(Vec::new());
    let as_guessed = import(&path, None, None, &scene, None).unwrap();
    let (direct, _, _) = read(&path, Some(Unit::Centimetre), Some(UpAxis::Y)).unwrap();

    let reread = as_guessed.reread(Unit::Centimetre, UpAxis::Y);

    assert_eq!(reread.mesh.triangles, direct.triangles);
    for (a, b) in reread.mesh.positions.iter().zip(&direct.positions) {
        assert!(
            (0..3).all(|axis| (a[axis] - b[axis]).abs() < 1e-9),
            "{a:?} {b:?}"
        );
    }
    assert_eq!(reread.placement, as_guessed.placement);
}

#[test]
fn coplanar_triangles_merge_into_one_face_each() {
    let (mesh, _, _) = read(&fixture("gable-house.glb"), None, None).unwrap();
    let shape = faces::shape(&mesh);

    // Floor, two gables, two walls and two roof planes.
    assert_eq!(shape.faces.len(), 7);
    assert!((shape.height - 9.0).abs() < 1e-3);
    for face in &shape.faces {
        let length = faces::dot(face.normal, face.normal).sqrt();
        assert!((length - 1.0).abs() < 1e-9);
    }
}

#[test]
fn a_packed_mesh_unpacks_to_the_same_triangles() {
    let (mesh, _, _) = read(&fixture("l-block-y-up.obj"), None, Some(UpAxis::Y)).unwrap();

    let unpacked = packed_mesh::unpack(&packed_mesh::pack(&mesh).unwrap()).unwrap();

    assert_eq!(unpacked.triangles, mesh.triangles);
    for (a, b) in unpacked.positions.iter().zip(&mesh.positions) {
        assert!((0..3).all(|axis| (a[axis] - b[axis]).abs() < 1e-5));
    }
}

#[test]
fn a_damaged_packed_mesh_is_refused_with_a_reason() {
    let error = packed_mesh::unpack("bm90IHpzdGQ=").unwrap_err();

    assert!(error.to_string().contains("the stored model is damaged"));
}

#[test]
fn ifc_and_unknown_files_are_refused_with_what_is_read() {
    let ifc = read(&PathBuf::from("house.ifc"), None, None).unwrap_err();
    let skp = read(&PathBuf::from("house.skp"), None, None).unwrap_err();

    assert!(ifc.to_string().contains("Save As"));
    assert!(skp.to_string().contains(READABLE_FORMATS));
}

#[test]
fn an_imported_model_stands_at_the_site_centre_and_replaces_what_it_covers() {
    let scene = site(vec![
        // Wholly under the 8 by 10 m house.
        building("under", (-3.0, -4.0), (3.0, 4.0)),
        // Only a metre of it under the house's east wall.
        building("beside", (3.0, -4.0), (13.0, 4.0)),
        building("away", (40.0, 40.0), (50.0, 50.0)),
    ]);

    let model = import(&fixture("gable-house-mm.obj"), None, None, &scene, None).unwrap();

    assert_eq!(model.placement, Placement::centre_of(&scene));
    assert_eq!(model.replaced, vec!["under".to_string()]);
    assert_eq!(model.file_name, "gable-house-mm.obj");
}

#[test]
fn a_quarter_turn_lays_a_long_model_the_other_way() {
    let scene = site(Vec::new());
    let (mesh, unit, up) = read(&fixture("gable-house-mm.obj"), None, None).unwrap();
    let mut placement = Placement::centre_of(&scene);
    placement.x += 100.0;
    placement.rotation_deg = 90.0;
    let model = Model::new(String::new(), unit, up, mesh, placement);

    let footprint = footprint_on_site(&model, &scene);
    let rect = geo::BoundingRect::bounding_rect(&geo::MultiPolygon::new(footprint)).unwrap();

    // 8 m east-west and 10 m north-south before the turn, centred 100 m east.
    assert!((rect.width() - 10.0).abs() < 1e-6 && (rect.height() - 8.0).abs() < 1e-6);
    assert!((rect.center().x - 100.0).abs() < 1e-6 && rect.center().y.abs() < 1e-6);
}

#[test]
fn a_placed_model_keeps_its_faces_and_height_on_flat_ground() {
    let scene = site(Vec::new());
    let model = import(&fixture("gable-house.glb"), None, None, &scene, None).unwrap();

    let placed = placed(&model, &scene);

    assert_eq!(placed.faces.len(), 7);
    assert!((placed.height - 9.0).abs() < 1e-3);
}

#[test]
fn a_place_that_is_not_a_number_is_refused() {
    let mut placement = Placement::centre_of(&site(Vec::new()));
    placement.rotation_deg = f64::NAN;

    assert!(placement.check().is_err());
}

#[test]
fn the_analysed_scene_has_the_model_in_place_of_what_it_replaces() {
    let scene = site(vec![
        building("under", (-3.0, -4.0), (3.0, 4.0)),
        building("away", (40.0, 40.0), (50.0, 50.0)),
    ]);
    let model = import(&fixture("gable-house-mm.obj"), None, None, &scene, None).unwrap();

    let analysed = scene_with_model(&scene, &model);

    let heights: Vec<Option<f64>> = analysed
        .features
        .iter()
        .map(|feature| feature.attributes.height_m)
        .collect();
    assert_eq!(analysed.features.len(), 2);
    assert_eq!(
        analysed.features[0].attributes.source_id.as_deref(),
        Some("away")
    );
    assert!((heights[1].unwrap() - 9.0).abs() < 1e-6);
}

#[test]
fn the_model_takes_hours_of_sun_from_the_ground_behind_it() {
    let scene = site(Vec::new());
    let model = import(&fixture("gable-house-mm.obj"), None, None, &scene, None).unwrap();
    // On the shortest day an open site has close to eight hours everywhere.
    let shaded_area = |scene: &Scene| -> f64 {
        let grid = crate::sunlight::sun_hours(scene, 12, 21);
        crate::sunlight::bands(&grid, &[4.0])
            .iter()
            .filter(|band| band.attributes.value.is_some_and(|hours| hours < 4.0))
            .map(|band| band.geometry.unsigned_area())
            .sum()
    };

    assert_eq!(shaded_area(&scene), 0.0);
    assert!(shaded_area(&scene_with_model(&scene, &model)) > 50.0);
}
