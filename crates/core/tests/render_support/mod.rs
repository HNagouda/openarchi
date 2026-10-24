//! A small made-up site for render tests: a 300 m circle with a few blocks,
//! a canal with an island, a road, a track, trees and parcels. Small enough
//! to reason about, varied enough to touch every kind of geometry.

#![allow(dead_code)]

use geo::{Coord, Geometry, LineString, Point, Polygon};
use openarchi_core::area::{Area, LonLat};
use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::diagram::Diagram;
use openarchi_core::project::Look;
use openarchi_core::scene::{Attributes, Credit, Feature, Layer, Scene};
use openarchi_core::theme::Theme;

pub const SITE_RADIUS_M: f64 = 300.0;
pub const ORIGIN: (f64, f64) = (120_733.0, 487_521.0);

pub fn circle(radius: f64) -> Polygon<f64> {
    let points: Vec<Coord<f64>> = (0..128)
        .map(|index| {
            let angle = index as f64 / 128.0 * std::f64::consts::TAU;
            Coord {
                x: radius * angle.cos(),
                y: radius * angle.sin(),
            }
        })
        .collect();
    Polygon::new(LineString::from(points), vec![])
}

pub fn square(x: f64, y: f64, side: f64) -> Polygon<f64> {
    Polygon::new(
        LineString::from(vec![
            (x, y),
            (x + side, y),
            (x + side, y + side),
            (x, y + side),
            (x, y),
        ]),
        vec![],
    )
}

fn feature(layer: Layer, geometry: Geometry<f64>, attributes: Attributes) -> Feature {
    Feature {
        layer,
        geometry,
        attributes,
    }
}

fn building(
    x: f64,
    y: f64,
    year: Option<i32>,
    use_class: Option<&str>,
    height: Option<f64>,
) -> Feature {
    let attributes = Attributes {
        year_built: year,
        use_class: use_class.map(str::to_string),
        height_m: height,
        ..Attributes::default()
    };
    feature(Layer::Building, square(x, y, 20.0).into(), attributes)
}

pub fn scene() -> Scene {
    let canal = Polygon::new(
        LineString::from(vec![
            (-250.0, -40.0),
            (250.0, -40.0),
            (250.0, 0.0),
            (-250.0, 0.0),
            (-250.0, -40.0),
        ]),
        // An island in the canal, to prove holes survive.
        vec![LineString::from(vec![
            (-10.0, -30.0),
            (10.0, -30.0),
            (10.0, -10.0),
            (-10.0, -10.0),
            (-10.0, -30.0),
        ])],
    );
    let features = vec![
        feature(Layer::Water, canal.into(), Attributes::default()),
        feature(
            Layer::Road,
            square(-200.0, 20.0, 400.0).into(),
            Attributes::default(),
        ),
        feature(
            Layer::Green,
            square(100.0, -150.0, 60.0).into(),
            Attributes::default(),
        ),
        feature(
            Layer::RailTrack,
            LineString::from(vec![(-280.0, 30.0), (280.0, 30.0)]).into(),
            Attributes::default(),
        ),
        feature(
            Layer::Tree,
            Point::new(50.0, 50.0).into(),
            Attributes::default(),
        ),
        feature(
            Layer::Tree,
            Point::new(70.0, 50.0).into(),
            Attributes::default(),
        ),
        feature(
            Layer::Parcel,
            square(-100.0, 80.0, 50.0).into(),
            Attributes::default(),
        ),
        building(-100.0, 80.0, Some(1650), Some("woonfunctie"), Some(14.0)),
        building(
            -60.0,
            80.0,
            Some(1925),
            Some("winkelfunctie, woonfunctie"),
            Some(18.5),
        ),
        building(-20.0, 80.0, Some(2015), Some("kantoorfunctie"), Some(32.0)),
        building(20.0, 80.0, None, None, None),
        building(60.0, 80.0, Some(1980), Some("iets onbekends"), Some(4.0)),
    ];

    Scene {
        area: Area::Circle {
            center: LonLat {
                lon: 4.884,
                lat: 52.3745,
            },
            radius_m: SITE_RADIUS_M,
        },
        crs: "EPSG:28992".to_string(),
        origin: ORIGIN,
        boundary: circle(SITE_RADIUS_M),
        features,
        credits: vec![Credit {
            name: "BGT, via PDOK".to_string(),
            license: "CC0 1.0".to_string(),
            url: "https://www.pdok.nl".to_string(),
            layers: Vec::new(),
        }],
        fetched_at: "2026-10-24T07:15:23Z".to_string(),
        warnings: vec![],
        measured: Default::default(),
    }
}

pub fn diagram(id: &str) -> Diagram {
    builtin_diagrams()
        .into_iter()
        .find(|diagram| diagram.id == id)
        .expect("built-in diagram")
}

pub fn theme(id: &str) -> Theme {
    builtin_themes()
        .into_iter()
        .find(|theme| theme.id == id)
        .expect("built-in theme")
}

pub fn look() -> Look {
    let mut look = Look::default();
    look.sheet.title = "Test site".to_string();
    look.sheet.subtitle = "Made up for the render tests".to_string();
    look
}
