//! Everything openarchi does without a window: fetch a site, style it, export it.
//!
//! The app and the MCP server are thin shells over this crate, so a site looks
//! the same whether a person or an assistant asked for it.

pub mod analysis;
pub mod area;
pub mod attribution;
pub mod blocks;
pub mod catalog;
pub mod crs;
pub mod diagram;
pub mod environment;
pub mod error;
pub mod export;
pub mod floors;
pub mod geocode;
pub mod library;
pub mod palette;
pub mod pin;
pub mod project;
pub mod ratios;
pub mod render;
pub mod report;
pub mod scene;
pub mod selection;
pub mod sheet;
pub mod site;
pub mod sources;
pub mod sun;
pub mod sunlight;
pub mod theme;
pub mod view;
pub mod walk;

pub use error::{Error, Result};
