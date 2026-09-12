#!/usr/bin/env python3
"""Generate the placeholder art set for `assets/`.

Every file this writes is sized and named to match `assets/README.md`, so the
combat UI can bind to real paths before the final art exists. Replacing a file
with hand-drawn art needs no code change as long as the dimensions hold.

Run: python3 tools/gen_placeholders.py
"""

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "assets"

# Supersample factor. Everything is drawn large and downscaled so the curves
# and diagonals come out clean without any antialiasing work.
SS = 4

# --- Palette -----------------------------------------------------------------
# Casino-noir, per issue #11: dark felt green/black with gold text. These seven
# are the whole vocabulary; generated backdrops get graded into them so the
# hand-drawn icons and the generated art agree.
FELT = (0x10, 0x39, 0x2C)  # table felt green
FELT_DEEP = (0x0A, 0x21, 0x1A)  # shadowed felt
NOIR = (0x08, 0x08, 0x0A)  # near-black
GOLD = (0xC9, 0xA2, 0x27)  # primary gold: frame stroke, text
GOLD_LIT = (0xF2, 0xDC, 0x8B)  # rim light, highlights
BONE = (0xED, 0xE4, 0xD0)  # high-contrast text on felt
BLOOD = (0x8E, 0x1B, 0x23)  # accent: Whiff, red suits

# --- Card frame geometry -----------------------------------------------------
# 7 cards across a ~1280px window is ~160px per card; authored at 2x on the
# 2.5:3.5 poker ratio. SLICE must exceed RADIUS + BORDER so a 9-slice corner
# region contains the entire corner.
CARD_W, CARD_H = 320, 448
RADIUS, BORDER, SLICE = 28, 6, 48
SAFE = 24  # text inset from the card edge

ICON = 128
BACKDROP_W, BACKDROP_H = 1920, 1080


def _lerp(a, b, t):
    return tuple(round(x + (y - x) * t) for x, y in zip(a, b))


def _rounded_mask(w, h, radius):
    """L-mode mask of a rounded rect, drawn supersampled and downscaled."""
    mask = Image.new("L", (w * SS, h * SS), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        (0, 0, w * SS - 1, h * SS - 1), radius=radius * SS, fill=255
    )
    return mask.resize((w, h), Image.LANCZOS)


def _vertical_gradient(w, h, top, bottom):
    grad = Image.new("RGB", (1, h))
    for y in range(h):
        grad.putpixel((0, y), _lerp(top, bottom, y / max(h - 1, 1)))
    return grad.resize((w, h), Image.BICUBIC)


def card_frame(path):
    """Opaque card body with a gold bezel; corners transparent for the 9-slice.

    The interior is left flat so name / Stack / Tell render legibly on top.
    """
    body = _vertical_gradient(CARD_W, CARD_H, FELT, FELT_DEEP).convert("RGBA")
    body.putalpha(_rounded_mask(CARD_W, CARD_H, RADIUS))

    # Outer gold stroke, then a hairline bezel inset from it.
    stroke = Image.new("RGBA", (CARD_W * SS, CARD_H * SS), (0, 0, 0, 0))
    d = ImageDraw.Draw(stroke)
    d.rounded_rectangle(
        (0, 0, CARD_W * SS - 1, CARD_H * SS - 1),
        radius=RADIUS * SS,
        outline=GOLD + (255,),
        width=BORDER * SS,
    )
    inset = 14 * SS
    d.rounded_rectangle(
        (inset, inset, CARD_W * SS - 1 - inset, CARD_H * SS - 1 - inset),
        radius=(RADIUS - 8) * SS,
        outline=GOLD_LIT + (110,),
        width=max(SS // 2, 1),
    )
    body.alpha_composite(stroke.resize((CARD_W, CARD_H), Image.LANCZOS))
    body.save(path)


def _icon_canvas():
    return Image.new("RGBA", (ICON * SS, ICON * SS), (0, 0, 0, 0))


def _save_icon(img, path):
    img.resize((ICON, ICON), Image.LANCZOS).save(path)


def tell_streak(path):
    """Double chevron: one play carrying into the next."""
    img = _icon_canvas()
    d = ImageDraw.Draw(img)
    w = 13 * SS
    for x0 in (30 * SS, 62 * SS):
        d.line(
            [(x0, 28 * SS), (x0 + 26 * SS, 64 * SS), (x0, 100 * SS)],
            fill=GOLD + (255,),
            width=w,
            joint="curve",
        )
    _save_icon(img, path)


def tell_all_in(path):
    """A card struck through: the sacrifice this Tell makes from the Draw.

    A side-on chip stack was the first instinct, but the gaps between chips
    vanish at the ~24px the icon actually renders at. A rectangle plus a
    diagonal keeps its silhouette all the way down, and reads distinctly
    against Streak's chevrons.
    """
    img = _icon_canvas()
    d = ImageDraw.Draw(img)
    d.rounded_rectangle(
        (34 * SS, 22 * SS, 94 * SS, 106 * SS),
        radius=9 * SS,
        outline=GOLD + (255,),
        width=8 * SS,
    )
    d.line(
        [(24 * SS, 112 * SS), (104 * SS, 16 * SS)],
        fill=BLOOD + (255,),
        width=13 * SS,
    )
    _save_icon(img, path)


def backdrop_combat(path):
    """Felt table under a low lamp: bright center falling off to near-black."""
    img = _vertical_gradient(BACKDROP_W, BACKDROP_H, FELT_DEEP, NOIR)
    px = img.load()
    cx, cy = BACKDROP_W / 2, BACKDROP_H * 0.42
    max_d = (cx**2 + cy**2) ** 0.5
    for y in range(BACKDROP_H):
        for x in range(0, BACKDROP_W, 2):
            t = 1.0 - min((((x - cx) ** 2 + (y - cy) ** 2) ** 0.5) / max_d, 1.0)
            glow = _lerp(px[x, y], FELT, t**2 * 0.55)
            px[x, y] = glow
            if x + 1 < BACKDROP_W:
                px[x + 1, y] = glow
    img.save(path)


def backdrop_lobby(path):
    """Noir entryway with a warm marquee glow bleeding down from the top."""
    img = _vertical_gradient(BACKDROP_W, BACKDROP_H, NOIR, (0x04, 0x04, 0x05))
    px = img.load()
    cx, cy = BACKDROP_W / 2, -BACKDROP_H * 0.12
    max_d = BACKDROP_H * 1.15
    for y in range(BACKDROP_H):
        for x in range(0, BACKDROP_W, 2):
            t = 1.0 - min((((x - cx) ** 2 + (y - cy) ** 2) ** 0.5) / max_d, 1.0)
            glow = _lerp(px[x, y], GOLD, t**3 * 0.30)
            px[x, y] = glow
            if x + 1 < BACKDROP_W:
                px[x + 1, y] = glow
    img.save(path)


def _relative_luminance(rgb):
    def channel(c):
        c /= 255
        return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4

    r, g, b = (channel(c) for c in rgb)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def contrast(fg, bg):
    a, b = sorted((_relative_luminance(fg), _relative_luminance(bg)), reverse=True)
    return (a + 0.05) / (b + 0.05)


def main():
    (ASSETS / "cards").mkdir(parents=True, exist_ok=True)
    (ASSETS / "icons").mkdir(parents=True, exist_ok=True)
    (ASSETS / "backdrops").mkdir(parents=True, exist_ok=True)

    card_frame(ASSETS / "cards" / "frame.png")
    tell_streak(ASSETS / "icons" / "tell_streak.png")
    tell_all_in(ASSETS / "icons" / "tell_all_in.png")
    backdrop_combat(ASSETS / "backdrops" / "combat.png")
    backdrop_lobby(ASSETS / "backdrops" / "lobby.png")

    # Text on the card body is the one readability risk worth checking.
    for name, fg in (("gold", GOLD), ("gold-lit", GOLD_LIT), ("bone", BONE)):
        print(f"contrast {name:9s} on felt-deep: {contrast(fg, FELT_DEEP):.2f}:1")


if __name__ == "__main__":
    main()
