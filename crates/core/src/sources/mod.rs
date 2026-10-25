//! One module per data service. Each turns a query area into scene features.
//!
//! Sources return features in absolute RD New coordinates; `site.rs` clips them
//! to the drawn boundary and moves them into the scene's local frame.

pub mod ahn;
pub mod bag;
pub mod bag_units;
pub mod bands;
pub mod bgt;
pub mod boundaries;
pub mod brk;
pub mod cache;
pub mod cbs;
pub mod clip;
pub mod contours;
pub mod edition;
pub mod grid;
pub mod heights;
pub mod http;
pub mod knmi;
pub mod liwo;
pub mod luchtfoto;
pub mod osm;
pub mod pdok;
pub mod pvgis;
pub mod raster;
pub mod rivm;
pub mod rws;
pub mod slope;
pub mod wcs;
