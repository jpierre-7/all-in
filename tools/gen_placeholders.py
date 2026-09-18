#!/usr/bin/env python3
"""Generate placeholder card art for `assets/`: the frame and the Tell icons.

Paths and sizes come from the merged combat UI (`src/combat/ui.rs`), not from
taste — `load_art` binds these exact relative paths, and a mismatch fails
silently as a text fallback. Replacing a file with hand-drawn art needs no code
change as long as the path and dimensions hold.

Backdrops are a separate job; see `tools/gen_backdrops.py`.

Run: python3 tools/gen_placeholders.py
"""

from pathlib import Path

import numpy as np
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
# The card node is a fixed 120x170 (src/combat/ui.rs), drawn with a plain
# ImageNode and no slicing, so the frame is simply stretched to fit. Authoring
# at exactly 2x keeps that stretch uniform and the bezel undistorted.
CARD_W, CARD_H = 240, 340
RADIUS, BORDER = 21, 5
# The node carries 8px of padding, so content lives inside a 104x154 box:
# 16px of inset in authored pixels.
SAFE = 16

# Tell icons render into a 28x28 node. 128 source gives room to redraw at
# higher fidelity without rebinding anything.
ICON = 128


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


def _guilloche(w, h, mask):
    """Engine-turned line work, the kind on a banknote or a casino plaque.

    Two rosettes beaten against each other: concentric rings whose radius is
    modulated by angle, plus a rotating second harmonic. The result is a fine
    lattice that reads as texture rather than pattern once the card is drawn at
    half size — which is the point. It is what makes the face look worked
    rather than filled.
    """
    yy, xx = np.mgrid[0:h, 0:w].astype(np.float32)
    nx = (xx - w / 2) / (w / 2)
    ny = (yy - h / 2) / (h / 2)
    r = np.sqrt(nx**2 + ny**2)
    th = np.arctan2(ny, nx)

    # Frequencies are kept low on purpose: the card is drawn at half the size
    # it is authored, and a denser lattice dissolves into noise on the way down.
    rings = np.sin(19.0 * r + 2.6 * np.sin(6.0 * th))
    weave = np.sin(12.0 * th + 14.0 * r)
    field = 0.62 * rings + 0.38 * weave

    # Keep only the crests, so we get lines instead of a wash.
    lines = np.clip(1.0 - np.abs(field) * 3.2, 0.0, 1.0) ** 1.2
    # Fade it out towards the edge, where the bezel takes over.
    lines *= np.clip(1.25 - r, 0.0, 1.0)

    layer = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    alpha = Image.fromarray((lines * 104).astype(np.uint8), "L")
    layer.paste(Image.new("RGBA", (w, h), GOLD_LIT + (255,)), (0, 0), alpha)
    layer.putalpha(Image.composite(layer.getchannel("A"),
                                   Image.new("L", (w, h), 0), mask))
    return layer


def _deco_corner(d, x, y, sx, sy, scale):
    """A stepped deco fan. Three rules turning the corner, longest outermost."""
    for i, (off, run) in enumerate(((0, 34), (7, 24), (14, 15))):
        o = off * scale
        length = run * scale
        width = max(int((3 - i * 0.6) * scale), 2)
        d.line((x + sx * o, y + sy * (o + length), x + sx * o, y + sy * o,
                x + sx * (o + length), y + sy * o),
               fill=GOLD + (255 - i * 45,), width=width, joint="curve")


def card_frame(path):
    """The card face: engine-turned felt inside a deco bezel.

    The node draws no border of its own while this art exists, so every edge the
    card has is drawn here.
    """
    body = _vertical_gradient(CARD_W, CARD_H, FELT, FELT_DEEP).convert("RGBA")
    shape = _rounded_mask(CARD_W, CARD_H, RADIUS)
    body.putalpha(shape)

    big = (CARD_W * SS, CARD_H * SS)
    inner_mask = Image.new("L", big, 0)
    ImageDraw.Draw(inner_mask).rounded_rectangle(
        (13 * SS, 13 * SS, big[0] - 13 * SS, big[1] - 13 * SS),
        radius=(RADIUS - 7) * SS, fill=255,
    )
    body.alpha_composite(
        _guilloche(*big, inner_mask).resize((CARD_W, CARD_H), Image.LANCZOS)
    )

    # Sink the body towards its edge so name and Stack sit on flat felt.
    shade = Image.new("RGBA", big, (0, 0, 0, 0))
    sd = ImageDraw.Draw(shade)
    for i in range(20):
        sd.rounded_rectangle(
            (i * SS, i * SS, big[0] - 1 - i * SS, big[1] - 1 - i * SS),
            radius=max(RADIUS - i, 2) * SS,
            outline=(0, 0, 0, int(30 * (1 - i / 19))), width=2 * SS,
        )
    body.alpha_composite(shade.resize((CARD_W, CARD_H), Image.LANCZOS))

    stroke = Image.new("RGBA", big, (0, 0, 0, 0))
    d = ImageDraw.Draw(stroke)
    # Outer rule, a dark channel, then a fine inner rule: three edges, not one.
    d.rounded_rectangle((0, 0, big[0] - 1, big[1] - 1), radius=RADIUS * SS,
                        outline=GOLD + (255,), width=BORDER * SS)
    d.rounded_rectangle((BORDER * SS, BORDER * SS,
                         big[0] - 1 - BORDER * SS, big[1] - 1 - BORDER * SS),
                        radius=(RADIUS - BORDER) * SS,
                        outline=FELT_DEEP + (190,), width=2 * SS)
    d.rounded_rectangle((11 * SS, 11 * SS, big[0] - 1 - 11 * SS, big[1] - 1 - 11 * SS),
                        radius=(RADIUS - 8) * SS, outline=GOLD_LIT + (165,), width=SS)

    for x, sx in ((20 * SS, 1), (big[0] - 20 * SS, -1)):
        for y, sy in ((20 * SS, 1), (big[1] - 20 * SS, -1)):
            _deco_corner(d, x, y, sx, sy, SS)

    body.alpha_composite(stroke.resize((CARD_W, CARD_H), Image.LANCZOS))

    # A raking highlight down the top-left of the bezel, so it reads as metal.
    sheen = Image.new("RGBA", big, (0, 0, 0, 0))
    ImageDraw.Draw(sheen).rounded_rectangle(
        (0, 0, big[0] - 1, big[1] - 1), radius=RADIUS * SS,
        outline=(255, 255, 255, 70), width=BORDER * SS,
    )
    grad = Image.linear_gradient("L").rotate(-35, resample=Image.BICUBIC).resize(big)
    sheen.putalpha(Image.composite(
        Image.new("L", big, 0), sheen.getchannel("A"), grad))
    body.alpha_composite(sheen.resize((CARD_W, CARD_H), Image.LANCZOS))

    body.putalpha(shape)
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


def tell_copycat(path):
    """Two cards, one behind the other: this card takes on the next one.

    The front card is drawn solid so the pair reads as one silhouette with a
    step in it at 28px, rather than two thin outlines that blur together.
    """
    img = _icon_canvas()
    d = ImageDraw.Draw(img)
    # The card behind, up and to the right: the one being copied.
    d.rounded_rectangle(
        (48 * SS, 14 * SS, 106 * SS, 94 * SS),
        radius=9 * SS,
        outline=GOLD + (255,),
        width=8 * SS,
    )
    # The copy in front, solid, with a dark rim so it cuts out of the one behind.
    d.rounded_rectangle(
        (22 * SS, 34 * SS, 80 * SS, 114 * SS),
        radius=9 * SS,
        fill=GOLD + (255,),
        outline=(0, 0, 0, 255),
        width=5 * SS,
    )
    _save_icon(img, path)


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
    (ASSETS / "tells").mkdir(parents=True, exist_ok=True)

    card_frame(ASSETS / "cards" / "frame.png")
    tell_streak(ASSETS / "tells" / "streak.png")
    tell_all_in(ASSETS / "tells" / "all_in.png")
    tell_copycat(ASSETS / "tells" / "copycat.png")

    # Text on the card body is the one readability risk worth checking.
    for name, fg in (("gold", GOLD), ("gold-lit", GOLD_LIT), ("bone", BONE)):
        print(f"contrast {name:9s} on felt-deep: {contrast(fg, FELT_DEEP):.2f}:1")


if __name__ == "__main__":
    main()
