#!/usr/bin/env python3
"""Génère le logotype PNG embarqué dans l'appli depuis assets/logo/wordmark-white.png (fourni par Andy, 2216 × 322).

Le PNG source est trop grand pour l'afficher directement (egui sans mipmaps crénelle quand on réduit beaucoup) :
on en tire une version de 640 px de large, nette en Retina pour les 128 à 170 pt d'affichage.
Les SVG (wordmark.svg, wordmark-white.svg) restent les sources vectorielles d'origine.
Usage : uv run --with pillow scripts/generer-logo.py
"""
from pathlib import Path

from PIL import Image

RACINE = Path(__file__).resolve().parent.parent
SOURCE = RACINE / "assets" / "logo" / "wordmark-white.png"
LARGEUR = 640


def main() -> None:
    image = Image.open(SOURCE).convert("RGBA")
    hauteur = round(image.height * LARGEUR / image.width)
    image.resize((LARGEUR, hauteur), Image.LANCZOS).save(RACINE / "assets" / "logo" / f"wordmark-white-{LARGEUR}.png", optimize=True)
    print(f"OK : {LARGEUR} × {hauteur}")


if __name__ == "__main__":
    main()
