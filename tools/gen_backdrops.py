#!/usr/bin/env python3
"""Generate the two casino backdrops.

Neon-noir rather than flat noir: saturated magenta/cyan/gold signage against a
dark ground. The colour lives at the edges and the centre stays dark on
purpose — `src/combat/ui.rs` lays gold and bone text straight over the combat
backdrop with no scrim, so a bright middle would swallow it.

Run: python3 tools/gen_backdrops.py
"""

import math
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "assets"
W, H = 1920, 1080

# Neon extensions to the base palette in assets/README.md.
MAGENTA = (0xFF, 0x2E, 0x88)
CYAN = (0x26, 0xD9, 0xE0)
GOLD = (0xFF, 0xC5, 0x3D)
VIOLET = (0x6B, 0x2F, 0xA8)
INDIGO = (0x16, 0x0A, 0x30)
TEAL = (0x0E, 0x4A, 0x44)
FELT = (0x10, 0x39, 0x2C)
NOIR = (0x08, 0x08, 0x0A)


def rgb(c):
    return np.array(c, dtype=np.float32) / 255.0


def canvas(colour):
    return np.tile(rgb(colour), (H, W, 1))


def coords():
    y, x = np.mgrid[0:H, 0:W].astype(np.float32)
    return x, y


def radial(cx, cy, radius, falloff=2.0):
    """A 0..1 field falling off from a point. The workhorse for every glow."""
    x, y = coords()
    d = np.sqrt((x - cx) ** 2 + (y - cy) ** 2) / radius
    return np.clip(1.0 - d, 0.0, 1.0) ** falloff


def screen(base, layer):
    """Screen blend — adds light without clipping to white the way addition does."""
    return 1.0 - (1.0 - base) * (1.0 - layer)


def add_glow(base, field, colour, strength=1.0):
    layer = field[..., None] * rgb(colour) * strength
    return screen(base, np.clip(layer, 0.0, 1.0))


def to_image(arr):
    return Image.fromarray((np.clip(arr, 0, 1) * 255).astype(np.uint8), "RGB")


def blurred_layer(draw_fn, blur):
    """Draw on black, blur, and hand back a float field to screen on top.

    Drawing the glow separately and blurring it is what makes the neon read as
    light rather than as a flat painted shape.
    """
    layer = Image.new("RGB", (W, H), (0, 0, 0))
    draw_fn(ImageDraw.Draw(layer))
    layer = layer.filter(ImageFilter.GaussianBlur(blur))
    return np.asarray(layer, dtype=np.float32) / 255.0


def grain(arr, amount=0.015, seed=7):
    """A little film grain. Flat procedural gradients band badly without it."""
    rng = np.random.default_rng(seed)
    return np.clip(arr + rng.normal(0, amount, arr.shape).astype(np.float32), 0, 1)


def vignette(arr, strength=0.85, radius=1.15):
    x, y = coords()
    d = np.sqrt(((x - W / 2) / (W / 2)) ** 2 + ((y - H / 2) / (H / 2)) ** 2) / radius
    return arr * (1.0 - strength * np.clip(d, 0, 1) ** 2.2)[..., None]


def protect_text_bands(arr):
    """Darken the strips the UI writes into.

    src/combat/ui.rs pads the root by 40px and spreads three rows top to
    bottom, so the enemy line and the player line sit in the outer ~180px.
    """
    y = coords()[1]
    band = np.clip((y - 60) / 260, 0, 1) * np.clip((H - 60 - y) / 260, 0, 1)
    return arr * (0.68 + 0.32 * np.sin(band * np.pi / 2) ** 2)[..., None]


def prose_scrim(arr):
    """Darken the middle of the lobby, where the overworld centres its prose.

    `Screen::spawn` uses `justify_content: Center`, so the text lands in the
    vertical middle — not in the outer bands the combat screen writes into.
    The falloff is horizontal as well as vertical so the marquee keeps its
    colour at the top and down both edges.
    """
    x, y = coords()
    band = np.exp(-(((y - H * 0.52) / (H * 0.30)) ** 2))
    horiz = np.exp(-(((x - W / 2) / (W * 0.46)) ** 2))
    return arr * (1.0 - 0.62 * band * horiz)[..., None]


# --- Combat ------------------------------------------------------------------


def suit_pattern(blur=1.5):
    """Card suits scattered through the felt, barely above the noise floor."""

    def draw(d):
        rng = np.random.default_rng(11)
        for _ in range(46):
            cx, cy = rng.integers(0, W), rng.integers(0, H)
            s = int(rng.integers(26, 64))
            tone = int(rng.integers(14, 30))
            fill = (tone, tone, tone)
            kind = rng.integers(0, 4)
            if kind == 0:  # diamond
                d.polygon(
                    [(cx, cy - s), (cx + s * 0.68, cy), (cx, cy + s), (cx - s * 0.68, cy)],
                    fill=fill,
                )
            elif kind == 1:  # heart
                r = s * 0.52
                d.ellipse((cx - r * 1.5, cy - r, cx - r * 0.1, cy + r * 0.4), fill=fill)
                d.ellipse((cx + r * 0.1, cy - r, cx + r * 1.5, cy + r * 0.4), fill=fill)
                d.polygon([(cx - r * 1.45, cy + r * 0.1), (cx + r * 1.45, cy + r * 0.1), (cx, cy + s)], fill=fill)
            elif kind == 2:  # spade
                r = s * 0.52
                d.polygon([(cx, cy - s), (cx + r * 1.45, cy + r * 0.35), (cx - r * 1.45, cy + r * 0.35)], fill=fill)
                d.ellipse((cx - r * 1.5, cy - r * 0.2, cx - r * 0.1, cy + r * 1.2), fill=fill)
                d.ellipse((cx + r * 0.1, cy - r * 0.2, cx + r * 1.5, cy + r * 1.2), fill=fill)
                d.polygon([(cx - r * 0.4, cy + s), (cx + r * 0.4, cy + s), (cx, cy + r * 0.4)], fill=fill)
            else:  # club
                r = s * 0.42
                for ox, oy in ((0, -r * 1.1), (-r, r * 0.35), (r, r * 0.35)):
                    d.ellipse((cx + ox - r, cy + oy - r, cx + ox + r, cy + oy + r), fill=fill)
                d.polygon([(cx - r * 0.4, cy + s), (cx + r * 0.4, cy + s), (cx, cy + r * 0.4)], fill=fill)

    return blurred_layer(draw, blur)


def combat():
    # Felt, lit from a lamp hanging just above the table.
    base = canvas(NOIR)
    base = add_glow(base, radial(W / 2, H * 0.40, W * 0.78, 2.0), FELT, 1.40)
    base = add_glow(base, radial(W / 2, H * 0.34, W * 0.38, 2.3), TEAL, 1.05)

    base = screen(base, suit_pattern() * 0.5)

    # Signage off-frame either side: the colour in the picture.
    base = add_glow(base, radial(-W * 0.02, H * 0.70, W * 0.60, 2.0), MAGENTA, 1.10)
    base = add_glow(base, radial(W * 1.02, H * 0.34, W * 0.58, 2.0), CYAN, 0.95)
    base = add_glow(base, radial(W * 0.18, -H * 0.06, W * 0.34, 2.2), VIOLET, 0.60)
    base = add_glow(base, radial(W / 2, -H * 0.10, W * 0.44, 2.3), GOLD, 0.40)

    base = protect_text_bands(base)
    base = vignette(base, strength=0.80)
    return grain(base * 0.94)


# --- Lobby -------------------------------------------------------------------


def marquee_rays(blur=26):
    """Art-deco sunburst behind the marquee, alternating magenta and gold."""

    def draw(d):
        apex = (W / 2, -H * 0.16)
        n, span, R = 26, 150.0, H * 2.0
        for i in range(n):
            a0 = math.radians(90 - span / 2 + span * i / n)
            a1 = math.radians(90 - span / 2 + span * (i + 0.55) / n)
            colour = (104, 20, 58) if i % 2 else (96, 68, 14)
            d.polygon(
                [apex,
                 (apex[0] + R * math.cos(a0), apex[1] + R * math.sin(a0)),
                 (apex[0] + R * math.cos(a1), apex[1] + R * math.sin(a1))],
                fill=colour,
            )

    return blurred_layer(draw, blur)


def marquee_arcs(blur=18):
    """Three neon arcs over the door, each its own colour."""

    def draw(d):
        cx, cy = W / 2, H * 0.06
        for r, colour, wide in (
            (330, MAGENTA, 13),
            (250, GOLD, 11),
            (172, CYAN, 9),
        ):
            d.arc((cx - r, cy - r, cx + r, cy + r), start=8, end=172, fill=colour, width=wide)

    return blurred_layer(draw, blur)


def bokeh(blur=11):
    """Out-of-focus bulbs. Cheap depth, and it breaks up the gradient."""

    def draw(d):
        rng = np.random.default_rng(3)
        palette = [GOLD, MAGENTA, CYAN, GOLD, VIOLET]
        for _ in range(165):
            cx = rng.integers(0, W)
            cy = int(rng.triangular(0, H * 0.25, H))
            r = int(rng.integers(5, 34))
            colour = palette[int(rng.integers(0, len(palette)))]
            k = float(rng.uniform(0.16, 0.58))
            d.ellipse(
                (cx - r, cy - r, cx + r, cy + r),
                fill=tuple(int(c * k) for c in colour),
            )

    return blurred_layer(draw, blur)


def lobby():
    base = canvas(NOIR)
    base = add_glow(base, radial(W / 2, -H * 0.05, W * 0.95, 1.9), INDIGO, 1.30)
    base = add_glow(base, radial(W / 2, -H * 0.05, W * 0.62, 2.4), VIOLET, 0.52)

    base = screen(base, marquee_rays())
    base = screen(base, marquee_arcs())
    base = screen(base, bokeh())

    # Warm spill pooling on the floor under the doorway.
    base = add_glow(base, radial(W / 2, H * 1.02, W * 0.46, 2.2), GOLD, 0.40)
    base = add_glow(base, radial(W * 0.08, H * 0.86, W * 0.34, 2.3), MAGENTA, 0.34)
    base = add_glow(base, radial(W * 0.92, H * 0.80, W * 0.34, 2.3), CYAN, 0.30)

    base = prose_scrim(base)

    base = vignette(base, strength=0.80)
    return grain(base * 0.94)


def main():
    (ASSETS / "backdrops").mkdir(parents=True, exist_ok=True)
    for name, fn in (("combat", combat), ("lobby", lobby)):
        path = ASSETS / "backdrops" / f"{name}.png"
        to_image(fn()).quantize(colors=256, dither=Image.FLOYDSTEINBERG).save(
            path, optimize=True
        )
        # .convert("RGB") matters: a palettised PNG reads back as indices.
        arr = np.asarray(Image.open(path).convert("RGB"), dtype=np.float32) / 255
        lum = 0.2126 * arr[..., 0] + 0.7152 * arr[..., 1] + 0.0722 * arr[..., 2]
        # Combat writes into the outer bands; the overworld centres its prose.
        # Each backdrop is judged where its own text actually lands.
        if name == "combat":
            zone = np.concatenate([lum[40:150].ravel(), lum[H - 150 : H - 40].ravel()])
            label = "outer bands"
        else:
            zone = lum[300:780].ravel()
            label = "centre"
        text_luma = 0.871 if name == "lobby" else 0.78
        p99 = np.percentile(zone, 99)
        print(
            f"{name:7s} {path.stat().st_size / 1024:6.0f} KB   "
            f"{label:11s} p99 {p99:.3f}   "
            f"text contrast {(text_luma + 0.05) / (p99 + 0.05):.2f}:1"
        )


if __name__ == "__main__":
    main()
