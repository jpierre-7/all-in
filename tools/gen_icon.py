#!/usr/bin/env python3
"""Draw the app icon: a pair of casino dice showing a natural seven.

Used for the README, the itch page and any pitch material. It is *not* wired as
the window icon — see the note in assets/README.md.

Run: python3 tools/gen_icon.py
"""

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "assets" / "icon.png"

S = 4
P = 256

RED = (168, 26, 42)
RED_LIT = (214, 48, 62)
BONE = (237, 228, 208)
GOLD = (201, 162, 39)
SHADOW = (0, 0, 0, 90)

# Pip layouts in unit coordinates on the die face.
PIPS = {
    1: [(0.5, 0.5)],
    2: [(0.28, 0.28), (0.72, 0.72)],
    5: [(0.28, 0.28), (0.72, 0.28), (0.5, 0.5), (0.28, 0.72), (0.72, 0.72)],
}


def die(size, value, body, pip):
    """One die face on its own transparent layer, so it can be rotated freely."""
    pad = 26 * S
    layer = Image.new("RGBA", (size + pad * 2, size + pad * 2), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    x0 = y0 = pad
    x1 = y1 = pad + size

    d.rounded_rectangle((x0, y0, x1, y1), radius=size // 5, fill=body + (255,),
                        outline=GOLD + (255,), width=3 * S)
    # A soft highlight down the top-left edge so the face is not flat.
    d.rounded_rectangle((x0 + 5 * S, y0 + 5 * S, x1 - 5 * S, y1 - 5 * S),
                        radius=size // 6, outline=RED_LIT + (110,), width=2 * S)

    r = size * 0.085
    for ux, uy in PIPS[value]:
        cx, cy = x0 + size * ux, y0 + size * uy
        d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=pip + (255,))
    return layer


def main():
    art = Image.new("RGBA", (P * S, P * S), (0, 0, 0, 0))

    # Back die first, so the front one overlaps it.
    back = die(96 * S, 2, RED, BONE).rotate(17, resample=Image.BICUBIC, expand=False)
    art.alpha_composite(back, (100 * S, 30 * S))

    front = die(116 * S, 5, BONE, RED).rotate(-13, resample=Image.BICUBIC, expand=False)
    art.alpha_composite(front, (26 * S, 76 * S))

    out = art.resize((P, P), Image.LANCZOS)
    OUT.parent.mkdir(parents=True, exist_ok=True)
    out.save(OUT)
    print(f"{OUT.relative_to(ROOT)}  {out.size}  {OUT.stat().st_size / 1024:.0f} KB")


if __name__ == "__main__":
    main()
