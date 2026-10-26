//! PDOK's colour aerial photo of the Netherlands, as JPEG tiles for the 3D
//! view to lay on the ground.
//!
//! Beeldmateriaal Nederland flies the whole country every year. PDOK serves
//! the newest flight as one seamless WMS layer, "Actueel_ortho25", in 25 cm
//! pixels; the yearly layers ("2025_ortho25") sit beside it. The current
//! layer is used, so a site always shows the newest photo without asking
//! which year that is; the tile cache's thirty days pick up a new flight.
//! Checked live on 2026-10-26: the layer is listed in the capabilities, and a
//! 250 m tile at 512 pixels is a JPEG of about 66 KB.

use crate::scene::Credit;
use crate::sources::http;
use crate::{Error, Result};

const SERVICE: &str = "PDOK luchtfoto";
const WMS_URL: &str = "https://service.pdok.nl/hwh/luchtfotorgb/wms/v1_0";
pub const LAYER: &str = "Actueel_ortho25";
/// Every JPEG starts with these two bytes; a WMS error is XML instead.
const JPEG_START: [u8; 2] = [0xFF, 0xD8];
/// How much of a WMS error to show.
const ERROR_TEXT_BYTES: usize = 300;

/// The photo over `rect` (west, south, east, north in RD New metres), `pixels` square, as JPEG bytes.
pub fn fetch(rect: [f64; 4], pixels: u32) -> Result<Vec<u8>> {
    let [west, south, east, north] = rect;
    let bbox = format!("{west},{south},{east},{north}");
    let size = pixels.to_string();
    let query = [
        ("service", "WMS"),
        ("version", "1.3.0"),
        ("request", "GetMap"),
        ("layers", LAYER),
        ("styles", ""),
        ("crs", "EPSG:28992"),
        ("bbox", bbox.as_str()),
        ("width", size.as_str()),
        ("height", size.as_str()),
        ("format", "image/jpeg"),
    ];
    let body = http::get(SERVICE, WMS_URL, &query)?;
    check_jpeg(&body)?;

    Ok(body)
}

/// Passes a JPEG; a WMS error document becomes an error that shows its text.
pub fn check_jpeg(body: &[u8]) -> Result<()> {
    if body.starts_with(&JPEG_START) {
        return Ok(());
    }

    Err(Error::Data {
        service: SERVICE.to_string(),
        detail: format!(
            "not a JPEG: {}",
            String::from_utf8_lossy(&body[..body.len().min(ERROR_TEXT_BYTES)])
        ),
    })
}

// The metadata in the Nationaal Georegister gives CC BY 4.0, "Naamsvermelding
// verplicht, Beeldmateriaal.nl": credit is required wherever the photo shows.
pub fn credit() -> Credit {
    Credit {
        name: "Beeldmateriaal Nederland, aerial photo, via PDOK".to_string(),
        license: "CC BY 4.0".to_string(),
        url: "https://www.beeldmateriaal.nl".to_string(),
        layers: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 250 m tile at 128 pixels round the Westerkerk from Actueel_ortho25, 2026-10-26.
    const WESTERKERK: &[u8] = include_bytes!("../../tests/fixtures/luchtfotorgb-westerkerk.jpg");

    #[test]
    fn a_photo_tile_passes_and_a_wms_error_shows_its_text() {
        assert!(check_jpeg(WESTERKERK).is_ok());
        let error = check_jpeg(b"<?xml version=\"1.0\"?><ServiceExceptionReport>").unwrap_err();
        assert!(
            error.to_string().contains("ServiceExceptionReport"),
            "{error}"
        );
    }

    #[test]
    #[ignore = "asks PDOK"]
    fn the_live_service_sends_a_photo_tile() {
        let jpeg = fetch([120500.0, 487500.0, 120750.0, 487750.0], 256).unwrap();
        assert!(jpeg.len() > 5_000, "{} bytes", jpeg.len());
    }
}
