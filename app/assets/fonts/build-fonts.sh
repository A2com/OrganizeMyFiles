#!/usr/bin/env bash
# Régénère les sous-ensembles de polices embarqués. Usage : build-fonts.sh <dossier-sources> (avec fonttools installé)
#   <dossier-sources>/ms.ttf        = MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].ttf (google/material-design-icons)
#   <dossier-sources>/ms.codepoints = fichier .codepoints associé
#   <dossier-sources>/inter/        = Inter-Regular.ttf, Inter-SemiBold.ttf, Inter-Bold.ttf (rsms/inter v4.1, extras/ttf)
set -euo pipefail
SRC="$1"; OUT="$(cd "$(dirname "$0")" && pwd)"
ICONS="dashboard folder_managed rule history memory tune search notifications settings bolt filter_alt drag_indicator close add check_circle schedule visibility power_settings_new sort arrow_forward"
UNI=""
for n in $ICONS; do
  cp=$(awk -v n="$n" '$1==n {print $2}' "$SRC/ms.codepoints"); UNI="$UNI,U+$cp"
done
python3 -m fontTools.varLib.instancer "$SRC/ms.ttf" FILL=0 GRAD=0 opsz=24 wght=400 -o "$SRC/ms-static.ttf"
python3 -m fontTools.subset "$SRC/ms-static.ttf" --unicodes="${UNI#,}" --output-file="$OUT/MaterialSymbolsOutlined-subset.ttf" --no-hinting
for w in Regular SemiBold Bold; do
  python3 -m fontTools.subset "$SRC/inter/Inter-$w.ttf" --unicodes="U+0020-007E,U+00A0-00FF,U+2013-2026,U+20AC,U+2190-2193,U+2318" --layout-features='kern,tnum,calt' --output-file="$OUT/Inter-$w.ttf" --no-hinting
done
