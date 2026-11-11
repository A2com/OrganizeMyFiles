#!/usr/bin/env bash
# Construit OrganizeFiles.app et OrganizeFiles.dmg (à lancer sur le Mac Studio, depuis la racine du projet).
# Signature « ad hoc » seulement : pas de compte Apple Developer, donc au premier lancement
# macOS demande un clic droit > Ouvrir. UNIVERSAL=1 ajoute l'architecture Intel (cible x86_64-apple-darwin requise).
set -euo pipefail
export PATH=/opt/homebrew/bin:$HOME/.cargo/bin:$PATH

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' app/Cargo.toml | head -1)"
DIST="$ROOT/dist"
APP="$DIST/OrganizeFiles.app"
BIN=file-organizer

rm -rf "$DIST"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

if [ "${UNIVERSAL:-0}" = "1" ]; then
  (cd app && cargo build --release --target aarch64-apple-darwin && cargo build --release --target x86_64-apple-darwin)
  lipo -create -output "$APP/Contents/MacOS/$BIN" \
    app/target/aarch64-apple-darwin/release/$BIN app/target/x86_64-apple-darwin/release/$BIN
else
  (cd app && cargo build --release)
  cp "app/target/release/$BIN" "$APP/Contents/MacOS/$BIN"
fi

sed "s/@VERSION@/$VERSION/g" assets/Info.plist.in > "$APP/Contents/Info.plist"

cp assets/icons/macos/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"

codesign --force --deep --sign - "$APP"
codesign --verify --deep --strict "$APP"

hdiutil create -volname "OrganizeFiles" -srcfolder "$APP" -ov -format UDZO "$DIST/OrganizeFiles-$VERSION.dmg" >/dev/null
echo "OK : $APP et $DIST/OrganizeFiles-$VERSION.dmg"
