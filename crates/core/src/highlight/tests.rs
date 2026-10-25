use super::*;

const WESTERMARKT: (f64, f64) = (120_900.0, 487_400.0);

fn highlight(shape: HighlightShape) -> Highlight {
    Highlight {
        id: "highlight-1".to_string(),
        label: "Plot".to_string(),
        shape,
        outline: Outline::Solid,
        colour: DEFAULT_COLOUR.to_string(),
        dims_rest: false,
        shows_area: false,
    }
}

fn rectangle(width: f64, height: f64) -> HighlightShape {
    HighlightShape::Rectangle {
        min: [WESTERMARKT.0, WESTERMARKT.1],
        max: [WESTERMARKT.0 + width, WESTERMARKT.1 + height],
    }
}

#[test]
fn a_rectangle_measures_its_width_times_its_height() {
    assert!((rectangle(20.0, 15.0).area_m2() - 300.0).abs() < 1e-6);
}

#[test]
fn a_circle_measures_exactly_and_its_ring_comes_close() {
    let circle = HighlightShape::Circle {
        center: [WESTERMARKT.0, WESTERMARKT.1],
        radius_m: 10.0,
    };
    let exact = std::f64::consts::PI * 100.0;
    assert!((circle.area_m2() - exact).abs() < 1e-9);
    let ring = circle.polygon_from(WESTERMARKT).unsigned_area();
    assert!((ring - exact).abs() / exact < 0.001, "{ring}");
}

#[test]
fn a_polygon_has_its_middle_at_its_centroid() {
    let triangle = HighlightShape::Polygon {
        points: vec![[0.0, 0.0], [30.0, 0.0], [0.0, 30.0]],
    };
    let (x, y) = triangle.middle();
    assert!((x - 10.0).abs() < 1e-9 && (y - 10.0).abs() < 1e-9);
    assert!((triangle.area_m2() - 450.0).abs() < 1e-9);
}

#[test]
fn a_tiny_or_broken_highlight_is_refused_with_the_reason() {
    let tiny = highlight(rectangle(1.0, 1.0)).check().unwrap_err();
    assert!(tiny.to_string().contains("at least 4 m²"), "{tiny}");

    let two_points = highlight(HighlightShape::Polygon {
        points: vec![[0.0, 0.0], [10.0, 0.0]],
    });
    assert!(two_points.check().unwrap_err().to_string().contains("3 to"));

    let flipped = highlight(HighlightShape::Rectangle {
        min: [10.0, 10.0],
        max: [0.0, 20.0],
    });
    assert!(flipped
        .check()
        .unwrap_err()
        .to_string()
        .contains("south-west"));

    let mut odd = highlight(rectangle(10.0, 10.0));
    odd.colour = "teal".to_string();
    assert!(odd.check().unwrap_err().to_string().contains("@accent"));
    odd.colour = "#1F7A8C".to_string();
    assert!(odd.check().is_ok());

    let mut long = highlight(rectangle(10.0, 10.0));
    long.label = "x".repeat(MAX_LABEL_CHARS + 1);
    assert!(long.check().unwrap_err().to_string().contains("80"));
}

#[test]
fn the_caption_adds_the_area_only_when_asked() {
    let mut plot = highlight(rectangle(20.0, 20.6));
    assert_eq!(plot.caption(), "Plot");
    plot.shows_area = true;
    assert_eq!(plot.caption(), "Plot · 412 m²");
    plot.label.clear();
    assert_eq!(plot.caption(), "412 m²");
    assert_eq!(plot.legend_label(), UNLABELLED);
    assert_eq!(area_text(13_000.0), "1.3 ha");
}

#[test]
fn a_shape_goes_to_a_sheet_area_and_back_to_the_same_place() {
    let shapes = [
        rectangle(40.0, 25.0),
        HighlightShape::Circle {
            center: [WESTERMARKT.0, WESTERMARKT.1],
            radius_m: 30.0,
        },
    ];
    for shape in shapes {
        let back = HighlightShape::from_area(&shape.to_area());
        let (before, after) = (shape.ring(), back.ring());
        // A rectangle comes back as its four corners.
        assert_eq!(before.len(), after.len());
        for ((x1, y1), (x2, y2)) in before.iter().zip(&after) {
            assert!((x1 - x2).abs() < 0.01 && (y1 - y2).abs() < 0.01);
        }
    }
}

#[test]
fn the_next_id_skips_ids_in_use() {
    let mut taken = highlight(rectangle(10.0, 10.0));
    taken.id = "highlight-2".to_string();
    assert_eq!(next_highlight_id(&[taken]), "highlight-3");
    assert_eq!(next_highlight_id(&[]), "highlight-1");
}

#[test]
fn an_old_highlight_without_its_style_fields_opens_with_the_defaults() {
    let json = r#"{"id": "highlight-1", "shape": {"kind": "circle", "center": [120900, 487400], "radius_m": 12}}"#;
    let opened: Highlight = serde_json::from_str(json).unwrap();
    assert_eq!(opened.outline, Outline::Solid);
    assert_eq!(opened.colour, DEFAULT_COLOUR);
    assert!(!opened.dims_rest && !opened.shows_area && opened.label.is_empty());
}
