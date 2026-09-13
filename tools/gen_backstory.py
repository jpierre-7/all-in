#!/usr/bin/env python3
"""Generate the five Opening frames and the title backdrop.

One image per paragraph of OPENING in `docs/narrative.md`, in frame order.

These are **symbolic** rather than illustrative — an object under a lamp, not a
drawn figure. That is an honest limit of geometry: procedural code can place a
photograph on felt convincingly and cannot draw a man's face. It also suits the
prose, which is about what Lucky Jack lost rather than what he looks like.

Shares its palette and light with `gen_backdrops.py` so the Opening and the
rest of the game read as one place.

Run: python3 tools/gen_backstory.py
"""

import math
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

sys.path.insert(0, str(Path(__file__).resolve().parent))

from gen_backdrops import (  # noqa: E402
    CYAN, FELT, GOLD, H, MAGENTA, NOIR, TEAL, W, add_glow, blurred_layer,
    canvas, coords, grain, radial, screen, to_image, vignette,
)

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "assets" / "backstory"
FONT = ROOT / "assets" / "fonts" / "Limelight-Regular.ttf"
BODY_FONT = ROOT / "assets" / "fonts" / "BarlowCondensed-Regular.ttf"

BONE = (237, 228, 208)
BLOOD = (142, 27, 35)
INK = (10, 10, 12)


def chip(d, cx, cy, r, face, rim=BONE, alpha=255):
    """A chip seen from above: disc, rim, and six edge ticks."""
    d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=face + (alpha,))
    d.ellipse((cx - r, cy - r, cx + r, cy + r), outline=rim + (alpha,), width=max(r // 7, 2))
    for k in range(6):
        a = math.radians(k * 60)
        x0, y0 = cx + math.cos(a) * r * 0.80, cy + math.sin(a) * r * 0.80
        x1, y1 = cx + math.cos(a) * r * 1.0, cy + math.sin(a) * r * 1.0
        d.line((x0, y0, x1, y1), fill=rim + (alpha,), width=max(r // 8, 2))
    d.ellipse((cx - r * 0.45, cy - r * 0.45, cx + r * 0.45, cy + r * 0.45),
              outline=rim + (alpha // 2,), width=max(r // 12, 1))


def suit(d, cx, cy, r, kind, colour):
    """One suit mark. Small vocabulary, but enough to read as a card."""
    if kind == "diamond":
        d.polygon([(cx, cy - r), (cx + r * 0.7, cy), (cx, cy + r), (cx - r * 0.7, cy)], fill=colour)
    elif kind == "heart":
        d.ellipse((cx - r, cy - r * 0.9, cx - r * 0.05, cy + r * 0.3), fill=colour)
        d.ellipse((cx + r * 0.05, cy - r * 0.9, cx + r, cy + r * 0.3), fill=colour)
        d.polygon([(cx - r * 0.96, cy + r * 0.05), (cx + r * 0.96, cy + r * 0.05), (cx, cy + r)], fill=colour)
    elif kind == "spade":
        d.polygon([(cx, cy - r), (cx + r * 0.95, cy + r * 0.3), (cx - r * 0.95, cy + r * 0.3)], fill=colour)
        d.ellipse((cx - r, cy - r * 0.1, cx - r * 0.05, cy + r * 0.85), fill=colour)
        d.ellipse((cx + r * 0.05, cy - r * 0.1, cx + r, cy + r * 0.85), fill=colour)
        d.polygon([(cx - r * 0.3, cy + r), (cx + r * 0.3, cy + r), (cx, cy + r * 0.3)], fill=colour)
    else:  # club
        q = r * 0.52
        for ox, oy in ((0, -q * 1.15), (-q, q * 0.4), (q, q * 0.4)):
            d.ellipse((cx + ox - q, cy + oy - q, cx + ox + q, cy + oy + q), fill=colour)
        d.polygon([(cx - r * 0.3, cy + r), (cx + r * 0.3, cy + r), (cx, cy + r * 0.3)], fill=colour)


def playing_card(w, h, lean, rank, kind, red=False):
    """A face-up card: corner index, matching suit, one big pip."""
    card = Image.new("RGBA", (w + 60, h + 60), (0, 0, 0, 0))
    cd = ImageDraw.Draw(card)
    cd.rounded_rectangle((30, 30, 30 + w, 30 + h), radius=w // 9,
                         fill=BONE + (255,), outline=(150, 142, 124, 255), width=2)
    colour = (BLOOD if red else INK) + (255,)
    index = ImageFont.truetype(str(BODY_FONT), int(w * 0.30))
    cd.text((30 + w * 0.10, 30 + h * 0.05), rank, font=index, fill=colour)
    suit(cd, 30 + w * 0.17, 30 + h * 0.29, w * 0.07, kind, colour)
    suit(cd, 30 + w * 0.52, 30 + h * 0.60, w * 0.22, kind, colour)
    return card.rotate(lean, resample=Image.BICUBIC, expand=False)


def felt_table(lamp_x=0.5, lamp_y=0.45, warmth=1.0):
    """The common ground: felt under a low lamp, everything else falling off."""
    base = canvas(NOIR)
    base = add_glow(base, radial(W * lamp_x, H * lamp_y, W * 0.78, 2.1), FELT, 1.25 * warmth)
    base = add_glow(base, radial(W * lamp_x, H * lamp_y, W * 0.32, 2.6), TEAL, 0.80 * warmth)
    return base


def finish(base, art_layer, vig=0.88, scrim=0.66):
    """Ground, then subject, then a scrim across the band the prose occupies.

    The scrim has to come *after* the subject: the chips, the photograph and
    the cards are the bright things in these frames, and they land exactly
    where the text does. Dimming only the background would not have helped.
    """
    img = to_image(vignette(base, strength=vig) * 0.95).convert("RGBA")
    img.alpha_composite(art_layer)

    arr = np.asarray(img.convert("RGB"), dtype=np.float32) / 255
    y = coords()[1]
    arr = arr * (1.0 - scrim * np.exp(-(((y - H * 0.82) / (H * 0.24)) ** 2)))[..., None]
    return to_image(grain(arr, amount=0.008)).convert("RGBA")


def frame_1():
    """Everything he owned, going the other way across the felt."""
    base = felt_table(0.38, 0.50)
    layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)

    # An empty ring where his stake used to sit.
    d.ellipse((300, 700, 620, 860), outline=(90, 110, 96, 150), width=4)

    # The pile, already most of the way to the House.
    rng = np.random.default_rng(5)
    for i in range(120):
        t = rng.random() ** 0.6
        cx = 620 + t * 1150 + rng.normal(0, 40)
        cy = 620 - t * 300 + rng.normal(0, 70)
        r = int(30 - 8 * t + rng.normal(0, 3))
        face = [GOLD, BLOOD, (36, 40, 52)][int(rng.integers(0, 3))]
        chip(d, cx, cy, max(r, 12), face, alpha=int(200 + 55 * t))
    return finish(base, layer)


def frame_2():
    """The boy: a photograph face-up on the felt, which is the whole scene."""
    base = felt_table(0.5, 0.46, warmth=0.8)
    layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)

    for cx, cy, r in ((600, 600, 26), (656, 632, 26), (1330, 540, 26), (1386, 506, 26)):
        chip(d, cx, cy, r, BLOOD if cx < 1000 else GOLD, alpha=210)

    photo = Image.new("RGBA", (420, 500), (0, 0, 0, 0))
    pd = ImageDraw.Draw(photo)
    pd.rectangle((0, 0, 419, 499), fill=BONE + (255,))
    pd.rectangle((22, 22, 397, 420), fill=(46, 52, 58, 255))
    # A boy, at the only fidelity geometry can honestly claim: a silhouette.
    pd.ellipse((168, 120, 252, 204), fill=(24, 27, 31, 255))
    pd.chord((120, 210, 300, 430), 180, 360, fill=(24, 27, 31, 255))
    photo = photo.rotate(-7, resample=Image.BICUBIC, expand=True)
    layer.alpha_composite(photo, (760, 150))
    return finish(base, layer, vig=0.9)


def frame_3():
    """It wasn't. Nothing on the felt but the hand that did it."""
    base = felt_table(0.5, 0.52, warmth=0.55)
    layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))

    # Nothing that could have won: low, off-suit, no pair.
    hand = [("2", "club", False), ("7", "diamond", True), ("4", "spade", False)]
    for i, (rank, kind, red) in enumerate(hand):
        card = playing_card(230, 320, lean=(i - 1) * 8, rank=rank, kind=kind, red=red)
        layer.alpha_composite(card, (620 + i * 250, 250))
    return finish(base, layer, vig=0.94)


def frame_4():
    """The name that stuck, in neon, with the luck burned out of it."""
    base = canvas(NOIR)
    base = add_glow(base, radial(W * 0.5, H * 0.42, W * 0.75, 2.3), (28, 20, 42), 1.5)

    word, dead = "LUCKY JACK", {1, 7}
    font = ImageFont.truetype(str(FONT), 190)

    def draw_word(d, lit_colour, dim_colour):
        x = 250
        for i, ch in enumerate(word):
            colour = dim_colour if i in dead else lit_colour
            d.text((x, 430), ch, font=font, fill=colour)
            x += font.getlength(ch) + 6

    base = screen(base, blurred_layer(lambda d: draw_word(d, MAGENTA, (26, 8, 16)), 22))
    base = add_glow(base, radial(W * 0.5, H * 0.72, W * 0.5, 2.6), MAGENTA, 0.22)

    layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    draw_word(ImageDraw.Draw(layer), MAGENTA + (255,), (48, 18, 30, 255))
    return finish(base, layer, vig=0.8)


def frame_5():
    """Back through the doors, with the light behind him."""
    base = canvas(NOIR)
    base = add_glow(base, radial(W * 0.5, H * 0.30, W * 0.42, 2.2), GOLD, 0.55)
    base = add_glow(base, radial(W * 0.12, H * 0.2, W * 0.3, 2.4), MAGENTA, 0.30)
    base = add_glow(base, radial(W * 0.88, H * 0.2, W * 0.3, 2.4), CYAN, 0.26)

    layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)

    # The facade, with one lit doorway punched out of it.
    d.rectangle((0, 0, W, 300), fill=(8, 8, 11, 255))
    d.rectangle((0, 300, 700, H), fill=(8, 8, 11, 255))
    d.rectangle((1220, 300, W, H), fill=(8, 8, 11, 255))
    d.rounded_rectangle((700, 300, 1220, H), radius=180, fill=(255, 214, 140, 40))
    d.rounded_rectangle((740, 340, 1180, H), radius=150, fill=(255, 222, 158, 70))

    # Him, in it. And the shadow it throws at the viewer.
    d.polygon([(880, 620), (1040, 620), (1180, H), (740, H)], fill=(6, 6, 8, 150))
    d.ellipse((922, 470, 998, 548), fill=(9, 9, 12, 255))
    d.chord((856, 556, 1064, 900), 180, 360, fill=(9, 9, 12, 255))
    d.rectangle((856, 720, 1064, H), fill=(9, 9, 12, 255))
    return finish(base, layer, vig=0.7)


def title():
    """Title backdrop: loud at the top, quiet across the middle for the name."""
    base = canvas(NOIR)
    base = add_glow(base, radial(W / 2, -H * 0.06, W * 0.95, 1.9), (24, 12, 46), 1.5)

    def rays(d):
        apex = (W / 2, -H * 0.2)
        for i in range(30):
            a0 = math.radians(90 - 80 + 160 * i / 30)
            a1 = math.radians(90 - 80 + 160 * (i + 0.5) / 30)
            colour = (96, 20, 54) if i % 2 else (92, 66, 14)
            d.polygon([apex,
                       (apex[0] + H * 2.2 * math.cos(a0), apex[1] + H * 2.2 * math.sin(a0)),
                       (apex[0] + H * 2.2 * math.cos(a1), apex[1] + H * 2.2 * math.sin(a1))],
                      fill=colour)

    base = screen(base, blurred_layer(rays, 26))
    base = add_glow(base, radial(W / 2, H * 1.04, W * 0.5, 2.2), GOLD, 0.34)

    x, y = coords()
    band = np.exp(-(((y - H * 0.46) / (H * 0.26)) ** 2))
    base = base * (1.0 - 0.66 * band)[..., None]
    return to_image(grain(vignette(base, strength=0.82) * 0.94, amount=0.008)).convert("RGBA")


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    (ROOT / "assets" / "backdrops").mkdir(parents=True, exist_ok=True)

    jobs = [
        (OUT / "opening_1.png", frame_1),
        (OUT / "opening_2.png", frame_2),
        (OUT / "opening_3.png", frame_3),
        (OUT / "opening_4.png", frame_4),
        (OUT / "opening_5.png", frame_5),
        (ROOT / "assets" / "backdrops" / "title.png", title),
    ]
    for path, fn in jobs:
        img = fn().convert("RGB")
        img.quantize(colors=256, dither=Image.FLOYDSTEINBERG).save(path, optimize=True)
        arr = np.asarray(Image.open(path).convert("RGB"), dtype=np.float32) / 255
        lum = 0.2126 * arr[..., 0] + 0.7152 * arr[..., 1] + 0.0722 * arr[..., 2]
        # Opening prose is centred over these, same as the lobby.
        zone = np.percentile(lum[760:1040], 99)
        print(f"{path.name:16s} {path.stat().st_size / 1024:6.0f} KB  "
              f"lower-third p99 {zone:.3f}  contrast {(0.871 + 0.05) / (zone + 0.05):.2f}:1")


if __name__ == "__main__":
    main()
