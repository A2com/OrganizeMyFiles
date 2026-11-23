# Versionnement d'OrganizeMyFiles

## Source unique

La version vit dans `app/Cargo.toml` (section `[package]`). Elle est propagée automatiquement :

- à l'écran **Réglages › À propos** par `env!("CARGO_PKG_VERSION")` (`app/src/ui.rs`) ;
- au `.app` et au `.dmg` par `scripts/emballer-mac.sh` (`Info.plist`, nom du fichier) ;
- à `app/Cargo.lock`, que `scripts/versionner.sh` met à jour via cargo.

`CFBundleVersion` reçoit la même valeur que `CFBundleShortVersionString` : pas d'App Store, pas de numéro de
build séparé. On n'en ajoutera un que si la notarisation ou l'App Store l'exigent.

## Règles

| Changement | Version |
|---|---|
| Correctif sans nouvelle fonctionnalité | patch : `0.1.0` → `0.1.1` |
| Fonctionnalité visible | mineure : `0.1.x` → `0.2.0` |
| Rupture de compatibilité (état, format, comportement) | majeure (décision explicite) |
| `1.0.0` | décision explicite d'Andy, pas automatique |

Tant qu'on reste en `0.1.x`, les correctifs incrémentent le dernier chiffre. Le numéro est choisi **au moment de
la préparation de version**, pas dans chaque PR de code : cela évite les conflits sur `CHANGELOG.md`.

## CHANGELOG

`CHANGELOG.md` suit le format Keep a Changelog, en français, avec les sections `Ajouté`, `Modifié`, `Corrigé`,
`Interne`.

- Les entrées en cours s'écrivent sous `## [Non publié]`.
- `scripts/versionner.sh X.Y.Z` transforme `[Non publié]` en section datée `## [X.Y.Z] - AAAA-MM-JJ`.
- Une entrée = une ligne courte avec le numéro de PR entre parenthèses.

## Procédure de version

```bash
# 1. Branche dédiée
git switch -c chore/version-0.1.1 origin/main

# 2. Écrire les entrées sous « ## [Non publié] » dans CHANGELOG.md

# 3. Bump (manifeste + lock + CHANGELOG)
scripts/versionner.sh 0.1.1

# 4. Vérifier, commiter, pousser, PR
scripts/versionner.sh --verifier
git diff
git add app/Cargo.toml app/Cargo.lock CHANGELOG.md
git commit -m "chore(version): 0.1.1"
git push -u origin chore/version-0.1.1
gh pr create --base main --title "chore(version) : 0.1.1" --body "…"

# 5. Après fusion et accord d'Andy : tag sur main
git switch main && git pull --ff-only
git tag -a v0.1.1 -m "OrganizeMyFiles 0.1.1"
git push origin v0.1.1

# 6. Build depuis le tag exact, sur le Mac Studio
git switch --detach v0.1.1
./scripts/emballer-mac.sh
git switch main

# 7. Release GitHub avec le .dmg (uniquement si Andy la demande)
gh release create v0.1.1 dist/OrganizeMyFiles-0.1.1.dmg \
  --title "OrganizeMyFiles 0.1.1" \
  --notes "$(awk '/^## \[0\.1\.1\]/{f=1;next}/^## \[/{f=0}f' CHANGELOG.md)"
```

Contrôles après build :

```bash
plutil -extract CFBundleShortVersionString raw dist/OrganizeMyFiles.app/Contents/Info.plist
plutil -extract CFBundleVersion raw dist/OrganizeMyFiles.app/Contents/Info.plist
ls -l dist/OrganizeMyFiles-0.1.1.dmg
```

Les deux clés, le nom du `.dmg`, `app/Cargo.toml` au tag et l'écran À propos doivent afficher la même version.

## Interdits

- Ne jamais déplacer ni réécrire un tag déjà publié : publier la version suivante.
- Ne jamais modifier `app/Cargo.lock` à la main : passer par cargo (`versionner.sh`).
- Ne pas créer de tag ni de release sans demande explicite d'Andy.
- Ne pas lancer le binaire avec `--version` : cette option n'existe pas, une option inconnue lance l'application
  normalement et bloque une commande SSH. Vérifier par l'écran À propos, le nom du `.dmg` ou `plutil`.

## Vérification rapide

```bash
scripts/versionner.sh --verifier
```

Le script contrôle que `app/Cargo.toml`, `app/Cargo.lock` et la dernière section publiée de `CHANGELOG.md`
portent la même version. Il s'exécute avant chaque PR de version et avant chaque build de release.

## Mises à jour intégrées

L'appli lit `https://api.github.com/repos/A2com/OrganizeMyFiles/releases/latest` (dépôt **public**, aucune clé).
Pour qu'une release soit proposée aux utilisateurs :

- le **tag** est `vX.Y.Z` (le « v » est ignoré à la lecture) et la version dépasse celle de l'appli installée ;
- la release contient le fichier `OrganizeMyFiles-X.Y.Z.dmg` produit par `scripts/emballer-mac.sh` ; le `.dmg` doit
  contenir `OrganizeMyFiles.app` à sa racine ;
- GitHub calcule l'empreinte SHA-256 du fichier ; l'appli la vérifie avant d'installer.

Une release marquée « pre-release » ou « brouillon » n'est jamais proposée (l'API `latest` les ignore).

## Fabrication du `.dmg`

`scripts/emballer-mac.sh` fabrique un installateur illustré avec `dmgbuild` (version épinglée, installé la première
fois dans `~/.cache/omf-dmg-venv` : **connexion Internet nécessaire au premier build**). Pas de Finder ni
d'AppleScript : la mise en page est écrite directement dans le `.DS_Store`.

- `scripts/dmg-reglages.py` : taille de fenêtre (658 × 398, celle du fond), taille des icônes, positions ;
- `assets/dmg/background.png` et `background@2x.png` : fond (1× et Retina, combinés par `dmgbuild`) ; `assets/dmg/applications-folder.png` : icône du raccourci ;
- `scripts/alias-applications.swift` : crée l'alias « Applications » (un alias, pas un lien symbolique, pour pouvoir
  lui donner une icône personnalisée).

`scripts/dmg-finder.sh` repasse ensuite l'image par Finder (AppleScript) : le `.DS_Store` écrit par `dmgbuild` n'était
pas toujours respecté (bande d'état affichée, fenêtre à la mauvaise taille) ; écrit par Finder lui-même, il est relu
fidèlement. Cette étape exige une **session ouverte sur le Mac** et l'autorisation d'automatiser Finder ; sinon
`emballer-mac.sh` avertit et garde l'image `dmgbuild`. Le fond vient de `~/…/OrganizeMyFiles/DMG Installer/`
(fichier `installer-background.jpg` en 1316 × 796, réduit en 658 × 398 pour la version 1×).

La fenêtre fait **exactement la taille du fond** : les bornes AppleScript de Finder comptent la barre de titre, dont la
hauteur dépend de macOS (28 pt avant macOS 26, 32 pt ensuite). `dmg-finder.sh` la **mesure** (`hauteur-barre-titre.swift`)
et ajoute le résultat aux 398 pt du fond ; sur macOS 27 : fenêtre de 658 × 430 pt, contenu de 658 × 398 pt. Un `.dmg`
construit sur un autre macOS aurait donc la bonne taille pour **ce** macOS, à 4 pt près sur l'autre.

L'appli reste à la **racine** du volume : la mise à jour intégrée l'y cherche.
