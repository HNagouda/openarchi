//! `openarchi-mcp`: lets an assistant such as Claude work in the user's
//! openarchi library over the Model Context Protocol, on stdin and stdout.
//!
//! stdout carries the protocol, so anything for a person goes to stderr.

mod inputs;
mod look;
mod resolve;
mod server;
mod summary;
mod user_themes;

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
