#!/usr/bin/env python3
"""Génère le logotype PNG embarqué dans l'appli depuis assets/logo/wordmark-white.svg.

egui ne lit pas le SVG et on évite une dépendance de plus : le PNG est versionné, comme assets/icons/.
Le thème de l'appli est sombre : on utilise la version à texte blanc (dégradé conservé sur « O » et « My »).
Usage : uv run --with cairosvg --with pillow scripts/generer-logo.py
"""
import io
from pathlib import Path

import cairosvg
from PIL import Image

RACINE = Path(__file__).resolve().parent.parent
SOURCE = RACINE / "assets" / "logo" / "wordmark-white.svg"
LARGEUR = 480  # points : assez pour la barre latérale, l'écran de bienvenue et « À propos » en Retina


def main() -> None:
    # Rendu à 2× puis réduction : bords plus nets que le rendu direct.
    png = cairosvg.svg2png(url=str(SOURCE), output_width=LARGEUR * 2)
    image = Image.open(io.BytesIO(png)).convert("RGBA")
    hauteur = round(image.height * LARGEUR / image.width)
    image.resize((LARGEUR, hauteur), Image.LANCZOS).save(RACINE / "assets" / "logo" / f"wordmark-white-{LARGEUR}.png", optimize=True)
    print(f"OK : {LARGEUR} × {hauteur}")


if __name__ == "__main__":
    main()
