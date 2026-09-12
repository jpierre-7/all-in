# Assets

The contract between art (#7) and the combat UI (#11). **Paths and dimensions
in this file are the interface.** Every PNG here is currently a placeholder;
each will be replaced by hand-drawn or generated art at the same path and the
same size, so no Rust code needs to change when the real art lands.

Regenerate the placeholders with `python3 tools/gen_placeholders.py`.

## Files

| Path | Size | Notes |
| --- | --- | --- |
| `cards/frame.png` | 320×448 | Card body. Transparent outside the corner radius. 9-slice. |
| `icons/tell_streak.png` | 128×128 | Tell icon, transparent. Renders at ~24–40px. |
| `icons/tell_all_in.png` | 128×128 | Tell icon, transparent. Renders at ~24–40px. |
| `backdrops/combat.png` | 1920×1080 | Opaque. Sits behind the duel. |
| `backdrops/lobby.png` | 1920×1080 | Opaque. Sits behind Lobby / prose screens. |

## Card frame

Authored at 2x: 7 cards across a ~1280px window is ~160px per card, on the
2.5:3.5 poker ratio.

- **9-slice inset: 48px on all four sides.** The corner radius (28px) plus the
  border stroke (6px) both fit inside that, so corners stay undistorted at any
  node size. Bevy UI draws this with a sliced image mode — confirm the exact
  enum path for 0.19 when wiring it.
- **Safe area: inset 24px from every edge** (a 272×400 rect at 24,24 in
  authored pixels, or 15% of the node's width/height). The body is flat inside
  it so name, Stack, and Tell render legibly on top. Keep text out of the bezel.
- Suggested layout, not binding: name top-left, Stack large and centered,
  Tell icon top-right at ~40×40.

## Palette

Casino-noir per #11 — dark felt green/black, gold text. These seven are the
whole vocabulary. Generated backdrops get graded into them so generated and
hand-drawn art agree; the values live in `tools/gen_placeholders.py`.

| Token | Hex | Use |
| --- | --- | --- |
| felt | `#10392C` | Table felt green |
| felt-deep | `#0A211A` | Shadowed felt, card body |
| noir | `#08080A` | Near-black |
| gold | `#C9A227` | Frame stroke, primary text |
| gold-lit | `#F2DC8B` | Rim light, highlights |
| bone | `#EDE4D0` | High-contrast text on felt |
| blood | `#8E1B23` | Accent: Whiff, red suits |

Contrast on felt-deep: gold 6.97:1, gold-lit 12.35:1, bone 13.34:1 — all clear
WCAG AA for body text.

## Replacing a placeholder

1. Match the path and the pixel dimensions in the table above.
2. Keep transparency where the table says transparent — the frame's corners and
   the icons' backgrounds.
3. Grade backdrops into the palette and keep them dark; gold text sits on top
   of them and untreated art will swallow it.
4. Icons must hold their silhouette at 24px. Check before committing.
5. Keep each file under ~1MB. There is no git-lfs here.
