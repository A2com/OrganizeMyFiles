// Affiche la hauteur (en points) de la barre de titre d'une fenêtre macOS standard sur CE Mac.
// Sert à `dmg-finder.sh` : les bornes AppleScript d'une fenêtre Finder incluent la barre de titre (28 pt sur les
// macOS d'avant 26, 32 pt sur le plus récent) ; le contenu doit faire exactement la taille du fond.
import AppKit

let contenu = NSRect(x: 0, y: 0, width: 658, height: 398)
let cadre = NSWindow.frameRect(forContentRect: contenu, styleMask: [.titled, .closable, .miniaturizable, .resizable])
print(Int((cadre.height - contenu.height).rounded()))
