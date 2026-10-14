# The whole game is drawn on a 640×360 canvas, scaled by whole numbers only

_Decided in #90, on the research in #92._

The art is moving to a pixel style for the whole presentation: UI, fonts, card frames and backdrops. Everything is laid out and drawn on a **640×360 canvas**, and the window shows that canvas at the largest whole-number scale that fits (2× at 720p, 3× at 1080p, 4× at 1440p, 6× at 4K), centred, with black bars on whatever is left over. A 1366×768 laptop gets 2× with thin bars. The window opens at 1280×720, never goes smaller than that, and has a fullscreen toggle.

This is hard to change later: every asset is drawn at the exact pixel size it appears on the canvas, so a different canvas means redrawing the art.

## What follows from it

- **One pixel size everywhere.** Nothing is scaled, rotated or moved by a fraction of a canvas pixel. A card that appears at two sizes is two drawn frames.
- **Every size in the UI is a whole number of canvas pixels**, including animated offsets, which are rounded before they are applied. The engine only snaps to the screen's pixels, not the canvas's, so this is a rule for code review.
- **Art is authored 1:1** as PNG, with transparency fully on or fully off, at the same paths as today.
- **Text is a pixel font at its native size or a whole multiple of it**, with smoothing off. One face, m6x11plus (free with attribution), at 1× for body and 2× for titles and hit numbers. The "ALL IN" marquee becomes drawn art, not type.
- The canvas is the camera's viewport plus `UiScale`, on the one window camera, with nearest-neighbour sampling.

## Considered Options

- **480×270.** Chunkier, nearer Undertale, and 4× at 1080p. Rejected: only 2× at 720p, and the info panel and card values get cramped. It stays the fallback if the style frames come out wanting it coarser, and it is one constant in code, but all art drawn by then is redrawn.
- **Fill the window at a fractional scale.** No bars. Rejected: uneven pixels and shimmer, which defeats the look.
- **Whole-number scale with backdrops bleeding into the bars.** Rejected: every backdrop has to be overdrawn past the canvas.
- **Render the UI to a 640×360 texture and upscale it.** The usual Bevy recipe, and it snaps everything to the canvas for free. Rejected: `bevy_ui`'s `Interaction` only works for cameras that render to a window, so card hover would need a second, hand-written pointer path, for the same pixels on screen.
- **Painterly backdrops under a pixel UI.** Rejected for a mixed look; backdrops and backstory frames are redrawn at 640×360.
