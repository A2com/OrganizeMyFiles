#!/usr/bin/env bash
# Finalise la mise en page du .dmg avec Finder lui-même (à lancer sur le Mac, session ouverte).
# Pourquoi : le .DS_Store écrit par dmgbuild n'est pas toujours respecté (bande d'état affichée, fenêtre à la
# mauvaise taille). Quand Finder écrit lui-même le .DS_Store, macOS le relit fidèlement.
# Usage : dmg-finder.sh <fichier.dmg>   (le fichier est réécrit sur place)
set -euo pipefail

DMG="${1:?Usage : dmg-finder.sh <fichier.dmg>}"
VOL="OrganizeMyFiles"
WORK="$(mktemp -d)"
MOUNT="/Volumes/$VOL"

cleanup() {
  hdiutil detach "$MOUNT" -force >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

if [ -e "$MOUNT" ]; then
  hdiutil detach "$MOUNT" >/dev/null 2>&1 || {
    echo "ERREUR : le volume « $VOL » est déjà monté ; éjectez-le puis relancez." >&2
    exit 1
  }
fi

hdiutil convert "$DMG" -format UDRW -o "$WORK/rw.dmg" -quiet
hdiutil attach "$WORK/rw.dmg" -noverify -noautoopen -quiet

# Fenêtre : 658 × 398 de contenu (426 avec la barre de titre), sans barre d'outils ni d'état, icônes de 125.
osascript <<OSA
tell application "Finder"
  tell disk "$VOL"
    open
    set cw to container window
    set current view of cw to icon view
    set toolbar visible of cw to false
    set statusbar visible of cw to false
    set the bounds of cw to {200, 120, 858, 546}
    set opts to the icon view options of cw
    set arrangement of opts to not arranged
    set icon size of opts to 125
    set text size of opts to 13
    set background picture of opts to file ".background.tiff"
    set position of item "OrganizeMyFiles.app" of cw to {165, 221}
    set position of item "Applications" of cw to {509, 221}
    close
    open
    delay 2
    close
  end tell
end tell
OSA

sleep 2
# Fichiers que macOS crée au montage : inutiles dans l'image livrée.
rm -rf "$MOUNT/.fseventsd" "$MOUNT/.Trashes" "$MOUNT/.Spotlight-V100" 2>/dev/null || true
sync

for essai in 1 2 3 4 5; do
  hdiutil detach "$MOUNT" -quiet 2>/dev/null && break
  sleep 2
done
[ ! -e "$MOUNT" ] || { echo "ERREUR : impossible d'éjecter $MOUNT." >&2; exit 1; }

hdiutil convert "$WORK/rw.dmg" -format UDZO -imagekey zlib-level=9 -ov -o "$DMG" -quiet
echo "OK : mise en page Finder appliquée à $DMG"
