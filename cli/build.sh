#!/usr/bin/env bash
# Builds the single-file Linux binary: packs the bundle, then compiles a static (musl) tpcloud
# inside Docker, for the machine Docker runs on (arm64 on a Raspberry Pi or an arm Mac, amd64
# elsewhere). The result is cli/dist/tpcloud-linux-<arch>.
set -euo pipefail

cli="$(cd "$(dirname "$0")" && pwd)"
"$cli/build-bundle.sh"
arch="$(docker version --format '{{.Server.Arch}}')"
mkdir -p "$cli/dist"
docker run --rm \
  -v "$cli":/src -w /src \
  -v tpcloud-cargo-registry:/usr/local/cargo/registry \
  -v tpcloud-cargo-target:/src/target \
  rust:1.88-alpine sh -c 'apk add -q musl-dev >/dev/null && cargo build --release --locked 2>&1 | tail -3 && cp target/release/tpcloud dist/tpcloud-linux-'"$arch"
ls -la "$cli/dist/tpcloud-linux-$arch"
