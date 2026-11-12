#!/usr/bin/env bash
# Compile le programme de lecture des PDF et des images (macOS) à côté des binaires de développement,
# pour que `cargo run` et `cargo test` le trouvent. Usage : scripts/construire-omf-extract.sh [debug|release]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROFILE="${1:-debug}"
mkdir -p "$ROOT/app/target/$PROFILE"
swiftc -O "$ROOT/tools/omf-extract.swift" -o "$ROOT/app/target/$PROFILE/omf-extract"
echo "OK : $ROOT/app/target/$PROFILE/omf-extract"
