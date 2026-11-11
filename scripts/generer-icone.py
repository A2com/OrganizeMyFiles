#!/usr/bin/env python3
"""Génère assets/AppIcon.png (1024 px) : un dossier orange sur fond nuit, sans dépendance externe."""
import struct
import sys
import zlib

N = 1024


def rrect(px, py, x0, y0, x1, y1, r):
    """Distance signée à un rectangle arrondi (négatif = dedans)."""
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    hx, hy = (x1 - x0) / 2 - r, (y1 - y0) / 2 - r
    dx, dy = abs(px - cx) - hx, abs(py - cy) - hy
    ox, oy = max(dx, 0), max(dy, 0)
    return (ox * ox + oy * oy) ** 0.5 + min(max(dx, dy), 0) - r


def over(dst, src, a):
    return tuple(d * (1 - a) + s * a for d, s in zip(dst, src))


def lerp(c0, c1, t):
    return tuple(a + (b - a) * t for a, b in zip(c0, c1))


def pixel(x, y):
    px, py = x + 0.5, y + 0.5
    d = rrect(px, py, 0, 0, N, N, 228)
    bg_alpha = min(max(0.5 - d, 0), 1)
    if bg_alpha == 0:
        return (0, 0, 0, 0)
    col = lerp((26, 32, 52), (12, 14, 21), py / N)
    # onglet puis corps du dossier (orange)
    for box, c0, c1 in (
        ((236, 318, 500, 460, 44), (255, 150, 80), (255, 120, 40)),
        ((236, 392, 788, 748, 56), (255, 150, 80), (255, 105, 30)),
    ):
        a = min(max(0.5 - rrect(px, py, *box), 0), 1)
        if a:
            col = over(col, lerp(c0, c1, (py - box[1]) / (box[3] - box[1])), a)
    # face avant du dossier (plus claire) et petit trait bleu
    a = min(max(0.5 - rrect(px, py, 236, 480, 788, 748, 56), 0), 1)
    if a:
        col = over(col, lerp((255, 190, 130), (255, 130, 50), (py - 480) / 268), a)
    a = min(max(0.5 - rrect(px, py, 330, 590, 694, 626, 18), 0), 1)
    if a:
        col = over(col, (2, 132, 199), a * 0.9)
    return (*[int(round(c)) for c in col], int(round(bg_alpha * 255)))


def main(out):
    rows = []
    for y in range(N):
        row = bytearray([0])
        for x in range(N):
            row.extend(pixel(x, y))
        rows.append(bytes(row))
    raw = zlib.compress(b"".join(rows), 9)

    def chunk(tag, data):
        body = tag + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", N, N, 8, 6, 0, 0, 0)) + chunk(b"IDAT", raw) + chunk(b"IEND", b"")
    with open(out, "wb") as f:
        f.write(png)


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "assets/AppIcon.png")
