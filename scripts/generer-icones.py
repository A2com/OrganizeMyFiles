#!/usr/bin/env python3
"""Génère toutes les icônes de l'appli depuis icon/icon.png (à lancer sur le Mac : Pillow + iconutil).

Sorties dans assets/icons/ (versionnées, pour que le build n'ait pas besoin de Pillow) :
  macos/AppIcon.icns, windows/OrganizeMyFiles.ico, linux/hicolor/<n>x<n>/apps/organizemyfiles.png, app-256.png
"""
import shutil
import subprocess
import tempfile
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "icon" / "icon.png"
OUT = ROOT / "assets" / "icons"
MAC_BODY = 824 / 1024  # gabarit Apple : le corps de l'icône occupe 824 px sur 1024


def square_source() -> Image.Image:
    """Source rendue carrée (bord transparent ajouté si besoin)."""
    img = Image.open(SOURCE).convert("RGBA")
    side = max(img.size)
    canvas = Image.new("RGBA", (side, side), (0, 0, 0, 0))
    canvas.paste(img, ((side - img.width) // 2, (side - img.height) // 2))
    return canvas


def render(src: Image.Image, size: int, body: float = 1.0) -> Image.Image:
    """Icône `size` × `size` ; `body` < 1 laisse une marge transparente autour."""
    inner = round(size * body)
    out = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    out.paste(src.resize((inner, inner), Image.LANCZOS), ((size - inner) // 2, (size - inner) // 2))
    return out


def main() -> None:
    src = square_source()
    if OUT.exists():
        shutil.rmtree(OUT)

    # macOS : iconset (16 à 512, @1x et @2x) puis iconutil
    with tempfile.TemporaryDirectory() as tmp:
        iconset = Path(tmp) / "AppIcon.iconset"
        iconset.mkdir()
        for s in (16, 32, 128, 256, 512):
            render(src, s, MAC_BODY).save(iconset / f"icon_{s}x{s}.png")
            render(src, s * 2, MAC_BODY).save(iconset / f"icon_{s}x{s}@2x.png")
        (OUT / "macos").mkdir(parents=True)
        subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(OUT / "macos" / "AppIcon.icns")], check=True)

    # Windows : un .ico à plusieurs tailles
    (OUT / "windows").mkdir()
    sizes = [16, 24, 32, 48, 64, 128, 256]
    render(src, 256).save(OUT / "windows" / "OrganizeMyFiles.ico", sizes=[(s, s) for s in sizes])

    # Linux : thème hicolor
    for s in (16, 32, 48, 64, 128, 256, 512):
        d = OUT / "linux" / "hicolor" / f"{s}x{s}" / "apps"
        d.mkdir(parents=True)
        render(src, s).save(d / "organizemyfiles.png")

    # Icône embarquée dans le programme (fenêtre, barre latérale)
    render(src, 256).save(OUT / "app-256.png")
    print("OK :", OUT)


if __name__ == "__main__":
    main()
