# Recette d'OrganizeMyFiles (macOS)

Checklist à cocher avant chaque version. **Auto** = déjà vérifié par des tests ou par l'agent sur le Mac Studio
(date de la dernière vérification entre parenthèses). **À faire** = à cocher à la main : clics, clavier, boîtes de
dialogue et invites de macOS ne peuvent pas être simulés depuis un terminal.

Version testée : 0.1.0 · Mac : ____________ · macOS : ____________ · Date : ____________ · Testeur : ____________

## 0. Préparation
- [ ] Récupérer `OrganizeMyFiles-0.1.0.dmg` (`scripts/emballer-mac.sh` sur le Mac Studio, dossier `dist/`).
- [ ] Sauvegarder (ou renommer) `~/Library/Application Support/OrganizeMyFiles/` pour repartir d'un état neuf.
- [ ] Créer un dossier de test avec quelques fichiers : une image `.png`, un `.pdf`, un `.txt`, une photo d'un reçu.

## 1. Installation et lancement
| | Contrôle | État |
|---|---|---|
| [ ] | Ouvrir le `.dmg`, glisser l'appli dans Applications | À faire |
| [ ] | Premier lancement : clic droit › Ouvrir (appli non notarisée : pas de compte Apple Developer) | À faire |
| [ ] | Le nom affiché dans le menu « À propos » est **OrganizeMyFiles** (pas « file-organizer ») | À faire |
| [x] | `OrganizeMyFiles.app` se construit, signature ad hoc valide, `omf-extract` présent dans le bundle | Auto (2026-11-12) |
| [ ] | L'icône de l'appli (Dock, Finder) est bien celle du dossier bleu/violet | À faire |
| [ ] | **Premier lancement** (aucun fichier `~/Library/Application Support/OrganizeMyFiles/state.json`) : écran d'introduction animé : icône (300 px max), « Bienvenue » qui s'écrit à la main sur le fond animé, présentation, bouton « Commencer » ; « Passer » ou un clic saute l'animation | À faire |
| [x] | Introduction : chronologie, tracé progressif du mot, captures à 0,5 / 1,5 / 2,5 / 3,5 / 5,5 s relues ; ancien état = introduction non remontrée | Auto (2026-11-12) |
| [ ] | Son d'introduction (4,5 s) pendant l'animation, calé sur l'écriture de « Bienvenue » ; « Passer » ou « Commencer » l'arrête en fondu court | À faire |
| [ ] | Son du dashboard (3 s) à l'arrivée sur l'Aperçu après « Commencer », puis à chaque lancement de l'appli ; pas au changement d'onglet, ni à la réouverture depuis la barre des menus, ni au démarrage de session | À faire |
| [x] | Sons : démarrage de l'intro (premier lancement), son du dashboard (lancement normal), rien si « Jouer les sons » est décoché ou en démarrage de session ; lecteur NSSound et fondu (test) | Auto (2026-11-12) |
| [ ] | Réglages › À propos › « Revoir l'introduction » rejoue l'animation ; option de lancement `--intro` | À faire |
| [ ] | La carte de bienvenue s'affiche une seule fois ; « Compris » la ferme définitivement | À faire |

## 2. Dossiers suivis
| | Contrôle | État |
|---|---|---|
| [ ] | « Choisir un dossier… » ouvre le sélecteur natif de macOS | À faire |
| [ ] | Glisser-déposer d'un dossier depuis le Finder sur l'écran Dossiers suivis | À faire |
| [ ] | Réordonner les dossiers en glissant la poignée ; la priorité (1, 2, 3…) change | À faire |
| [ ] | Retirer un dossier puis « Annuler » dans les 8 s : dossier et règles reviennent | À faire |
| [x] | Retrait d'un dossier partagé entre règles, puis annulation (test) | Auto (2026-11-12) |
| [ ] | Dossier avec des accents (« Relevés », « Œuvres ») : nom bien affiché | À faire |

## 3. Règles
| | Contrôle | État |
|---|---|---|
| [ ] | Créer une règle : nom, dossiers cibles (pastilles + panneau de recherche), conditions, action | À faire |
| [ ] | Extensions en pastilles : Entrée, virgule, espace, collage `.png, .jpg`, Retour arrière | À faire |
| [ ] | Interrupteur Activée/Désactivée et corbeille de suppression | À faire |
| [ ] | Réordonner conditions et actions par glisser-déposer | À faire |
| [ ] | Action « Déplacer » : le bouton « Choisir… » remplit le dossier de destination | À faire |
| [x] | Une règle sur plusieurs dossiers s'applique à chacun ; ancienne sauvegarde relue | Auto (2026-11-12) |
| [x] | Parcours complet : dossier → règle → organiser → annuler → relance | Auto (test `e2e`, 2026-11-12) |

## 4. Lecture du contenu (PDF et images)
| | Contrôle | État |
|---|---|---|
| [x] | Texte d'un PDF, OCR d'une photo de reçu, OCR d'un PDF scanné (vrai programme) | Auto (2026-11-12) |
| [x] | Règle « Contenu contient total » : range le reçu et le scan, pas le PDF de facture | Auto (2026-11-12) |
| [ ] | Même contrôle avec tes propres PDF et photos de reçus | À faire |
| [ ] | Une règle sans condition « Contenu » reste rapide sur un dossier de 500 fichiers | À faire |

## 5. Surveillance automatique
| | Contrôle | État |
|---|---|---|
| [x] | Fichier déposé : rangé tout seul (FSEvents réel) ; téléchargement partiel ignoré ; veille respectée ; anti-boucle | Auto (2026-11-12) |
| [ ] | Déposer une image dans Téléchargements, appli ouverte puis fenêtre réduite : elle est rangée | À faire |
| [ ] | Mettre l'appli « En veille » (interrupteur de la barre latérale) : plus rien n'est rangé | À faire |
| [ ] | Annuler un rangement : le fichier restauré n'est **pas** re-rangé aussitôt | À faire |

## 6. Barre des menus, notifications, démarrage
| | Contrôle | État |
|---|---|---|
| [ ] | L'icône « Dossiers suivis » apparaît dans la barre des menus (lisible en mode clair **et** sombre ; atténuée en veille) | À faire |
| [ ] | Le menu : titre, état « Surveillance active », « N dossiers suivis · M règles actives », dernier rangement, icônes cohérentes et teintées selon le mode | À faire |
| [ ] | Entrées : Ouvrir (⌘O), Organiser maintenant (⌘R), Aperçu (⌘P), Mettre en veille / Reprendre, Dossiers suivis ▸ (clic = afficher dans le Finder, « Gérer les dossiers… »), Règles, Historique, Réglages… (⌘,) | À faire |
| [ ] | Ligne « Surveillance » avec un vrai interrupteur : un clic active/désactive l'appli, et le bouton « Actif » de la fenêtre suit (et inversement) | À faire |
| [x] | Interrupteur du menu : `performClick` → l'appli bascule, l'état de l'interrupteur suit, l'état est sauvegardé | Auto (2026-11-12) |
| [ ] | Les compteurs se mettent à jour quand on ajoute un dossier ou une règle, ou qu'on en met un en pause | À faire |
| [ ] | Bouton rouge : la fenêtre **se ferme** (pas dans le Dock), l'icône du Dock disparaît, l'icône de la barre reste et la surveillance continue | À faire |
| [x] | Fenêtre fermée : processus vivant, fichier déposé rangé, historique enregistré ; « Ouvrir » ramène l'appli au premier plan (politique d'activation vérifiée sur le `.app`) | Auto (2026-11-12) |
| [ ] | « Ouvrir » (menu de l'icône) ramène la fenêtre et l'icône du Dock | À faire |
| [ ] | « Quitter » du menu de l'icône, et « Quitter OrganizeMyFiles » / ⌘Q du menu de l'appli : le processus se termine | À faire |
| [x] | « Quitter » termine le processus proprement et l'état est sauvegardé | Auto (2026-11-12) |
| [ ] | Réglage « Afficher dans la barre des menus » décoché : plus d'icône, et le bouton rouge quitte l'appli | À faire |
| [ ] | Une notification apparaît après un rangement (macOS demande l'autorisation la première fois) | À faire |
| [ ] | Décocher « Prévenir après chaque organisation » : plus de notification | À faire |
| [ ] | « Lancement au démarrage » coché : l'appli démarre à la prochaine ouverture de session, fenêtre fermée (icône de la barre des menus seulement) | À faire |
| [ ] | « Lancement au démarrage » décoché : le fichier `~/Library/LaunchAgents/com.a2com.organizemyfiles.plist` disparaît | À faire |

## 7. Permissions macOS
| | Contrôle | État |
|---|---|---|
| [x] | Détection d'un dossier illisible, messages d'erreur d'accès, bandeau et pastille (captures) | Auto (2026-11-12) |
| [ ] | Refuser l'accès à Téléchargements/Bureau/Documents : bandeau « macOS empêche… » + pastille « Accès refusé » | À faire |
| [ ] | « Ouvrir les réglages » ouvre Réglages Système › Accès complet au disque | À faire |
| [ ] | Accorder l'accès puis relancer : bandeau et pastille disparaissent | À faire |

## 8. Historique
| | Contrôle | État |
|---|---|---|
| [x] | Aucun texte ne déborde des cartes (noms de 100 caractères, accents, échecs, plusieurs jours) | Auto (captures, 2026-11-12) |
| [x] | Annulation d'un lot dans le bon ordre (déplacement puis renommage) | Auto (test, 2026-11-12) |
| [ ] | Replier/déplier un lot ; « Annuler le lot » ; « Annuler » d'une ligne | À faire |
| [ ] | Filtres Tous/Appliqués/Échecs/Annulés ; la recherche du haut filtre par nom de fichier | À faire |
| [ ] | « Afficher dans le Finder » ouvre le bon dossier avec le fichier sélectionné | À faire |
| [ ] | Les heures affichées correspondent à l'horloge du Mac (heure locale, y compris l'heure d'été) | À faire |
| [x] | Conversion en heure locale (tests : Paris, Auckland, décalage de jour) | Auto (2026-11-12) |

## 9. Réglages, Aperçu et persistance
| | Contrôle | État |
|---|---|---|
| [ ] | Réglages : la case « Jouer les sons » coupe tous les sons (intro et dashboard) | À faire |
| [ ] | Aperçu : le bouton « Organiser maintenant » a un halo qui pulse (pas d'animation si l'appli est en veille ou si « Animer le fond » est décoché) | À faire |
| [ ] | Pastille d'état en bas : verte avec point qui pulse après un rangement réussi, rouge après une erreur ou un échec ; s'estompe seule ; un clic la ferme | À faire |
| [x] | Pastille : durée de vie, pulsation, vert/rouge selon le lot (tests) ; captures verte et rouge | Auto (2026-11-12) |
| [ ] | Changer la couleur d'accent ; « Animer le fond » coupe l'animation | À faire |
| [ ] | Le curseur de rétention purge les entrées plus anciennes ; « Purger l'ancien historique » | À faire |
| [ ] | Fermer (Quitter) puis relancer : dossiers, règles, historique et réglages sont retrouvés | À faire |
| [x] | Sauvegarde et restauration d'état (aller-retour, fichier corrompu, version inconnue) | Auto (2026-11-12) |
| [ ] | Fluidité du fond animé (pas de saccade, processeur raisonnable dans le Moniteur d'activité) | À faire |

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
