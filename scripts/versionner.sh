#!/usr/bin/env bash
# Prépare une version (app/Cargo.toml, Cargo.lock, CHANGELOG) ou vérifie leur cohérence.
#
#   scripts/versionner.sh X.Y.Z      prépare la version X.Y.Z
#   scripts/versionner.sh --verifier contrôle seulement (n'écrit rien)
#
# Ne commit rien, ne tague rien, ne pousse rien.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
CARGO_FILE="app/Cargo.toml"
LOCK_FILE="app/Cargo.lock"
CHANGELOG="CHANGELOG.md"
export CARGO_TARGET_DIR="${TMPDIR:-/tmp}/omf-versionner-target"

die() {
  echo "ERREUR : $*" >&2
  exit 1
}

is_semver() {
  printf '%s' "$1" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'
}

# Refuse les zéros de tête (« 01.2.3 »), que SemVer interdit.
has_no_leading_zero() {
  local a b c part
  IFS=. read -r a b c <<EOF
$1
EOF
  for part in "$a" "$b" "$c"; do
    case "$part" in
      0 | [1-9]*) ;;
      *) return 1 ;;
    esac
  done
}

manifest_version() {
  awk '
    /^\[package\]/ { inpkg = 1; next }
    /^\[/ { inpkg = 0 }
    inpkg && /^version = "/ { sub(/^version = "/, ""); sub(/"$/, ""); print; exit }
  ' "$CARGO_FILE"
}

lock_version() {
  awk '
    /^name = "OrganizeMyFiles"$/ { found = 1; next }
    found && /^version = "/ { sub(/^version = "/, ""); sub(/"$/, ""); print; exit }
  ' "$LOCK_FILE"
}

changelog_version() {
  awk '
    /^## \[[0-9]+\.[0-9]+\.[0-9]+\]/ { sub(/^## \[/, ""); sub(/\].*$/, ""); print; exit }
  ' "$CHANGELOG"
}

# Vrai si $1 est strictement supérieure à $2.
version_newer() {
  local a b c x y z
  IFS=. read -r a b c <<EOF
$1
EOF
  IFS=. read -r x y z <<EOF
$2
EOF
  [ "$a" -gt "$x" ] && return 0
  [ "$a" -lt "$x" ] && return 1
  [ "$b" -gt "$y" ] && return 0
  [ "$b" -lt "$y" ] && return 1
  [ "$c" -gt "$z" ]
}

unreleased_is_empty() {
  awk '
    /^## \[Non publié\]$/ { inun = 1; next }
    inun && /^## \[/ { inun = 0 }
    inun && /^- / { found = 1 }
    END { exit found ? 0 : 1 }
  ' "$CHANGELOG"
}

verify() {
  local mv lv cv
  mv="$(manifest_version)"
  [ -n "$mv" ] || die "version introuvable dans la section [package] de $CARGO_FILE"
  is_semver "$mv" || die "version « $mv » du manifeste : format X.Y.Z attendu"
  has_no_leading_zero "$mv" || die "version « $mv » : zéros de tête interdits"

  lv="$(lock_version)"
  [ -n "$lv" ] || die "entrée OrganizeMyFiles introuvable dans $LOCK_FILE"
  cv="$(changelog_version)"
  [ -n "$cv" ] || die "aucune section de version publiée dans $CHANGELOG"

  [ "$mv" = "$lv" ] || die "manifeste ($mv) et lock ($lv) diffèrent : lancer « cargo metadata --format-version 1 » dans app/"
  [ "$mv" = "$cv" ] || die "manifeste ($mv) et dernière version publiée du CHANGELOG ($cv) diffèrent"

  echo "OK : version $mv (manifeste, lock et CHANGELOG cohérents)."
}

prepare() {
  local new="$1" current tmp day
  is_semver "$new" || die "version « $new » : format X.Y.Z attendu"
  has_no_leading_zero "$new" || die "version « $new » : zéros de tête interdits"

  verify
  current="$(manifest_version)"
  version_newer "$new" "$current" || die "la nouvelle version $new doit être supérieure à $current"

  if git rev-parse -q --verify "refs/tags/v$new" >/dev/null; then
    die "le tag v$new existe déjà"
  fi
  if ! unreleased_is_empty; then
    die "la section [Non publié] du CHANGELOG est vide : ajouter les entrées avant de versionner"
  fi

  # 1. Version dans [package] uniquement.
  tmp="$(mktemp "${TMPDIR:-/tmp}/omf-cargo-toml.XXXXXX")"
  awk -v new="$new" '
    /^\[package\]/ { inpkg = 1 }
    /^\[/ && !/^\[package\]/ { inpkg = 0 }
    inpkg && /^version = "/ { sub(/^version = ".*"/, "version = \"" new "\"") }
    { print }
  ' "$CARGO_FILE" >"$tmp"
  mv "$tmp" "$CARGO_FILE"

  # 2. Lock par cargo, jamais à la main.
  if ! (cd app && cargo update -p OrganizeMyFiles --precise "$new" >/dev/null 2>&1); then
    (cd app && cargo metadata --format-version 1 --offline >/dev/null) ||
      (cd app && cargo metadata --format-version 1 >/dev/null)
  fi
  [ "$(lock_version)" = "$new" ] || die "le lock indique « $(lock_version) » au lieu de $new : vérifier app/Cargo.lock"

  # 3. Section datée dans le CHANGELOG (les entrées restent dessous).
  day="$(date +%Y-%m-%d)"
  tmp="$(mktemp "${TMPDIR:-/tmp}/omf-changelog.XXXXXX")"
  awk -v ver="$new" -v day="$day" '
    /^## \[Non publié\]$/ && !inserted {
      print "## [Non publié]"
      print ""
      print "## [" ver "] - " day
      inserted = 1
      next
    }
    { print }
  ' "$CHANGELOG" >"$tmp"
  mv "$tmp" "$CHANGELOG"

  echo "Version $current → $new préparée."
  git --no-pager diff --stat -- app/Cargo.toml app/Cargo.lock CHANGELOG.md
  cat <<EOF

Étapes suivantes :
  1. relire « git diff »
  2. git add app/Cargo.toml app/Cargo.lock CHANGELOG.md
  3. git commit -m "chore(version): $new"
  4. pull request, fusion, puis tag v$new (voir docs/VERSIONNEMENT.md)
EOF
}

case "${1:-}" in
  --verifier)
    verify
    ;;
  "")
    echo "Usage : $0 X.Y.Z | --verifier" >&2
    exit 2
    ;;
  *)
    prepare "$1"
    ;;
esac
