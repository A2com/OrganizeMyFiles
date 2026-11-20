# Recette d'OrganizeMyFiles (macOS)

Checklist à cocher avant chaque version. **Auto** = déjà vérifié par des tests ou par l'agent sur le Mac Studio
(date de la dernière vérification entre parenthèses). **À faire** = à cocher à la main : clics, clavier, boîtes de
dialogue et invites de macOS ne peuvent pas être simulés depuis un terminal.

Version testée : X.Y.Z (celle de `app/Cargo.toml` et du `.dmg`) · Mac : ____________ · macOS : ____________ · Date : ____________ · Testeur : ____________

## 0. Préparation
- [ ] Récupérer `OrganizeMyFiles-X.Y.Z.dmg` (`scripts/emballer-mac.sh` sur le Mac Studio, dossier `dist/`).
- [x] Sauvegarder (ou renommer) `~/Library/Application Support/OrganizeMyFiles/` pour repartir d'un état neuf.
- [x] Créer un dossier de test avec quelques fichiers : une image `.png`, un `.pdf`, un `.txt`, une photo d'un reçu.

## 1. Installation et lancement
| | Contrôle | État |
|---|---|---|
| [x] | Ouvrir le `.dmg`, glisser l'appli dans Applications | Validé par Andy (2026-11-12) |
| [x] | Premier lancement : clic droit › Ouvrir (appli non notarisée : pas de compte Apple Developer) | Validé par Andy (2026-11-12) |
| [x] | Le nom affiché dans le menu « À propos » est **OrganizeMyFiles** (pas « file-organizer ») | Validé par Andy (2026-11-12) |
| [x] | `OrganizeMyFiles.app` se construit, signature ad hoc valide, `omf-extract` présent dans le bundle | Auto (2026-11-12) |
| [x] | L'icône de l'appli (Dock, Finder) est bien celle du dossier bleu/violet | Validé par Andy (2026-11-12) |
| [x] | **Premier lancement** (aucun fichier `~/Library/Application Support/OrganizeMyFiles/state.json`) : écran d'introduction animé : icône (300 px max), « Bienvenue » qui s'écrit à la main sur le fond animé, présentation, bouton « Commencer » ; « Passer » ou un clic saute l'animation | Validé par Andy (2026-11-12) |
| [x] | Introduction : chronologie, tracé progressif du mot, captures à 0,5 / 1,5 / 2,5 / 3,5 / 5,5 s relues ; ancien état = introduction non remontrée | Auto (2026-11-12) |
| [x] | Son d'introduction (4,5 s) pendant l'animation, calé sur l'écriture de « Bienvenue » ; « Passer » ou « Commencer » l'arrête en fondu court | Validé par Andy (2026-11-12) |
| [x] | Son du dashboard (3 s) à l'arrivée sur l'Aperçu après « Commencer », puis à chaque lancement de l'appli ; pas au changement d'onglet, ni à la réouverture depuis la barre des menus, ni au démarrage de session | Validé par Andy (2026-11-12) |
| [x] | Sons : démarrage de l'intro (premier lancement), son du dashboard (lancement normal), rien si « Jouer les sons » est décoché ou en démarrage de session ; lecteur NSSound et fondu (test) | Auto (2026-11-12) |
| [x] | Réglages › À propos › « Revoir l'introduction » rejoue l'animation ; option de lancement `--intro` | Validé par Andy (2026-11-12) |
| [x] | La carte de bienvenue s'affiche une seule fois ; « Compris » la ferme définitivement | Validé par Andy (2026-11-12) |

## 2. Dossiers suivis
| | Contrôle | État |
|---|---|---|
| [x] | « Choisir un dossier… » ouvre le sélecteur natif de macOS | Validé par Andy (2026-11-12) |
| [x] | Glisser-déposer d'un dossier depuis le Finder sur l'écran Dossiers suivis | Validé par Andy (2026-11-12) |
| [x] | Réordonner les dossiers en glissant la poignée ; la priorité (1, 2, 3…) change | Validé par Andy (2026-11-12) |
| [x] | Retirer un dossier puis « Annuler » dans les 8 s : dossier et règles reviennent | Validé par Andy (2026-11-12) |
| [x] | Retrait d'un dossier partagé entre règles, puis annulation (test) | Auto (2026-11-12) |
| [x] | Dossier avec des accents (« Relevés », « Œuvres ») : nom bien affiché | Validé par Andy (2026-11-12) |

## 3. Règles
| | Contrôle | État |
|---|---|---|
| [x] | Créer une règle : nom, dossiers cibles (pastilles + panneau de recherche), conditions, action | Validé par Andy (2026-11-12) |
| [x] | Extensions en pastilles : Entrée, virgule, espace, collage `.png, .jpg`, Retour arrière | Validé par Andy (2026-11-12) |
| [x] | Interrupteur Activée/Désactivée et corbeille de suppression | Validé par Andy (2026-11-12) |
| [x] | Réordonner conditions et actions par glisser-déposer | Validé par Andy (2026-11-12) |
| [x] | Action « Déplacer » : le bouton « Choisir… » remplit le dossier de destination | Validé par Andy (2026-11-12) |
| [x] | Une règle sur plusieurs dossiers s'applique à chacun ; ancienne sauvegarde relue | Auto (2026-11-12) |
| [x] | Parcours complet : dossier → règle → organiser → annuler → relance | Auto (test `e2e`, 2026-11-12) |

## 4. Lecture du contenu (PDF et images)
| | Contrôle | État |
|---|---|---|
| [x] | Texte d'un PDF, OCR d'une photo de reçu, OCR d'un PDF scanné (vrai programme) | Auto (2026-11-12) |
| [x] | Règle « Contenu contient total » : range le reçu et le scan, pas le PDF de facture | Auto (2026-11-12) |
| [x] | Même contrôle avec tes propres PDF et photos de reçus | Validé par Andy (2026-11-12) |
| [x] | Une règle sans condition « Contenu » reste rapide sur un dossier de 500 fichiers | Validé par Andy (2026-11-12) |

## 5. Surveillance automatique
| | Contrôle | État |
|---|---|---|
| [x] | Fichier déposé : rangé tout seul (FSEvents réel) ; téléchargement partiel ignoré ; veille respectée ; anti-boucle | Auto (2026-11-12) |
| [x] | Déposer une image dans Téléchargements, appli ouverte puis fenêtre réduite : elle est rangée | Validé par Andy (2026-11-12) |
| [x] | Mettre l'appli « En veille » (interrupteur de la barre latérale) : plus rien n'est rangé | Validé par Andy (2026-11-12) |
| [x] | Annuler un rangement : le fichier restauré n'est **pas** re-rangé aussitôt | Validé par Andy (2026-11-12) |

## 6. Barre des menus, notifications, démarrage
| | Contrôle | État |
|---|---|---|
| [x] | L'icône « Dossiers suivis » apparaît dans la barre des menus (lisible en mode clair **et** sombre ; atténuée en veille) | Validé par Andy (2026-11-12) |
| [x] | Le menu : titre, état « Surveillance active », « N dossiers suivis · M règles actives », dernier rangement, icônes cohérentes et teintées selon le mode | Validé par Andy (2026-11-12) |
| [x] | Entrées : Ouvrir (⌘O), Organiser maintenant (⌘R), Aperçu (⌘P), Mettre en veille / Reprendre, Dossiers suivis ▸ (clic = afficher dans le Finder, « Gérer les dossiers… »), Règles, Historique, Réglages… (⌘,) | Validé par Andy (2026-11-12) |
| [x] | Ligne « Surveillance » avec un vrai interrupteur : un clic active/désactive l'appli, et le bouton « Actif » de la fenêtre suit (et inversement) | Validé par Andy (2026-11-12) |
| [x] | Interrupteur du menu : `performClick` → l'appli bascule, l'état de l'interrupteur suit, l'état est sauvegardé | Auto (2026-11-12) |
| [x] | Les compteurs se mettent à jour quand on ajoute un dossier ou une règle, ou qu'on en met un en pause | Validé par Andy (2026-11-12) |
| [x] | Bouton rouge : la fenêtre **se ferme** (pas dans le Dock), l'icône du Dock disparaît, l'icône de la barre reste et la surveillance continue | Validé par Andy (2026-11-12) |
| [x] | Fenêtre fermée : processus vivant, fichier déposé rangé, historique enregistré ; « Ouvrir » ramène l'appli au premier plan (politique d'activation vérifiée sur le `.app`) | Auto (2026-11-12) |
| [x] | « Ouvrir » (menu de l'icône) ramène la fenêtre et l'icône du Dock | Validé par Andy (2026-11-12) |
| [x] | « Quitter » du menu de l'icône, et « Quitter OrganizeMyFiles » / ⌘Q du menu de l'appli : le processus se termine | Validé par Andy (2026-11-12) |
| [x] | « Quitter » termine le processus proprement et l'état est sauvegardé | Auto (2026-11-12) |
| [x] | Réglage « Afficher dans la barre des menus » décoché : plus d'icône, et le bouton rouge quitte l'appli | Validé par Andy (2026-11-12) |
| [x] | Une notification apparaît après un rangement (macOS demande l'autorisation la première fois) | Validé par Andy (2026-11-12) |
| [x] | Décocher « Prévenir après chaque organisation » : plus de notification | Validé par Andy (2026-11-12) |
| [x] | « Lancement au démarrage » coché : l'appli démarre à la prochaine ouverture de session, fenêtre fermée (icône de la barre des menus seulement) | Validé par Andy (2026-11-12) |
| [x] | « Lancement au démarrage » décoché : le fichier `~/Library/LaunchAgents/com.a2com.organizemyfiles.plist` disparaît | Validé par Andy (2026-11-12) |

## 7. Permissions macOS
| | Contrôle | État |
|---|---|---|
| [x] | Détection d'un dossier illisible, messages d'erreur d'accès, bandeau et pastille (captures) | Auto (2026-11-12) |
| [x] | Refuser l'accès à Téléchargements/Bureau/Documents : bandeau « macOS empêche… » + pastille « Accès refusé » | Validé par Andy (2026-11-12) |
| [x] | « Ouvrir les réglages » ouvre Réglages Système › Accès complet au disque | Validé par Andy (2026-11-12) |
| [x] | Accorder l'accès puis relancer : bandeau et pastille disparaissent | Validé par Andy (2026-11-12) |

## 8. Historique
| | Contrôle | État |
|---|---|---|
| [x] | Aucun texte ne déborde des cartes (noms de 100 caractères, accents, échecs, plusieurs jours) | Auto (captures, 2026-11-12) |
| [x] | Annulation d'un lot dans le bon ordre (déplacement puis renommage) | Auto (test, 2026-11-12) |
| [x] | Replier/déplier un lot ; « Annuler le lot » ; « Annuler » d'une ligne | Validé par Andy (2026-11-12) |
| [x] | Filtres Tous/Appliqués/Échecs/Annulés ; la recherche du haut filtre par nom de fichier | Validé par Andy (2026-11-12) |
| [x] | « Afficher dans le Finder » ouvre le bon dossier avec le fichier sélectionné | Validé par Andy (2026-11-12) |
| [x] | Les heures affichées correspondent à l'horloge du Mac (heure locale, y compris l'heure d'été) | Validé par Andy (2026-11-12) |
| [x] | Conversion en heure locale (tests : Paris, Auckland, décalage de jour) | Auto (2026-11-12) |

## 9. Réglages, Aperçu et persistance
| | Contrôle | État |
|---|---|---|
| [x] | Réglages : la case « Jouer les sons » coupe tous les sons (intro et dashboard) | Validé par Andy (2026-11-12) |
| [x] | Aperçu : le bouton « Organiser maintenant » a une lueur externe lisse (type Photoshop) qui pulse doucement (pas d'animation si l'appli est en veille ou si « Animer le fond » est décoché) | Validé par Andy (2026-11-12) |
| [x] | Pastille d'état en bas : verte avec point qui pulse après un rangement réussi, rouge après une erreur ou un échec ; s'estompe seule ; un clic la ferme | Validé par Andy (2026-11-12) |
| [x] | Pastille : durée de vie, pulsation, vert/rouge selon le lot (tests) ; captures verte et rouge | Auto (2026-11-12) |
| [x] | Changer la couleur d'accent ; « Animer le fond » coupe l'animation | Validé par Andy (2026-11-12) |
| [x] | Le curseur de rétention purge les entrées plus anciennes ; « Purger l'ancien historique » | Validé par Andy (2026-11-12) |
| [x] | Fermer (Quitter) puis relancer : dossiers, règles, historique et réglages sont retrouvés | Validé par Andy (2026-11-12) |
| [x] | Sauvegarde et restauration d'état (aller-retour, fichier corrompu, version inconnue) | Auto (2026-11-12) |
| [x] | Fluidité du fond animé (pas de saccade, processeur raisonnable dans le Moniteur d'activité) | Validé par Andy (2026-11-12) |

## 10. Contrôles automatiques à relancer avant la recette
```
cd app && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
./scripts/construire-omf-extract.sh debug      # pour les tests PDF/OCR réels (macOS)
./scripts/captures-ecrans.sh                   # 15 captures (5 écrans × vide / démo / erreur) dans captures/
./scripts/emballer-mac.sh                      # .app et .dmg dans dist/
```
La CI (GitHub Actions) exécute les trois premières commandes sur chaque pull request.

## Anomalies constatées
| Date | Écran / action | Ce qui s'est passé | Corrigé ? |
|---|---|---|---|
| | | | |
