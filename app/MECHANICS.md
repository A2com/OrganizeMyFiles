# Mécanique de Forel — relevé Deep (décompilation du programme original)

Cible : `/Applications/Forel.app/Contents/MacOS/ForelApp` (Mach-O universal x86_64+arm64, sha256 `4dd949c0cff10656463a2891f9aba8d23f722c46556991f2635cef5af81056ff`).
Chaque constante est donnée avec **l'adresse de la fonction qui la porte**, relevée au décompilateur (native_function_pseudocode). Les valeurs hexadécimales sont celles vues dans le pseudo-code.

## 1. Plan gratuit — plafonds (ForelCore.FreePlan)

| Valeur | Adresse de fonction | Code observé |
|---|---|---|
| `folderLimit = 2` | `0x1000e2110` `FreePlan.folderLimitSivgZ` | `w0 = 2; return w0;` |
| `monthlyFileLimit = 300` (0x12c) | `0x100167f24` `FreePlan.monthlyFileLimitSivgZ` | `w0 = 0x12c; return w0;` |

Règles d'accès :
- `FreePlan.canAddFolder(count:isPro:)` — `0x100167fb0` : `isPro || count < folderLimit` (la fonction fait `cset w8, lt` puis `w8 = isPro | w8`).
- `FreePlan.allows(folderID:folders:isPro:)` — `0x100167fc4` (stub) → corps `0x100168984` : si `isPro`, vrai ; sinon, vrai seulement si `folderID` est déjà dans la liste des dossiers suivis (comparaison de chaînes via `stringCompareWithSmolCheck`). Un dossier déjà suivi reste donc autorisé même au-delà du plafond.
- `FreePlan.validate(_:isPro:)` — `0x100167f40` : si `isPro` est faux **et** que la règle a un certain drapeau (offset `+0x30`) alors elle appelle `FreePlan.requiredFeature(for:)` et lève `FreePlanError` si une fonctionnalité Pro est requise.
- `FreePlan.requiredFeature(for: ActionKind)` — `0x100167f2c` : renvoie 1 ou 2 selon le discriminant `ActionKind` (`cinc w0, w8, ne`) — les fonctions Pro (import médiathèques / OCR) sont celles dont la valeur n'est pas 2.

## 2. Quota mensuel (ForelCore.MonthlyFileUsageStore)

- `MonthlyFileUsageStore.pendingFilesKey` — `0x1001344e8` : `"free.quotaPendingFiles"` (littéral Swift dans le pseudo-code). Clé de persistance des fichiers en attente de quota.
- Clé de mois `"%04d-%02d"` : construite dans `MonthlyFileUsageStore.runWithFreePlan(...)` — `0x1001697f8`. Les immédiats vus à l'appel de `String(format:_:)` sont `0x3025 ("%0")`, `0x6434 ("4d")`, `0x252d ("-%")`, `0x64 ("d")`, `0x3230 ("02")` → la chaîne complète est bien `"%04d-%02d"` (année sur 4 chiffres, mois sur 2). Le mois est tiré de `Calendar.dateComponents([.year, .month], from:)` puis `Calendar.dateInterval(of: .month, for:)` pour borner la période.
- Comportement : si `isPro == 1`, la fonction saute directement à `RuleEngine.run` (aucune compte ni quota) ; sinon elle charge l'usage du mois courant, exécute `RuleEngine.run`, puis compte les fichiers traités dans la clé `free.monthlyFileUsage.<AAAA-MM>` (préfixe relevé dans les chaînes). Le mode Dry Run (aperçu) ne consomme pas le quota.

## 3. Extraction de contenu (ForelCore.ContentExtractor)

| Règle | Adresse de fonction | Code observé |
|---|---|---|
| Limite texte : `size >= 0x6400000` (104 857 600 = 100 Mo) | `0x10014d10c` `ContentExtractor.extractPlainText` | `w8 = 0x6400000; if (size < w8) … else` renvoie `"File exceeds the 100 MB text limit."` |
| Limite OCR : `size >= 0x1900000` (26 214 400 = 25 Mo) | `0x100150c7c` `ContentExtractor.extractImageOCR` | `w8 = 0x1900000; … else "Image exceeds the 25 MB OCR limit."` |
| Limite dimension image OCR : largeur `> 0x2ee0` (12 000) **ou** hauteur `>= 0x2ee1` (12 001) | `0x100150c7c` | `w8 = 0x2ee0; if (width > w8) → "Image is too large for OCR."` puis `w8 = 0x2ee1; if (height < w8) → OCR sinon même refus` |
| Encodages texte tentés dans l'ordre : UTF-8, UTF-16, ISO-Latin-1 | `0x10014d10c` | `String.Encoding.utf8 / .utf16 / .isoLatin1` puis `"Could not decode text."` si aucun ne décode |

## 4. Validation des noms de fichiers (ActionExecutor.validateMacOSFilename)

Adresse : `0x100138bec` `ActionExecutor.validateMacOSFilename`. Règles, dans l'ordre de contrôle (chaque refus a son message exact dans le binaire) :

1. contient `/` → `"rename pattern produced an invalid filename (contains '/')"`.
2. vaut `.` ou `..` → `"…('.' and '..' are not allowed)"`.
3. `UTF8.count > 0xff` (255) → `"…longer than 255 characters"` (comparaison `if (count > 0xff)`).
4. finit par `.` ou un espace → `"…ending with '.' or space — not supported by macOS"`.

## 5. Motifs de renommage (ActionExecutor.archiveFileName)

Adresse : `0x100140a14` `ActionExecutor.archiveFileName(pattern:sourceName:at:)`. Jetons remplacés (par `replacingOccurrences`) :

- `{name}` → nom de base de la source.
- `{date_created}` (vu à `var_88` : `"{date_created}"` en immédiats `0x637b 0x7275 0x6572 0x746e 0x645f 0x7461 0x7d65`).
- `{timestamp}` → date formatée `"yyyy-MM-dd_HH-mm-ss"` (format littéral vu dans le pseudo-code, locale `en_US_POSIX`).
- `{current_date}` → même format `"yyyy-MM-dd_HH-mm-ss"`.

Puis le nom est nettoyé (`trimmingCharacters(in: .whitespacesAndNewlines)`) et validé (refus : vide, `.`/`..`, contient `/`, contient un caractère nul, finit par `.` ou espace). Extension `.zip` **ajoutée automatiquement** si le nom ne finit pas déjà par `.zip` (comparaison `lowercased().hasSuffix(".zip")` puis `append(".zip")`). Longueur finale limitée à **255 octets UTF-8 y compris `.zip`** (`if (utf8Count >= 0x100)` → `"Archive name is too long (maximum 255 bytes including .zip).")`.

## 6. Nommage unique des destinations (ActionExecutor.uniqueDest)

Adresse : `0x100138560` `ActionExecutor.uniqueDest(dir:fileName:)`. Si la destination existe déjà, la fonction boucle en suffixant le nom par `" ("`, un compteur entier, puis `")"` — ex. `photo (2).jpg` — jusqu'à trouver un nom libre (compteur `w26` initialisé à 1, incrémenté à chaque tour via `x26 += 1`).

## 7. Décompression (ActionExecutor.uncompress)

Adresse : `0x100130cd8`. Dossier temporaire `.forel-uncompress-<UUID>` (préfixe littéral vu dans le pseudo-code) ; extraction via `NSFileManager.unzipItem` (ZIPFoundation). Plan d'extraction ZIP : `zipExtractionPlan` (`0x100132f0c` zone, corps à `0x1001313b4`…).

## 8. Règles, opérateurs, actions

Schéma SQL (relevé sur la base, fnd_3) inchangé : règles triées par `(priority, created_at)`, `condition_match` défaut `all`, `recursion_depth` défaut 0 ; conditions `(kind, operator, value)` ; actions `(kind, params JSON, position)` triées par `position`. Opérateurs de date `before`, `after`, `within_last` (unités `days/weeks/months/years`). Archives reconnues : `zip tar gz 7z rar xz zst tgz tbz cab`.

## 9. Annulation (Undo) — règles décompilées (ForelCore)

`UndoChecker.evaluate` — `0x10015e564` (corps `0x10015e2c8`). Contrôles **avant** annulation, dans cet ordre, avec leurs messages exacts du binaire :

1. drapeau `+0x79` bit 0 = 0 → `"This action cannot be undone."`
2. champ `+0x7a` ≠ 0 → `"This action is not currently applied."`
3. fichier déplacé introuvable → `"The file Forel moved no longer exists at <path>."`
4. copie créée disparue → `"The copy Forel created no longer exists."`
5. emplacement d'origine occupé → `"A file already exists at the original location."`
6. le fichier présent n'est plus le même (`_stat`, comparaison inode/taille stockés dans l'entrée) → `"The file at this location is no longer the same file Forel moved."`
7. fichier absent à l'emplacement attendu → `"The file no longer exists at its expected location."`
8. une règle active retraiterait aussitôt le fichier restauré (`ruleThatWouldReprocess`, même objet `0x10015e564`) → `The rule "<nom>" would immediately reprocess this file once it's restored.`

`ActionExecutor.revert` — `0x1001336c8` : annulation selon le discriminant `Undo` (0 = déplacer/renommer, 1 = copier, 2 = compresser, 3 = retirer les tags via `FinderTags.ready`/`write`, 4 = retirer la couleur via `FinderTags.setColorLabel`) ; sinon `ActionError` `"this action cannot be undone"` (minuscules, message distinct de l'évaluateur). Si l'emplacement source est occupé : `"<chemin>: a file already exists there"`.

## 10. Réglages persistants (base `app_settings`, ForelCore.Database)

- Clé `history_max_days` : chaîne à `0x1001ded10` ; lue dans `AppModel.init` (closure `0x10000a048`, `Database.getSetting`, entier parse en radix 10, signes `+`/`-` acceptés, défaut implicite si absente), écrite par `AppModel.setHistoryMaxDays` (closure `0x10000c210`, `Database.setSetting`, valeur convertie en texte décimal).
- Clé de paramètre d'action `on_conflict` : `ActionParam.onConflict` — `0x100174b34`, chaîne à `0x1001ab120` ; valeurs `replace` / `skip` (libellé UI `"If a file already exists"` à `0x1001e0780`).
- Tags Finder : xattr `com.apple.metadata:_kMDItemUserTags` (chaîne `0x1001e35f0`), opérations `FinderTags.read` (`0x1001533b4`), `write` (`0x100153488`, `0x1001534dc`, `0x100153658`), `setColorLabel`, `ready` (`0x100153010`).

## 12. Purge d'historique, tags couleur et moteur d'actions (ForelCore)

- `Database.purgeHistory(before:)` — `0x10017a438` : SQL exact `"DELETE FROM action_history WHERE datetime(created_at) < datetime(?1)"` (chaîne 0x44 caractères, `SQLiteStatement` + `sqlite3_bind_text` sur l'ISO date) ; formule de la borne : `now − jours × 86400 s` (`addingTimeInterval` sur un `Double` converti — constante flottante `0x3f80000000000000`-famille = 86400.0), fuseau `"UTC"` (bytes 0x54, 0x55).
- `FinderTags.colorIndex` — `0x1001539b8` : correspondance **insensible à la casse** (lowercased puis comparaison) `gray→1, green→2, purple→3, blue→4, yellow→5, red→6, orange→7`, sinon 0 (aucun indice).
- `FinderTags.setColorLabel` — `0x100154104` : lit les tags existants (`ready`/`write`), construit l'entrée `"Nom\n<indice>"` (première lettre en capitale, reste en minuscules, `\n` puis l'indice décimal) et réécrit le xattr `com.apple.metadata:_kMDItemUserTags`.
- `RuleEngine.runActions` — `0x1001582b0` : trie les actions (`sort by` sur `[Action]`), planifie chaque action (`ActionExecutor.plan`), construit l'historique (`HistoryEntry` + `Undo.toJSON`, `batchId` = `UUID.uuidString`), dates en `"UTC"` ; messages de saut exacts : `"A file already exists at the destination."`, `"Already in destination"`, `"Skipped because the action would not change this file."`.
- `PreviewResult.matchLimit` / `reachedMatchLimit` — `0x1001553fc` / `0x100155408` : le plafond d'aperçu est un **paramètre optionnel réel** (`matchLimit: Int?`) ; la valeur 200 utilisée par la reconstruction reste estimée.

## 13. Ce qui reste estimé (pas trouvé dans le binaire)

- Conservation de l'historique 30 jours : la **clé** `history_max_days` est réelle (chaîne `0x1001ded10`, get `0x10000a048`, set `0x10000c210`), mais **aucun littéral de valeur par défaut** n'a été trouvé (la closure d'init renvoie nil quand la clé est absente) ; 30 jours (plage du curseur 1..=365 dans la reconstruction) reste estimé.
- Taille de page de l'historique 50, limite d'aperçu 200, délai de script 60 s : estimés (reconstruits), aucune adresse de fonction trouvée.
- Limites du plan gratuit appliquées **non** dans la reconstruction (demande client : tout débloqué, sans compte) — les adresses ci-dessus documentent l'original.

## 14. Écarts volontaires de la reconstruction (non présents dans l'original)

- `on_conflict = "number"` : au lieu de bloquer, le fichier prend un suffixe « (n) » (`numbered_path`, inspiré de uniqueDest 0x100138560). Les valeurs d'origine `replace` / `skip` sont inchangées ; vide = `skip`.
- Condition Extension : plusieurs extensions séparées par des virgules (`.png, .jpg`), avec ou sans point, casse ignorée. L'original ne documente qu'une seule valeur.
- Écran Dossiers suivis : ajout par sélecteur natif, glisser-déposer, raccourcis ou saisie ; réordonnancement par glisser ; retrait annulable pendant 8 s (le dossier et ses règles sont rétablis). Pure ergonomie : aucune règle d'origine ni valeur décompilée n'est touchée.
