#!/usr/bin/env python3
"""Build the concept-art deck the judges watch beside the build.

Ten 16:9 slides out to `docs/concept-art.pdf`. Every image is read from
`assets/`, so the deck cannot drift from what actually shipped; the "what it
replaced" slides read superseded versions straight out of git history for the
same reason.

Run: python3 tools/gen_concept_art.py
"""

import subprocess
from io import BytesIO
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "assets"
OUT = ROOT / "docs" / "concept-art.pdf"

W, H = 1920, 1080
NOIR = (9, 9, 12)
FELT_DEEP = (10, 33, 26)
GOLD = (201, 162, 39)
GOLD_LIT = (242, 220, 139)
BONE = (237, 228, 208)
DIM = (140, 135, 122)
BLOOD = (142, 27, 35)

DISPLAY = str(ASSETS / "fonts" / "Limelight-Regular.ttf")
BODY = str(ASSETS / "fonts" / "BarlowCondensed-Regular.ttf")


def font(path, size):
    return ImageFont.truetype(path, size)


def at_commit(rev, path):
    """An earlier version of a file, straight from git. None if it has gone."""
    try:
        blob = subprocess.run(["git", "show", f"{rev}:{path}"], cwd=ROOT,
                              capture_output=True, check=True).stdout
        return Image.open(BytesIO(blob)).convert("RGBA")
    except (subprocess.CalledProcessError, OSError):
        return None


def slide(heading, caption):
    """Common furniture: ground, rule, heading, caption."""
    img = Image.new("RGB", (W, H), NOIR)
    d = ImageDraw.Draw(img)
    d.rectangle((0, 0, W, 150), fill=(13, 14, 18))
    d.line((96, 150, W - 96, 150), fill=GOLD, width=3)
    d.text((96, 62), heading, font=font(DISPLAY, 52), fill=GOLD_LIT)
    d.text((96, H - 112), caption, font=font(BODY, 36), fill=BONE)
    return img, d


def fit(img, box_w, box_h):
    scale = min(box_w / img.width, box_h / img.height)
    return img.resize((max(int(img.width * scale), 1), max(int(img.height * scale), 1)),
                      Image.LANCZOS)


def paste(dst, src, cx, cy, label=None, d=None):
    """Centre `src` on (cx, cy), with an optional label under it."""
    im = src.convert("RGBA")
    dst.paste(im, (int(cx - im.width / 2), int(cy - im.height / 2)), im)
    if label and d:
        w = d.textlength(label, font=font(BODY, 28))
        d.text((cx - w / 2, cy + im.height / 2 + 18), label, font=font(BODY, 28), fill=DIM)


def card_at(size, tell=None, value="8"):
    """A card as `combat::ui` actually draws it.

    Playing-card layout: the value in two opposite corners, and in the middle
    either the Tell's icon or — for a card with no Tell — the value again,
    large. Mirrors `corner_row` and `centre` so the deck cannot show a card the
    game does not.
    """
    frame = Image.open(ASSETS / "cards" / "frame.png").resize(size, Image.LANCZOS)
    card = Image.new("RGBA", size, (10, 20, 15, 255))
    card.alpha_composite(frame)
    d = ImageDraw.Draw(card)
    k = size[0] / 150  # the node is 150 wide in game

    corner = font(BODY, max(int(20 * k), 9))
    d.text((26 * k, 20 * k), value, font=corner, fill=GOLD)
    w = d.textlength(value, font=corner)
    d.text((size[0] - 26 * k - w, size[1] - 20 * k - corner.size * 1.2), value,
           font=corner, fill=GOLD)

    if tell:
        box = int(66 * k)
        icon = Image.open(ASSETS / "tells" / f"{tell}.png").resize((box, box), Image.LANCZOS)
        card.alpha_composite(icon, (int((size[0] - box) / 2), int((size[1] - box) / 2)))
    else:
        big = font(BODY, max(int(44 * k), 14))
        w = d.textlength(value, font=big)
        d.text(((size[0] - w) / 2, (size[1] - big.size * 1.3) / 2), value,
               font=big, fill=GOLD)
    return card


# --- Slides ------------------------------------------------------------------


def s_title():
    img = Image.open(ASSETS / "backdrops" / "title.png").convert("RGB").resize((W, H))
    d = ImageDraw.Draw(img)
    f = font(DISPLAY, 150)
    w = d.textlength("ALL IN", font=f)
    d.text(((W - w) / 2, 360), "ALL IN", font=f, fill=GOLD_LIT)
    f2 = font(BODY, 54)
    w2 = d.textlength("Concept art and design", font=f2)
    d.text(((W - w2) / 2, 550), "Concept art and design", font=f2, fill=BONE)
    icon = fit(Image.open(ASSETS / "icon.png"), 150, 150)
    paste(img, icon, W / 2, 760)
    f3 = font(BODY, 32)
    w3 = d.textlength("HackRice 16  ·  Rust + Bevy  ·  every asset drawn procedurally", font=f3)
    d.text(((W - w3) / 2, 900),
           "HackRice 16  ·  Rust + Bevy  ·  every asset drawn procedurally",
           font=f3, fill=DIM)
    return img


def s_system():
    img, d = slide("The system", "Seven colours and two faces. Everything else follows from them.")
    swatches = [("felt", (16, 57, 44)), ("felt-deep", FELT_DEEP), ("noir", NOIR),
                ("gold", GOLD), ("gold-lit", GOLD_LIT), ("bone", BONE), ("blood", BLOOD)]
    x = 96
    for name, col in swatches:
        d.rounded_rectangle((x, 250, x + 220, 470), radius=12, fill=col,
                            outline=(60, 60, 66), width=2)
        d.text((x, 490), name, font=font(BODY, 30), fill=BONE)
        d.text((x, 524), "#%02X%02X%02X" % col, font=font(BODY, 26), fill=DIM)
        x += 246
    d.text((96, 640), "Limelight — display", font=font(BODY, 30), fill=DIM)
    d.text((96, 682), "ALL IN  ·  THE PIT", font=font(DISPLAY, 68), fill=GOLD_LIT)
    d.text((96, 806), "Barlow Condensed — body, and the game's default face",
           font=font(BODY, 30), fill=DIM)
    d.text((96, 848), "The lobby smells like carpet shampoo and old cigarettes.",
           font=font(BODY, 44), fill=BONE)
    return img


def s_card():
    img, d = slide("The card face",
                   "A guilloche rosette inside a three-part bezel. A card with no Tell shows its value in the middle.")
    paste(img, fit(Image.open(ASSETS / "cards" / "frame.png"), 440, 620), 470, 560,
          "240 x 340 authored", d)
    row_cx = 1310
    hand = [(None, "8"), ("streak", "5"), (None, "3"), ("all_in", "7")]
    for i, (tell, value) in enumerate(hand):
        paste(img, card_at((170, 238), tell, value), row_cx - 285 + i * 190, 545)
    label = "value in both corners; the Tell, or the value again, in the middle"
    w = d.textlength(label, font=font(BODY, 28))
    d.text((row_cx - w / 2, 700), label, font=font(BODY, 28), fill=DIM)
    return img


def s_card_cut():
    img, d = slide("The card face: three tries",
                   "The first was off-ratio; the second notched its own corners against a square border.")
    v1 = at_commit("5035c03", "assets/cards/frame.png")
    v2 = at_commit("157eb8c", "assets/cards/frame.png")
    now = Image.open(ASSETS / "cards" / "frame.png")
    for i, (im, label) in enumerate(((v1, "320 x 448 — wrong ratio"),
                                     (v2, "double rule — reads flat"),
                                     (now, "engine-turned — shipped"))):
        if im is None:
            continue
        paste(img, fit(im, 340, 470), 400 + i * 560, 540, label, d)
    return img


def s_tells():
    img, d = slide("The Tells",
                   "They render at 28 pixels, so the silhouette is the entire design.")
    for i, name in enumerate(("streak", "all_in")):
        ic = Image.open(ASSETS / "tells" / f"{name}.png")
        paste(img, fit(ic, 280, 280), 430 + i * 420, 450,
              "Streak" if i == 0 else "All In", d)
        paste(img, fit(ic, 28, 28), 430 + i * 420, 700, "at 28px", d)
    d.text((1090, 380),
           "All In was a stack of chips first.\nThe gaps between them vanished at\n28px, so it became a card with a\nline through it — which is what the\nTell does anyway: it burns one.",
           font=font(BODY, 34), fill=BONE, spacing=14)
    return img


def s_backdrops():
    img, d = slide("The backdrops",
                   "Colour at the edges, dark where the text lands — each graded for its own screen.")
    paste(img, fit(Image.open(ASSETS / "backdrops" / "combat.png"), 820, 470), 500, 440,
          "combat — text sits in the outer bands", d)
    paste(img, fit(Image.open(ASSETS / "backdrops" / "lobby.png"), 820, 470), 1400, 440,
          "lobby — prose sits in the middle", d)
    d.text((96, 790),
           "The lobby first measured 2.33:1 against its own text because it had been tuned for the",
           font=font(BODY, 32), fill=DIM)
    d.text((96, 832), "combat layout. Same art, different screen, wrong place kept dark.",
           font=font(BODY, 32), fill=DIM)
    return img


def s_bosses():
    img, d = slide("The House and its floor bosses",
                   "Nobody has a face. The House is a pair of hands that never move.")
    for i, (name, label) in enumerate((("slotz", "SLOTZ"), ("pit_boss", "THE PIT BOSS"),
                                       ("the_house", "THE HOUSE"))):
        paste(img, fit(Image.open(ASSETS / "portraits" / f"{name}.png"), 380, 380),
              420 + i * 540, 500, label, d)
    return img


def s_opening_a():
    img, d = slide("The Opening: one table, three moments",
                   "Same lamp, same hands. Only the felt changes — that is the whole story.")
    for i in range(3):
        paste(img, fit(Image.open(ASSETS / "backstory" / f"opening_{i + 1}.png"), 580, 340),
              370 + i * 590, 530,
              ["everything he owned", "the boy beside the chair", "the chair empty"][i], d)
    return img


def s_opening_b():
    img, d = slide("The Opening: the walk back",
                   "Twenty-five years later, and the doors closing behind him.")
    paste(img, fit(Image.open(ASSETS / "backstory" / "opening_4.png"), 880, 500), 510, 530,
          "across the street, under the sign", d)
    paste(img, fit(Image.open(ASSETS / "backstory" / "opening_5.png"), 880, 500), 1410, 530,
          "inside, doors swinging shut", d)
    return img


def s_opening_cut():
    img, d = slide("The Opening: what it replaced",
                   "Drawn to the wrong beats before the narrative split landed, then redrawn to the script.")
    pairs = [("64faa00", "assets/backstory/opening_1.png", "assets/backstory/opening_1.png",
              "first pass: chips leaving", "shipped: chips stacked high"),
             ("64faa00", "assets/backstory/opening_2.png", "assets/backstory/opening_2.png",
              "first pass: a photograph", "shipped: a boy beside the chair")]
    for row, (rev, old_path, new_path, old_label, new_label) in enumerate(pairs):
        old = at_commit(rev, old_path)
        if old is not None:
            paste(img, fit(old, 560, 300), 520, 340 + row * 350, old_label, d)
        paste(img, fit(Image.open(ROOT / new_path), 560, 300), 1300, 340 + row * 350,
              new_label, d)
    return img


def main():
    slides = [s_title(), s_system(), s_card(), s_card_cut(), s_tells(),
              s_backdrops(), s_bosses(), s_opening_a(), s_opening_b(), s_opening_cut()]
    pages = [s.convert("RGB") for s in slides]
    OUT.parent.mkdir(parents=True, exist_ok=True)
    pages[0].save(OUT, "PDF", resolution=150, save_all=True, append_images=pages[1:])
    print(f"{OUT.relative_to(ROOT)}  {len(pages)} slides  "
          f"{OUT.stat().st_size / 1024 / 1024:.1f} MB")


if __name__ == "__main__":
    main()
