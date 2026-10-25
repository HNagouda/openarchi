//! `openarchi-mcp`: lets an assistant such as Claude work in the user's
//! openarchi library over the Model Context Protocol, on stdin and stdout.
//!
//! stdout carries the protocol, so anything for a person goes to stderr.

mod export;
mod export_inputs;
mod figure_inputs;
mod figures;
mod highlight_inputs;
mod highlights;
mod inputs;
mod laser;
mod look;
mod model;
mod model_inputs;
mod palettes;
mod reports;
mod resolve;
mod sections;
mod series;
mod server;
mod sheet_inputs;
mod sheets;
mod style_inputs;
mod styles;
mod summary;
mod text;
mod text_inputs;
mod user_themes;
mod view_point;
mod wind;

use std::process::ExitCode;

use openarchi_core::library::Library;
use rmcp::transport::stdio;
use rmcp::ServiceExt;

use crate::server::OpenArchi;

#[tokio::main]
async fn main() -> ExitCode {
    let library = match Library::open_default() {
        Ok(library) => library,
        Err(error) => {
            eprintln!("openarchi-mcp: could not open the project library: {error}");
            return ExitCode::FAILURE;
        }
    };

    let server = OpenArchi::new(library.root().to_path_buf());
    let running = match server.serve(stdio()).await {
        Ok(running) => running,
        Err(error) => {
            eprintln!("openarchi-mcp: could not start: {error}");
            return ExitCode::FAILURE;
        }
    };

    match running.waiting().await {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("openarchi-mcp: stopped with an error: {error}");
            ExitCode::FAILURE
        }
    }
}
