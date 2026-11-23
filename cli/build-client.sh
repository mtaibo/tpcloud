#!/usr/bin/env bash
# The client-only `tpcloud` (no server inside: ~2 MB) for Linux, static, for the architecture
# Docker runs on here: what `curl …/get | sh` installs on a computer that isn't a server.
#
#   cli/build-client.sh   → cli/dist/tpcloud-cli-linux-<arch>
set -euo pipefail
cli="$(cd "$(dirname "$0")" && pwd)"
arch="$(docker version --format '{{.Server.Arch}}')"
docker run --rm -v "$cli":/src -w /src \
  -v "tpcloud-cargo-registry-$arch":/usr/local/cargo/registry \
  -v "tpcloud-cargo-client-$arch":/src/target \
  rust:1.88-alpine sh -c "apk add -q musl-dev && cargo build --release --locked --no-default-features && mkdir -p dist && cp target/release/tpcloud dist/tpcloud-cli-linux-$arch"
ls -la "$cli/dist/tpcloud-cli-linux-$arch"
