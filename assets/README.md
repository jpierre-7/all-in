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
| `backdrops/lobby.png` | 1920×1080 | every prose screen | `screens.rs` — `OverworldArt::lobby` |
| `portraits/slotz.png` | 256×256 | — | **not wired yet** |
| `portraits/pit_boss.png` | 256×256 | — | **not wired yet** |
| `portraits/the_house.png` | 256×256 | — | **not wired yet** |
| `fonts/BarlowCondensed-Regular.ttf` | — | all text | `theme.rs` — the default font |
| `fonts/Limelight-Regular.ttf` | — | screen titles | `theme.rs` — `Fonts::display` |
| `icon.png` | 256×256 | — | **not wired** — see below |
| `backstory/opening_1…5.png` | 1920×1080 | one per Opening frame | **awaiting #38** |
| `backdrops/title.png` | 1920×1080 | title screen | **awaiting #36** |

## Card frame

The card node is a fixed **120×170** and the frame is drawn with a plain
`ImageNode` — no nine-slice, no tiling. The image is simply **stretched to
fill**, so authoring at exactly 2x keeps the scale uniform and the bezel
undistorted. Any other aspect ratio will skew the corners.

- **Safe area: 16px inset** in authored pixels. The node carries 8px of padding,
  leaving a 104×154 content box for name, Stack, and Tell icon.
- **The frame owns the card's edge.** When `art.frame` is `Some` the node sets
  `BorderColor` to `Color::NONE`, because a square 2px border against the art's
  rounded bezel left a gold notch in every corner. Without art the border is the
  only edge there is, so it keeps its old gold/neon behaviour.
- A pending All In sacrifice is shown by **tinting the frame** through
  `ImageNode::color`, not by recolouring a border. The tint is kept light
  (`SACRIFICE_TINT`): multiplying green felt by a saturated red crushes it to
  black.
- The bezel is a double rule with a diamond in each corner. A single heavy rule
  read as a slab at 120×170.

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

## Boss portraits — not wired

Stretch item 4 on the art ticket, drawn to Dev 1's designs: Slotz is a
chrome-and-neon slot machine on a rolling base, the Pit Boss a brass balance
scale with one pan already loaded, The House a pair of hands on the felt.
Transparent ground, and they hold up down to ~90px.

Nothing displays them yet, and wiring them means editing `src/combat/ui.rs`,
which Dev 1 owns. For whoever picks it up: `ActiveDuel` carries `enemy_name`
but **not** `EncounterId`, so the portrait cannot be chosen in `redraw` as it
stands. `start_duel` in `plugin.rs` does have `encounter.id` — either add
`id: encounter.id` to `ActiveDuel` and match in `redraw`, or pick the handle in
`start_duel`. Loading follows the existing `load_art` pattern; the three files
map to `EncounterId::Slotz`, `PitBoss`, and `TheHouse`. Minions have no
portrait.

## Fonts

Two faces, both OFL, licences committed beside them:

- **Barlow Condensed** — body. Condensed like ticket and signage type, and
  legible at the 16–20px the prose screens use.
- **Limelight** — display. Art deco, for screen titles.

They are **embedded with `include_bytes!`**, not loaded from `assets/`, so they
can never go missing and never race the first frame.

`bevy_text` installs FiraMono into `Assets<Font>` at the default asset id, and
every `TextFont` that names no font resolves there. `ThemePlugin` overwrites
that one entry, which re-types the whole game — combat and overworld alike —
without touching a single call site. That is why there is no font threading
through `Screen` or the combat UI.

The display face is opt-in: tag a text entity with `DisplayText` and a system
swaps it over on the next frame, then drops the tag. Screen titles use it. The
combat UI does not yet — the enemy name and the Stack numbers are the obvious
candidates, and the tag is all it takes.

## The app icon is not the window icon

`assets/icon.png` is a pair of dice showing a natural seven. It is for the
README, the itch page and pitch material.

It is **not** set as the window icon, deliberately. Bevy re-exports only
`EventLoopProxy` and the cursor types from winit — not `winit::window::Icon` —
so wiring it would mean adding `winit` as a direct dependency that has to stay
version-locked to whatever Bevy depends on (0.30.13 today). And
`set_window_icon` **does nothing on macOS**: the Dock and title bar read from an
app bundle, not the running process. A new pinned dependency for no effect on
the machine we demo from was not worth it. On Windows and Linux it would work,
if someone wants it later.

The window title *is* set, in `main.rs`: **ALL-IN**.

## The Opening frames

One image per frame of `OPENING` in `docs/narrative.md`. The split into
`OPENING_1`…`OPENING_5` is #37's, and each frame there carries a *Scene* line
written for the artist. Those scene lines are the brief and this art follows
them.

**Frames 1–3 are one staging.** Same table, same lamp, same pair of House hands
on the far side — only what is on the felt changes: chips stacked high, then
gone with a boy beside the chair, then cards turned over and the chair empty.
Holding the camera still is what makes those three beats land, so if one is
redrawn all three should be.

Neither these nor `title.png` are loaded yet: #38 builds the Opening paging and
#36 the title screen. Any frame whose image is missing pages as text only, so
they can land before the code does.

**Everyone is a silhouette.** Geometry cannot draw a face, so nobody has one:
Jack is a back-lit shape in the foreground, the boy is child proportions beside
a chair, and the House is a pair of hands that never move. Casino-noir carries
that happily, and the prose is about what Lucky Jack lost rather than what
anyone looked like.

**Every frame keeps its lower third dark**, because that is where the prose
goes. The subject lives in the upper two thirds. `gen_backstory.py` reports it:

```
opening_1  lower-third p99 0.081  contrast 7.01:1
opening_2  lower-third p99 0.064  contrast 8.06:1
opening_3  lower-third p99 0.054  contrast 8.82:1
opening_4  lower-third p99 0.061  contrast 8.28:1
opening_5  lower-third p99 0.112  contrast 5.68:1
title      lower-third p99 0.206  contrast 3.60:1
```

An earlier pass scrimmed the *middle* and measured 1.04:1, because the chips,
the photograph and the cards are the bright things in these frames and they sat
exactly where the text did. Dimming only the background would not have helped.

These are the heaviest files in the repo, ~1.6MB each. 64-colour quantising
takes them to ~1.1MB but risks banding on long dark gradients, which is a bad
trade on the most cinematic screens in the game.

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
