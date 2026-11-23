# Journal des changements

Toutes les modifications notables de ce projet sont consignées ici.
Le format s'inspire de [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/) et le versionnement suit
[SemVer](https://semver.org/lang/fr/). La source unique de version est `app/Cargo.toml` ; la procédure est
décrite dans `docs/VERSIONNEMENT.md`.

## [Non publié]

### Modifié

- Images et formes plus lisses : textures avec versions réduites précalculées (mipmaps) et lissage matériel des bords (MSAA ×4).
- Règles (mode simple) : les pictogrammes plats sont remplacés par les icônes illustrées fournies par Andy (`assets/ui/rules/`, préparées par `scripts/generer-icones-regles.sh`) : cartes agrandies, icône sur un halo de sa teinte qui s'intensifie au choix et au survol, dossier illustré pour l'étape 1 et la liste des dossiers.
- Halos lumineux adoucis (réglage unique `GLOW_SOFTNESS`), pastilles d'icônes plates et plus lisibles, mots clés du résumé en gras seulement.
- Règles (mode simple) : « Suivant » / « Terminer » à droite ; sélecteur Simple / Avancé avec des espaces réguliers entre les boutons.
- Logotype remplacé par la nouvelle version fournie par Andy ; « Bienvenue » réduit de moitié sur l'écran d'introduction.
- Installateur `.dmg` illustré : fond, icône de l'appli à gauche, raccourci « Applications » avec son icône à droite (glisser-déposer).
- Règles (mode simple) plus « friendly » : grande illustration du dossier qui déroule, avec animation, la liste de tous les dossiers suivis ; modèles de conditions et actions en cartes illustrées (pictogrammes colorés, état choisi, coche) ; frise d'étapes à pastilles ; carte « En résumé » mise en évidence (mots clés surlignés) ; animations coupées en mouvement réduit.

### Ajouté

- Logotype « OrganizeMyFiles » (SVG fourni par Andy) intégré avec discrétion : barre latérale, pied de l'écran de bienvenue, Réglages › À propos.

### Corrigé

- Écran de bienvenue : « Bienvenue » s'écrit en un geste continu (lettres liées, stylo à vitesse régulière, point du « i » en dernier) au lieu de lettres posées les unes après les autres.

## [0.2.1] - 2026-11-22

### Corrigé

- Dock : un clic sur l'icône de l'appli rouvre la fenêtre quand elle a été fermée avec le bouton rouge.
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
