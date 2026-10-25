//! An approximate vegetation index from PDOK's colour infrared aerial photo.
//!
//! The infrared photo moves every colour one place down: its red channel is
//! near infrared, its green channel is red light. Plants reflect near infrared
//! strongly and absorb red, so (NIR - red) / (NIR + red), the Normalized
//! Difference Vegetation Index (Rouse et al. 1974, "Monitoring vegetation
//! systems in the Great Plains with ERTS"), is high over plants and near zero
//! or below over roofs, paving and water. The photo is colour-corrected for
//! display, not calibrated reflectance, so the values are approximate: they
//! rank places on one site well, but do not compare with a satellite NDVI.
//!
//! PDOK serves the summer photo of each year as a WMS layer "<year>_ortho25IR",
//! in 25 cm pixels and only as JPEG. Each tile is asked at 1 m pixels and the
//! index is averaged to 5 m cells, the cells of the RIVM tree map.

use std::io::Cursor;

use geo::Rect;
use zune_jpeg::JpegDecoder;

use crate::scene::{Credit, Layer};
use crate::sources::edition::{Edition, ASK_TIMEOUT};
use crate::sources::grid::TILE_M;
use crate::sources::raster::Grid;
use crate::sources::{http, wcs};
use crate::{Error, Result};

const SERVICE: &str = "PDOK luchtfoto infrarood";
const WMS_URL: &str = "https://service.pdok.nl/hwh/luchtfotocir/wms/v1_0";
/// The summer photo of a year, "2025_ortho25IR". The "<year>_orthoHRIR"
/// layers are winter photos, taken with the trees bare, so they are left out.
const LAYER_SUFFIX: &str = "_ortho25IR";
const YEAR_DIGITS: usize = 4;
const NAME_TAG: &str = "<Name>";

/// The newest summer photo on 2026-10-25, used when the service cannot be asked.
pub const KNOWN_LAYER: &str = "2025_ortho25IR";

/// Pixels asked per metre: 1 m is fine enough to average and keeps a tile
/// near 30 KB.
const PIXEL_M: f64 = 1.0;
/// The cell the index is kept in.
pub const CELL_M: f64 = 5.0;
/// Bytes per pixel of a decoded colour JPEG.
const RGB: usize = 3;
/// Outside the photo, or before PDOK has filled a new year in, the WMS sends
/// pure white. No real photo pixel is exactly this in all three channels.
const NO_PHOTO: [u8; 3] = [255, 255, 255];

/// Every layer name the WMS lists, for finding the newest summer photo.
pub fn list_layers() -> Result<Vec<String>> {
    let query = [
        ("service", "WMS"),
        ("version", "1.3.0"),
        ("request", "GetCapabilities"),
    ];
    let body = http::get_once_within(SERVICE, WMS_URL, &query, ASK_TIMEOUT)?;
    // A plain scan: the document is 32 KB and the names are plain.
    let names = wcs::tag_texts(&String::from_utf8_lossy(&body), NAME_TAG);
    if names.is_empty() {
        return Err(Error::Data {
            service: SERVICE.to_string(),
            detail: "the capabilities list no layers".to_string(),
        });
    }

    Ok(names)
}

/// The newest summer photo among `names`, or `None` when none has its shape.
pub fn newest(names: &[String]) -> Option<Edition> {
    names
        .iter()
        .filter_map(|name| Some((year_of(name)?, name)))
        .max_by_key(|(year, _)| *year)
        .map(|(year, name)| Edition {
            year,
            id: name.clone(),
        })
}

/// The edition named in code, for when the service cannot be asked.
pub fn known() -> Edition {
    // A broken constant, caught by `the_known_layer_has_the_shape_of_a_summer_photo`.
    let year = year_of(KNOWN_LAYER).expect("the known layer has the shape of a summer photo");
    Edition {
        year,
        id: KNOWN_LAYER.to_string(),
    }
}

/// "2025_ortho25IR" -> 2025.
fn year_of(name: &str) -> Option<i32> {
    let year = name.strip_suffix(LAYER_SUFFIX)?;
    let is_year = year.len() == YEAR_DIGITS && year.bytes().all(|byte| byte.is_ascii_digit());
    if !is_year {
        return None;
    }

    year.parse().ok()
}

/// The vegetation index over one 250 m tile, in 5 m cells. A cell the photo
/// does not cover is NaN.
pub fn fetch_tile(layer: &str, rect: Rect<f64>) -> Result<Grid> {
    let pixels = ((TILE_M / PIXEL_M) as usize).to_string();
    let bbox = format!(
        "{},{},{},{}",
        rect.min().x,
        rect.min().y,
        rect.max().x,
        rect.max().y
    );
    let query = [
        ("service", "WMS"),
        ("version", "1.3.0"),
        ("request", "GetMap"),
        ("layers", layer),
        ("styles", ""),
        ("crs", "EPSG:28992"),
        ("bbox", bbox.as_str()),
        ("width", pixels.as_str()),
        ("height", pixels.as_str()),
        ("format", "image/jpeg"),
    ];
    let body = http::get(SERVICE, WMS_URL, &query)?;
    let (rgb, width, height) = decode(&body)?;

    Ok(Grid {
        west: rect.min().x,
        north: rect.max().y,
        cell_m: CELL_M,
        width: width / pixels_per_cell(),
        height: height / pixels_per_cell(),
        values: index_cells(&rgb, width, height),
    })
}

/// A colour JPEG as RGB bytes, row by row from the top, with its size.
fn decode(body: &[u8]) -> Result<(Vec<u8>, usize, usize)> {
    let data_error = |detail: String| Error::Data {
        service: SERVICE.to_string(),
        detail,
    };
    let mut decoder = JpegDecoder::new(Cursor::new(body));
    // A WMS error is an XML document, which is the useful thing to show.
    let rgb = decoder.decode().map_err(|error| {
        data_error(format!(
            "not a JPEG ({error}): {}",
            String::from_utf8_lossy(&body[..body.len().min(300)])
        ))
    })?;
    let (width, height) = decoder
        .dimensions()
        .ok_or_else(|| data_error("the JPEG has no size".to_string()))?;
    if rgb.len() != width * height * RGB {
        return Err(data_error(format!(
            "expected {width} x {height} colour pixels, got {} bytes",
            rgb.len()
        )));
    }

    Ok((rgb, width, height))
}

fn pixels_per_cell() -> usize {
    (CELL_M / PIXEL_M) as usize
}

/// The mean index of the photo pixels in each cell, NaN for a cell without any.
fn index_cells(rgb: &[u8], width: usize, height: usize) -> Vec<f32> {
    let step = pixels_per_cell();
    let (columns, rows) = (width / step, height / step);
    let mut sums = vec![0.0_f64; columns * rows];
    let mut counts = vec![0usize; columns * rows];
    for (index, pixel) in rgb.as_chunks::<RGB>().0.iter().enumerate() {
        let (column, row) = ((index % width) / step, (index / width) / step);
        if column >= columns || row >= rows {
            continue;
        }
        if let Some(value) = vegetation_index(pixel) {
            sums[row * columns + column] += value;
            counts[row * columns + column] += 1;
        }
    }

    sums.iter()
        .zip(&counts)
        .map(|(sum, count)| match count {
            0 => f32::NAN,
            _ => (sum / *count as f64) as f32,
        })
        .collect()
}

/// NDVI of one infrared photo pixel, `None` outside the photo or for black.
fn vegetation_index(pixel: &[u8; RGB]) -> Option<f64> {
    if *pixel == NO_PHOTO {
        return None;
    }
    let near_infrared = f64::from(pixel[0]);
    let red = f64::from(pixel[1]);
    let total = near_infrared + red;
    if total == 0.0 {
        return None;
    }

    Some((near_infrared - red) / total)
}

// Beeldmateriaal Nederland publishes the photos under CC BY 4.0 and asks
// publications to refer to beeldmateriaal.nl.
pub fn credit() -> Credit {
    Credit {
        name: "Beeldmateriaal Nederland, infrared aerial photo, via PDOK".to_string(),
        license: "CC BY 4.0".to_string(),
        url: "https://www.beeldmateriaal.nl".to_string(),
        layers: vec![Layer::VegetationIndex],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The layer list of the capabilities of 2026-10-25, cut to its names.
    const CAPABILITIES: &str = include_str!("../../tests/fixtures/luchtfotocir-capabilities.xml");
    /// A 250 x 250 tile at 1 m around the Westerkerk from 2025_ortho25IR, 2026-10-25.
    const WESTERKERK: &[u8] = include_bytes!("../../tests/fixtures/luchtfotocir-westerkerk.jpg");

    #[test]
    fn the_newest_summer_photo_is_picked_and_winter_photos_are_not() {
        let names = wcs::tag_texts(CAPABILITIES, NAME_TAG);
        assert!(names.contains(&"2026_orthoHRIR".to_string()));
        assert_eq!(newest(&names), Some(known()));
    }

    #[test]
    fn the_known_layer_has_the_shape_of_a_summer_photo() {
        assert_eq!(year_of(KNOWN_LAYER), Some(2025));
        assert_eq!(year_of("Actueel_ortho25IR"), None);
        assert_eq!(year_of("2026_orthoHRIR"), None);
    }

    #[test]
    fn near_infrared_over_red_reads_as_plants() {
        // A tree crown in the photo is bright red: much near infrared, little red light.
        let crown = vegetation_index(&[200, 60, 80]).unwrap();
        // A roof is grey: as much of one as of the other.
        let roof = vegetation_index(&[120, 125, 130]).unwrap();
        assert!(crown > 0.5, "{crown}");
        assert!(roof.abs() < 0.05, "{roof}");
        assert_eq!(vegetation_index(&NO_PHOTO), None);
    }

    #[test]
    fn a_tile_becomes_five_metre_cells_with_trees_above_streets() {
        let (rgb, width, height) = decode(WESTERKERK).unwrap();
        let cells = index_cells(&rgb, width, height);

        assert_eq!((width, height), (250, 250));
        assert_eq!(cells.len(), 50 * 50);
        let highest = cells.iter().copied().fold(f32::MIN, f32::max);
        let lowest = cells.iter().copied().fold(f32::MAX, f32::min);
        assert!(highest > 0.3, "the canal elms read green: {highest}");
        assert!(lowest < 0.0, "roofs and water read below zero: {lowest}");
    }

    #[test]
    fn a_wms_error_is_shown_rather_than_a_decoder_message_alone() {
        let error = decode(b"<?xml version=\"1.0\"?><ServiceExceptionReport>").unwrap_err();
        assert!(
            error.to_string().contains("ServiceExceptionReport"),
            "{error}"
        );
    }

    #[test]
    #[ignore = "asks PDOK"]
    fn the_live_service_lists_a_summer_photo_and_sends_a_tile() {
        let edition = newest(&list_layers().unwrap()).unwrap();
        let rect = Rect::new((120500.0, 487500.0), (120750.0, 487750.0));
        let grid = fetch_tile(&edition.id, rect).unwrap();
        assert_eq!((grid.width, grid.height), (50, 50));
        assert!(grid.values.iter().all(|value| value.is_finite()));
    }
}
