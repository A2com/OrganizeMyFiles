#!/usr/bin/env bash
# Construit OrganizeMyFiles.app et OrganizeMyFiles.dmg (à lancer sur le Mac Studio, depuis la racine du projet).
# Signature « ad hoc » seulement : pas de compte Apple Developer, donc au premier lancement
# macOS demande un clic droit > Ouvrir. UNIVERSAL=1 ajoute l'architecture Intel (cible x86_64-apple-darwin requise).
set -euo pipefail
export PATH=/opt/homebrew/bin:$HOME/.cargo/bin:$PATH

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
# Version lue uniquement dans la section [package] du manifeste.
VERSION="$(awk '
  /^\[package\]/ { inpkg = 1; next }
  /^\[/ { inpkg = 0 }
  inpkg && /^version = "/ { sub(/^version = "/, ""); sub(/"$/, ""); print; exit }
' app/Cargo.toml)"
if [ -z "$VERSION" ]; then
  echo "ERREUR : version introuvable dans la section [package] de app/Cargo.toml." >&2
  exit 1
fi
if ! printf '%s' "$VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "ERREUR : version « $VERSION » : format X.Y.Z attendu." >&2
  exit 1
fi
LOCK_VERSION="$(awk '
  /^name = "OrganizeMyFiles"$/ { found = 1; next }
  found && /^version = "/ { sub(/^version = "/, ""); sub(/"$/, ""); print; exit }
' app/Cargo.lock)"
if [ "$LOCK_VERSION" != "$VERSION" ]; then
  echo "ERREUR : app/Cargo.lock indique « $LOCK_VERSION » au lieu de « $VERSION »." >&2
  echo "         Lancer « cargo metadata --format-version 1 » dans app/, ou scripts/versionner.sh." >&2
  exit 1
fi
DIST="$ROOT/dist"
APP="$DIST/OrganizeMyFiles.app"
BIN=OrganizeMyFiles

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

# Programme de lecture des PDF et des images (PDFKit + Vision)
swiftc -O tools/omf-extract.swift -o "$APP/Contents/MacOS/omf-extract"

sed "s/@VERSION@/$VERSION/g" assets/Info.plist.in > "$APP/Contents/Info.plist"
PLIST="$APP/Contents/Info.plist"
SHORT="$(plutil -extract CFBundleShortVersionString raw "$PLIST")"
BUILD="$(plutil -extract CFBundleVersion raw "$PLIST")"
if [ "$SHORT" != "$VERSION" ] || [ "$BUILD" != "$VERSION" ]; then
  echo "ERREUR : Info.plist ($SHORT / $BUILD) ne porte pas la version $VERSION." >&2
  exit 1
fi

cp assets/icons/macos/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"

codesign --force --deep --sign - "$APP"
codesign --verify --deep --strict "$APP"

DMG="$DIST/OrganizeMyFiles-$VERSION.dmg"
hdiutil create -volname "OrganizeMyFiles" -srcfolder "$APP" -ov -format UDZO "$DMG" >/dev/null
if [ ! -f "$DMG" ]; then
  echo "ERREUR : le .dmg attendu n'a pas été créé : $DMG" >&2
  exit 1
fi
echo "OK : $APP et $DMG"
