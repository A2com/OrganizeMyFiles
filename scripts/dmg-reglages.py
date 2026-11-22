# Réglages de dmgbuild pour l'installateur : fond illustré, icône de l'appli à gauche, « Applications » à droite.
# Appelé par scripts/emballer-mac.sh : dmgbuild -s scripts/dmg-reglages.py -D racine=… -D app=… -D alias=… …
# La fenêtre fait la taille du fond (658 × 398) ; les positions visent le chevron au centre du fond.
import os

racine = defines["racine"]  # noqa: F821  (fourni par dmgbuild)
app = defines["app"]  # noqa: F821
alias = defines["alias"]  # noqa: F821

format = "UDZO"
filesystem = "HFS+"
files = [app, alias]
symlinks = {}
icon = os.path.join(racine, "assets", "icons", "macos", "AppIcon.icns")
background = os.path.join(racine, "assets", "dmg", "background.jpg")

window_rect = ((200, 120), (658, 398))
default_view = "icon-view"
show_icon_preview = False
show_status_bar = False
show_tab_view = False
show_toolbar = False
show_pathbar = False
show_sidebar = False
icon_size = 125
text_size = 13
arrange_by = None
icon_locations = {
    "OrganizeMyFiles.app": (165, 221),
    "Applications": (509, 221),
}
