#!/usr/bin/env bash
# Prépare les icônes illustrées des règles : réduit les originaux (1024 px) à 256 px et met à zéro le bruit d'opacité
# résiduel des coins (alpha < 8). Usage : scripts/generer-icones-regles.sh <dossier des originaux>
set -euo pipefail

src="${1:?Usage : $0 <dossier des originaux>}"
dest="$(cd "$(dirname "$0")/.." && pwd)/assets/ui/rules"
mkdir -p "$dest"

for name in archive-file audio-file calandar-30J-file document-file folder-copy folder-delete folder-empty folder-move \
  folder-rename folder-tag folder-zip images-file pdf-file rules tt video-file; do
  ffmpeg -loglevel error -y -i "$src/$name.png" \
    -vf "format=rgba,geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='if(lt(alpha(X,Y),8),0,alpha(X,Y))',scale=256:256:flags=lanczos" \
    "$dest/$name.png"
done
echo "Icônes écrites dans $dest"
