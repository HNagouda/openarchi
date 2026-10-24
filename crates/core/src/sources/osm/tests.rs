use super::*;

const FIXTURE: &str = include_str!("../../../tests/fixtures/overpass-westermarkt.json");

fn groups(features: &[Feature]) -> Vec<(Layer, String)> {
    features
        .iter()
        .map(|feature| {
            (
                feature.layer,
                feature.attributes.group.clone().unwrap_or_default(),
            )
        })
        .collect()
}

#[test]
fn places_are_kept_by_group_and_street_furniture_is_dropped() {
    let osm = parse(FIXTURE.as_bytes()).unwrap();

    assert_eq!(
        groups(&osm.places),
        [
            (Layer::Business, "shop".to_string()),
            (Layer::Business, "food_drink".to_string()),
            (Layer::Facility, "culture".to_string()),
        ]
    );
    assert_eq!(
        osm.places[2].attributes.name.as_deref(),
        Some("Anne Frank Huis")
    );
    assert_eq!(osm.places[1].attributes.kind.as_deref(), Some("restaurant"));
}

#[test]
fn the_platforms_of_a_tram_stop_become_one_stop_with_its_lines() {
    let osm = parse(FIXTURE.as_bytes()).unwrap();

    assert_eq!(osm.stops.len(), 1);
    let stop = &osm.stops[0].attributes;
    assert_eq!(stop.name.as_deref(), Some("Westermarkt"));
    assert_eq!(stop.group.as_deref(), Some("tram"));
    assert_eq!(stop.lines, ["Tram 13", "Tram 17"]);
}

#[test]
fn motorways_and_ways_with_gaps_are_not_part_of_the_walk_network() {
    let osm = parse(FIXTURE.as_bytes()).unwrap();

    assert_eq!(osm.ways.len(), 2);
    assert_eq!(osm.ways[0].nodes, [1, 2]);
    // The pavement is walked but not drawn.
    assert!(osm.ways[0].is_drawn);
    assert!(!osm.ways[1].is_drawn);
}

#[test]
fn a_query_that_ran_out_of_time_is_an_error_not_a_partial_answer() {
    let body = r#"{"elements": [], "remark": "runtime error: Query timed out in \"query\" at line 3 after 181 seconds."}"#;
    let error = parse(body.as_bytes()).unwrap_err().to_string();
    assert!(error.contains("timed out"), "{error}");
}

#[test]
fn an_html_error_page_is_an_error() {
    assert!(parse(b"<html>server at capacity</html>").is_err());
}

#[test]
fn the_query_asks_for_every_kept_value_and_no_street_furniture() {
    let query = query(
        LonLat {
            lon: 4.884,
            lat: 52.3745,
        },
        1500,
    );

    assert!(query.contains("(around:1500,52.374500,4.884000)"));
    assert!(query.contains("restaurant|cafe"));
    assert!(query.contains("|parking)$"));
    assert!(!query.contains("bench"));
    assert!(query.contains("out geom qt;"));
}

#[test]
fn the_query_covers_a_walk_from_anywhere_in_the_tile() {
    // 1260 m of walk plus 177 m from the tile's middle to its corner, rounded up to tiles.
    assert_eq!(radius_for(300.0), 1500);
    assert_eq!(radius_for(1400.0), 1750);
}

#[test]
fn every_kept_place_carries_the_osm_credit_layers() {
    assert!(credit().layers.contains(&Layer::Business));
    assert!(credit().name.contains("OpenStreetMap contributors"));
}
