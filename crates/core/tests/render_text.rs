mod render_support;

use openarchi_core::export::{export, ExportFormat};
use openarchi_core::project::Look;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::sheet::{Align, Paper, Position, TextBox, TextStyle};
use render_support::{diagram, look, scene, theme};

const MM_PER_PT: f64 = 25.4 / 72.0;

fn render(look: &Look) -> openarchi_core::Result<String> {
    let scene = scene();
    render_svg(&RenderInput {
        scene: &scene,
        diagram: &diagram("building-use"),
        theme: &theme("paper"),
        look,
        pins: &[],
    })
}

fn svg_for(look: &Look) -> String {
    render(look).expect("render")
}

/// The opening tag of the group for the text with this id.
fn text_group<'a>(svg: &'a str, id: &str) -> &'a str {
    let start = svg
        .find(&format!(r#"<g data-text="{id}""#))
        .unwrap_or_else(|| panic!("no text {id:?} in the sheet"));
    let end = start + svg[start..].find('>').expect("tag ends");
    &svg[start..=end]
}

fn attribute<'a>(tag: &'a str, name: &str) -> &'a str {
    let marker = format!(r#" {name}=""#);
    let start = tag
        .find(&marker)
        .unwrap_or_else(|| panic!("no {name} in {tag}"))
        + marker.len();
    let end = start + tag[start..].find('"').expect("value ends");
    &tag[start..end]
}

fn number(tag: &str, name: &str) -> f64 {
    attribute(tag, name).parse().expect("a number")
}

/// The part of the SVG inside the named top-level layer.
fn layer<'a>(svg: &'a str, id: &str) -> &'a str {
    let start = svg
        .find(&format!(r#"<g id="{id}""#))
        .unwrap_or_else(|| panic!("no layer {id:?}"));
    let end = svg[start..].find("</g>\n").expect("layer ends");
    &svg[start..start + end]
}

/// The y of the highest faint rule in the furniture: the one over the legend.
fn legend_rule_y(svg: &str) -> f64 {
    layer(svg, "furniture")
        .split("<path d=\"M")
        .skip(1)
        .filter(|path| path.contains(r#"opacity="0.35""#))
        .map(|path| {
            let y = path.split(['H', ' ']).nth(1).expect("rule y");
            y.parse::<f64>().expect("a number")
        })
        .fold(f64::INFINITY, f64::min)
}

fn text_box(id: &str, text: &str, style: TextStyle, position: Option<Position>) -> TextBox {
    TextBox {
        id: id.to_string(),
        text: text.to_string(),
        style,
        position,
    }
}

#[test]
fn title_in_the_layout_sits_in_the_furniture_with_its_anchor() {
    let svg = svg_for(&look());

    let title = text_group(layer(&svg, "furniture"), "title");
    assert_eq!(attribute(title, "data-align"), "left");
    assert!(number(title, "data-x") > 300.0, "{title}");
}

#[test]
fn moved_title_is_drawn_at_its_position_in_the_text_layer() {
    let mut look = look();
    look.sheet.title_position = Some(Position {
        x_mm: 40.0,
        y_mm: 30.0,
    });

    let svg = svg_for(&look);

    assert!(!layer(&svg, "furniture").contains(r#"data-text="title""#));
    let title = text_group(layer(&svg, "text"), "title");
    assert_eq!(number(title, "data-x"), 40.0);
    assert_eq!(number(title, "data-y"), 30.0);
}

#[test]
fn legend_moves_up_into_the_space_a_moved_title_left() {
    let before = svg_for(&look());
    let mut moved = look();
    moved.sheet.title_position = Some(Position {
        x_mm: 40.0,
        y_mm: 30.0,
    });
    moved.sheet.subtitle_position = Some(Position {
        x_mm: 40.0,
        y_mm: 50.0,
    });

    let after = svg_for(&moved);

    assert!(
        legend_rule_y(&after) + 10.0 < legend_rule_y(&before),
        "{} vs {}",
        legend_rule_y(&after),
        legend_rule_y(&before)
    );
}

#[test]
fn text_box_carries_family_weight_italic_size_and_alignment() {
    let mut look = look();
    let style = TextStyle {
        family: Some("Source Serif 4".to_string()),
        bold: Some(true),
        italic: Some(true),
        size_pt: Some(24.0),
        align: Some(Align::Centre),
        colour: Some("@accent".to_string()),
        opacity: Some(0.5),
    };
    look.sheet.texts.push(text_box(
        "text-1",
        "Westerkerk",
        style,
        Some(Position {
            x_mm: 100.0,
            y_mm: 80.0,
        }),
    ));

    let svg = svg_for(&look);

    let group = text_group(layer(&svg, "text"), "text-1");
    assert!(attribute(group, "font-family").contains("Source Serif 4"));
    assert_eq!(attribute(group, "font-weight"), "600");
    assert_eq!(attribute(group, "font-style"), "italic");
    assert_eq!(attribute(group, "text-anchor"), "middle");
    assert_eq!(attribute(group, "fill"), theme("paper").palette.accent);
    assert_eq!(attribute(group, "fill-opacity"), "0.5");
    assert!((number(group, "font-size") - 24.0 * MM_PER_PT).abs() < 0.01);
    assert_eq!(attribute(group, "data-size-pt"), "24");
}

#[test]
fn size_in_points_prints_the_same_on_every_paper() {
    let mut sizes = Vec::new();
    for paper in [Paper::A4, Paper::A1] {
        let mut look = look();
        look.sheet.paper = paper;
        look.sheet.title_style.size_pt = Some(30.0);
        let svg = svg_for(&look);
        sizes.push(number(text_group(&svg, "title"), "font-size"));
    }

    assert_eq!(sizes[0], sizes[1]);
}

#[test]
fn line_breaks_in_a_text_box_become_lines_and_blank_boxes_are_skipped() {
    let mut look = look();
    let at = Some(Position {
        x_mm: 50.0,
        y_mm: 50.0,
    });
    look.sheet.texts.push(text_box(
        "text-1",
        "One\nTwo\n\nFour",
        TextStyle::default(),
        at,
    ));
    look.sheet
        .texts
        .push(text_box("text-2", "  \n ", TextStyle::default(), at));

    let svg = svg_for(&look);

    let text_layer = layer(&svg, "text");
    let start = text_layer.find(r#"data-text="text-1""#).expect("text-1");
    let lines = text_layer[start..].split("</g>").next().expect("group");
    assert_eq!(lines.matches("<text ").count(), 3);
    assert!(!text_layer.contains(r#"data-text="text-2""#));
}

#[test]
fn centred_title_in_the_layout_anchors_at_the_middle_of_its_column() {
    let left = svg_for(&look());
    let mut look = look();
    look.sheet.title_style.align = Some(Align::Centre);

    let centred = svg_for(&look);

    let left_x = number(text_group(&left, "title"), "data-x");
    let centred_x = number(text_group(&centred, "title"), "data-x");
    assert!(centred_x > left_x + 20.0, "{left_x} vs {centred_x}");
    assert_eq!(
        attribute(text_group(&centred, "title"), "text-anchor"),
        "middle"
    );
}

#[test]
fn bad_text_colour_or_size_is_an_error_that_names_the_text() {
    let mut colour = look();
    colour.sheet.texts.push(text_box(
        "text-7",
        "Hello",
        TextStyle {
            colour: Some("red".to_string()),
            ..TextStyle::default()
        },
        None,
    ));
    let mut size = look();
    size.sheet.subtitle_style.size_pt = Some(0.0);

    let colour_error = render(&colour).unwrap_err().to_string();
    let size_error = render(&size).unwrap_err().to_string();

    assert!(colour_error.contains("text-7"), "{colour_error}");
    assert!(size_error.contains("subtitle"), "{size_error}");
}

#[test]
fn italic_and_bold_text_embeds_its_own_faces_in_the_pdf() {
    let scene = scene();
    let mut look = look();
    look.sheet.texts.push(text_box(
        "text-1",
        "Westerkerk",
        TextStyle {
            family: Some("IBM Plex Mono".to_string()),
            bold: Some(true),
            italic: Some(true),
            ..TextStyle::default()
        },
        Some(Position {
            x_mm: 60.0,
            y_mm: 60.0,
        }),
    ));
    let input = RenderInput {
        scene: &scene,
        diagram: &diagram("context"),
        theme: &theme("paper"),
        look: &look,
        pins: &[],
    };
    let folder = tempfile::tempdir().expect("temp folder");
    let path = folder.path().join("sheet.pdf");

    export(&input, ExportFormat::Pdf, 300, &path).expect("export");

    let pdf = std::fs::read(&path).expect("read the PDF");
    let text = String::from_utf8_lossy(&pdf);
    assert!(
        text.contains("IBMPlexMono-SemiBoldItalic"),
        "the bold italic face is not embedded"
    );
}
