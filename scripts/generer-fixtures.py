#!/usr/bin/env python3
"""Génère les fichiers d'essai de la lecture de contenu dans app/tests/fixtures/ (à lancer sur le Mac : Pillow + police système).

  facture-texte.pdf : PDF avec du vrai texte  « Facture mars 2026 »
  recu.png          : image avec le texte      « TOTAL 42,50 EUR - Reçu boulangerie »
  scan.pdf          : PDF qui ne contient qu'une image de ce texte (PDF scanné)
"""
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

OUT = Path(__file__).resolve().parent.parent / "app" / "tests" / "fixtures"
OUT.mkdir(parents=True, exist_ok=True)

FONT = "/System/Library/Fonts/Helvetica.ttc"


def receipt_image() -> Image.Image:
    img = Image.new("RGB", (1000, 360), "white")
    font = ImageFont.truetype(FONT, 64)
    draw = ImageDraw.Draw(img)
    draw.text((40, 60), "TOTAL 42,50 EUR", font=font, fill="black")
    draw.text((40, 180), "Reçu boulangerie", font=font, fill="black")
    return img


def text_pdf(text: str) -> bytes:
    """PDF minimal d'une page (police Helvetica standard) avec le texte donné."""
    stream = f"BT /F1 28 Tf 72 700 Td ({text}) Tj ET".encode("latin-1")
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
        b"<< /Length " + str(len(stream)).encode() + b" >>\nstream\n" + stream + b"\nendstream",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    ]
    out = bytearray(b"%PDF-1.4\n")
    offsets = []
    for i, body in enumerate(objects, start=1):
        offsets.append(len(out))
        out += f"{i} 0 obj\n".encode() + body + b"\nendobj\n"
    xref = len(out)
    out += f"xref\n0 {len(objects) + 1}\n0000000000 65535 f \n".encode()
    for off in offsets:
        out += f"{off:010d} 00000 n \n".encode()
    out += f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
    return bytes(out)


receipt_image().save(OUT / "recu.png")
receipt_image().save(OUT / "scan.pdf", "PDF", resolution=150)
(OUT / "facture-texte.pdf").write_bytes(text_pdf("Facture mars 2026"))
print("OK :", OUT)
