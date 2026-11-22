# Journal des changements

Toutes les modifications notables de ce projet sont consignées ici.
Le format s'inspire de [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/) et le versionnement suit
[SemVer](https://semver.org/lang/fr/). La source unique de version est `app/Cargo.toml` ; la procédure est
décrite dans `docs/VERSIONNEMENT.md`.

## [Non publié]

### Corrigé

- Barre du haut : le raccourci « ⌘ K » de la recherche ne chevauche plus la pastille « Actif » (largeur du groupe de droite mesurée au lieu d'une constante).

## [0.2.0] - 2026-11-20

### Ajouté

- Réglages : raccourci ⌘, pour ouvrir les réglages (#38).
- Règles : mode simple en trois étapes (dossiers, conditions, action) avec modèles de conditions, phrase récapitulative et
  essai sans rien toucher ; le mode avancé reste l'écran habituel. Bascule « Simple / Avancé » mémorisée (nouveaux
  utilisateurs : simple ; sauvegardes existantes : avancé).

- Mises à jour : l'appli cherche les nouvelles versions sur GitHub (au lancement puis une fois par jour, désactivable) ;
  bandeau « Version X disponible » et installation en un clic (téléchargement, vérification SHA-256, remplacement,
  redémarrage). Bouton « Rechercher des mises à jour » dans Réglages › À propos.

### Corrigé

- Icône : sous macOS, le Dock affiche l'icône du `.app` au gabarit Apple (elle paraissait trop grosse) (#45).
- Interface : champs de saisie toujours sombres même en mode clair macOS (#37).
- Aperçu : labels « N dossiers » et « N règles actives » distincts, anneau de progression aligné (#39).
- Aperçu : fond coloré discret derrière les labels « N dossiers » et « N règles actives » (#43).
- Aperçu : tuiles KPI sur deux lignes sous 1200 px, plus de texte tronqué (#43).
- Règles : aide « Récursion » compréhensible par un novice (#40).
- Interface : les textes ne sortent plus des cartes en fenêtre étroite (#41).

## [0.1.0] - 2026-11-20

### Ajouté

- Thème Nocturne, écrans Aperçu et Dossiers refondus, fond animé, polices Outfit (#3).
- Sauvegarde et restauration des données (#4).
- Carte « À propos » dans les Réglages (#6).
- Actions « ouvrir une application » et « lancer un raccourci » (#7).
- Surveillance automatique des dossiers suivis (#10).
- Emballage macOS : `OrganizeMyFiles.app`, `.dmg` et icônes (#12).
- Une règle peut cibler plusieurs dossiers suivis (#14).
- Barre des menus et notifications (#15), menu premium avec fermeture vers la barre des menus (#27).
- Ergonomie de l'écran Règles (#18).
- Historique par lots repliables, filtres et annulation (#20).
- Autorisations macOS : bandeau d'aide et pastille « Accès refusé » (#23).
- Lecture du texte des PDF et OCR des images (#24).
- Pastille d'état verte/rouge et interrupteur dans le menu (#28).
- Écran d'introduction animé (#29).
- Sons d'introduction et du tableau de bord (#30).
- Liste déroulante pour la condition « Type » (#34).
- Organisation automatique des dossiers suivis (#36).

### Corrigé

- Chemins absolus hors du dossier personnel (#5).
- Nom « OrganizeMyFiles » partout (#8, #17).
- Textes de l'historique en français (#9).
- Carte Application allégée et réglage « Lancement au démarrage » (#13).
- Affichage et recherche des noms accentués (#16).
- Icône « Dossiers suivis » dans la barre des menus (#19).
- Heure locale du système dans l'historique et les variables (#22).
- Lueur externe du bouton « Organiser maintenant » lissée (#31).
- Condition « Type est folder » (#33).
- Déplacement d'un dossier vers un autre disque (#35).

### Interne

- Fichiers Craft Anything et source Photoshop ignorés (#1, #21).
- Plan de finalisation (#2).
- Tests d'enchaînement et intégration continue (#25).
- Recette macOS et documentation (#26, #32).
