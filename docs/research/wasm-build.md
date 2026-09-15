# Research: a Bevy 0.19 web (WASM) build for All In

Resolves #91 (part of #86). Researched 2026-09-14 against Bevy `v0.19.1`
sources, the Bevy CLI docs, the official Bevy 2D template, cpal/rodio
sources, the Web Audio spec, Chrome's autoplay policy, and itch.io's HTML5
docs. Everything cited is a primary source; the two crate-level findings
come from reading this repo.

## Verdict

**Feasible, and cheap.** Bevy 0.19.1 with default features compiles for
`wasm32-unknown-unknown` with no extra dependencies for this crate. The
tooling is a solved path (`bevy build web --bundle`, or
`cargo build --target wasm32-unknown-unknown` + `wasm-bindgen`), the itch.io
upload is a zip with `index.html` at the root, and the game's own assets
(15 MB) plus a ~8 MB gzipped wasm are far under itch's limits.

**Effort: a weekend, not a project.** Roughly one evening to get a build
running locally, one more to fix the one code-level blocker, tune the
`index.html`, and upload. The whole thing is a few hours of tooling plus one
small refactor.

**Biggest blocker is in our code, not Bevy:** `combat::ui::load_art` and
`overworld::screens::load_overworld_art` gate every image behind
`Path::new("assets").join(..).exists()` and `std::fs::read_dir`. Those are
`std::fs` calls; on `wasm32-unknown-unknown` they always fail, so a web build
would render bare felt with no backdrops, portraits, or card frame. Music is
unaffected (`music.rs` deliberately does not probe the disk).

## 1. Build steps on `wasm32-unknown-unknown`

### What Bevy 0.19.1 needs

- Bevy's own instructions (`examples/README.md`, "Wasm" section, at
  `v0.19.1`):

  ```sh
  rustup target add wasm32-unknown-unknown
  cargo install wasm-bindgen-cli
  cargo build --release --target wasm32-unknown-unknown
  wasm-bindgen --out-name all-in --out-dir web/target --target web \
    target/wasm32-unknown-unknown/release/all-in.wasm
  ```

  Then serve the directory holding `index.html`, `target/`, and `assets/`
  over HTTP (e.g. `python3 -m http.server`). Bevy's reference `index.html`
  is 20 lines: a `<script type="module">` that does
  `import init from './target/wasm_example.js'; init()`.
  Source: https://github.com/bevyengine/bevy/blob/v0.19.1/examples/README.md#wasm
  and https://github.com/bevyengine/bevy/blob/v0.19.1/examples/wasm/index.html

- **No `web` feature flag is needed.** `bevy_asset` and `bevy_audio` each
  declare `[target.'cfg(target_arch = "wasm32")'.dependencies]` that force
  `bevy_app/web` and `bevy_reflect/web` on, so plain `bevy = "0.19.1"`
  gets the browser glue automatically.
  Source: https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_asset/Cargo.toml
  (lines 78–94), https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_audio/Cargo.toml
  (lines 30–41).

- **No `getrandom` backend flag is needed for this crate.** Bevy's build
  tool sets `RUSTFLAGS='--cfg getrandom_backend="wasm_js"'` and the Bevy
  CLI docs say the same, because `getrandom` refuses to pick a backend on
  wasm. But `cargo tree --target wasm32-unknown-unknown -i getrandom`
  against this repo's `Cargo.lock` prints "nothing to print": the only
  `getrandom` in our tree is pulled by `winit → ahash` on native. If a
  future dependency (e.g. `rand`) drags it in, add the flag and the
  `getrandom = { version = "0.3", features = ["wasm_js"] }` target dep the
  template uses.
  Sources: https://github.com/bevyengine/bevy/blob/v0.19.1/tools/build-wasm-example/src/main.rs
  (line 93), https://github.com/TheBevyFlock/bevy_cli/blob/main/docs/src/cli/web/getrandom.md,
  https://github.com/TheBevyFlock/bevy_new_2d/blob/main/Cargo.toml (lines 19–20).

- **Size profile.** Bevy's workspace defines
  `[profile.wasm-release] inherits = "release", opt-level = "z", lto = "fat",
  codegen-units = 1`, and recommends `wasm-opt -Oz` on the `_bg.wasm`. Their
  table (a small 3D example) goes from 13 MB (default) to 4.8 MB
  ("z" + fat LTO + cgu=1 + wasm-opt).
  Source: https://github.com/bevyengine/bevy/blob/v0.19.1/Cargo.toml (lines 5169–5173)
  and the README "Optimizing" section.

### wasm-bindgen vs trunk vs Bevy CLI

Three ways to run the same two steps (compile, then `wasm-bindgen`):

| Route | What it is | Fit for All In |
|---|---|---|
| **Plain `cargo` + `wasm-bindgen-cli`** | The commands above; you write `index.html` and copy `assets/` yourself. | Fine; ~15 lines of shell in a `just`/`make` target. |
| **Bevy CLI** (`bevy build --release web --bundle`) | Unofficial but Bevy-org tool. Compiles, runs `wasm-bindgen` and `wasm-opt`, injects the `getrandom` flag if needed, provides a default `index.html` with a loading spinner and the audio-resume shim, and writes a deployable folder to `target/bevy_web/web-release/<crate>/` with `index.html`, `build/<crate>.js`, `build/<crate>_bg.wasm`, and `assets/`. Override the page by adding `web/index.html`. Uses a `web-release` profile you can define in `Cargo.toml`. Install: `cargo install --git https://github.com/TheBevyFlock/bevy_cli --tag cli-v0.1.0-alpha.2 --locked bevy_cli`. | **Recommended.** It is what the official `bevy_new_2d` template's release workflow uses, and its default page already handles the audio gesture (see §2). |
| **trunk** | Generic Rust-wasm bundler driven by `<link data-trunk rel="rust">` in `index.html`; `rel="copy-dir" href="assets"` copies the asset tree; `data-wasm-opt="z"` runs wasm-opt. Defaults to `--target no-modules` bindgen and `public_url = "/"`. | Works, but itch requires relative paths, so you must build with `--public-url ./`. More knobs than we need. |

Sources: https://github.com/TheBevyFlock/bevy_cli/blob/main/docs/src/cli/web.md,
https://github.com/TheBevyFlock/bevy_new_2d/blob/main/.github/workflows/release.yaml
(web job: `bevy build --locked --release --yes web --bundle`, then `butler push`),
https://github.com/trunk-rs/trunk/blob/main/guide/src/assets/index.md,
https://github.com/trunk-rs/trunk/blob/main/guide/src/configuration/index.md
(`public_url = "/"` default).

### Shape of the itch.io upload

From itch.io's HTML5 docs (https://itch.io/docs/creators/html5):

- Upload a **zip whose root contains `index.html`**; tick "This file will
  be played in the browser". The zip may contain subfolders (`build/`,
  `assets/`).
- Limits: extracted content ≤ 500 MB, any single file ≤ 200 MB, ≤ 1,000
  files. All In's bundle is ~45 MB / ~25 files.
- "You must use relative paths to access other files"; the file host is
  case-sensitive; **missing files return 403, not 404**. Bevy 0.19.1's
  `HttpWasmAssetReader` already maps `403 | 404` to `NotFound` with a
  comment naming itch.io's CDN as the reason.
  Source: https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_asset/src/io/wasm.rs
  (lines 113–116).
- The CDN gzips `.wasm` (and html/js/css) automatically; PNG and Ogg are
  served as-is.
- Embed options: "Embed in page" with a fixed width/height, or "Click to
  launch in fullscreen". "Click to Play" is on by default. Mobile always
  gets the fullscreen-launch mode.
- **Viewport:** Bevy's default `WindowResolution` is **1280×720**
  (`bevy_window/src/window.rs` lines 910–914), and the game sets no
  resolution, so set the itch embed to 1280×720. That is also what the
  official template's itch page uses (`data-width="1280" data-height="720"`
  on https://the-bevy-flock.itch.io/bevy-new-2d). Optionally set
  `Window { fit_canvas_to_parent: true, .. }` so the canvas follows the
  itch frame in fullscreen mode (field is web-only, no effect natively).

The zip layout the Bevy CLI produces, and the one to upload:

```
all-in.zip
├── index.html
├── build/
│   ├── all-in.js
│   └── all-in_bg.wasm
└── assets/
    ├── backdrops/  backstory/  cards/  portraits/  tells/
    └── music/deadly_roulette.ogg
```

## 2. Audio in the browser

- **Stack:** `bevy_audio` → `rodio` (with the `wasm-bindgen` feature on
  wasm32) → `cpal`'s WebAudio host. `vorbis` decoding is via `lewton`, pure
  Rust, so the Ogg loop decodes in the browser exactly as it does natively.
  Source: https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_audio/Cargo.toml.

- **When the `AudioContext` is created:** at `AudioPlugin` build, before
  any frame runs, because `AudioOutput::default()` calls
  `DeviceSinkBuilder::open_default_sink()` eagerly.
  Source: https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_audio/src/audio_output.rs
  (lines 20–30). cpal then calls `ctx.resume()` once in `play()` and drives
  playback with chained `AudioBufferSourceNode.start()` calls on
  `setTimeout`.
  Source: https://github.com/RustAudio/cpal/blob/v0.17.3/src/host/webaudio/mod.rs
  (lines 239, 428–460).

- **The rule:** the Web Audio spec lets a user agent create the context
  `suspended` and only allow it to run once the document has *sticky
  activation*; "authors are encouraged to create or resume the
  `AudioContext` in response to user gesture or interaction"
  (https://webaudio.github.io/web-audio-api/). Chrome (since 71): "If an
  AudioContext is created before the document receives a user gesture, it
  will be created in the 'suspended' state, and you will need to call
  resume() after the user gesture," and it **auto-resumes** when "the user
  has interacted with a page" and "the start() method of a source node is
  called" (https://developer.chrome.com/blog/web-audio-autoplay/,
  https://developer.chrome.com/blog/autoplay). Firefox applies the same
  block to Web Audio by default (`media.autoplay.block-webaudio = true`,
  https://developer.mozilla.org/en-US/docs/Web/Media/Guides/Autoplay).
  Bevy itself documents the constraint and points at the Chrome shim:
  "In browsers, audio is not authorized to start without being triggered by
  an user interaction" (examples README, "Audio in the browsers").

- **How Bevy games handle it:** the Bevy CLI's default `index.html` (and
  therefore the official template) installs the Chrome-blog shim before the
  game loads: a `Proxy` around `self.AudioContext` records every context,
  and listeners on `click/mousedown/pointerup/touchend/keydown/keyup/...`
  call `resume()` on each until they are all `running`. Bevy needs no Rust
  changes for this. Verified in the live deployed page:
  https://html-classic.itch.zone/html/11543195-1860706/bevy-new-2d/bevy_new_2d/index.html.

- **What that means for All In:** the music is spawned at `Startup`
  (`music.rs`), so the context starts suspended. The player's first key
  press on the Title screen (the game already waits for "any key") is the
  gesture; with the shim, the loop starts on that key in every browser, and
  in Chrome it would start on the next buffer even without the shim. On
  itch specifically, the "Click to Play" button is a gesture on the parent
  document and itch's iframe carries `allow="autoplay; ..."`, which Chrome
  documents as the top frame delegating autoplay to the iframe — so the
  loop may well start before the first key. Either way there is nothing to
  design around; the current "loop from Startup, M to mute" model survives.
  Playing silent until the first gesture is standard for browser games.

- **Is a ~2 MB Ogg loop fine?** Yes. `assets/music/deadly_roulette.ogg` is
  1.9 MB; it is one HTTP fetch and rodio decodes it as a stream. The one
  Bevy caveat is "everything is single threaded, this can lead to
  stuttering when playing audio in browsers" (examples README): heavy
  frames can starve the WebAudio buffer callback. This game is UI-only
  with a handful of sprites, so that risk is low; test on a mid-range laptop.

## 3. Asset loading on wasm

- **Embedded fonts (`theme.rs`, `include_bytes!`)**: part of the `.wasm`,
  available before the first frame, nothing to do. They add 236 KB to the
  binary.

- **`assets/` fetches:** on wasm32 Bevy's default `AssetReader` is
  `HttpWasmAssetReader::new("assets")`, which does `window.fetch()` of
  `assets/<path>` relative to the page URL and reads the whole response
  into memory. `read_directory` and `is_directory` are unsupported (they
  log an error and return empty).
  Source: https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_asset/src/io/source.rs
  (lines 479–482), `io/wasm.rs` (lines 121–145).

- **`.meta` round-trips:** `AssetMetaCheck::Always` is the default, so each
  asset load first fetches `<path>.meta`, which on itch is a 403 that Bevy
  swallows as NotFound. Setting
  `AssetPlugin { meta_check: AssetMetaCheck::Never, ..default() }` on
  `DefaultPlugins` removes one wasted request per asset (~12 here).
  Source: https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_asset/src/lib.rs
  (lines 316–322).

- **Total size and load time.** Measured in this repo (`du`):

  | Item | Size |
  |---|---|
  | `assets/` on disk (all) | 15 MB |
  | of which PNG backdrops/backstory (8 files) | ~12.8 MB |
  | `music/deadly_roulette.ogg` | 1.9 MB |
  | portraits, frame, tells, icon | ~0.2 MB |
  | fonts (embedded, not fetched) | 0.24 MB |

  The wasm itself: the official Bevy 0.19 2D template (features
  `2d, ui, audio`, `web-release` profile with `opt-level = "s"`, wasm-opt)
  deploys on itch as **27.2 MB raw, 7.7 MB gzip-on-the-wire** (measured with
  `curl` against the itch CDN above). All In uses default features
  (`2d, 3d, ui, audio`) so expect ~30 MB raw / ~9 MB wire; switching to
  `bevy = { version = "0.19.1", default-features = false, features = ["2d", "ui", "audio"] }`
  like the template would trim it. Ballpark first-load: ~24 MB over the
  wire → ~4 s at 50 Mbps, ~20 s at 10 Mbps, plus a few seconds of wasm
  compile. Repeat visits are cached by the browser. If that matters, the
  1.6 MB PNGs are the lever (they are 8 of the 15 MB; re-encoding or
  downscaling them is an art-side call, not a wasm one).

- **Where the game loads:** `load_art`, `load_overworld_art`, and
  `start_music` all run at `Startup`, so the entire 15 MB is requested up
  front. On the web that is the right shape (one burst, then nothing).

## 4. Known blockers

1. **`std::fs` probes in the crate (real, ours, small).**
   `combat/ui.rs::load_art` wraps every `assets.load` in
   `Path::new("assets").join(rel).exists()` and enumerates card faces with
   `std::fs::read_dir("assets/cards/faces")`; `overworld/screens.rs::
   load_overworld_art` does the same `exists()` check. `std::fs` exists on
   `wasm32-unknown-unknown` but every call errors, so `exists()` is `false`
   and `read_dir` yields nothing: no backdrop, no portraits, no card frame,
   no faces on the web. `music.rs` already documents why probing the disk is
   wrong even natively. Fix: load unconditionally (Bevy logs a missing
   asset; the `Option` art fields can be resolved from
   `AssetServer::load_state` or an `AssetEvent`), or at minimum
   `#[cfg(not(target_arch = "wasm32"))]` the probe. For faces, replace
   `read_dir` with a manifest (a `const` list, or a `faces.ron` fetched as
   an asset) — the wasm reader cannot list directories at all.

2. **`std::fs` in tests only:** `music::tests::the_track_is_committed…`
   reads the file with `std::fs`; tests do not run on wasm, so not a
   blocker.

3. **itch file sizes:** not a blocker (≤ 500 MB extracted, ≤ 200 MB per
   file, ≤ 1,000 files; we are ~45 MB / ~25 files).

4. **WebGL2 vs WebGPU in Bevy 0.19.1:** `webgl2` is in
   `default_platform` and is what a default-feature build targets in the
   browser. `webgpu` is a separate feature that *overrides* `webgl2`:
   "builds with the `webgpu` feature enabled won't be able to run on
   browsers that don't support WebGPU", and Bevy still calls its WebGPU
   support "experimental". Ship WebGL2; it runs everywhere. A 2D UI game
   loses nothing.
   Source: https://github.com/bevyengine/bevy/blob/v0.19.1/Cargo.toml
   (`default_platform`, and lines 637–640) and the examples README
   "WebGL2 and WebGPU".

5. **Unsupported Bevy features in this crate's default set:** none.
   Checked each `default_platform` entry against `v0.19.1` sources:
   `bevy_winit` (web backend), `bevy_gilrs` (web gamepad), `bevy_clipboard`
   (has wasm32 `web-sys` deps; `system_clipboard`/arboard is off by
   default), `sysinfo_plugin` (its `sysinfo` dep is only declared for
   linux/windows/macos/android/bsd targets, so it compiles out),
   `multi_threaded` (falls back to single-threaded on wasm),
   `x11`/`wayland` (native-only deps). The crate's own randomness is a
   local xorshift seeded from `Res<Time>` (`combat/plugin.rs`), which is
   wasm-safe; there is no `std::time::Instant`, threading, or `std::env`
   outside the `#[cfg(debug_assertions)]` dev entry point.

6. **Not verifiable on this machine:** the `wasm32-unknown-unknown`
   standard library is not installed here (Arch, no `rustup`), so this
   research did not do a proof compile. Step one of the implementation is
   `rustup target add wasm32-unknown-unknown` (or `pacman -S rust-wasm`)
   and `cargo build --release --target wasm32-unknown-unknown`.

## 5. Effort estimate

| Step | Time |
|---|---|
| Install target, `wasm-bindgen-cli`, `wasm-opt`, Bevy CLI; first `bevy build --release web --bundle` | 1–2 h (mostly compile time) |
| Fix the `std::fs` probes (both loaders) and add a faces manifest | 1–2 h, plus tests |
| `web/index.html` (copy the CLI default, retitle, keep the audio shim), `AssetMetaCheck::Never`, optionally `fit_canvas_to_parent` | 1 h |
| Try it in Chrome, Firefox, Safari; check music starts on first key; check load time | 1–2 h |
| itch upload (zip, "played in browser", 1280×720, Click to Play), or a `butler push` job in CI copied from the template workflow | 1 h |

**Total: roughly a weekend (6–10 hours), most of it waiting for the
compiler.** No engine work, no unsupported features, one small refactor in
our code. It is worth doing: a playable browser build removes the
"unzip and run `all-in.sh`" step that the pitch currently asks judges to do.

## Recommended next steps

1. Open a ticket for the `std::fs` probes (blocker §4.1) — it is a native
   bug too, per `music.rs`'s own reasoning about `BEVY_ASSET_ROOT`.
2. Add `[profile.web-release]` and `[package.metadata.bevy_cli.web]` to
   `Cargo.toml`, a `web/index.html`, and a `just web` / CI job modelled on
   the template's `release.yaml`.
3. Upload to the existing itch page as a second file marked "played in
   browser", keep the Linux zip alongside.
