#!/usr/bin/env bash
# Builds openarchi-mcp and puts it where Tauri looks for a sidecar binary.
#
#   scripts/build-mcp-sidecar.sh universal-apple-darwin
#   scripts/build-mcp-sidecar.sh x86_64-pc-windows-msvc
#
# app/src-tauri/tauri.installer.conf.json lists binaries/openarchi-mcp under
# bundle.externalBin. Tauri then expects app/src-tauri/binaries/openarchi-mcp-<target>
# (plus .exe on Windows) and copies it next to the app's own executable in the
# bundle, so every installer ships the MCP server.

set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
sidecar_dir="$repo/app/src-tauri/binaries"
mac_targets=(aarch64-apple-darwin x86_64-apple-darwin)

if [[ $# -ne 1 ]]; then
    echo "usage: scripts/build-mcp-sidecar.sh <target triple, or universal-apple-darwin>" >&2
    exit 2
fi
target="$1"

build_one() {
    cargo build --manifest-path "$repo/Cargo.toml" --release --locked -p openarchi-mcp --target "$1"
}

mkdir -p "$sidecar_dir"

if [[ "$target" == "universal-apple-darwin" ]]; then
    for arch_target in "${mac_targets[@]}"; do
        build_one "$arch_target"
        # A universal app build compiles each architecture on its own, and each of
        # those passes looks for a sidecar named after its own target.
        cp "$repo/target/$arch_target/release/openarchi-mcp" "$sidecar_dir/openarchi-mcp-$arch_target"
    done

    lipo -create \
        "$repo/target/aarch64-apple-darwin/release/openarchi-mcp" \
        "$repo/target/x86_64-apple-darwin/release/openarchi-mcp" \
        -output "$sidecar_dir/openarchi-mcp-universal-apple-darwin"
    exit 0
fi

build_one "$target"

extension=""
if [[ "$target" == *windows* ]]; then
    extension=".exe"
fi
cp "$repo/target/$target/release/openarchi-mcp$extension" "$sidecar_dir/openarchi-mcp-$target$extension"
