#!/usr/bin/env python3
"""Generate the clipd app icon at all required macOS sizes.

Design: macOS-style squircle base with a violet→indigo gradient, three offset
cards (front-most has accent lines) and a soft top-light highlight.
"""

from __future__ import annotations
import math
import os
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter


ICONS_DIR = Path(__file__).resolve().parent.parent / "src-tauri" / "icons"
ICONSET_DIR = ICONS_DIR / "icon.iconset"


def squircle_mask(size: int, radius_ratio: float = 0.2237) -> Image.Image:
    """Apple-style superellipse mask (n≈5)."""
    s = size
    img = Image.new("L", (s, s), 0)
    px = img.load()
    n = 5.0
    cx = cy = (s - 1) / 2.0
    r = s / 2.0
    for y in range(s):
        for x in range(s):
            dx = abs(x - cx) / r
            dy = abs(y - cy) / r
            d = (dx ** n + dy ** n) ** (1.0 / n)
            if d <= 1.0:
                # smooth edge
                edge = (1.0 - d) * r
                a = max(0, min(255, int(edge * 255)))
                px[x, y] = 255 if d <= 0.985 else int(min(255, (1.0 - d) / 0.015 * 255))
    # Gentle blur for anti-aliased edge
    img = img.filter(ImageFilter.GaussianBlur(radius=max(0.7, s / 256)))
    # threshold + restore highs
    return img


def lerp(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(len(a)))


def diagonal_gradient(size: int, top: tuple, bottom: tuple) -> Image.Image:
    img = Image.new("RGB", (size, size))
    px = img.load()
    for y in range(size):
        for x in range(size):
            # diagonal mix with slight curve for richness
            t = (y + x * 0.35) / (size * 1.35)
            t = max(0.0, min(1.0, t))
            # ease
            t = t * t * (3 - 2 * t)
            px[x, y] = lerp(top, bottom, t)
    return img


def add_inner_highlight(base: Image.Image, mask: Image.Image) -> Image.Image:
    s = base.size[0]
    hi = Image.new("RGBA", base.size, (255, 255, 255, 0))
    d = ImageDraw.Draw(hi)
    # Top half soft-light gradient
    for y in range(int(s * 0.55)):
        a = int(60 * (1 - y / (s * 0.55)))
        d.line([(0, y), (s, y)], fill=(255, 255, 255, a))
    # Subtle bottom glow
    glow = Image.new("RGBA", base.size, (255, 255, 255, 0))
    gd = ImageDraw.Draw(glow)
    cy = int(s * 1.15)
    for r in range(int(s * 0.65), 0, -1):
        a = int(20 * (1 - r / (s * 0.65)))
        gd.ellipse([s // 2 - r, cy - r, s // 2 + r, cy + r],
                   fill=(180, 200, 255, a))
    out = base.convert("RGBA")
    out.alpha_composite(hi)
    out.alpha_composite(glow)
    # clip to squircle
    out.putalpha(mask)
    return out


def rounded_rect(size, fill, corner):
    img = Image.new("RGBA", size, (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([0, 0, size[0] - 1, size[1] - 1], radius=corner, fill=fill)
    return img


def stacked_cards(size: int) -> Image.Image:
    """Three offset cards centered in the icon, scaled to icon size."""
    s = size
    card = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(card)

    # Card dimensions
    cw = int(s * 0.50)
    ch = int(s * 0.62)
    corner = int(s * 0.085)

    # Three cards offset diagonally
    offsets = [(-int(s * 0.06), int(s * 0.06)),
               (0, 0),
               (int(s * 0.06), -int(s * 0.06))]
    fills = [
        (255, 255, 255, 60),    # back ghost
        (255, 255, 255, 130),   # mid
        (255, 255, 255, 245),   # front (solid)
    ]
    border = (255, 255, 255, 30)

    cx, cy = s // 2, s // 2
    for (ox, oy), fill in zip(offsets, fills):
        x0 = cx - cw // 2 + ox
        y0 = cy - ch // 2 + oy
        # Shadow under the front card
        if fill[3] > 200:
            shadow = Image.new("RGBA", (s, s), (0, 0, 0, 0))
            sd = ImageDraw.Draw(shadow)
            sd.rounded_rectangle(
                [x0 + 2, y0 + int(s * 0.025), x0 + cw + 2, y0 + ch + int(s * 0.025)],
                radius=corner,
                fill=(0, 0, 0, 80),
            )
            shadow = shadow.filter(ImageFilter.GaussianBlur(radius=max(2, s / 80)))
            card.alpha_composite(shadow)
        d.rounded_rectangle(
            [x0, y0, x0 + cw, y0 + ch],
            radius=corner,
            fill=fill,
            outline=border,
            width=max(1, s // 256),
        )

    # Accent lines on the front card to evoke text
    front_off = offsets[2]
    fx0 = cx - cw // 2 + front_off[0]
    fy0 = cy - ch // 2 + front_off[1]
    line_color_dark = (90, 100, 200, 230)
    line_color_light = (150, 160, 220, 200)
    pad = int(cw * 0.16)
    line_h = max(2, int(s * 0.03))
    gap = int(s * 0.07)
    start_y = fy0 + int(ch * 0.22)
    widths = [0.62, 0.5, 0.74, 0.42]
    palette = [line_color_dark, line_color_light, line_color_dark, line_color_light]
    for i, (w_ratio, color) in enumerate(zip(widths, palette)):
        y = start_y + i * gap
        d.rounded_rectangle(
            [fx0 + pad, y, fx0 + pad + int(cw * w_ratio), y + line_h],
            radius=line_h // 2,
            fill=color,
        )
    return card


def make_master(size: int) -> Image.Image:
    # Brand gradient: deep indigo → bright violet
    top = (96, 110, 255)
    bottom = (138, 80, 230)
    grad = diagonal_gradient(size, top, bottom)
    mask = squircle_mask(size)
    base = add_inner_highlight(grad, mask)
    cards = stacked_cards(size)
    base.alpha_composite(cards)
    return base


def main():
    ICONS_DIR.mkdir(parents=True, exist_ok=True)
    ICONSET_DIR.mkdir(parents=True, exist_ok=True)

    # Master 1024×1024
    master = make_master(1024)
    master.save(ICONS_DIR / "icon.png")

    # Sizes Tauri needs
    sizes = {
        "32x32.png": 32,
        "128x128.png": 128,
        "128x128@2x.png": 256,
    }
    for name, size in sizes.items():
        img = master.resize((size, size), Image.LANCZOS)
        img.save(ICONS_DIR / name)

    # iconset sizes for .icns via iconutil
    iconset = {
        "icon_16x16.png": 16,
        "icon_16x16@2x.png": 32,
        "icon_32x32.png": 32,
        "icon_32x32@2x.png": 64,
        "icon_128x128.png": 128,
        "icon_128x128@2x.png": 256,
        "icon_256x256.png": 256,
        "icon_256x256@2x.png": 512,
        "icon_512x512.png": 512,
        "icon_512x512@2x.png": 1024,
    }
    for name, size in iconset.items():
        img = master.resize((size, size), Image.LANCZOS)
        img.save(ICONSET_DIR / name)

    print(f"wrote icons to {ICONS_DIR}")


if __name__ == "__main__":
    main()
