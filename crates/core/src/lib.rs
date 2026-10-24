//! Everything openarchi does without a window: fetch a site, style it, export it.
//!
//! The app and the MCP server are thin shells over this crate, so a site looks
//! the same whether a person or an assistant asked for it.

pub mod area;
pub mod catalog;
pub mod crs;
pub mod diagram;
pub mod environment;
pub mod error;
pub mod export;
pub mod geocode;
pub mod library;
pub mod project;
pub mod ratios;
pub mod render;
pub mod scene;
pub mod sheet;
pub mod site;
pub mod sources;
pub mod sun;
pub mod theme;
pub mod view;

pub use error::{Error, Result};
