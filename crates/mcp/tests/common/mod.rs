//! Drives the real `openarchi-mcp` binary over stdio, the way Claude does,
//! against a throwaway library. Shared by every protocol test file; each one
//! covers a tool area.
//!
//! Tests marked `#[ignore]` need the data or render layer, which are still
//! `todo!()`. Run them with `cargo test -p openarchi-mcp -- --ignored` once
//! those land, then drop the `#[ignore]`.

// Each test file is its own crate and uses only some of these helpers.
#![allow(dead_code)]

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use openarchi_core::library::Library;
use openarchi_core::scene::Scene;
use serde_json::{json, Value};
use tempfile::TempDir;

/// The newest protocol version that still starts with an initialize handshake.
pub const PROTOCOL_VERSION: &str = "2025-11-25";

/// The core's recorded Westermarkt scene, so a test can have a fetched site offline.
pub const WESTERMARKT_SCENE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../core/tests/fixtures/westermarkt.scene.json"
);

/// A running server, initialized, with its own empty library.
pub struct Server {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
    pub library: TempDir,
}

impl Server {
    pub fn start() -> Server {
        let library = TempDir::new().unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_openarchi-mcp"))
            .env("OPENARCHI_LIBRARY_DIR", library.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());

        let mut server = Server {
            child,
            stdin,
            stdout,
            next_id: 1,
            library,
        };
        server.request(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": "protocol-test", "version": "0" },
            }),
        );
        server.send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
        server
    }

    pub fn send(&mut self, message: Value) {
        writeln!(self.stdin, "{message}").unwrap();
        self.stdin.flush().unwrap();
    }

    /// Sends a request and returns its `result`, skipping any notifications.
    pub fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));

        loop {
            let mut line = String::new();
            assert!(
                self.stdout.read_line(&mut line).unwrap() > 0,
                "server closed"
            );
            let message: Value = serde_json::from_str(&line).unwrap();
            if message["id"] == json!(id) {
                assert!(message.get("error").is_none(), "{message}");
                return message["result"].clone();
            }
        }
    }

    /// Calls a tool and returns its result as is, error or not.
    pub fn call(&mut self, tool: &str, arguments: Value) -> Value {
        self.request(
            "tools/call",
            json!({ "name": tool, "arguments": arguments }),
        )
    }

    /// Calls a tool that must succeed and parses its JSON text.
    pub fn call_ok(&mut self, tool: &str, arguments: Value) -> Value {
        let result = self.call(tool, arguments);
        assert_eq!(result["isError"], json!(false), "{tool} failed: {result}");
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap()
    }

    /// Highlights a circle around the Westerkerk and opens it as a sheet, the
    /// only way to a sheet of part of the site. Returns every tab.
    pub fn open_circle_as_sheet(&mut self, id: &str, label: &str, radius_m: f64) -> Value {
        let added = self.call_ok(
            "add_highlight",
            json!({
                "id": id,
                "label": label,
                "shape": { "kind": "circle", "center": { "lon": 4.8840, "lat": 52.3745 }, "radius_m": radius_m },
            }),
        );
        let highlights = added["highlights"].as_array().unwrap();
        let highlight_id = highlights.last().unwrap()["id"].clone();
        self.call_ok(
            "open_highlight_as_sheet",
            json!({ "id": id, "highlight_id": highlight_id }),
        )
    }

    /// Calls a tool that must fail and returns its message.
    pub fn call_err(&mut self, tool: &str, arguments: Value) -> String {
        let result = self.call(tool, arguments);
        assert_eq!(result["isError"], json!(true), "{tool} succeeded: {result}");
        result["content"][0]["text"].as_str().unwrap().to_string()
    }

    pub fn create_westermarkt(&mut self) -> String {
        let created = self.call_ok(
            "create_project",
            json!({
                "name": "Westermarkt",
                "area": {
                    "kind": "circle",
                    "center": { "lon": 4.8837, "lat": 52.3747 },
                    "radius_m": 150.0,
                },
            }),
        );
        created["id"].as_str().unwrap().to_string()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A project holding the recorded Westermarkt scene, so sheets and pins have a site offline.
pub fn fetched_westermarkt(server: &mut Server) -> String {
    let id = server.create_westermarkt();
    let library = Library::at(server.library.path()).unwrap();
    let mut project = library.load(&id).unwrap();
    let scene: Scene =
        serde_json::from_str(&std::fs::read_to_string(WESTERMARKT_SCENE).unwrap()).unwrap();
    project.scene = Some(scene);
    library.save(&mut project).unwrap();
    id
}
