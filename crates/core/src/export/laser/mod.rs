//! Laser cutting files for a physical site model.
//!
//! The base plate is the site boundary, with roads, water, green and every
//! building footprint scored on it so the student knows where to glue. Each
//! building becomes a stack of identical footprint pieces, as many as its
//! height needs in the chosen board. Every piece carries an engraved id, and
//! the plate engraves the same id at the footprint. The pieces are packed onto
//! beds of the laser's size, and each bed is written as one file in real
//! millimetres: cut lines red, score lines blue, both hairlines, the colours
//! nearly every laser shop expects.

mod dxf;
mod glyphs;
mod pack;
mod pieces;
mod plate;
mod svg;

use std::path::{Path, PathBuf};

use geo::{BoundingRect, Polygon};
use serde::{Deserialize, Serialize};

use crate::area::Area;
use crate::scene::{Layer, Scene};
use crate::selection;
use crate::{Error, Result};

pub use pieces::layer_count;

/// Space between two pieces on the bed, so the beam never cuts one edge twice.
const GAP_MM: f64 = 2.0;
/// Space kept free along the bed's edges, where the material is clamped or warped.
const MARGIN_MM: f64 = 5.0;
const MIN_SCALE: u32 = 100;
const MAX_SCALE: u32 = 10_000;
const MIN_THICKNESS_MM: f64 = 0.5;
const MAX_THICKNESS_MM: f64 = 20.0;
const MIN_BED_MM: f64 = 100.0;
const MAX_BED_MM: f64 = 3000.0;
const MM2_PER_M2: f64 = 1_000_000.0;
/// The scales site models are usually built at. A sheet drawn at one of
/// them gives the model its scale; any other starts at `DEFAULT_SCALE`.
const MODEL_SCALES: [u32; 5] = [200, 500, 1000, 2000, 2500];
const DEFAULT_SCALE: u32 = 1000;
/// Grey board, the usual material of a model workshop.
pub const DEFAULT_THICKNESS_MM: f64 = 2.0;
/// The bed of the most common small laser cutters in schools.
pub const DEFAULT_BED_MM: (f64, f64) = (600.0, 400.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaserFormat {
    Svg,
    Dxf,
}

impl LaserFormat {
    pub fn extension(self) -> &'static str {
        match self {
            LaserFormat::Svg => "svg",
            LaserFormat::Dxf => "dxf",
        }
    }
}

/// What the student asks for: the model's scale, the board and the laser.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaserSettings {
    /// 1000 for 1:1000.
    pub scale: u32,
    pub thickness_mm: f64,
    pub bed_width_mm: f64,
    pub bed_height_mm: f64,
    pub has_base_plate: bool,
    pub has_buildings: bool,
    /// Terrain contour lines scored on the base plate, when the site has them.
    pub has_contours: bool,
    pub format: LaserFormat,
}

/// What the files hold, for the student to check before and after.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaserSummary {
    pub sheet_count: usize,
    /// Every piece the laser cuts: the base plate's tiles and every building layer.
    pub piece_count: usize,
    pub building_count: usize,
    /// Buildings whose footprint is smaller than the smallest piece worth cutting.
    pub dropped_buildings: usize,
    /// Buildings without a measured height, stacked to the default height.
    pub estimated_heights: usize,
    pub plate_width_mm: f64,
    pub plate_height_mm: f64,
    /// 1 when the base plate fits the bed whole.
    pub plate_tiles: usize,
    /// Whether the site has contour lines to score, whatever the settings ask.
    pub has_contours: bool,
    /// The board the sheets take: every bed, whole.
    pub material_m2: f64,
    /// The area of every piece together.
    pub pieces_m2: f64,
}

/// The files written, and what they hold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaserExport {
    pub files: Vec<String>,
    pub summary: LaserSummary,
}

/// One piece to cut, in millimetres, with its bounding box's top-left corner at (0, 0) and y down.
#[derive(Debug, Clone)]
pub(crate) struct Part {
    /// Closed rings, holes and outlines alike, each ending where it starts.
    pub cut: Vec<Vec<(f64, f64)>>,
    /// Open lines to score.
    pub engrave: Vec<Vec<(f64, f64)>>,
    pub width: f64,
    pub height: f64,
    pub area_mm2: f64,
}

/// Ground metres to model millimetres: x east, y down from the site's north edge.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Frame {
    pub mm_per_m: f64,
    pub west: f64,
    pub north: f64,
}

impl Frame {
    fn new(boundary: &Polygon<f64>, scale: u32) -> Frame {
        let (west, north) = boundary
            .bounding_rect()
            .map(|rect| (rect.min().x, rect.max().y))
            .unwrap_or((0.0, 0.0));
        Frame {
            mm_per_m: 1000.0 / scale as f64,
            west,
            north,
        }
    }

    pub fn to_mm(self, x: f64, y: f64) -> (f64, f64) {
        (
            (x - self.west) * self.mm_per_m,
            (self.north - y) * self.mm_per_m,
        )
    }
}

/// Every part, and which bed each lands on.
struct Plan {
    parts: Vec<Part>,
    /// Per bed, its parts as (index into `parts`, x, y) in bed millimetres.
    sheets: Vec<Vec<(usize, f64, f64)>>,
    summary: LaserSummary,
}

/// The scale a model of a sheet drawn at `sheet_scale` starts at.
pub fn default_scale(sheet_scale: u32) -> u32 {
    if MODEL_SCALES.contains(&sheet_scale) {
        sheet_scale
    } else {
        DEFAULT_SCALE
    }
}

/// "westermarkt-laser-2.svg" for the second bed.
pub fn file_name(stem: &str, number: usize, format: LaserFormat) -> String {
    format!("{stem}-laser-{number}.{}", format.extension())
}

/// What exporting `settings` would write, without writing it.
pub fn summarize(
    scene: &Scene,
    selection: Option<&Area>,
    settings: &LaserSettings,
) -> Result<LaserSummary> {
    let scene = selection::framed_or_whole(scene, selection)?;
    Ok(plan(&scene, settings)?.summary)
}

/// Writes one file per bed into `folder`, named "`stem`-laser-1.svg" and on.
pub fn export(
    scene: &Scene,
    selection: Option<&Area>,
    settings: &LaserSettings,
    folder: &Path,
    stem: &str,
) -> Result<LaserExport> {
    let scene = selection::framed_or_whole(scene, selection)?;
    let plan = plan(&scene, settings)?;
    let bed = (settings.bed_width_mm, settings.bed_height_mm);

    let mut files = Vec::new();
    for (index, sheet) in plan.sheets.iter().enumerate() {
        let number = index + 1;
        let path: PathBuf = folder.join(file_name(stem, number, settings.format));
        let title = format!(
            "{stem}, laser sheet {number} of {}, 1:{}, {} mm board",
            plan.sheets.len(),
            settings.scale,
            settings.thickness_mm
        );
        let text = match settings.format {
            LaserFormat::Svg => svg::write(&plan.parts, sheet, bed, &title),
            LaserFormat::Dxf => dxf::write(&plan.parts, sheet, bed),
        };
        std::fs::write(&path, text).map_err(|source| Error::io(&path, source))?;
        files.push(path.display().to_string());
    }

    Ok(LaserExport {
        files,
        summary: plan.summary,
    })
}

fn plan(scene: &Scene, settings: &LaserSettings) -> Result<Plan> {
    check(settings)?;
    let frame = Frame::new(&scene.boundary, settings.scale);
    let usable = (
        settings.bed_width_mm - 2.0 * MARGIN_MM,
        settings.bed_height_mm - 2.0 * MARGIN_MM,
    );
    let buildings = pieces::buildings(scene, frame, settings);
    let has_contours = scene
        .features
        .iter()
        .any(|feature| feature.layer == Layer::Contour);
    let (plate_width, plate_height) = plate_size(&scene.boundary, frame);

    let mut parts = Vec::new();
    let mut plate_tiles = 0;
    if settings.has_base_plate {
        let tiles = plate::tiles(scene, frame, &buildings.footprints, settings, usable)?;
        plate_tiles = tiles.len();
        parts.extend(tiles);
    }
    if settings.has_buildings {
        parts.extend(buildings.pieces);
    }
    if parts.is_empty() {
        return Err(Error::Invalid(
            "Nothing to cut: choose the base plate, the buildings or both.".to_string(),
        ));
    }

    let sizes: Vec<(f64, f64)> = parts.iter().map(|part| (part.width, part.height)).collect();
    let placements = pack::pack(&sizes, usable, GAP_MM)?;
    let sheet_count = placements
        .iter()
        .map(|place| place.sheet + 1)
        .max()
        .unwrap_or(0);
    let mut sheets = vec![Vec::new(); sheet_count];
    for (index, place) in placements.iter().enumerate() {
        sheets[place.sheet].push((index, place.x + MARGIN_MM, place.y + MARGIN_MM));
    }

    let bed_m2 = settings.bed_width_mm * settings.bed_height_mm / MM2_PER_M2;
    let summary = LaserSummary {
        sheet_count,
        piece_count: parts.len(),
        building_count: if settings.has_buildings {
            buildings.building_count
        } else {
            0
        },
        dropped_buildings: buildings.dropped,
        estimated_heights: buildings.estimated_heights,
        plate_width_mm: plate_width,
        plate_height_mm: plate_height,
        plate_tiles,
        has_contours,
        material_m2: sheet_count as f64 * bed_m2,
        pieces_m2: parts.iter().map(|part| part.area_mm2).sum::<f64>() / MM2_PER_M2,
    };
    Ok(Plan {
        parts,
        sheets,
        summary,
    })
}

/// The base plate's bounding box in millimetres, whether or not it is cut.
fn plate_size(boundary: &Polygon<f64>, frame: Frame) -> (f64, f64) {
    boundary
        .bounding_rect()
        .map(|rect| {
            (
                rect.width() * frame.mm_per_m,
                rect.height() * frame.mm_per_m,
            )
        })
        .unwrap_or((0.0, 0.0))
}

/// Settings from the app or an assistant, checked where they come in.
fn check(settings: &LaserSettings) -> Result<()> {
    if !(MIN_SCALE..=MAX_SCALE).contains(&settings.scale) {
        return Err(Error::Invalid(format!(
            "The model scale must be between 1:{MIN_SCALE} and 1:{MAX_SCALE}, got 1:{}.",
            settings.scale
        )));
    }
    if !(MIN_THICKNESS_MM..=MAX_THICKNESS_MM).contains(&settings.thickness_mm) {
        return Err(Error::Invalid(format!(
            "The board must be {MIN_THICKNESS_MM} to {MAX_THICKNESS_MM} mm thick, got {} mm.",
            settings.thickness_mm
        )));
    }
    for side in [settings.bed_width_mm, settings.bed_height_mm] {
        if !(MIN_BED_MM..=MAX_BED_MM).contains(&side) {
            return Err(Error::Invalid(format!(
                "Each side of the laser bed must be {MIN_BED_MM} to {MAX_BED_MM} mm, got {side} mm."
            )));
        }
    }

    Ok(())
}
