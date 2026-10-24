//! Raster tiles from an OGC Web Coverage Service, as float GeoTIFFs in RD New.
//!
//! AHN, the RIVM noise and air quality maps and the LIWO flood depths all come
//! through this one path. The services differ in three ways, which is all a
//! `Coverage` spells out: where the service lives, what it calls the axes
//! (PDOK says `x` and `y`, the GeoServers of RIVM and LIWO say `X` and `Y`),
//! and the coverage id. Each answer is placed by the georeferencing in the
//! GeoTIFF itself, because a coverage need not line up with the tile we asked
//! for: the RIVM air quality grid starts 11.6 m east of a round number.

use std::io::Cursor;

use tiff::decoder::{Decoder, DecodingResult};
use tiff::tags::Tag;

use crate::sources::http;
use crate::sources::raster::Grid;
use crate::{Error, Result};
use geo::Rect;

/// Some services mark missing cells with the largest f32 and do not always
/// say so in the file. Nothing measured here comes near this.
const NODATA_ABOVE: f32 = 1.0e30;

/// One coverage on one WCS 2.0.1 service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Coverage {
    /// Names the service in error messages: "RIVM".
    pub service: &'static str,
    pub url: &'static str,
    pub id: &'static str,
    /// The axis labels the service expects in `subset`: ("x", "y") or ("X", "Y").
    pub axes: (&'static str, &'static str),
}

/// The cells of `coverage` that the service returns for `rect` (RD New),
/// placed where the GeoTIFF says they are. Missing cells are NaN.
pub fn fetch(coverage: &Coverage, rect: Rect<f64>) -> Result<Grid> {
    let subset_x = format!("{}({},{})", coverage.axes.0, rect.min().x, rect.max().x);
    let subset_y = format!("{}({},{})", coverage.axes.1, rect.min().y, rect.max().y);
    let query = [
        ("service", "WCS"),
        ("version", "2.0.1"),
        ("request", "GetCoverage"),
        ("coverageId", coverage.id),
        ("format", "image/tiff"),
        ("subset", subset_x.as_str()),
        ("subset", subset_y.as_str()),
    ];

    let body = http::get(coverage.service, coverage.url, &query)?;
    decode(coverage.service, &body)
}

/// A single-band GeoTIFF as a grid: 32-bit floats or 16-bit integers, placed
/// by its model transformation or by its tie point and pixel scale.
pub fn decode(service: &str, body: &[u8]) -> Result<Grid> {
    let data_error = |detail: String| Error::Data {
        service: service.to_string(),
        detail,
    };

    let mut decoder = Decoder::new(Cursor::new(body)).map_err(|error| {
        data_error(format!(
            "not a GeoTIFF ({error}): {}",
            String::from_utf8_lossy(&body[..body.len().min(300)])
        ))
    })?;
    let (width, height) = decoder
        .dimensions()
        .map_err(|error| data_error(error.to_string()))?;
    let (west, north, cell_m) = placement(&mut decoder).map_err(data_error)?;
    let nodata = decoder
        .get_tag_ascii_string(Tag::GdalNodata)
        .ok()
        .and_then(|text| text.trim().trim_end_matches('\0').parse::<f32>().ok());

    let raw: Vec<f32> = match decoder
        .read_image()
        .map_err(|error| data_error(error.to_string()))?
    {
        DecodingResult::F32(values) => values,
        DecodingResult::I16(values) => values.into_iter().map(f32::from).collect(),
        _ => {
            return Err(data_error(
                "cells are neither 32-bit floats nor 16-bit integers".to_string(),
            ))
        }
    };
    let values = raw
        .into_iter()
        .map(|value| {
            let is_missing = !value.is_finite() || value >= NODATA_ABOVE || Some(value) == nodata;
            if is_missing {
                f32::NAN
            } else {
                value
            }
        })
        .collect();

    Ok(Grid {
        west,
        north,
        cell_m,
        width: width as usize,
        height: height as usize,
        values,
    })
}

/// West edge, north edge and cell size from the GeoTIFF tags. Only square,
/// north-up cells are accepted, which is what every service here sends.
fn placement<R: std::io::Read + std::io::Seek>(
    decoder: &mut Decoder<R>,
) -> std::result::Result<(f64, f64, f64), String> {
    // A 4 x 4 matrix: [scale_x, 0, 0, west, 0, -scale_y, 0, north, ...].
    if let Ok(matrix) = decoder.get_tag_f64_vec(Tag::ModelTransformationTag) {
        if matrix.len() >= 8 && matrix[1] == 0.0 && matrix[4] == 0.0 {
            return square_cells(matrix[0], -matrix[5]).map(|cell| (matrix[3], matrix[7], cell));
        }
        return Err(format!("a rotated grid is not supported: {matrix:?}"));
    }

    let scale = decoder
        .get_tag_f64_vec(Tag::ModelPixelScaleTag)
        .map_err(|error| format!("no pixel scale: {error}"))?;
    // (column, row, 0, x, y, 0): one raster point tied to one map point.
    let tiepoint = decoder
        .get_tag_f64_vec(Tag::ModelTiepointTag)
        .map_err(|error| format!("no tie point: {error}"))?;
    if scale.len() < 2 || tiepoint.len() < 6 {
        return Err(format!("incomplete georeferencing: {scale:?} {tiepoint:?}"));
    }
    let cell = square_cells(scale[0], scale[1])?;
    Ok((
        tiepoint[3] - tiepoint[0] * cell,
        tiepoint[4] + tiepoint[1] * cell,
        cell,
    ))
}

fn square_cells(across: f64, down: f64) -> std::result::Result<f64, String> {
    if across > 0.0 && (across - down).abs() < 1e-9 {
        Ok(across)
    } else {
        Err(format!("cells are not square: {across} by {down}"))
    }
}
