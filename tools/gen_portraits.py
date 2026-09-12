#!/usr/bin/env python3
"""Generate the three boss portraits.

Designs are Dev 1's, from the comment on the art ticket: Slotz is a
chrome-and-neon slot machine on a rolling base, the Pit Boss is a man-sized
brass balance scale with one pan already piled with chips, and The House is
just a pair of hands on the felt.

Flat neon signage rather than illustration — it is what geometry can carry
honestly, and it matches the backdrops. Transparent ground so they sit on
whatever is behind them.

Run: python3 tools/gen_portraits.py
"""

import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "assets" / "portraits"

S = 4  # supersample
P = 256  # portrait edge

CHROME_LIT = (206, 214, 222)
CHROME = (128, 140, 152)
CHROME_DARK = (58, 66, 76)
BRASS_LIT = (240, 206, 122)
BRASS = (186, 142, 58)
BRASS_DARK = (96, 68, 24)
MAGENTA = (255, 46, 136)
CYAN = (38, 217, 224)
GOLD = (255, 197, 61)
BLOOD = (176, 30, 48)
SKIN = (214, 176, 140)
SKIN_DARK = (120, 92, 68)
INK = (14, 14, 18)


def canvas():
    return Image.new("RGBA", (P * S, P * S), (0, 0, 0, 0))


def s(*vals):
    return tuple(v * S for v in vals)


def finish(art, glow_fn, blur=9):
    """Lay a blurred copy of the neon strokes under the crisp art."""
    layer = Image.new("RGB", (P * S, P * S), (0, 0, 0))
    glow_fn(ImageDraw.Draw(layer))
    layer = layer.filter(ImageFilter.GaussianBlur(blur * S))
    glow = layer.convert("RGBA")
    glow.putalpha(layer.convert("L").point(lambda v: min(255, int(v * 2.2))))

    out = Image.alpha_composite(glow, art)
    return out.resize((P, P), Image.LANCZOS)


def slotz():
    art = canvas()
    d = ImageDraw.Draw(art)

    # Cabinet.
    d.rounded_rectangle(s(46, 52, 210, 206), radius=16 * S, fill=CHROME_DARK + (255,),
                        outline=CHROME_LIT + (255,), width=3 * S)
    # Crown.
    d.rounded_rectangle(s(62, 24, 194, 58), radius=14 * S, fill=INK + (255,),
                        outline=MAGENTA + (255,), width=3 * S)
    d.ellipse(s(120, 32, 136, 48), fill=MAGENTA + (255,))

    # Three reels.
    for i in range(3):
        x0 = 60 + i * 48
        d.rounded_rectangle(s(x0, 84, x0 + 38, 138), radius=5 * S, fill=INK + (255,),
                            outline=GOLD + (255,), width=2 * S)
    # Reel symbols: a seven, a bar, a cherry.
    d.polygon(s(70, 96, 90, 96, 80, 128), fill=BLOOD + (255,))
    d.rectangle(s(114, 104, 146, 118), fill=CYAN + (255,))
    d.ellipse(s(160, 110, 182, 130), fill=BLOOD + (255,))
    d.line(s(171, 110, 180, 94), fill=(70, 150, 70, 255), width=3 * S)

    # Payout tray and lever.
    d.rounded_rectangle(s(66, 158, 190, 190), radius=6 * S, fill=INK + (255,),
                        outline=CHROME + (255,), width=2 * S)
    for i in range(4):
        d.ellipse(s(78 + i * 26, 166, 96 + i * 26, 178), fill=GOLD + (255,))
    d.line(s(210, 92, 232, 66), fill=CHROME_LIT + (255,), width=4 * S)
    d.ellipse(s(222, 52, 244, 74), fill=BLOOD + (255,), outline=CHROME_LIT + (255,), width=2 * S)

    # Rolling base.
    d.rectangle(s(60, 206, 196, 216), fill=CHROME + (255,))
    for cx in (84, 172):
        d.ellipse(s(cx - 13, 214, cx + 13, 240), fill=CHROME_DARK + (255,),
                  outline=CHROME_LIT + (255,), width=2 * S)

    def glow(g):
        g.rounded_rectangle(s(62, 24, 194, 58), radius=14 * S, outline=MAGENTA, width=4 * S)
        g.rounded_rectangle(s(46, 52, 210, 206), radius=16 * S, outline=CYAN, width=3 * S)
        for i in range(3):
            x0 = 60 + i * 48
            g.rounded_rectangle(s(x0, 84, x0 + 38, 138), radius=5 * S, outline=GOLD, width=3 * S)

    return finish(art, glow)


def pit_boss():
    art = canvas()
    d = ImageDraw.Draw(art)

    # Plinth and column.
    d.polygon(s(92, 236, 164, 236, 152, 210, 104, 210), fill=BRASS_DARK + (255,))
    d.rectangle(s(120, 74, 136, 212), fill=BRASS + (255,))
    d.rectangle(s(120, 74, 126, 212), fill=BRASS_LIT + (255,))

    # Beam, tilted: the loaded pan hangs lower.
    d.line(s(40, 96, 216, 68), fill=BRASS_LIT + (255,), width=7 * S)
    d.ellipse(s(118, 66, 138, 86), fill=BRASS_LIT + (255,), outline=BRASS_DARK + (255,), width=2 * S)

    def pan(cx, cy, loaded):
        d.line((cx * S - 34 * S, cy * S - 46 * S, cx * S, cy * S), fill=BRASS + (255,), width=2 * S)
        d.line((cx * S + 34 * S, cy * S - 46 * S, cx * S, cy * S), fill=BRASS + (255,), width=2 * S)
        d.chord(s(cx - 40, cy - 18, cx + 40, cy + 26), 0, 180, fill=BRASS + (255,),
                outline=BRASS_LIT + (255,), width=2 * S)
        if loaded:
            for i in range(4):
                w = 30 - i * 5
                cyy = cy - 4 - i * 9
                fill = GOLD if i % 2 == 0 else BLOOD
                d.ellipse(s(cx - w, cyy - 6, cx + w, cyy + 6), fill=fill + (255,),
                          outline=BRASS_LIT + (255,), width=S)

    pan(48, 150, loaded=True)
    pan(208, 116, loaded=False)

    def glow(g):
        g.line(s(40, 96, 216, 68), fill=BRASS_LIT, width=8 * S)
        g.rectangle(s(120, 74, 136, 212), fill=BRASS)
        g.chord(s(8, 132, 88, 176), 0, 180, outline=GOLD, width=5 * S)

    return finish(art, glow, blur=7)


def _one_hand():
    """One hand on its own layer, palm down, so it can be rotated and mirrored."""
    layer = canvas()
    d = ImageDraw.Draw(layer)
    outline = SKIN_DARK + (255,)

    # Fingers first; the palm then overlaps their base and the seam reads as
    # knuckles. Drawn as separate rects with real gaps — the first attempt fanned
    # lines out of the palm centre and they merged into a mitten.
    for x0, top in ((99, 60), (114, 48), (129, 57), (144, 76)):
        d.rounded_rectangle(s(x0, top, x0 + 13, 116), radius=6 * S,
                            fill=SKIN + (255,), outline=outline, width=2 * S)

    # Thumb, out to the side and angled down.
    d.line(s(106, 152, 82, 132), fill=SKIN + (255,), width=17 * S)
    d.ellipse(s(74, 124, 92, 142), fill=SKIN + (255,), outline=outline, width=2 * S)

    d.rounded_rectangle(s(97, 100, 159, 170), radius=17 * S,
                        fill=SKIN + (255,), outline=outline, width=2 * S)
    return layer


def the_house():
    """Two hands, palms down on the felt. No face: that is the point of them."""
    art = canvas()

    hand = _one_hand()
    right = hand.rotate(-11, resample=Image.BICUBIC, center=(128 * S, 140 * S))
    left = hand.transpose(Image.FLIP_LEFT_RIGHT).rotate(
        11, resample=Image.BICUBIC, center=(128 * S, 140 * S)
    )
    # Palms down and side by side, the thumbs face each other; set the hands
    # wide enough apart that they do not merge into one lump in the middle.
    art.alpha_composite(left, (-58 * S, 4 * S))
    art.alpha_composite(right, (58 * S, 4 * S))

    # Chips on the felt between them, so the hands read as resting on a table.
    d = ImageDraw.Draw(art)
    for cx, cy in ((128, 226), (150, 232)):
        d.ellipse(s(cx - 19, cy - 8, cx + 19, cy + 8), fill=BLOOD + (255,),
                  outline=GOLD + (255,), width=S)

    def glow(g):
        g.ellipse(s(20, 80, 236, 230), fill=(44, 18, 26))

    return finish(art, glow, blur=16)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for name, fn in (("slotz", slotz), ("pit_boss", pit_boss), ("the_house", the_house)):
        path = OUT / f"{name}.png"
        fn().save(path)
        print(f"{name:10s} {path.stat().st_size / 1024:5.0f} KB  {Image.open(path).size}")


if __name__ == "__main__":
    main()
