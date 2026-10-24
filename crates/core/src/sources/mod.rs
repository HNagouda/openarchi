//! One module per data service. Each turns a query area into scene features.
//!
//! Sources return features in absolute RD New coordinates; `site.rs` clips them
//! to the drawn boundary and moves them into the scene's local frame.

pub mod ahn;
pub mod bag;
pub mod bgt;
pub mod brk;
pub mod clip;
pub mod contours;
pub mod heights;
pub mod http;
pub mod pdok;
