//! Reading a DXF the app wrote back into layers and entities, for the DXF tests.

#![allow(dead_code)]

use openarchi_core::export::{export, ExportFormat};
use openarchi_core::highlight::Highlight;
use openarchi_core::pin::Pin;
use openarchi_core::project::Look;
use openarchi_core::render::RenderInput;
use openarchi_core::scene::Scene;
use openarchi_core::section::SectionLine;

use crate::render_support::{diagram, theme, ORIGIN};

/// One entity read back: its kind, layer, points in drawing units, and text.
#[derive(Debug, Default)]
pub struct Entity {
    pub kind: String,
    pub layer: String,
    pub points: Vec<(f64, f64)>,
    pub text: String,
}

/// Every layer declared in the LAYER table, with its colour and line type,
/// and every entity, a polyline holding its vertices.
pub struct Dxf {
    pub layers: Vec<(String, i32, String)>,
    pub entities: Vec<Entity>,
}

impl Dxf {
    pub fn on(&self, layer: &str) -> Vec<&Entity> {
        self.entities
            .iter()
            .filter(|entity| entity.layer == layer)
            .collect()
    }

    pub fn layer(&self, name: &str) -> Option<&(String, i32, String)> {
        self.layers.iter().find(|layer| layer.0 == name)
    }
}

fn parse(text: &str) -> Dxf {
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len() % 2, 0, "DXF is pairs of lines");
    let pairs: Vec<(i32, &str)> = lines
        .chunks(2)
        .map(|pair| (pair[0].trim().parse().expect("group code"), pair[1]))
        .collect();
    assert_eq!(pairs.last(), Some(&(0, "EOF")));

    let mut layers = Vec::new();
    let mut entities: Vec<Entity> = Vec::new();
    let mut section = "";
    let mut index = 0;
    while index < pairs.len() {
        let (code, value) = pairs[index];
        if code == 2 && index > 0 && pairs[index - 1] == (0, "SECTION") {
            section = value;
        }
        if code == 0 && section == "TABLES" && value == "LAYER" {
            let fields = &pairs[index + 1..index + 6];
            layers.push((
                fields[0].1.to_string(),
                fields[2].1.parse().expect("colour"),
                fields[3].1.to_string(),
            ));
        }
        if code == 0 && section == "ENTITIES" && !matches!(value, "SEQEND" | "ENDSEC" | "EOF") {
            let mut entity = Entity {
                kind: value.to_string(),
                ..Entity::default()
            };
            let mut x = 0.0;
            index += 1;
            while index < pairs.len() && pairs[index].0 != 0 {
                match pairs[index] {
                    (8, layer) => entity.layer = layer.to_string(),
                    (1, text) => entity.text = text.to_string(),
                    (10, number) => x = number.parse().expect("x"),
                    (20, number) => entity.points.push((x, number.parse().expect("y"))),
                    _ => {}
                }
                index += 1;
            }
            if entity.kind == "VERTEX" {
                let polyline = entities.last_mut().expect("a polyline before its vertex");
                polyline.points.extend(entity.points);
            } else {
                // A polyline's own point is its elevation, not a vertex.
                if entity.kind == "POLYLINE" {
                    entity.points.clear();
                }
                entities.push(entity);
            }
            continue;
        }
        index += 1;
    }

    Dxf { layers, entities }
}

/// The DXF export of `look` over `scene` in the paper palette, read back.
pub fn dxf_of(
    scene: &Scene,
    diagram_id: &str,
    look: &Look,
    pins: &[Pin],
    highlights: &[Highlight],
) -> Dxf {
    let diagram = diagram(diagram_id);
    let theme = theme("paper");
    let input = RenderInput {
        scene,
        diagram: &diagram,
        theme: &theme,
        look,
        pins,
        highlights,
    };
    let folder = tempfile::tempdir().expect("temp folder");
    let path = folder.path().join("sheet.dxf");
    export(&input, ExportFormat::Dxf, 0, &path).expect("export the DXF");

    parse(&std::fs::read_to_string(&path).expect("read the DXF back"))
}

/// West to east through the row of five test buildings, which stand at y 80 to 100.
pub fn through_the_row() -> SectionLine {
    SectionLine {
        start: (ORIGIN.0 - 150.0, ORIGIN.1 + 90.0),
        end: (ORIGIN.0 + 150.0, ORIGIN.1 + 90.0),
        depth_m: 60.0,
        scale: None,
        exaggeration: 1,
    }
}
