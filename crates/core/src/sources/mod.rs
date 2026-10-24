//! One module per data service. Each turns a query area into scene features.
//!
//! Sources return features in absolute RD New coordinates; `site.rs` clips them
//! to the drawn boundary and moves them into the scene's local frame.

pub mod ahn;
pub mod bag;
pub mod bgt;
pub mod boundaries;
pub mod brk;
pub mod cache;
pub mod clip;
pub mod contours;
pub mod grid;
pub mod heights;
pub mod http;
pub mod knmi;
pub mod osm;
pub mod pdok;
pub mod pvgis;
