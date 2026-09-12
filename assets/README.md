# Assets

The contract between art (#7) and the combat UI (#11). **The paths below are not
a convention — they are bound in code.** `load_art` in `src/combat/ui.rs` checks
each one with `.exists()` and falls back to a text label when it is missing, so
a misnamed file fails silently rather than erroring. Match the path exactly.

Regenerate with `python3 tools/gen_placeholders.py` (frame, Tell icons) and
`python3 tools/gen_backdrops.py` (backdrops).

## Files

| Path | Source size | Drawn at | Bound in |
| --- | --- | --- | --- |
| `cards/frame.png` | 240×340 | 120×170 node | `ui.rs` — `Art::frame` |
| `tells/streak.png` | 128×128 | 28×28 node | `ui.rs` — `Art::streak` |
| `tells/all_in.png` | 128×128 | 28×28 node | `ui.rs` — `Art::all_in` |
| `backdrops/combat.png` | 1920×1080 | full screen | `ui.rs` — `Art::backdrop` |
| `backdrops/lobby.png` | 1920×1080 | — | **not wired yet** |

`backdrops/lobby.png` has no slot in the overworld. The file is here and ready;
the overworld screens need an `ImageNode` before it appears.

## Card frame

The card node is a fixed **120×170** and the frame is drawn with a plain
`ImageNode` — no nine-slice, no tiling. The image is simply **stretched to
fill**, so authoring at exactly 2x keeps the scale uniform and the bezel
undistorted. Any other aspect ratio will skew the corners.

- **Safe area: 16px inset** in authored pixels. The node carries 8px of padding,
  leaving a 104×154 content box for name, Stack, and Tell icon.
- The node already draws its own `BackgroundColor(CARD_FACE)` **and a 2px gold
  border**, both underneath the frame image. A drawn bezel will read as a double
  border unless that `BorderColor` is dropped when `art.frame` is `Some`.

## Tell icons

Rendered into a 28×28 node, so **silhouette is the whole job** — check any
redraw at 28px before committing. 128px source leaves room to work.

`all_in.png` is a card with a diagonal strike: the Tell sacrifices a card from
the Draw. A side-on chip stack was the first attempt and it turned to mush at
28px — the gaps between chips disappear.

## Backdrops

Neon-noir: saturated magenta, cyan and gold signage against a dark ground. The
colour sits at the edges and the centre stays dark **on purpose** — the combat
UI lays gold and bone text directly over this image with no scrim.

If you replace these, keep the luminance discipline. `gen_backdrops.py` prints
the check it cares about:

```
combat   overall p99 0.382   text-band p99 0.182
lobby    overall p99 0.331   text-band p99 0.236
```

The **text band** is the outer ~150px top and bottom, where the Stack and House
Edge lines are drawn. Keep its p99 luminance under ~0.25 and gold text stays
legible; the middle can be brighter.

## Palette

| Token | Hex | Use |
| --- | --- | --- |
| felt | `#10392C` | Table felt green |
| felt-deep | `#0A211A` | Shadowed felt, card body |
| noir | `#08080A` | Near-black ground |
| gold | `#C9A227` | Frame stroke, primary text |
| gold-lit | `#F2DC8B` | Rim light, highlights |
| bone | `#EDE4D0` | High-contrast text on felt |
| blood | `#8E1B23` | Accent: Whiff, red suits |

Neon extensions, backdrops only:

| Token | Hex |
| --- | --- |
| magenta | `#FF2E88` |
| cyan | `#26D9E0` |
| gold-neon | `#FFC53D` |
| violet | `#6B2FA8` |
| indigo | `#160A30` |
| teal | `#0E4A44` |

Contrast on felt-deep: gold 6.97:1, gold-lit 12.35:1, bone 13.34:1.

## Replacing a file

1. Match the path and the source size in the table above.
2. Keep transparency where it exists — the frame's corners, the icons' ground.
3. Check icons at 28px.
4. Keep backdrops' text-band luminance low (see above).
5. Backdrops are palettised PNGs to hold the file size down; Bevy's decoder
   expands them on load. Keep each file near or under ~1.5MB — there is no
   git-lfs here.
