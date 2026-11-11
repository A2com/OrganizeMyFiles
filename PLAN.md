# Plan : nettoyage du dépôt puis finalisation de OrganizeFiles (en Rust)

> **Statut au 2026-11-11** — Partie A (nettoyage) : **faite** (historique réécrit, dépôt, disque local et Mac Studio nettoyés).
> Partie B : **à réaliser**, une branche par tâche, dans l'ordre du tableau.
>
> Avancement partie B : **B0 fait** · **B1 `feat/persistance` fait** (PR en relecture) · B2 fait · B3 fait (PR en relecture) · B4 à B10 à faire.

## Contexte
L'appli (écrans Aperçu, Dossiers, Règles, Historique, Réglages ; moteur de règles) est complète côté **interface**.
Pour qu'elle soit **vraiment utilisable**, il manque : sauvegarde des données, surveillance automatique, actions réelles,
réglages branchés, lecture des PDF/images, application macOS installable, droits d'accès macOS, tests.
Dépôt privé : https://github.com/A2com/OrganizeMyFiles (branches `main` = import initial, `feat/theme-nocturne` = tout le travail).

## Règles de travail (tout le plan)
- **Une branche par tâche**, nommée selon son type : `chore/…`, `feat/…`, `fix/…`, `test/…`, `docs/…`. Jamais de travail direct sur `main`.
- Commits atomiques, Conventional Commits en français ; **une pull request par branche** (description : quoi, pourquoi, comment vérifié, non vérifié).
- Avant chaque PR : `cargo test`, `cargo clippy`, build release **sur le Mac Studio** (règle du studio) + captures d'écran relues.
- Nouvelle dépendance = justifiée (maintenue, licence MIT/Apache, sans faille connue).
- Taille estimée : petit / moyen / grand (pas de durées inventées).

## Mini-glossaire (pour Andy)
- **FSEvents** : le système de macOS qui prévient quand un fichier change dans un dossier (sert à la surveillance automatique).
- **Bundle `.app`** : le « paquet » qu'on double-clique pour lancer une appli Mac (icône, nom, identifiant, droits).
- **Signature / notarisation** : tampons d'Apple qui évitent l'alerte « développeur non identifié ».
- **TCC** : les autorisations macOS (accès Bureau, Documents, Téléchargements, notifications…).
- **Barre des menus** : la zone en haut à droite de l'écran Mac, où vivent les petites icônes d'applis.

---

# PARTIE A — Nettoyage (exécution immédiate après approbation)

**Objectif** : supprimer **partout** (dépôt GitHub, historique git, disque du serveur, Mac Studio) les fichiers de la session Craft Anything d'origine.
Seul `app/` (code, `MECHANICS.md`, `mechanics.json`, polices) et `.gitignore` restent.

**À supprimer** : `attachments/`, `build-activity.log`, `capture/`, `checks/`, `craft.json`, `feed.json`, `inbox/`, `investigation.cany/`, `pinned/`, `reference/`, `transcript.json`, `usage.json`.

Branche de travail : `chore/nettoyage-craft` (pour le commit « supprime… » et le `.gitignore`) ; la réécriture d'historique s'applique à **toutes** les branches.

| # | Étape | Détail / garde-fou |
|---|---|---|
| A1 | Vérifier avant de détruire | `app/` du dépôt = copie du Mac (`rsync -rcn`), `cargo test` vert, et un `git clone` neuf du dépôt compile. |
| A2 | Sauvegarde **temporaire** | `git bundle create <scratchpad>/backup.bundle --all` (supprimée à la fin, étape A9). |
| A3 | Réécrire l'historique | `git filter-repo` n'est pas installé : on utilise `git filter-branch --index-filter 'git rm -r --cached --ignore-unmatch -q <chemins>' -- --all` (toutes les branches). |
| A4 | Purger | suppression de `refs/original`, `git reflog expire --expire=now --all`, `git gc --prune=now --aggressive`. |
| A5 | Vérifier | `git ls-tree -r` ne contient plus que `app/` + `.gitignore` ; `cargo test` vert ; le disque local n'a plus ces dossiers (sinon `rm -rf`). |
| A6 | `.gitignore` | ajouter ces noms pour qu'ils ne reviennent jamais par erreur. |
| A7 | **Force-push** | `git push --force-with-lease origin main feat/theme-nocturne` — **accord explicite d'Andy** (« supprime partout dans le dépôt »). Dépôt privé et récent. Note : GitHub garde les anciens commits accessibles *par leur numéro* un certain temps ; un nettoyage complet côté GitHub passe par leur support si besoin. |
| A8 | Mac Studio | après A7 et un clone de contrôle réussi : `rm -rf "/Users/andyaugustine/Documents/Craft Anything/OrganizeMyFiles"` (877 Mo, contient aussi un `app/target` de 871 Mo). |
| A9 | Ménage | supprimer la sauvegarde temporaire (A2). |
| A10 | Docs | créer `PLAN.md` (ce document) sur `docs/plan-finalisation` + PR vers `main`. Puis PR `feat/theme-nocturne` → `main`. |

**Conservés volontairement** (dis-le-moi si tu veux les supprimer aussi) :
`~/forel-build` sur le Mac (copie de compilation + captures, hors du dossier Craft) et `Craft Anything/forel` (4 Ko, dossier créé par l'outil Craft).

---

# PARTIE B — Feuille de route (une tâche = une branche)

## Vue d'ensemble et ordre
| Ordre | Branche | Résout | Taille | Dépend de |
|---|---|---|---|---|
| B0 | `docs/plan-finalisation` + PR du thème | ce document, fusion du travail existant | petit | A |
| B1 | `feat/persistance` | **#1** données perdues à la fermeture | moyen | B0 |
| B2 | `feat/actions-reelles` | **#3** 3 actions factices | moyen | B1 (indépendant en pratique) |
| B3 | `fix/textes-fr-historique` | textes anglais du moteur dans l'historique | petit | – |
| B4 | `feat/surveillance-auto` | **#2** pas de suivi automatique, lancement à l'ouverture de session | grand | B1 |
| B5 | `feat/lecture-contenu` | **#5** PDF + OCR pour la condition « Contenu » | grand | B1 |
| B6 | `feat/emballage-macos` | **#6** vraie appli `.app` signée et notarisée | grand | B1–B5 stables |
| B7 | `feat/permissions-macos` | accès Bureau/Documents/Téléchargements (TCC) | moyen | B6 |
| B8 | `feat/barre-des-menus-notifications` | **#4** réglages branchés (menu bar, notifications, rétention) | moyen | B4, B6 |
| B9 | `test/e2e-et-ci` | tests d'ensemble + intégration continue | moyen | en continu |
| B10 | `qa/test-manuel-mac` | checklist de recette sur le Mac | petit | tout |

Chaque tâche suit : **branche → code → tests → build Mac → captures → PR → relecture d'Andy → fusion**.

---

## B1 — `feat/persistance` (point 1 : rien n'est sauvegardé)
**But** : retrouver dossiers, règles, historique et réglages à chaque lancement.
- **Fichier** : `~/Library/Application Support/OrganizeMyFiles/state.json` (dossier standard macOS), écriture **atomique** (fichier temporaire puis renommage) pour ne jamais corrompre.
- **Contenu** : `schema_version`, dossiers (chemin, actif, priorité), règles (conditions, actions + paramètres, activée, récursion), historique (avec infos d'annulation), réglages (couleur d'accent, animation du fond, rétention, « En veille », menu bar, notifications), compteur du mois, compteurs d'identifiants.
- **Dépendances** : `serde` + `serde_json` (standards, MIT/Apache) ; dérivations sur `Folder`, `Rule`, `Condition`, `Action`, `HistoryEntry`, `Undo`, `Status`.
- **Quand sauvegarder** : après toute modification (drapeau « modifié », écriture groupée ~0,5 s plus tard) et à la fermeture (`eframe::App::on_exit`).
- **Robustesse** : fichier illisible → renommé `state.corrupt-<date>.json`, l'appli démarre vide avec un message clair ; migration par `schema_version` ; l'option `--screenshot` n'écrit jamais dans le vrai fichier.
- **Rétention** : au chargement et après chaque lot, purge de l'historique plus ancien que `retention_days` (réutilise `purge_history`).
- **Fichiers** : nouveau `app/src/store.rs`, `app/src/engine.rs` (dérivations), `app/src/ui.rs` (chargement/sauvegarde).
- **Tests** : aller-retour (écrire → relire = identique), fichier corrompu, version inconnue, écriture interrompue, purge à la rétention.
- **Fini quand** : je configure l'appli, je la quitte, je la relance : tout est là (vérifié à la main sur le Mac).

## B2 — `feat/actions-reelles` (point 3 : trois actions « coquilles vides »)
**But** : chacune fait réellement son travail, ou échoue **honnêtement** (statut « Échec » + message), jamais un faux « appliqué ».
- **Ouvrir une application** : `open -a "<application>" "<fichier>"` ; erreur si l'app est introuvable (déjà testée) ou si `open` échoue.
- **Importer dans une médiathèque** : Photos (`photos`), Musique (`music`), TV (`tv`) via AppleScript (`osascript`) ; première utilisation = demande d'autorisation macOS (Apple Events), message d'aide si refusée. Syntaxes exactes **à valider sur le Mac**.
- **Lancer un raccourci** : `shortcuts run "<nom>" --input-path "<fichier>"`, délai maximal **60 s** (`SCRIPT_TIMEOUT_SECONDS`), processus tué au dépassement, sortie d'erreur remontée.
- **Architecture** : petite interface `CommandRunner` pour pouvoir tester sans lancer de vraies applis ; hors macOS → « Disponible seulement sur macOS ». L'aperçu (simulation) ne lance rien.
- **Fichiers** : `app/src/engine.rs` (`ActionKind::{OpenApplication, ImportToLibrary, RunShortcut}`), nouveau `app/src/system.rs`.
- **Tests** : constructeurs de commandes (échappement des guillemets et chemins avec espaces), délai dépassé, échec propagé, simulation sans effet.
- **Fini quand** : chaque action a été exécutée pour de vrai sur le Mac Studio avec un fichier test.

## B3 — `fix/textes-fr-historique` (textes encore en anglais)
- Les messages du moteur viennent de l'application d'origine (valeurs exactes, notés dans `MECHANICS.md`) : on **ne les change pas**.
  On ajoute une **couche de traduction à l'affichage** (`fr_message()`) pour les messages connus (« A file already exists at the destination. », « Already in destination », « Skipped because… », « restored »…).
- Données de démonstration : libellés d'actions et messages en français.
- **Tests** : chaque message connu a sa traduction ; message inconnu = affiché tel quel.

## B4 — `feat/surveillance-auto` (point 2 : pas de suivi automatique)
**But** : les règles s'exécutent toutes seules quand un fichier arrive, sans cliquer sur « Organiser ».
- **Détection** : bibliothèque `notify` (utilise FSEvents sur macOS) sur chaque dossier actif ; respecte la profondeur de récursion de chaque règle.
- **Fiabilité** : attente de ~3 s après le dernier événement ; **on ignore les téléchargements en cours** (`.download`, `.crdownload`, `.part`, taille qui bouge) ; **anti-boucle** (on ignore les fichiers que l'appli vient elle-même de déplacer) ; respect de « En veille » et du réglage par dossier.
- **Exécution** : fil de travail en arrière-plan (jamais dans l'interface), résultats envoyés à l'historique par un canal ; badge/indicateur « surveillance active » réel (aujourd'hui seulement un texte).
- **Lancement à l'ouverture de session** : réglage « Lancer au démarrage » → fichier LaunchAgent `~/Library/LaunchAgents/com.a2com.organizemyfiles.plist` (`RunAtLoad`), option `--arriere-plan` (démarre sans fenêtre). Remplacé par l'API officielle (`SMAppService`) une fois l'appli emballée (B6).
- **Fichiers** : nouveau `app/src/watcher.rs`, `app/src/ui.rs` (état + réglage), `app/Cargo.toml` (`notify`).
- **Tests** : dossier temporaire → on dépose un fichier → la règle s'applique ; fichier partiel ignoré ; pas de boucle ; mise en veille.
- **Fini quand** : je dépose une image dans Téléchargements, l'appli ouverte ou réduite, et elle est rangée seule (vérifié sur le Mac).

## B5 — `feat/lecture-contenu` (point 5 : « Contenu » limité aux fichiers texte)
- **PDF** : extraction du texte avec une bibliothèque Rust (`pdf-extract` ou `lopdf`), limite **100 Mo** (valeur d'origine, `TEXT_LIMIT_BYTES`).
- **Images (OCR)** : reconnaissance de texte **native macOS (framework Vision)** ; limites d'origine : **25 Mo** et **12 000 px** ; langues français + anglais ; réservé à macOS.
- **Bureautique (Word, Excel, PowerPoint)** : deuxième temps (lecture du XML dans l'archive zip), noté « optionnel ».
- **Performance** : lecture à la demande seulement si une règle utilise « Contenu », cache par fichier (date de modification), jamais dans l'interface.
- **Fichiers** : `app/src/engine.rs` (`read_text_content`), nouveau `app/src/extract.rs`.
- **Tests** : PDF d'exemple, texte introuvable, fichier trop gros, image trop grande, absence de plantage sur fichier corrompu.
- **Fini quand** : une règle « Contenu contient *facture* » classe un vrai PDF et une photo d'un reçu.

## B6 — `feat/emballage-macos` (point 6 : pas de vraie appli Mac)
**But** : un `OrganizeMyFiles.app` (et un `.dmg`) qu'on installe comme n'importe quelle appli.
- **Script** `scripts/emballer-mac.sh` (à lancer sur le **Mac Studio**) : build release **universelle** (Apple Silicon + Intel, `lipo`), création du bundle, `Info.plist` (nom, identifiant `com.a2com.organizemyfiles`, version, catégorie, descriptions d'usage des dossiers), **icône `.icns`** générée depuis un visuel 1024 px, signature `codesign` (Developer ID, hardened runtime), **notarisation** (`notarytool`) puis `stapler`, image disque `hdiutil`.
- **Prérequis côté Andy** : compte **Apple Developer** (Developer ID Application + identifiants de notarisation). Sans lui : appli signée « ad hoc » seulement (alerte à l'ouverture).
- **Version** : lue dans `Cargo.toml` ; changelog court.
- **Fichiers** : `scripts/`, `assets/AppIcon.png`, `assets/Info.plist.in`, `assets/entitlements.plist`.
- **Fini quand** : le `.dmg` s'installe sur un Mac « propre » sans alerte et l'appli se lance au double-clic.

## B7 — `feat/permissions-macos` (droits d'accès)
- Descriptions d'usage dans `Info.plist` : `NSDesktopFolderUsageDescription`, `NSDocumentsFolderUsageDescription`, `NSDownloadsFolderUsageDescription`, `NSAppleEventsUsageDescription`.
- **Erreur « permission refusée »** détectée au moment de lire/déplacer : bandeau explicatif + bouton « Ouvrir les réglages de confidentialité » (`x-apple.systempreferences:…Privacy_FilesAndFolders`).
- Premier lancement : écran d'accueil court expliquant les autorisations avant que macOS ne les demande.
- Le sélecteur de dossier natif (déjà en place) donne l'accès au dossier choisi.
- **Tests** : simulation d'erreur de permission, message clair ; contrôle manuel : refuser puis accorder.

## B8 — `feat/barre-des-menus-notifications` (point 4 : réglages sans effet)
- **Barre des menus** : icône avec menu (Ouvrir, Organiser maintenant, Aperçu, En veille, Quitter) via `tray-icon` ; « Afficher dans la barre des menus » l'active/désactive ; fermer la fenêtre garde l'appli active quand l'icône est affichée.
- **Notifications** : réglage « Prévenir après chaque organisation » → notification native (« 3 fichiers rangés ») via `UNUserNotificationCenter` ; clic = ouvre l'historique.
- **Rétention de l'historique** : le curseur s'applique **automatiquement** (déjà prévu en B1), plus seulement via « Purger ».
- **Tests** : réglage coupé = aucune notification ; rétention appliquée ; contrôle manuel sur le Mac.

## B9 — `test/e2e-et-ci` (tests d'ensemble)
- **Tests d'enchaînement** (dossier temporaire) : ajouter dossier → créer règle → déposer fichier → exécuter → vérifier le résultat → annuler → vérifier le retour.
- **Tests d'écrans** : captures `--screenshot` de chaque écran (non vide, taille attendue) lancées sur le Mac ; vérification des états vide / liste / erreur.
- **Intégration continue** (GitHub Actions, runner macOS) : `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, build release.
- Nettoyage des avertissements actuels (`raw`, `needs_ocr`, `month_day`, `ComboBox::from_id_source` déprécié).

## B10 — `qa/test-manuel-mac` (recette)
Checklist à cocher sur le Mac Studio (et un second Mac si possible), consignée dans le dépôt :
sélecteur de dossier natif · glisser-déposer depuis le Finder · glisser pour réordonner · bouton Annuler (bandeau du bas et activité récente) · clics sur les tuiles de l'Aperçu · fluidité du fond animé · surveillance automatique · lancement à l'ouverture de session · icône barre des menus · notifications · refus puis octroi des permissions · mise en veille · fermeture/relance (données conservées) · installation du `.dmg`.

---

## Définition de « complètement fonctionnel »
1. Je relance l'appli : **tout est retrouvé**. 2. Un fichier déposé dans un dossier suivi est **rangé automatiquement**.
3. **Les 12 actions font ce qu'elles disent** (ou échouent clairement). 4. Chaque réglage a un **effet réel**.
5. « Contenu » lit **texte, PDF et images**. 6. Un `.dmg` **signé et notarisé** s'installe sans alerte.
7. Les permissions macOS sont **expliquées et gérées**. 8. Tests + CI **verts**, recette manuelle cochée.

## Vérification de cette étape (nettoyage)
- `git ls-tree -r --name-only HEAD` (toutes branches) → uniquement `app/**` et `.gitignore`.
- `git clone` neuf du dépôt → `cargo test` vert → build release sur le Mac réussi.
- `ls` du dossier local et du Mac : plus aucun des fichiers listés ; `Craft Anything/OrganizeMyFiles` absent.
- `gh repo view` : dépôt toujours **privé**, branches à jour.

## Risques
- **Force-push** : historique réécrit (accord donné) ; sauvegarde temporaire jusqu'à vérification complète.
- Suppression du dossier Mac (877 Mo) irréversible : exécutée **en dernier**, seulement après le clone de contrôle réussi.
- Notarisation et AppleScript d'import : dépendent du compte Apple et d'essais sur le Mac (non vérifiables ici).
