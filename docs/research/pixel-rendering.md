# Pixel-perfect rendering in Bevy 0.19 for a `bevy_ui`-only game

Research for #92 (part of #86, "Decide the pixel-art direction"). Everything
below was checked against the Bevy 0.19.1 source and examples as shipped in the
crate (`~/.cargo/registry/src/*/bevy*-0.19.1/`; the same files are at
<https://github.com/bevyengine/bevy/tree/v0.19.1>) and, for fonts, the licence
files the foundries publish. Paths like `bevy_ui/src/update.rs:154` are
line numbers in the 0.19.1 crate sources.

## TL;DR

- **Recommended: integer-scaled camera viewport + `UiScale`**, not render-to-
  texture. Pick a virtual canvas, on every resize compute
  `k = floor(min(win_w / VW, win_h / VH))`, set the camera's `Viewport` to a
  centred `VW*k × VH*k` rectangle and `UiScale(k / window_scale_factor)`. The
  whole existing UI keeps working (layout, `Interaction`, picking, `percent`
  roots); letterbox bars come for free from `ClearColor`; every `Val::Px(1)`
  becomes exactly `k` physical pixels; pixel TTF fonts at their native size
  render as exact `k×` replicas with `FontSmoothing::None`.
- **Candidate virtual resolution: 640 × 360** (3× at 1080p, 2× at 720p, 4× at
  1440p, 6× at 4K). 480 × 270 is the chunkier fallback if the art team wants
  the Undertale end of the register.
- **Images**: `ImagePlugin::default_nearest()` is honoured by `ImageNode`
  (the UI pipeline binds `gpu_image.sampler`, which comes from the plugin
  default). Frames/icons/backdrops need to be re-authored at virtual-pixel size
  and drawn in nodes of exactly that size.
- **Fonts**: TTF pixel fonts (bitmap-style outlines), never `.fnt` bitmaps
  (Bevy has no loader). Top pick **m6x11 / m6x11plus** (Balatro's actual font,
  16 px, "free to use with attribution"); strictly-OFL/CC0 top pick **Pixel
  Operator** (CC0, 16 px, same designer as Undertale's 8-Bit Operator).
- **`ThemePlugin`**: the default-font-id override still works in 0.19.1 and is
  the right lever. What changes: default `TextFont` needs `font_smoothing:
  FontSmoothing::None` (there is no global switch; it has to be on every
  `TextFont`), font sizes must be quantised to the face's native size (8/16/32,
  not 14/18/30), and the camera needs `UiAntiAlias::Off` + `Msaa::Off`.

## 1. Rendering to a fixed low-res canvas and integer-scaling it

Three candidate mechanisms were evaluated against how `bevy_ui` 0.19.1
actually computes layout and picks input.

### How `bevy_ui` sizes itself (the fact everything hinges on)

The UI root's layout size and scale come from the **camera's viewport**, not
the window:

```rust
// bevy_ui/src/update.rs:154-163
let (scale_factor, physical_size) = camera_query.get(camera).ok().map(|camera| (
    camera.target_scaling_factor().unwrap_or(1.) * ui_scale.0,
    camera.physical_viewport_size().unwrap_or(UVec2::ZERO),
))
```

So `scale_factor = window_scale × UiScale`, and `percent(100)` means the
camera's `Viewport`, not the window. Layout runs in physical pixels and, by
default, rounds every node to the physical grid (`LayoutConfig::use_rounding`,
"Defaults to true", `bevy_ui/src/ui_node.rs:2911-2916`). `UiScale` is
documented as "A multiplier to fixed-sized ui values … will only affect fixed
ui values like `Val::Px`" (`bevy_ui/src/lib.rs:120-126`).

### Option A — render-to-texture canvas (`pixel_grid_snap` / `render_ui_to_texture`)

The canonical Bevy recipe, `examples/2d/pixel_grid_snap.rs`, renders a 160×90
`Image` (with `TextureUsages::RENDER_ATTACHMENT`) from an inner `Camera2d`
with `RenderTarget::Image`, then shows that image as a `Sprite` through an
outer camera whose orthographic `projection.scale = 1. / h_scale.min(v_scale).round()`
(integer only). `ImagePlugin::default_nearest()` keeps the upscale crisp.
`examples/ui/render_ui_to_texture.rs` proves `bevy_ui` can be the thing
rendered to the image: a root node with `UiTargetCamera(texture_camera)`
lays out against the image (`ImageRenderTarget.scale_factor` "should almost
always be 1.0", `bevy_camera/src/camera.rs:985-991`; `bevy_render/src/camera.rs:286-290`).

For a UI-only game the outer side can also be UI: a second window camera
marked `IsDefaultUiCamera` (`bevy_ui/src/ui_node.rs:2980`) with a root that
centres one `ImageNode` of the canvas at `px(VW*k) × px(VH*k)` on a black
background. That gives letterboxing with no projection maths.

**What works**: everything snaps to the virtual grid automatically, including
animated `Val::Px` offsets (`hits.rs` rises), because the canvas *is* the grid.
Text is rasterised once at 1× and upscaled with the canvas.

**What breaks — input**. `Interaction` is only ever updated for window cameras:

```rust
// bevy_ui/src/focus.rs:191-197
// Interactions are only supported for cameras rendering to a window.
let Some(NormalizedRenderTarget::Window(window_ref)) = render_target.normalize(primary_window)
else { return None; };
```

`src/combat/ui.rs:378` drives card hover off `Changed<Interaction>`, so that
system goes dead. The picking backend (`bevy_ui/src/picking_backend.rs:122-131`)
matches pointers to cameras by render target, so it *can* work, but only with
a synthetic pointer: `render_ui_to_texture.rs` has to write its own
`drive_diegetic_pointer` system that emits `PointerInput` with
`Location { target: <the image> }`. For us that is a small system (window
cursor → subtract letterbox offset → divide by `k`) plus porting hover to
`On<Pointer<Over>>` observers. Doable, but it is a second input path to keep
correct.

**Other trade-offs**: two cameras, `RenderLayers` discipline, canvas
re-allocation on resolution change, and text is limited to whatever fits at
1× (no "sharper text at higher k" escape hatch — which is also the point).

### Option B — integer camera `Viewport` + `UiScale` (recommended)

Because the UI lays out against `camera.physical_viewport_size()` and scales
by `UiScale`, the canvas can be *virtual*: one window camera, on
`WindowResized`

1. `k = floor(min(win_phys_w / VW, win_phys_h / VH)).max(1)`;
2. `camera.viewport = Some(Viewport { physical_position: centred, physical_size: UVec2::new(VW*k, VH*k), .. })`
   (`bevy_camera/src/camera.rs:62-71` — position and size are physical pixels);
3. `UiScale(k as f32 / window.resolution.scale_factor())`, so the UI's
   `scale_factor` is exactly `k` regardless of HiDPI.

Then every `px(n)` is `n*k` physical pixels, every `percent` is relative to the
virtual canvas, and node rounding lands on the physical grid. The area outside
the viewport is the `ClearColor`: the attachment is cleared with
`LoadOp::Clear` on first use for the whole texture, not the viewport
(`bevy_render/src/texture/texture_attachment.rs:41-56`), so the letterbox is
black without extra nodes.

**Input just works**: `ui_focus_system` subtracts `physical_viewport_rect().min`
(`focus.rs:200-211`) and the picking backend does the same
(`picking_backend.rs:133-140`). `Interaction`, `Button`, `RelativeCursorPosition`
all keep working unchanged.

**Text**: rasterised at `font_size × k` physical pixels. `TextFont::font_size`
"is multiplied by the window scale factor and `UiScale` … rounded to the
nearest pixel" (`bevy_text/src/text.rs:385-393`). With a pixel TTF whose
outlines sit on a 1/N-em grid, rendering at `N*k` px with hinting off (Bevy
disables hinting when `FontSmoothing::None`, `bevy_text/src/pipeline.rs:363-372`)
puts every edge on the physical grid — an exact `k×` replica, identical to
Option A's upscaled result but with no second camera.

**The one discipline it needs**: `Val::Px` values are not snapped to the
*virtual* grid, only to the physical one. `px(0.5)` at `k=3` is 1.5 → 2
physical px, i.e. a half-virtual-pixel step. Animated offsets
(`src/combat/hits.rs:132`, `margin: UiRect::top(px(-RISE * t))`) must be
`.round()`ed in virtual units, and every design constant must be an integer.
That is a code-review rule, not an engine feature.

### Option C — camera projection scaling

`OrthographicProjection`/`ScalingMode` do not touch `bevy_ui` at all: the UI
target size is the viewport and the scale is `UiScale` (above); `FontSize`
docs say it is "not [multiplied by] the text entity's transform or camera
projection" (`bevy_text/src/text.rs:388`). Projection scaling is only relevant
for sprites/meshes, which this game has none of. Rejected.

### Trade-off table

| | A: render-to-texture | B: viewport + `UiScale` |
|---|---|---|
| Works with `bevy_ui` in 0.19.1 | Yes (`render_ui_to_texture` example) | Yes (`update.rs:154`) |
| `Interaction` / `Button` | **No** for image targets (`focus.rs:191`) | Yes, viewport-aware |
| Picking observers | Only with a hand-written `PointerInput` driver | Yes, viewport-aware |
| Letterboxing | Centre the canvas node/sprite yourself | Free (`ClearColor` outside viewport) |
| Sub-virtual-pixel motion | Impossible (snapped by construction) | Must round `Val::Px` manually |
| Text crispness | Exact 1× then nearest upscale | Exact `k×` raster at native size; same pixels |
| HiDPI | Ignore window scale entirely | Fold it into `UiScale` |
| Cameras | 2 + `RenderLayers` | 1 |

## 2. Nearest-neighbour sampling for UI images

`ImagePlugin::default_nearest()` sets `default_sampler: ImageSamplerDescriptor::nearest()`
(min/mag/mipmap all `Nearest`; `bevy_image/src/image.rs:191-204, 893-900`).
`RenderPlugin` copies it into the render world as `DefaultImageSamplerDescriptor`
(`bevy_render/src/texture/mod.rs:61-66`) and every `Image` whose sampler is
`ImageSampler::Default` (the default for loaded PNGs) gets that sampler when
uploaded (`bevy_render/src/texture/gpu_image.rs:165-168`).

The UI pipeline binds the image's own sampler — `BindGroupEntries::sequential((&gpu_image.texture_view, &gpu_image.sampler))`
(`bevy_ui_render/src/lib.rs:1655-1664`) with a `SamplerBindingType::Filtering`
slot (`pipeline.rs:36`) and a plain `textureSample` in `ui.wgsl:217`. So
**yes, `ImageNode` respects `default_nearest()`**; no per-image work needed.
Per-image override is `image.sampler = ImageSampler::nearest()` on the asset if
a specific texture should stay linear (e.g. a full-bleed painted backdrop).

Consequences for `assets/`:

- Nearest sampling only looks right at integer scale. `frame.png` is 240×340 in
  a 150×210 node (`src/combat/ui.rs:271-273`), i.e. 0.625× — that will shimmer
  under nearest. Art must be authored at the virtual size it is drawn at
  (e.g. a 60×85 or 80×113 frame for a 640×360 canvas) and the node sized to
  match exactly; `NodeImageMode::Stretch` then becomes a no-op rather than a
  resample.
- Tell icons at 128×128 drawn at 72 (and 28) px have the same problem; re-author
  at 24/16 px, or whatever the virtual layout gives them.
- 1920×1080 backdrops through `Stretch` into a 640×360 root will be
  downsampled 3:1 with nearest — decimation, visibly wrong. Backdrops need
  redrawing at canvas size (or, if the team wants painterly backgrounds under
  pixel UI, keep them linear per-image as above — but that is a mixed look,
  not the Undertale/Balatro look).
- `UiAntiAlias::Off` on the camera turns off the UI shader's edge AA for
  borders/rounded corners; `Msaa::Off` as well (`bevy_ui_render/src/lib.rs:153-167`;
  `FontSmoothing` docs say to combine all three, `bevy_text/src/text.rs:1183-1190`).

## 3. Pixel fonts in Bevy text

### Bitmap vs TTF

- `bevy_text`'s loader accepts only `["ttf", "otf"]` (`bevy_text/src/font_loader.rs:38-40`).
  There is no BMFont/`.fnt`/spritesheet text path; monogram's PNG+JSON
  spritesheet, for example, cannot be used as a *font*. All candidates below
  are TTF/OTF "pixel fonts": outlines drawn on a pixel grid so they raster
  cleanly at multiples of one size.
- The rasteriser is `swash` (0.19 moved layout to parley:
  `_release-content/migration-guides/bevy_text_now_uses_parley.md`). Its
  render sources are `ColorOutline`, `ColorBitmap(BestFit)`, `Outline`
  (`bevy_text/src/font_atlas.rs:190-197`), so a font with embedded bitmap
  strikes (sbix/CBDT) would be used at best-fit size — but none of the
  shortlisted fonts rely on that; they are outline fonts.

### Rendering without blur

`TextFont::font_smoothing = FontSmoothing::None` does three things in 0.19.1:

1. thresholds the swash alpha mask to 0/255 (`font_atlas.rs:218-223`);
2. gives the glyph atlas a nearest sampler (`font_atlas.rs:55-57`);
3. floors glyph positions to whole pixels (`pipeline.rs:387-393`) and disables
   hinting for that run (`pipeline.rs:363-372`).

The doc comment is explicit that this "may require specially-crafted pixel
fonts to look good, especially at small sizes" (`text.rs:1183-1190`) — a
normal vector face like Barlow Condensed will look ragged, not retro. A pixel
TTF at exactly its native size (or an integer multiple) with `None` gives
exact pixel replication; at any other size it is the thresholded mess. There
is no global "all text unsmoothed" resource; it is a field on each `TextFont`
(`text.rs:404`, default `AntiAliased`, `text.rs:472`).

Rule for this codebase: **every `TextFont` size must be `native × integer`**
for the face in use (8, 16, 24, 32… for an 8-px face; 16, 32, 48 for a 16-px
face), and `font_smoothing: None`.

### Shortlist (Undertale / Balatro register)

Licence claims below are from each foundry's own page or the Google Fonts
repo `METADATA.pb` (`https://github.com/google/fonts/tree/main/ofl/<name>`).

| Font | Designer | Licence | Native size | Feel / why | Source |
|---|---|---|---|---|---|
| **m6x11 / m6x11plus** | Daniel Linssen | "free to use with attribution" (not OFL/CC0 — credit line required) | 16 px (plus: 18 px) | **Balatro's main font** ([localthunk](https://x.com/LocalThunk/status/1739882509826248882), [Fonts In Use](https://fontsinuse.com/uses/65816/balatro-computer-game)). Chunky proportional, reads at small sizes. | <https://managore.itch.io/m6x11> |
| **Pixel Operator** (+ Mono, Bold, SC, HB, 8) | Jayvee Enaguas | CC0 1.0 | 16 px (8-px variants) | Same designer as Undertale's 8-Bit Operator ([Fonts In Use](https://fontsinuse.com/uses/51307/undertale-dialogue-and-interfaces)); closest *free-and-clear* Undertale dialogue look. Full family incl. mono for the card numbers. | <https://www.dafont.com/pixel-operator.font> |
| **Press Start 2P** | CodeMan38 | OFL | 8 px ("multiples of 8") | Namco-arcade caps; heavy, all-caps-ish. Good for the marquee / titles, tiring as body. | <https://github.com/google/fonts/tree/main/ofl/pressstart2p> |
| **Public Pixel** | GGBotNet | CC0 1.0 | 8×8 mono (8/16/32/64) | Monospace, 1,324 glyphs; ideal for numbers/stack readouts that must not jitter. | <https://ggbot.itch.io/public-pixel-font> |
| **Pixeloid** (Sans, Sans Bold, Mono) | GGBotNet | OFL 1.1 | 9 px (9/18/36/72) | Softer, more modern pixel sans with a real bold — Balatro-ish UI weight. | <https://ggbot.itch.io/pixeloid-font> |
| **monogram** | datagoblin | CC0 1.0 | 5-px-wide mono | Tiny, very legible; secondary labels / debug / info panel lines. | <https://datagoblin.itch.io/monogram> |
| **Departure Mono** | Helena Zhang | OFL | 11 px ("increments of 11px") | Terminal/pixel mono with taste; readable info screens. | <https://github.com/rektdeckard/departure-mono> |
| **Silkscreen** | Jason Kottke | OFL | small (designed for tiny sizes) | 1999 web classic, very "8-bit HUD"; two weights. | <https://github.com/google/fonts/tree/main/ofl/silkscreen> |
| Also OFL on Google Fonts: **VT323** (Peter Hull, terminal look), **Pixelify Sans** (Stefie Justprince, variable wght 400–700), **Jersey 10** (Sarah Cadigan-Fried, 10-px caps), **Tiny5** (Stefan Schmidt, 5-px grid, "increments of 8 px"). | | | | | |

Undertale's actual fonts (8-Bit Operator, 8-Bit Wonder, Trouble Beneath The
Dome, Hachicro, modified Comic Sans/Papyrus) are Dafont freeware or Microsoft
system fonts — not OFL/CC0 — which is why Pixel Operator (the designer's CC0
successor) is the safe stand-in.

**Top pick**: m6x11plus for body + Press Start 2P (or m6x11 at 32) for
display, *if* the team is happy to put "font: m6x11 by Daniel Linssen" in the
credits. If the licence must be OFL/CC0 outright: Pixel Operator body + Pixel
Operator Mono for card values + Press Start 2P for the marquee.

## 4. What changes in `ThemePlugin`

The override trick — `fonts.insert(AssetId::default(), Font::from_bytes(BODY.to_vec()))`
in `Plugin::build` — still works in 0.19.1: `TextPlugin` installs FiraMono at
`AssetId::default()` the same way (`bevy_text/src/lib.rs:141-147`), and
`load_font_assets_into_font_collection` registers whatever is at each asset id
with parley on the first frame (`bevy_text/src/font.rs:51-98`), which is after
`build`. Keep it. What has to change:

1. **Swap the bytes**: `BODY` → the pixel body face (m6x11plus or Pixel
   Operator), `DISPLAY` → the display pixel face (Press Start 2P), plus the
   licence text files in `assets/fonts/` (OFL requires shipping the licence;
   m6x11 needs a credit line — Dafont/itch licences are per-author, so keep a
   `LICENSE-<font>.txt` next to each).
2. **Smoothing is per-`TextFont`, not per-font asset.** There is no resource
   to flip. Options, in order of preference:
   - add a `Text`-level system in `ThemePlugin` (like `apply_display_font`) that
     runs on `Added<TextFont>` and sets `font_smoothing = FontSmoothing::None`
     and snaps `font_size` to the face's native multiple; or
   - expose a `theme::text(size_step) -> TextFont` constructor and migrate the
     10 call sites (`TextFont::from_font_size(...)` in `combat/ui.rs`,
     `combat/info.rs`, `combat/hits.rs`, `overworld/screens.rs`) to it.
   The `Added` system is the one that "re-types the entire game without
   touching a call site", matching the plugin's existing doc comment.
3. **Quantise sizes.** Current sizes are 14, 16, 18, 20, 30, 38, 56, 128
   logical px on a ~1920-wide layout. On a 640×360 canvas with a 16-px body
   face the scale is 16 (body/prompts), 32 (titles, damage numbers), 48/64
   (marquee); 8-px faces cover captions. A `FontSize::Px(f32)` that is not a
   native multiple must be rejected (debug assert) rather than silently
   thresholded.
4. **Camera**: `spawn_camera` (`src/overworld/mod.rs:72`) grows
   `UiAntiAlias::Off` and `Msaa::Off`, the viewport/`UiScale` fitting system,
   and `main.rs` sets `DefaultPlugins.set(ImagePlugin::default_nearest())`.
   (`ThemePlugin` is the natural owner of the fit system and the `UiScale`
   resource; the camera stays in overworld.)
5. **`DisplayText`** keeps working as is (`font.font = handle.into()` is the
   0.19 `FontSource::Handle` form, already what the code does).

## 5. Recommendation

**Option B — integer-scaled camera viewport + `UiScale`, virtual canvas
640 × 360, `ImagePlugin::default_nearest()`, `FontSmoothing::None` everywhere,
m6x11plus (or Pixel Operator) at 16 px as the body face.**

Why B over A: the game is 100 % `bevy_ui` with mouse hover on cards; A kills
`Interaction` for image targets and needs a hand-rolled pointer, two cameras
and render layers, for a result that is pixel-identical to B for integer
scales. B is one system (~30 lines: resize → `k`, viewport, `UiScale`) and
zero changes to any screen. The cost — rounding animated `Val::Px` to whole
virtual pixels — is a lint-able convention.

Why 640 × 360:

- Integer to every common window: 1280×720 (2×), 1920×1080 (3×), 2560×1440
  (4×), 3840×2160 (6×). A 1366×768 laptop gets 2× with thin bars. 480×270 gives
  4× at 1080p but only 2× at 720p with heavy bars, and 720 × 405 has no
  integer fit at 1080p.
- Room for the layout as designed: the combat screen is a 3-row table with
  five ~60×85 cards plus corner readouts; at 640×360 that is 16-px body text
  with 2–3 lines of prompt, an info panel of ~18 lines at 16 px, and a
  Press-Start-2P marquee at 48 px. At 480×270 the info panel and card values
  get cramped and the 8-px faces have to carry more.
- It is where Balatro's chunkiness sits visually (large, readable pixel type on
  a felt table) while still chunky enough that nearest-scaled art reads as
  pixel art; Undertale's 640×480 is the same class, in 4:3.

Fallback if the art direction wants coarser: 480 × 270 with the same code and
one constant changed. Nothing in Option B depends on the number.

### Open follow-ups (not researched here)

- A `WindowResolution::with_scale_factor_override(1.0)` check on a HiDPI
  laptop: B folds the window scale into `UiScale`, so it should not matter,
  but it wants one manual test.
- Whether the 1920×1080 painted backdrops become pixel art (redraw) or stay
  painterly with a per-image linear sampler under pixel UI (mixed look) is an
  art-direction call for #86, not an engine constraint.
