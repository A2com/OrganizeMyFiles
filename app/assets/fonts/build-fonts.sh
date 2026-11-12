#!/usr/bin/env bash
# Régénère les sous-ensembles de polices embarqués. Usage : build-fonts.sh <dossier-sources> (avec fonttools installé)
#   <dossier-sources>/ms.ttf        = MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].ttf (google/material-design-icons)
#   <dossier-sources>/ms.codepoints = fichier .codepoints associé
#   <dossier-sources>/outfit/Outfit-var.ttf = Outfit[wght].ttf (google/fonts, ofl/outfit)
set -euo pipefail
SRC="$1"; OUT="$(cd "$(dirname "$0")" && pwd)"
ICONS="dashboard folder_managed rule history memory tune search notifications settings bolt filter_alt drag_indicator close add check_circle schedule visibility power_settings_new sort arrow_forward delete folder_open refresh check undo"
UNI=""
for n in $ICONS; do
  cp=$(awk -v n="$n" '$1==n {print $2}' "$SRC/ms.codepoints"); UNI="$UNI,U+$cp"
done
python3 -m fontTools.varLib.instancer "$SRC/ms.ttf" FILL=0 GRAD=0 opsz=24 wght=400 -o "$SRC/ms-static.ttf"
python3 -m fontTools.subset "$SRC/ms-static.ttf" --unicodes="${UNI#,}" --output-file="$OUT/MaterialSymbolsOutlined-subset.ttf" --no-hinting
for pair in Regular:400 SemiBold:600 Bold:700; do
  name="${pair%%:*}"; wght="${pair##*:}"
  python3 -m fontTools.varLib.instancer "$SRC/outfit/Outfit-var.ttf" wght="$wght" -o "$SRC/outfit/Outfit-$name-full.ttf"
  python3 -m fontTools.subset "$SRC/outfit/Outfit-$name-full.ttf" --unicodes="U+0020-007E,U+00A0-00FF,U+0100-017F,U+2013-2026,U+20AC,U+2190-2193,U+2318" --layout-features='kern,tnum,calt,liga' --output-file="$OUT/Outfit-$name.ttf" --no-hinting
done
