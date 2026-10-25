use std::path::Path;

use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::export::{export, render_png, ExportFormat};
use openarchi_core::project::Look;
use openarchi_core::render::RenderInput;
use openarchi_core::scene::Scene;
use openarchi_core::sheet::{Align, Frame, Paper, Position, Sheet, TextBox, TextStyle};

use crate::SAMPLE_WIDTH_PX;

/// The same diagram on portrait, square and A4 sheets, and in each frame shape.
pub(crate) fn write_layouts(scene: &Scene, out_dir: &Path) {
    let diagrams = builtin_diagrams();
    let themes = builtin_themes();
    let variants = [
        ("a3-portrait", Paper::A3, false, 2500, Frame::Circle),
        ("square", Paper::Square, false, 3500, Frame::Circle),
        ("a4-landscape", Paper::A4, true, 4000, Frame::Circle),
        ("a1-landscape", Paper::A1, true, 1000, Frame::Circle),
        ("frame-square", Paper::A3, true, 2500, Frame::Square),
        ("frame-area", Paper::A3, true, 2500, Frame::Area),
        ("cropped", Paper::A4, false, 1500, Frame::Circle),
    ];
    for (name, paper, landscape, scale, frame) in variants {
        let sheet = Sheet {
            paper,
            landscape,
            scale,
            frame,
            title: "Westermarkt, Amsterdam".to_string(),
            subtitle: "Building use within 300 m of the Westerkerk".to_string(),
            ..Sheet::default()
        };
        let look = Look {
            sheet,
            ..Look::default()
        };
        let diagram = diagrams
            .iter()
            .find(|d| d.id == "building-use")
            .expect("building-use exists");
        let theme = &themes[0];
        let input = RenderInput {
            scene,
            diagram,
            theme,
            look: &look,
            pins: &[],
            highlights: &[],
            model: None,
        };
        let png = render_png(&input, SAMPLE_WIDTH_PX).expect("render the layout");
        std::fs::write(out_dir.join(format!("layout--{name}.png")), png).expect("write the layout");
    }

    let look = Look {
        sheet: Sheet {
            title: "Westermarkt".to_string(),
            ..Sheet::default()
        },
        ..Look::default()
    };
    let input = RenderInput {
        scene,
        diagram: &diagrams[0],
        theme: &themes[0],
        look: &look,
        pins: &[],
        highlights: &[],
        model: None,
    };
    for format in [ExportFormat::Svg, ExportFormat::Pdf, ExportFormat::Dxf] {
        let path = out_dir.join(format!("westermarkt.{}", format.extension()));
        export(&input, format, 150, &path).expect("export");
    }
}

/// A sheet with the title moved onto the drawing, a restyled subtitle and two
/// text boxes in every family, as PNG and PDF, to check type and embedding.
pub(crate) fn write_custom_text(scene: &Scene, out_dir: &Path) {
    let position = |x_mm, y_mm| Some(Position { x_mm, y_mm });
    let sheet = Sheet {
        title: "Westermarkt".to_string(),
        subtitle: "Building use within 300 m of the Westerkerk".to_string(),
        title_position: position(24.0, 22.0),
        title_style: TextStyle {
            family: Some("Source Serif 4".to_string()),
            italic: Some(true),
            size_pt: Some(40.0),
            ..TextStyle::default()
        },
        subtitle_style: TextStyle {
            family: Some("IBM Plex Mono".to_string()),
            colour: Some("@accent".to_string()),
            ..TextStyle::default()
        },
        texts: vec![
            TextBox {
                id: "text-1".to_string(),
                text: "Westerkerk\n1631, Hendrick de Keyser".to_string(),
                style: TextStyle {
                    bold: Some(true),
                    size_pt: Some(12.0),
                    align: Some(Align::Centre),
                    ..TextStyle::default()
                },
                position: position(150.0, 150.0),
            },
            TextBox {
                id: "text-2".to_string(),
                text: "Prinsengracht".to_string(),
                style: TextStyle {
                    family: Some("IBM Plex Mono".to_string()),
                    italic: Some(true),
                    bold: Some(true),
                    size_pt: Some(10.0),
                    align: Some(Align::Right),
                    colour: Some("#1D4E89".to_string()),
                    opacity: Some(0.8),
                },
                position: position(280.0, 260.0),
            },
        ],
        ..Sheet::default()
    };
    let look = Look {
        diagram_id: "building-use".to_string(),
        sheet,
        ..Look::default()
    };
    let diagrams = builtin_diagrams();
    let themes = builtin_themes();
    let diagram = diagrams
        .iter()
        .find(|d| d.id == "building-use")
        .expect("building-use exists");
    let input = RenderInput {
        scene,
        diagram,
        theme: &themes[0],
        look: &look,
        pins: &[],
        highlights: &[],
        model: None,
    };
    let png = render_png(&input, SAMPLE_WIDTH_PX).expect("render the custom text");
    std::fs::write(out_dir.join("text--custom.png"), png).expect("write the custom text");
    export(
        &input,
        ExportFormat::Pdf,
        150,
        &out_dir.join("text--custom.pdf"),
    )
    .expect("export the custom text");
}
