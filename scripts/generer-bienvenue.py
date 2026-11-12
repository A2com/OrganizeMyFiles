#!/usr/bin/env python3
"""Génère assets/intro/bienvenue.json : le mot « Bienvenue » en tracés de stylo (ligne unique), dans l'ordre d'écriture.

Source : polices-tracés de Vara (https://github.com/akzhy/Vara, licence MIT), dossier fonts/ :
  Parisienne/Parisienne.json (OFL) ou Satisfy/SatisfySL.json (Apache 2.0).
Chaque lettre y est une liste de courbes de stylo (chemins SVG relatifs m / c / q / z) avec un point de départ (mx, my).
La mise en page reprend celle de Vara : chaque lettre est décalée de la largeur du dessin de la précédente.

Usage : generer-bienvenue.py <police.json> [--texte Bienvenue] [--sortie assets/intro/bienvenue.json] [--apercu apercu.png]
"""
import argparse
import json
import math
import re
from pathlib import Path

CURVE_STEPS = 16


def numbers(text):
    return [float(x) for x in re.findall(r"-?\d*\.?\d+(?:e-?\d+)?", text)]


def flatten_path(d, origin):
    """Suite de points absolus d'un chemin SVG relatif (m, c, q, z) dont le départ est `origin`."""
    points = []
    cur = start = (origin[0], origin[1])
    for cmd, args in re.findall(r"([mcqzMCQZ])([^mcqzMCQZ]*)", d):
        nums = numbers(args)
        if cmd == "m":
            pairs = [nums[i : i + 2] for i in range(0, len(nums) - 1, 2)]
            cur = (cur[0] + pairs[0][0], cur[1] + pairs[0][1])
            start = cur
            points.append(cur)
            for dx, dy in pairs[1:]:  # paires suivantes : segments droits
                cur = (cur[0] + dx, cur[1] + dy)
                points.append(cur)
        elif cmd == "c":
            for i in range(0, len(nums) - 5, 6):
                x1, y1, x2, y2, x3, y3 = nums[i : i + 6]
                p0 = cur
                p1, p2, p3 = (p0[0] + x1, p0[1] + y1), (p0[0] + x2, p0[1] + y2), (p0[0] + x3, p0[1] + y3)
                for s in range(1, CURVE_STEPS + 1):
                    t = s / CURVE_STEPS
                    u = 1 - t
                    points.append(
                        (
                            u**3 * p0[0] + 3 * u * u * t * p1[0] + 3 * u * t * t * p2[0] + t**3 * p3[0],
                            u**3 * p0[1] + 3 * u * u * t * p1[1] + 3 * u * t * t * p2[1] + t**3 * p3[1],
                        )
                    )
                cur = p3
        elif cmd == "q":
            for i in range(0, len(nums) - 3, 4):
                x1, y1, x2, y2 = nums[i : i + 4]
                p0 = cur
                p1, p2 = (p0[0] + x1, p0[1] + y1), (p0[0] + x2, p0[1] + y2)
                for s in range(1, CURVE_STEPS + 1):
                    t = s / CURVE_STEPS
                    u = 1 - t
                    points.append((u * u * p0[0] + 2 * u * t * p1[0] + t * t * p2[0], u * u * p0[1] + 2 * u * t * p1[1] + t * t * p2[1]))
                cur = p2
        elif cmd in ("z", "Z"):
            points.append(start)
            cur = start
    return points


def build(font, text):
    glyphs = font["c"]
    advance = 0.0
    strokes = []
    for ch in text:
        glyph = glyphs[str(ord(ch))]
        paths = [flatten_path(p["d"], (p["mx"], -p["my"])) for p in glyph["paths"]]
        xs = [x for path in paths for x, _ in path]
        left, width = min(xs), max(xs) - min(xs)
        for path in paths:
            strokes.append([(x - left + advance, y) for x, y in path])
        advance += width
    xs = [x for s in strokes for x, _ in s]
    ys = [y for s in strokes for _, y in s]
    min_x, min_y = min(xs), min(ys)
    strokes = [[(round(x - min_x, 2), round(y - min_y, 2)) for x, y in s] for s in strokes]
    return strokes, round(max(xs) - min_x, 2), round(max(ys) - min_y, 2)


def preview(strokes, width, height, pen, out):
    from PIL import Image, ImageDraw

    scale = 1100 / width
    img = Image.new("RGB", (int(width * scale) + 60, int(height * scale) + 60), (20, 24, 40))
    draw = ImageDraw.Draw(img)
    for s in strokes:
        pts = [(30 + x * scale, 30 + y * scale) for x, y in s]
        draw.line(pts, fill=(255, 255, 255), width=max(2, round(pen * scale * 5)), joint="curve")
    img.save(out)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("police")
    ap.add_argument("--texte", default="Bienvenue")
    ap.add_argument("--sortie", default=str(Path(__file__).resolve().parent.parent / "assets" / "intro" / "bienvenue.json"))
    ap.add_argument("--apercu")
    args = ap.parse_args()
    font = json.load(open(args.police))
    strokes, width, height = build(font, args.texte)
    pen = font["p"]["bsw"]
    out = {"texte": args.texte, "largeur": width, "hauteur": height, "trait": pen, "traits": strokes}
    Path(args.sortie).write_text(json.dumps(out, separators=(",", ":")))
    total = sum(math.dist(a, b) for s in strokes for a, b in zip(s, s[1:]))
    print(f"OK : {len(strokes)} traits, {width} × {height}, longueur {total:.0f} -> {args.sortie}")
    if args.apercu:
        preview(strokes, width, height, pen, args.apercu)


if __name__ == "__main__":
    main()
