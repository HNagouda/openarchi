//! Protocol tests for the site edit tools and the edits in get_project.

use serde_json::{json, Value};

mod common;

use common::{fetched_westermarkt, Server};

/// A street tree in the recorded Westermarkt scene, and where it stands in RD New metres.
const TREE: &str = "G0363.02ede7c08cac49f8b85ba4b4cd0b84f2";
const TREE_RD: [f64; 2] = [120_812.77, 487_373.61];
/// A point inside a building of the same scene.
const BUILDING: &str = "0363100012164990";
const BUILDING_RD: [f64; 2] = [120_851.23, 487_290.95];
/// The scene's site centre, which is inside the site.
const CENTRE_RD: [f64; 2] = [120_733.0, 487_521.66];

fn ids(edits: &Value) -> Vec<&str> {
    edits
        .as_array()
        .unwrap()
        .iter()
        .map(|thing| thing["id"].as_str().unwrap())
        .collect()
}

#[test]
fn a_tree_is_found_removed_listed_and_restored() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);

    let picked = server.call_ok("pick_on_site", json!({ "id": id, "rd": TREE_RD }));
    assert_eq!(picked["found"]["id"], json!(TREE), "{picked}");
    assert_eq!(picked["found"]["layer"], json!("tree"));

    let removed = server.call_ok("remove_from_site", json!({ "id": id, "thing": TREE }));
    assert_eq!(ids(&removed["edits"]), vec![TREE]);
    assert_eq!(removed["edits"][0]["is_removed"], json!(true));
    let project = server.call_ok("get_project", json!({ "id": id }));
    assert_eq!(ids(&project["edits"]), vec![TREE]);
    let preview = server.call("render_preview", json!({ "id": id, "width_px": 400 }));
    assert_eq!(preview["isError"], json!(false), "{preview}");

    let restored = server.call_ok("restore_on_site", json!({ "id": id, "thing": TREE }));
    assert_eq!(restored["edits"], json!([]));
    let again = server.call_err("restore_on_site", json!({ "id": id, "thing": TREE }));
    assert!(again.contains("neither removed nor moved"), "{again}");
}

#[test]
fn a_building_is_found_by_a_point_inside_it_and_cannot_be_moved() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);

    let picked = server.call_ok("pick_on_site", json!({ "id": id, "rd": BUILDING_RD }));
    assert_eq!(picked["found"]["id"], json!(BUILDING), "{picked}");
    assert!(picked["found"]["outline"][0].as_array().unwrap().len() > 3);

    let moved = server.call_err(
        "move_tree",
        json!({ "id": id, "tree": BUILDING, "rd": CENTRE_RD }),
    );
    assert!(
        moved.contains("cannot") || moved.contains("not moved"),
        "{moved}"
    );
    let unknown = server.call_err(
        "remove_from_site",
        json!({ "id": id, "thing": "nothing-here" }),
    );
    assert!(unknown.contains("pick_on_site"), "{unknown}");
}

#[test]
fn a_tree_moves_inside_the_site_and_a_planted_one_can_go_again() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);

    let moved = server.call_ok(
        "move_tree",
        json!({ "id": id, "tree": TREE, "rd": CENTRE_RD }),
    );
    assert_eq!(moved["edits"][0]["at"], json!(CENTRE_RD));
    assert_eq!(
        moved["edits"][0]["surveyed_at"][0]
            .as_f64()
            .unwrap()
            .round(),
        TREE_RD[0].round()
    );
    let outside = server.call_err(
        "move_tree",
        json!({ "id": id, "tree": TREE, "rd": [CENTRE_RD[0] + 1000.0, CENTRE_RD[1]] }),
    );
    assert!(outside.contains("outside"), "{outside}");

    let planted = server.call_ok(
        "plant_tree",
        json!({ "id": id, "rd": [CENTRE_RD[0] + 10.0, CENTRE_RD[1]] }),
    );
    assert_eq!(planted["tree"], json!("planted-1"));
    let gone = server.call_ok(
        "remove_from_site",
        json!({ "id": id, "thing": "planted-1" }),
    );
    assert_eq!(ids(&gone["edits"]), vec![TREE]);
}

#[test]
fn the_model_takes_away_the_trees_in_its_way_and_gives_them_back_when_it_goes() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);
    // A tree planted where the model will stand, and a building removed by hand.
    server.call_ok("plant_tree", json!({ "id": id, "rd": CENTRE_RD }));
    server.call_ok("remove_from_site", json!({ "id": id, "thing": BUILDING }));

    let imported = server.call_ok(
        "import_model",
        json!({ "id": id, "path": format!("{}/../core/tests/fixtures/models/gable-house.glb", env!("CARGO_MANIFEST_DIR")) }),
    );
    let trees = imported["model"]["covered_trees"].as_array().unwrap();
    assert!(
        trees.iter().any(|tree| tree["id"] == json!("planted-1")),
        "{imported}"
    );
    assert!(imported["model"]["replaced"]
        .as_array()
        .unwrap()
        .contains(&json!("planted-1")));

    server.call_ok("remove_model", json!({ "id": id }));
    let edits = server.call_ok("list_site_edits", json!({ "id": id }));
    let removed: Vec<&str> = edits["edits"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|thing| thing["is_removed"] == json!(true))
        .map(|thing| thing["id"].as_str().unwrap())
        .collect();
    assert_eq!(removed, vec![BUILDING]);
}
