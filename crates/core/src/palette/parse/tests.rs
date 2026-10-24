use super::*;

/// The five colours every format below holds, in the palette's order.
const FIVE: [&str; 5] = ["#F79F79", "#F7D08A", "#E3F09B", "#87B6A7", "#5B5941"];
const NAMES: [&str; 5] = [
    "Tangerine Dream",
    "Jasmine",
    "Mindaro",
    "Cambridge blue",
    "Ebony",
];

fn hexes(swatches: &[Swatch]) -> Vec<&str> {
    swatches.iter().map(|swatch| swatch.hex.as_str()).collect()
}

fn names(swatches: &[Swatch]) -> Vec<&str> {
    swatches
        .iter()
        .map(|swatch| swatch.name.as_deref().unwrap_or(""))
        .collect()
}

fn error_of(text: &str) -> String {
    parse(text).unwrap_err().to_string()
}

#[test]
fn svg_swatches_come_left_to_right_by_x() {
    let svg = include_str!("../../../tests/fixtures/palettes/swatches.svg");
    assert_eq!(hexes(&parse(svg).unwrap()), FIVE);
}

#[test]
fn svg_skips_rects_without_a_colour_and_reads_style_fills() {
    let svg = r##"<svg><rect width="100%" fill="none"/><rect x="10" style="fill: #f7d08a"/><rect x='0' fill='#F79F79'/></svg>"##;
    assert_eq!(hexes(&parse(svg).unwrap()), ["#F79F79", "#F7D08A"]);
}

#[test]
fn comma_separated_codes_without_hashes() {
    let swatches = parse("f79f79,f7d08a,e3f09b,87b6a7,5b5941").unwrap();
    assert_eq!(hexes(&swatches), FIVE);
}

#[test]
fn codes_with_hashes_and_spaces() {
    let swatches = parse("#f79f79, #f7d08a, #e3f09b, #87b6a7, #5b5941").unwrap();
    assert_eq!(hexes(&swatches), FIVE);
}

#[test]
fn one_code_per_line() {
    let swatches = parse("#f79f79\n#f7d08a\r\n#e3f09b\n\n#87b6a7\n#5b5941\n").unwrap();
    assert_eq!(hexes(&swatches), FIVE);
}

#[test]
fn json_array_of_codes() {
    let swatches = parse(r##"["f79f79","f7d08a","e3f09b","87b6a7","5b5941"]"##).unwrap();
    assert_eq!(hexes(&swatches), FIVE);
}

#[test]
fn json_object_keeps_names_in_the_order_written() {
    let text = r##"{"Tangerine Dream":"f79f79","Jasmine":"f7d08a","Mindaro":"e3f09b","Cambridge blue":"87b6a7","Ebony":"5b5941"}"##;
    let swatches = parse(text).unwrap();
    assert_eq!(hexes(&swatches), FIVE);
    assert_eq!(names(&swatches), NAMES);
}

#[test]
fn extended_json_array_reads_hex_and_name() {
    let text = include_str!("../../../tests/fixtures/palettes/extended.json");
    let swatches = parse(text).unwrap();
    assert_eq!(hexes(&swatches), FIVE);
    assert_eq!(names(&swatches), NAMES);
}

#[test]
fn xml_palette_reads_hex_and_name() {
    let text = include_str!("../../../tests/fixtures/palettes/palette.xml");
    let swatches = parse(text).unwrap();
    assert_eq!(hexes(&swatches), FIVE);
    assert_eq!(names(&swatches), NAMES);
}

#[test]
fn xml_colour_without_hex_falls_back_to_channels() {
    let text =
        r##"<palette><color name="a" r="247" g="159" b="121"/><color hex="5b5941"/></palette>"##;
    assert_eq!(hexes(&parse(text).unwrap()), ["#F79F79", "#5B5941"]);
}

#[test]
fn link_ending_in_dashed_codes() {
    let text = "https://palettes.example/palette/f79f79-f7d08a-e3f09b-87b6a7-5b5941?ref=share";
    assert_eq!(hexes(&parse(text).unwrap()), FIVE);
}

#[test]
fn three_digit_codes_are_doubled() {
    assert_eq!(hexes(&parse("#fa7, 0c3").unwrap()), ["#FFAA77", "#00CC33"]);
}

#[test]
fn css_variables_fall_back_to_their_hashed_codes() {
    let css = "--tangerine: #f79f79ff; --jasmine: #f7d08a; --ebony: #5b5941;";
    // The 8 digit code has alpha and is not a palette colour.
    assert_eq!(hexes(&parse(css).unwrap()), ["#F7D08A", "#5B5941"]);
}

#[test]
fn ten_colours_fit_and_eleven_do_not() {
    let ten = ["abcdef"; 10].join(",");
    assert_eq!(parse(&ten).unwrap().len(), 10);
    let eleven = ["abcdef"; 11].join(",");
    assert!(
        error_of(&eleven).contains("at most 10"),
        "{}",
        error_of(&eleven)
    );
}

#[test]
fn one_colour_is_not_a_palette() {
    assert!(error_of("#f79f79").contains("at least 2"));
}

#[test]
fn junk_names_what_was_wrong() {
    assert!(error_of("").contains("nothing to read"));
    assert!(error_of("hello there").contains("\"hello\" is not a hex colour"));
    assert!(error_of("f79f79, banana").contains("\"banana\""));
    assert!(error_of("[1, 2]").contains("is not a hex colour"));
    assert!(error_of("{\"a\": ").contains("JSON could not be read"));
    assert!(error_of("[{\"name\": \"x\"}]").contains("no \"hex\""));
    assert!(error_of("<html><body>hi</body></html>").contains("neither"));
    assert!(error_of("<svg><circle fill=\"#fff\"/></svg>").contains("no <rect> swatches"));
    assert!(error_of("https://example.com/about").contains("does not end in hex codes"));
}

#[test]
fn names_are_cut_short_and_lose_control_characters() {
    let long = "x".repeat(200);
    let text = format!(r##"{{"{long}":"f79f79","a\u0007b":"5b5941"}}"##);
    let swatches = parse(&text).unwrap();
    assert_eq!(swatches[0].name.as_ref().unwrap().len(), MAX_NAME_CHARS);
    assert_eq!(swatches[1].name.as_deref(), Some("ab"));
}

#[test]
fn script_in_an_svg_is_never_kept() {
    let svg = r##"<svg><script>alert(1)</script><rect x="0" fill="#f79f79" onload="alert(2)"/><rect x="1" fill="#5b5941"/></svg>"##;
    let swatches = parse(svg).unwrap();
    assert_eq!(hexes(&swatches), ["#F79F79", "#5B5941"]);
    assert!(swatches.iter().all(|swatch| swatch.name.is_none()));
}

#[test]
fn oversized_text_is_refused() {
    let huge = "a".repeat(MAX_INPUT_BYTES + 1);
    assert!(error_of(&huge).contains("too large"));
}
