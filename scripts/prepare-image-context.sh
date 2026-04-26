#!/usr/bin/env bash
# Populate build/image/ for `docker build -f Dockerfile build/image`.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/release/website"
if [[ ! -f "$BIN" ]]; then
  echo "error: missing $BIN — run: cargo build --release --locked" >&2
  exit 1
fi
mkdir -p "$ROOT/build/image"
cp "$BIN" "$ROOT/build/image/website"
chmod 755 "$ROOT/build/image/website"
rm -rf "$ROOT/build/image/static"
cp -a "$ROOT/static" "$ROOT/build/image/static"
