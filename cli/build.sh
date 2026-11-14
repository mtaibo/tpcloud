#!/usr/bin/env bash
# Builds the single-file Linux binary: the runtime (build-runtime.sh, unless SKIP_RUNTIME=1 and one
# is already there), then a static (musl) tpcloud carrying it, for the architecture Docker runs on
# here. Result: cli/dist/tpcloud-linux-<arch>. Docker is needed only on the building machine.
set -euo pipefail

cli="$(cd "$(dirname "$0")" && pwd)"
arch="$(docker version --format '{{.Server.Arch}}')"
if [[ "${SKIP_RUNTIME:-}" != 1 || ! -f "$cli/dist/runtime-$arch.tar.zst" ]]; then
  "$cli/build-runtime.sh"
fi
cp "$cli/dist/runtime-$arch.tar.zst" "$cli/runtime.tar.zst"
docker run --rm \
  -v "$cli":/src -w /src \
  -v tpcloud-cargo-registry:/usr/local/cargo/registry \
  -v tpcloud-cargo-target:/src/target \
  rust:1.88-alpine sh -c 'apk add -q musl-dev >/dev/null && cargo build --release --locked 2>&1 | grep -E "^(warning|error)|-->|Finished" ; cp target/release/tpcloud dist/tpcloud-linux-'"$arch"
ls -la "$cli/dist/tpcloud-linux-$arch"
