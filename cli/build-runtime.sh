#!/usr/bin/env bash
# Builds runtime.tar.zst, everything a server runs without Docker (see runtime/Dockerfile), for the
# architecture Docker runs on here. Docker is needed only on the machine that builds.
#
#   TPHOME_DIR=../tphome cli/build-runtime.sh      → cli/dist/runtime-<arch>.tar.zst
set -euo pipefail

cli="$(cd "$(dirname "$0")" && pwd)"
cloud="$(dirname "$cli")"
home="${TPHOME_DIR:-$(dirname "$cloud")/tphome}"
[[ -f "$home/docker-compose.yml" ]] || { echo "No tphome checkout at $home (set TPHOME_DIR)" >&2; exit 1; }
arch="$(docker version --format '{{.Server.Arch}}')"

ctx="$(mktemp -d)"
trap 'rm -rf "$ctx"' EXIT
pack() { # pack <repo> <name> <paths...>: the tracked files under <paths>, as in the working tree
  local repo="$1" name="$2"; shift 2
  mkdir -p "$ctx/$name"
  (cd "$repo" && git ls-files -z -- "$@" | xargs -0 -I{} sh -c 'test -e "{}" && echo "{}"' | tar -cf - -T -) | tar -xf - -C "$ctx/$name"
}
pack "$cloud" tpcloud services web
pack "$home" tphome api
cp "$cli/runtime/Dockerfile" "$ctx/Dockerfile"

mkdir -p "$cli/dist"
docker build --output "type=local,dest=$ctx/out" "$ctx"
mv "$ctx/out/runtime.tar.zst" "$cli/dist/runtime-$arch.tar.zst"
ls -la "$cli/dist/runtime-$arch.tar.zst"
