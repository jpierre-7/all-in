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


def finish(base, art_layer, vig=0.88, scrim=0.5):
    """Ground, then subject, then a scrim across the band the prose occupies.

    The scrim comes *after* the subject: the bright things in these frames are
    the chips and the cards, and they sit where the text does, so dimming only
    the background would not have helped. Jack himself is a silhouette, so the
    foreground he occupies is already the darkest part of the frame.
    """
    img = to_image(vignette(base, strength=vig) * 0.95).convert("RGBA")
    img.alpha_composite(art_layer)

    arr = np.asarray(img.convert("RGB"), dtype=np.float32) / 255
    y = coords()[1]
    arr = arr * (1.0 - scrim * np.exp(-(((y - H * 0.22) / (H * 0.20)) ** 2)))[..., None]
    return to_image(grain(arr, amount=0.008)).convert("RGBA")


def chip(d, cx, cy, r, face, rim=BONE, alpha=255):
    """A chip seen edge-on, for stacking."""
    d.ellipse((cx - r, cy - r * 0.34, cx + r, cy + r * 0.34), fill=face + (alpha,),
              outline=rim + (alpha,), width=max(r // 9, 2))


def stack(d, cx, base_y, height, face):
    """A column of chips, tallest first."""
    for i in range(height):
        chip(d, cx, base_y - i * 15, 44, face if i % 2 == 0 else GOLD)


def suit(d, cx, cy, r, kind, colour):
    if kind == "diamond":
        d.polygon([(cx, cy - r), (cx + r * 0.7, cy), (cx, cy + r), (cx - r * 0.7, cy)], fill=colour)
    elif kind == "spade":
        d.polygon([(cx, cy - r), (cx + r * 0.95, cy + r * 0.3), (cx - r * 0.95, cy + r * 0.3)], fill=colour)
        d.ellipse((cx - r, cy - r * 0.1, cx - r * 0.05, cy + r * 0.85), fill=colour)
        d.ellipse((cx + r * 0.05, cy - r * 0.1, cx + r, cy + r * 0.85), fill=colour)
    else:
        q = r * 0.52
        for ox, oy in ((0, -q * 1.15), (-q, q * 0.4), (q, q * 0.4)):
            d.ellipse((cx + ox - q, cy + oy - q, cx + ox + q, cy + oy + q), fill=colour)


def playing_card(w, h, lean, rank, kind, red=False):
    card = Image.new("RGBA", (w + 60, h + 60), (0, 0, 0, 0))
    cd = ImageDraw.Draw(card)
    cd.rounded_rectangle((30, 30, 30 + w, 30 + h), radius=w // 9,
                         fill=BONE + (255,), outline=(150, 142, 124, 255), width=2)
    colour = (BLOOD if red else INK) + (255,)
    cd.text((30 + w * 0.11, 30 + h * 0.05), rank,
            font=ImageFont.truetype(str(BODY_FONT), int(w * 0.32)), fill=colour)
    suit(cd, 30 + w * 0.52, 30 + h * 0.62, w * 0.21, kind, colour)
    return card.rotate(lean, resample=Image.BICUBIC, expand=False)


def hands_across_the_felt(d):
    """The House. Palms down on the far side, and they never move."""
    for cx in (770, 1150):
        d.rounded_rectangle((cx - 66, 470, cx + 66, 560), radius=30, fill=(9, 10, 12, 255))
        for i in range(4):
            x = cx - 48 + i * 32
            d.rounded_rectangle((x - 13, 404, x + 13, 492), radius=13, fill=(9, 10, 12, 255))
        d.rounded_rectangle((cx + 52, 500, cx + 112, 532), radius=16, fill=(9, 10, 12, 255))


def jack(d, hat=False):
    """Lucky Jack from behind, close to camera: the dark the prose sits on."""
    d.ellipse((855, 760, 1065, 970), fill=(7, 8, 10, 255))
    d.chord((640, 900, 1280, 1420), 180, 360, fill=(7, 8, 10, 255))
    d.rectangle((640, 1050, 1280, H), fill=(7, 8, 10, 255))
    if hat:
        d.rounded_rectangle((836, 726, 1084, 782), radius=22, fill=(7, 8, 10, 255))
        d.rounded_rectangle((790, 770, 1130, 800), radius=14, fill=(7, 8, 10, 255))


def chair(d, x):
    """The chair beside him. Frame 2 fills it; frame 3 does not."""
    d.rounded_rectangle((x - 78, 700, x + 78, 940), radius=22, fill=(13, 15, 17, 255))
    for i in range(3):
        d.rounded_rectangle((x - 52 + i * 44, 730, x - 34 + i * 44, 910),
                            radius=8, fill=(24, 28, 30, 255))
    d.rectangle((x - 70, 940, x - 50, H), fill=(13, 15, 17, 255))
    d.rectangle((x + 50, 940, x + 70, H), fill=(13, 15, 17, 255))


def the_table(lamp=1.0):
    """One table, one lamp. Frames 1-3 are the same room at three moments."""
    base = canvas(NOIR)
    base = add_glow(base, radial(W * 0.5, H * 0.42, W * 0.52, 2.0), FELT, 1.45 * lamp)
    base = add_glow(base, radial(W * 0.5, H * 0.30, W * 0.24, 2.5), TEAL, 0.85 * lamp)
    base = add_glow(base, radial(W * 0.5, H * 0.02, W * 0.16, 2.2), (236, 206, 138), 0.70 * lamp)

    layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)

    # The cone, and the shade it falls from.
    cone = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    ImageDraw.Draw(cone).polygon([(890, 120), (1030, 120), (1500, H), (420, H)],
                                 fill=(246, 216, 150, 30))
    cone = cone.filter(ImageFilter.GaussianBlur(40))
    layer.alpha_composite(cone)
    d.polygon([(892, 0), (1028, 0), (1092, 128), (828, 128)], fill=(10, 10, 12, 255))
    d.ellipse((828, 106, 1092, 150), fill=(246, 216, 150, 190))

    # The felt, seen across.
    d.ellipse((250, 360, 1670, 1120), fill=(11, 34, 26, 255))
    d.ellipse((250, 360, 1670, 1120), outline=(58, 44, 24, 220), width=9)
    d.ellipse((286, 388, 1634, 1092), outline=(20, 58, 44, 160), width=3)
    return base, layer, d


def frame_1():
    """Chips stacked high, and the hands already waiting."""
    base, layer, d = the_table()
    hands_across_the_felt(d)
    for x, face, n in ((690, BLOOD, 7), (800, GOLD, 9), (905, BLOOD, 6),
                       (1015, (36, 40, 52), 8), (1120, BLOOD, 5)):
        stack(d, x, 726, n, face)
    jack(d)
    return finish(base, layer)


def frame_2():
    """The chips are gone. There is a boy beside the chair instead."""
    base, layer, d = the_table(lamp=0.92)
    hands_across_the_felt(d)
    chair(d, 1330)
    # The boy: child proportions — big head, narrow shoulders — and short
    # enough that the tabletop crosses him above the waist.
    bx = 1448
    d.ellipse((bx - 46, 690, bx + 46, 786), fill=(7, 8, 10, 255))
    d.rounded_rectangle((bx - 42, 778, bx + 42, 972), radius=30, fill=(7, 8, 10, 255))
    d.rounded_rectangle((bx - 58, 800, bx - 30, 916), radius=14, fill=(7, 8, 10, 255))
    d.rounded_rectangle((bx + 30, 800, bx + 58, 916), radius=14, fill=(7, 8, 10, 255))
    for ox in (-22, 8):
        d.rounded_rectangle((bx + ox, 950, bx + ox + 16, H), radius=8, fill=(7, 8, 10, 255))
    jack(d)
    return finish(base, layer)


def frame_3():
    """Cards turned over, and the chair beside him empty."""
    base, layer, d = the_table(lamp=0.72)
    hands_across_the_felt(d)
    chair(d, 1395)
    for i, (rank, kind, red) in enumerate((("2", "club", False), ("7", "diamond", True),
                                           ("4", "spade", False))):
        layer.alpha_composite(playing_card(150, 210, (i - 1) * 9, rank, kind, red),
                              (740 + i * 170, 470))
    jack(d)
    return finish(base, layer)


def frame_4():
    """Twenty-five years later, across the street, under the sign."""
    base = canvas(NOIR)
    # Everything sits low: the prose now runs along the top of the frame.
    base = add_glow(base, radial(W * 0.5, H * 0.56, W * 0.6, 2.2), (30, 14, 44), 1.5)

    word, dead = "LUCKY JACK", {1, 7}
    font = ImageFont.truetype(str(FONT), 132)
    total = sum(font.getlength(c) + 5 for c in word)

    def draw_word(d, lit, dim):
        x = (W - total) / 2
        for i, ch in enumerate(word):
            d.text((x, 470), ch, font=font, fill=dim if i in dead else lit)
            x += font.getlength(ch) + 5

    base = screen(base, blurred_layer(lambda d: draw_word(d, MAGENTA, (22, 7, 14)), 20))
    base = add_glow(base, radial(W * 0.5, H * 0.84, W * 0.34, 2.4), MAGENTA, 0.30)

    layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    draw_word(ImageDraw.Draw(layer), MAGENTA + (255,), (44, 16, 28, 255))
    d = ImageDraw.Draw(layer)
    # The facade the sign is bolted to: dark, with lit windows above the street.
    d.rectangle((0, 636, W, 900), fill=(11, 10, 15, 255))
    for i in range(14):
        x = 40 + i * 140
        d.rectangle((x, 672, x + 62, 730), fill=(196, 158, 92, 26))
    d.rectangle((0, 892, W, 906), fill=(150, 118, 60, 60))
    # Pavement under the sign, so he has something to stand against.
    d.polygon([(560, 906), (1360, 906), (1560, H), (360, H)],
              fill=(188, 120, 150, 22))
    # Him, on the far kerb, in a coat that has seen better decades.
    d.ellipse((922, 706, 998, 782), fill=(7, 8, 10, 255))
    d.rounded_rectangle((906, 696, 1014, 722), radius=12, fill=(7, 8, 10, 255))
    d.rounded_rectangle((874, 772, 1046, 920), radius=38, fill=(7, 8, 10, 255))
    d.polygon([(878, 886), (1042, 886), (1066, H), (854, H)], fill=(7, 8, 10, 255))
    return finish(base, layer, vig=0.74)


def frame_5():
    """Inside now, with the doors swinging shut behind him."""
    base = canvas(NOIR)
    base = add_glow(base, radial(W * 0.5, H * 0.56, W * 0.40, 2.1), (236, 202, 132), 0.85)
    base = add_glow(base, radial(W * 0.5, H * 0.86, W * 0.7, 2.1), BLOOD, 0.34)

    layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)

    d.rectangle((0, 0, W, H), fill=(11, 10, 14, 255))

    # The aperture in the wall, and the frame around it.
    d.rectangle((700, 346, 1220, 936), fill=(6, 6, 8, 255))
    d.rectangle((686, 332, 1234, 950), outline=(150, 118, 60, 130), width=7)

    # Two leaves, nearly shut. What is left between them is the street.
    d.polygon([(952, 360), (968, 360), (968, 928), (952, 928)], fill=(246, 222, 164, 210))
    for sign, hinge in ((-1, 706), (1, 1214)):
        lead = 960 + sign * 34
        d.polygon([(hinge, 352), (lead, 384), (lead, 906), (hinge, 934)],
                  fill=(38, 32, 28, 255))
        d.polygon([(hinge, 352), (lead, 384), (lead, 906), (hinge, 934)],
                  outline=(150, 118, 60, 150), width=5)
        inset = sign * 36
        d.polygon([(hinge + inset, 420), (lead - inset, 448),
                   (lead - inset, 722), (hinge + inset, 742)],
                  outline=(150, 118, 60, 90), width=4)
        d.rounded_rectangle((lead - sign * 30 - 8, 664, lead - sign * 30 + 8, 738),
                            radius=8, fill=(198, 160, 84, 235))

    # The lobby carpet, running away from the doors towards the reader.
    for i in range(11):
        x = i * (W / 10)
        d.polygon([(x - 60, H), (x + 60, H), (960 + (x - 960) * 0.12, 936)],
                  fill=(84, 18, 26, 70) if i % 2 else (56, 12, 18, 70))
    d.polygon([(0, 936), (W, 936), (W, 980), (0, 980)], fill=(0, 0, 0, 110))

    # Him, just through them.
    d.ellipse((926, 566, 994, 634), fill=(6, 7, 9, 255))
    d.rounded_rectangle((884, 626, 1036, 890), radius=44, fill=(6, 7, 9, 255))
    d.polygon([(884, 770), (1036, 770), (1066, 946), (854, 946)], fill=(6, 7, 9, 255))
    return finish(base, layer, vig=0.72)


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
        zone = np.percentile(lum[170:420], 99)
        print(f"{path.name:16s} {path.stat().st_size / 1024:6.0f} KB  "
              f"prose-band p99 {zone:.3f}  contrast {(0.871 + 0.05) / (zone + 0.05):.2f}:1")


if __name__ == "__main__":
    main()
