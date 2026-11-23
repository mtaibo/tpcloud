#!/usr/bin/env bash
# Builds the desktop app for the architecture Docker runs on here, in Debian (glibc: what desktop
# distributions use; Wayland, X11 and Vulkan are loaded at run time, so it needs none of their
# development files).
#
#   desktop/build.sh   → desktop/dist/tpcloud-app-linux-<arch>
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
arch="$(docker version --format '{{.Server.Arch}}')"
docker run --rm -e CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback \
  -v "$here":/src -w /src \
  -v "tpcloud-desktop-registry-$arch":/usr/local/cargo/registry \
  -v "tpcloud-desktop-target-$arch":/src/target \
  rust:1.88-bookworm sh -c "cargo build --release --locked && mkdir -p dist && cp target/release/tpcloud-app dist/tpcloud-app-linux-$arch"
ls -la "$here/dist/tpcloud-app-linux-$arch"
