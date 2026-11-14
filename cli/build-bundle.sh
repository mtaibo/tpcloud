#!/usr/bin/env bash
# Packs what the tpcloud binary carries inside into cli/bundle.tar.gz: tpcloud's compose file,
# services and web apps, and tphome's compose file and API. Only files git tracks (as they are in
# the working tree), so nothing local like .env, data or node_modules ever goes in.
#
#   TPHOME_DIR=../tphome cli/build-bundle.sh      (default: tphome next to this repo)
set -euo pipefail

cli="$(cd "$(dirname "$0")" && pwd)"
cloud="$(dirname "$cli")"
home="${TPHOME_DIR:-$(dirname "$cloud")/tphome}"
[[ -f "$home/docker-compose.yml" ]] || { echo "No tphome checkout at $home (set TPHOME_DIR)" >&2; exit 1; }

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

pack() { # pack <repo> <name> <paths...>: copy the tracked files under <paths> into $tmp/<name>
  local repo="$1" name="$2"; shift 2
  mkdir -p "$tmp/$name"
  (cd "$repo" && git ls-files -z -- "$@" | xargs -0 -I{} sh -c 'test -e "{}" && echo "{}"' | tar -cf - -T -) | tar -xf - -C "$tmp/$name"
}

pack "$cloud" tpcloud docker-compose.yml .env.example services web
pack "$home" tphome docker-compose.yml .env.example api

# Fixed owner and times, so the same sources make the same bundle.
tar --sort=name --owner=0 --group=0 --numeric-owner --mtime='2026-01-01 00:00Z' -czf "$cli/bundle.tar.gz" -C "$tmp" tpcloud tphome 2>/dev/null \
  || tar -czf "$cli/bundle.tar.gz" -C "$tmp" tpcloud tphome
echo "bundle: $(du -h "$cli/bundle.tar.gz" | cut -f1), $(tar -tzf "$cli/bundle.tar.gz" | grep -vc '/$') files"
