#!/usr/bin/env bash
# Capture chaque écran de l'appli dans trois états (vide, démonstration, erreur) et vérifie que les
# images ne sont pas vides. À lancer sur le Mac, depuis la racine du projet. Sortie : captures/*.png
set -euo pipefail
export PATH=/opt/homebrew/bin:$HOME/.cargo/bin:$PATH

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/app"
cargo build --release >/dev/null
BIN="$ROOT/app/target/release/OrganizeMyFiles"
OUT="$ROOT/captures"
STATE="$(mktemp -d)"
rm -rf "$OUT" && mkdir -p "$OUT"

# État « erreur » : un dossier illisible, une action refusée par macOS, un échec.
python3 - "$STATE" <<'PY'
import json, os, sys, time
d = sys.argv[1]
os.makedirs(d + "/ok", exist_ok=True)
os.makedirs(d + "/locked", exist_ok=True)
os.chmod(d + "/locked", 0)
now = int(time.time())
def entry(status, message):
    return {"batch_id": 1, "rule_name": "Tri", "action_label": "Déplacer vers un dossier",
            "original_path": d + "/ok/a.pdf", "result_path": d + "/ok/a.pdf", "undo": None, "reversible": False,
            "status": status, "message": message, "created": {"secs_since_epoch": now - 60, "nanos_since_epoch": 0}}
state = {"schema_version": 1,
         "folders": [{"id": 1, "path": d + "/ok", "enabled": True, "priority": 0},
                     {"id": 2, "path": d + "/locked", "enabled": True, "priority": 1},
                     {"id": 3, "path": d + "/absent", "enabled": True, "priority": 2}],
         "rules": [], "history": [entry("Failed", "Operation not permitted (os error 1)"), entry("BlockedByConflict", "A file already exists at the destination.")],
         "settings": {"accent": 0, "menu_bar": False, "notify": False, "animate_bg": False, "retention_days": 30,
                      "app_enabled": True, "launch_at_login": False, "welcome_seen": True},
         "next_id": 10, "next_batch": 2, "monthly_used": 0, "last_run": None, "last_run_dry": True}
os.makedirs(d + "/state", exist_ok=True)
json.dump(state, open(d + "/state/state.json", "w"))
PY

fail=0
for tab in dashboard folders rules history settings; do
  "$BIN" --tab "$tab" --empty --screenshot "$OUT/$tab-vide.png" >/dev/null
  "$BIN" --tab "$tab" --screenshot "$OUT/$tab-demo.png" >/dev/null
  ORGANIZEMYFILES_STATE_DIR="$STATE/state" "$BIN" --tab "$tab" --screenshot "$OUT/$tab-erreur.png" >/dev/null
done
chmod 755 "$STATE/locked"

for f in "$OUT"/*.png; do
  size=$(stat -f %z "$f")
  width=$(sips -g pixelWidth "$f" | awk '/pixelWidth/ {print $2}')
  if [ "$size" -lt 20000 ] || [ "$width" -lt 1000 ]; then
    echo "ÉCHEC : $f ($size octets, $width px)"; fail=1
  fi
done
[ "$fail" -eq 0 ] && echo "OK : $(ls "$OUT"/*.png | wc -l | tr -d ' ') captures dans $OUT"
exit "$fail"
