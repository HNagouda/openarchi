#!/usr/bin/env bash
# Calls one tool on the openarchi MCP server and prints the result.
#
#   scripts/mcp.sh list_projects '{}'
#   scripts/mcp.sh search_places '{"query": "Westermarkt 20 Amsterdam"}'
#   scripts/mcp.sh --release render_preview '{"id": "2026-10-24-westermarkt-3f9a"}'
#   scripts/mcp.sh tools/list
#
# It builds the server (debug, or release with --release), then speaks the
# protocol the way Claude does: initialize, notifications/initialized, then
# tools/call. Images are saved under .scratch/mcp/ and their paths printed,
# rather than dumped as base64.
#
# The library defaults to .scratch/library so a test run never touches real
# projects. Set OPENARCHI_LIBRARY_DIR to point it elsewhere.

set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

profile="debug"
if [[ "${1:-}" == "--release" ]]; then
    profile="release"
    shift
fi

if [[ $# -lt 1 ]]; then
    echo "usage: scripts/mcp.sh [--release] <tool> ['<json args>']" >&2
    echo "       scripts/mcp.sh [--release] tools/list" >&2
    exit 2
fi
tool="$1"
args="${2:-{\}}"

cargo_flags=(--quiet -p openarchi-mcp)
if [[ "$profile" == "release" ]]; then
    cargo_flags+=(--release)
fi
# Compiler warnings would bury the result, so the build log only shows on failure.
if ! build_log="$(cargo build --manifest-path "$repo/Cargo.toml" "${cargo_flags[@]}" 2>&1)"; then
    echo "$build_log" >&2
    exit 1
fi

binary="$repo/target/$profile/openarchi-mcp"
if [[ -f "$binary.exe" ]]; then
    binary="$binary.exe"
fi

export OPENARCHI_LIBRARY_DIR="${OPENARCHI_LIBRARY_DIR:-$repo/.scratch/library}"
image_dir="$repo/.scratch/mcp"
mkdir -p "$OPENARCHI_LIBRARY_DIR" "$image_dir"

# Python drives the exchange because the server must stay connected until
# the reply arrives; closing stdin early makes it stop mid-request.
python3 - "$binary" "$tool" "$args" "$image_dir" <<'PYTHON'
import base64
import json
import subprocess
import sys
import time

# The newest protocol version that still starts with an initialize handshake.
PROTOCOL_VERSION = "2025-11-25"

binary, tool, raw_args, image_dir = sys.argv[1:]
try:
    args = json.loads(raw_args)
except json.JSONDecodeError as error:
    sys.exit(f"arguments are not valid JSON: {error}")

server = subprocess.Popen([binary], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)


def send(message):
    server.stdin.write(json.dumps(message) + "\n")
    server.stdin.flush()


def reply_to(request_id):
    for line in server.stdout:
        message = json.loads(line)
        if message.get("id") == request_id:
            return message
    sys.exit("the server closed before replying; its log is above")


send({
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": {},
        "clientInfo": {"name": "openarchi-mcp.sh", "version": "0"},
    },
})
reply_to(1)
send({"jsonrpc": "2.0", "method": "notifications/initialized"})

if tool == "tools/list":
    send({"jsonrpc": "2.0", "id": 2, "method": "tools/list"})
else:
    send({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": tool, "arguments": args}})
response = reply_to(2)
server.stdin.close()
server.wait()

if "error" in response:
    print(json.dumps(response["error"], indent=2))
    sys.exit(1)

result = response["result"]
if tool == "tools/list":
    for listed in result["tools"]:
        print(f"{listed['name']}: {json.dumps(listed['inputSchema'].get('properties', {}))}")
    sys.exit(0)

for number, content in enumerate(result.get("content", [])):
    if content["type"] == "image":
        extension = content["mimeType"].split("/")[-1]
        path = f"{image_dir}/{tool}-{time.strftime('%Y%m%d-%H%M%S')}-{number}.{extension}"
        with open(path, "wb") as image:
            image.write(base64.b64decode(content["data"]))
        print(f"image saved to {path}")
    elif content["type"] == "text":
        print(content["text"])
    else:
        print(json.dumps(content, indent=2))

if result.get("isError"):
    sys.exit(1)
PYTHON
