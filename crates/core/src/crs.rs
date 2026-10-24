//! Conversion between WGS84 (what the map shows) and RD New (what Dutch data uses).
//!
//! Owned by the data layer. Accurate to well under a metre across the Netherlands,
//! which is enough to crop a site; the data itself arrives in RD New already.
//!
//! Uses the polynomial approximation by Schreutelkamp and Strang van Hees
//! ("Benaderingsformules voor de transformatie tussen RD- en WGS84-kaartcoördinaten"):
//! a short power series around Amersfoort, the origin of RD. It avoids a full
//! datum transformation library for a job that only needs to place a crop.

use crate::area::LonLat;

/// Amersfoort, the RD origin, in both systems.
const ORIGIN_X: f64 = 155_000.0;
const ORIGIN_Y: f64 = 463_000.0;
const ORIGIN_LAT: f64 = 52.155_174_40;
const ORIGIN_LON: f64 = 5.387_206_21;

const SECONDS_PER_DEGREE: f64 = 3600.0;

// Each table is (p, q, coefficient): the term coefficient * a^p * b^q.

/// RD to latitude. a = dx, b = dy in units of 100 km; result in arc seconds.
const LAT_TERMS: [(i32, i32, f64); 11] = [
    (0, 1, 3235.65389),
    (2, 0, -32.58297),
    (0, 2, -0.24750),
    (2, 1, -0.84978),
    (0, 3, -0.06550),
    (2, 2, -0.01709),
    (1, 0, -0.00738),
    (4, 0, 0.00530),
    (2, 3, -0.00039),
    (4, 1, 0.00033),
    (1, 1, -0.00012),
];

/// RD to longitude. a = dx, b = dy in units of 100 km; result in arc seconds.
const LON_TERMS: [(i32, i32, f64); 12] = [
    (1, 0, 5260.52916),
    (1, 1, 105.94684),
    (1, 2, 2.45656),
    (3, 0, -0.81885),
    (1, 3, 0.05594),
    (3, 1, -0.05607),
    (0, 1, 0.01199),
    (3, 2, -0.00256),
    (1, 4, 0.00128),
    (0, 2, 0.00022),
    (2, 0, -0.00022),
    (5, 0, 0.00026),
];

/// WGS84 to RD x. a = dlat, b = dlon in units of 10 000 arc seconds; result in metres.
const X_TERMS: [(i32, i32, f64); 9] = [
    (0, 1, 190_094.945),
    (1, 1, -11_832.228),
    (2, 1, -114.221),
    (0, 3, -32.391),
    (1, 0, -0.705),
    (3, 1, -2.340),
    (1, 3, -0.608),
    (0, 2, -0.008),
    (2, 3, 0.148),
];

/// WGS84 to RD y. a = dlat, b = dlon in units of 10 000 arc seconds; result in metres.
const Y_TERMS: [(i32, i32, f64); 10] = [
    (1, 0, 309_056.544),
    (0, 2, 3638.893),
    (2, 0, 73.077),
    (1, 2, -157.984),
    (3, 0, 59.788),
    (0, 1, 0.433),
    (2, 2, -6.439),
    (1, 1, -0.032),
    (0, 4, 0.092),
    (1, 4, -0.054),
];

/// WGS84 longitude/latitude to RD New x/y in metres.
pub fn wgs84_to_rd(point: LonLat) -> (f64, f64) {
    // 0.36 turns degrees into units of 10 000 arc seconds.
    let dlat = 0.36 * (point.lat - ORIGIN_LAT);
    let dlon = 0.36 * (point.lon - ORIGIN_LON);

    let x = ORIGIN_X + series(&X_TERMS, dlat, dlon);
    let y = ORIGIN_Y + series(&Y_TERMS, dlat, dlon);
    (x, y)
}

/// RD New x/y in metres to WGS84 longitude/latitude.
pub fn rd_to_wgs84(x: f64, y: f64) -> LonLat {
    let dx = (x - ORIGIN_X) * 1e-5;
    let dy = (y - ORIGIN_Y) * 1e-5;

    LonLat {
        lon: ORIGIN_LON + series(&LON_TERMS, dx, dy) / SECONDS_PER_DEGREE,
        lat: ORIGIN_LAT + series(&LAT_TERMS, dx, dy) / SECONDS_PER_DEGREE,
    }
}

fn series(terms: &[(i32, i32, f64)], a: f64, b: f64) -> f64 {
    terms
        .iter()
        .map(|&(p, q, coefficient)| coefficient * a.powi(p) * b.powi(q))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amersfoort_maps_to_the_rd_origin() {
        let (x, y) = wgs84_to_rd(LonLat {
            lon: ORIGIN_LON,
            lat: ORIGIN_LAT,
        });
        assert!((x - ORIGIN_X).abs() < 1e-6);
        assert!((y - ORIGIN_Y).abs() < 1e-6);
    }

    #[test]
    fn rd_origin_maps_to_amersfoort() {
        let point = rd_to_wgs84(ORIGIN_X, ORIGIN_Y);
        assert!((point.lon - ORIGIN_LON).abs() < 1e-9);
        assert!((point.lat - ORIGIN_LAT).abs() < 1e-9);
    }

    #[test]
    fn westermarkt_20_matches_the_kadaster_coordinates_within_a_metre() {
        // Both coordinates as PDOK Locatieserver gives them for this address.
        let (x, y) = wgs84_to_rd(LonLat {
            lon: 4.884_317_19,
            lat: 52.374_936_37,
        });
        assert!((x - 120_755.124).abs() < 1.0, "x was {x}");
        assert!((y - 487_570.292).abs() < 1.0, "y was {y}");
    }

    #[test]
    fn round_trips_stay_within_a_metre() {
        let corners = [
            LonLat {
                lon: 4.8840,
                lat: 52.3745,
            },
            LonLat {
                lon: 3.5,
                lat: 51.4,
            },
            LonLat {
                lon: 6.9,
                lat: 53.3,
            },
            LonLat {
                lon: 5.7,
                lat: 50.8,
            },
        ];

        for start in corners {
            let (x, y) = wgs84_to_rd(start);
            let back = rd_to_wgs84(x, y);
            let (x_again, y_again) = wgs84_to_rd(back);
            assert!((x - x_again).abs() < 1.0, "{start:?}: x {x} vs {x_again}");
            assert!((y - y_again).abs() < 1.0, "{start:?}: y {y} vs {y_again}");
        }
    }
}
