# ElevenLabs narration for All In

Research for [#42](https://github.com/jpierre-7/all-in/issues/42) (part of #1).
Question: can we add voiced narration this weekend without risking the build?

Pinned versions checked: `bevy = "0.19.1"` in `Cargo.toml`; `Cargo.lock` resolves
`bevy_audio 0.19.1`, `rodio 0.22.2`, `lewton 0.10.2`. Every claim below cites the
page or file it came from. Researched 2026-09-12.

## Recommendation

**Do shape (a): pre-generate the clips offline with a short script, ship them
under `assets/narration/` as Ogg Vorbis, and play them with `AudioPlayer` +
`PlaybackSettings::DESPAWN`. Do not call the API from the game.**

Four facts drive this:

1. **The script fits the free tier with room for one re-take pass.** The 28
   constants in `src/overworld/narrative.rs` total 5,045 characters (4,845 if
   the seven UI labels such as `ANY_KEY` are skipped). Free is 10,000 credits a
   month and Multilingual v2 costs 1 credit per character, so one full pass is
   about half the month's budget ([pricing](https://elevenlabs.io/pricing)).
2. **Free-tier audio may ship in a non-commercial hackathon build, with
   attribution.** The Terms limit Free Users to non-commercial use, and the
   help centre says free-plan output "always requires attribution" ("elevenlabs.io"
   in the title/credits). If the game is ever sold, regenerate on Starter ($6,
   commercial licence) ([terms §1(c)](https://elevenlabs.io/terms-of-use),
   [help article](https://elevenlabs.io/docs/help-center/legal/can-i-publish-the-content-i-generate-on-the-platform)).
3. **Bevy 0.19.1 plays Ogg Vorbis with zero Cargo changes.** `DefaultPlugins`
   already includes `AudioPlugin`, and the default `audio` feature is
   `["bevy_audio", "vorbis"]`. MP3/WAV/FLAC each need a feature flag. Note
   ElevenLabs' "opus" output is Opus, not Vorbis, and Bevy's default decoder
   (lewton) is Vorbis-only, so the script should transcode MP3 to `.ogg` with
   ffmpeg rather than touch `Cargo.toml`
   ([bevy 0.19.1 features](https://docs.rs/crate/bevy/0.19.1/features),
   [bevy_audio 0.19.1 features](https://docs.rs/crate/bevy_audio/0.19.1/features),
   [lewton](https://docs.rs/lewton/0.10.2/lewton/)).
4. **Runtime calls buy nothing and cost a lot.** The text is static, ElevenLabs
   has no Rust SDK, the game would need an HTTP client, an API key in the
   binary or the environment, and a network round-trip before every screen.
   On conference wifi that is a visible stall or a silent failure at the exact
   moment the demo is being judged.

Fallback is free: if no audio device is present Bevy logs
`No audio device found.` and skips playback, it does not panic (see §3.4).

Rough cost: about half a day for one person, including listening to every clip
once (§4).

## 1. ElevenLabs text-to-speech API

### 1.1 Endpoint

- `POST /v1/text-to-speech/{voice_id}` on `https://api.elevenlabs.io`. The
  voice is chosen by the `voice_id` path parameter ("ID of the voice to be
  used"). Auth is the `xi-api-key` header. The response body is "The generated
  audio file" (binary).
  Source: [Text to speech › Convert](https://elevenlabs.io/docs/api-reference/text-to-speech/convert),
  [Authentication](https://elevenlabs.io/docs/api-reference/authentication).
- Request body: `text` (required), `model_id` (default `eleven_multilingual_v2`),
  optional `voice_settings` (`stability`, `similarity_boost`, `style`, `speed`,
  `use_speaker_boost`), `seed`, `previous_text` / `next_text` (for continuity
  between clips), and others. Same source.
- API keys are created at `https://elevenlabs.io/app/settings/api-keys`; the
  docs use the env var `ELEVENLABS_API_KEY`. The quickstart example uses
  `voice_id="JBFqnCBsd6RMkjVDRZzb"` (George), `model_id="eleven_v3"`,
  `output_format="mp3_44100_128"`.
  Source: [Quickstart](https://elevenlabs.io/docs/quickstart).
- Official SDKs are Python and Node.js only ("via our official Python bindings
  or our official Node.js libraries"). No Rust.
  Source: [API reference introduction](https://elevenlabs.io/docs/api-reference/introduction).

### 1.2 Choosing a voice

- `GET https://api.elevenlabs.io/v2/voices` lists voices available to the
  account; each has `voice_id`, `name`, and `category` (`premade`, `cloned`,
  `generated`, `professional`, `famous`, `high_quality`). Pick a `premade`
  voice for the narrator and pin its `voice_id` in the script.
  Source: [Voices › Search](https://elevenlabs.io/docs/api-reference/voices/search).

### 1.3 Models and per-request limits

| `model_id`               | Max chars / request | Notes |
|--------------------------|--------------------:|-------|
| `eleven_multilingual_v2` | 10,000 | "Most stable on long-form generations"; 1 credit per character |
| `eleven_v3`              | 5,000  | Highest expressiveness; the quickstart's default |
| `eleven_flash_v2_5`      | 40,000 | ~75 ms latency; "between 0.5 and 1 credit per character" on the API |

Our longest constant (`OPENING`) is 644 characters, so every model fits.
Sources: [Models](https://elevenlabs.io/docs/overview/models),
[Text to speech capability](https://elevenlabs.io/docs/capabilities/text-to-speech),
[Pricing](https://elevenlabs.io/pricing).

### 1.4 Output formats

`output_format` query values: `mp3_22050_32`, `mp3_24000_48`, `mp3_44100_32/64/96/128/192`,
`opus_48000_32/64/96/128/192`, `pcm_8000/16000/22050/24000/32000/44100/48000`,
`wav_8000/16000/22050/24000/32000/44100/48000`, `ulaw_8000`, `alaw_8000`.

Tier gates: "MP3 with 192kbps bitrate requires you to be subscribed to Creator
tier or above. PCM and WAV formats with 44.1kHz sample rate requires you to be
subscribed to Pro tier or above." So on Free use `mp3_44100_128` (the default)
or `wav_22050` / `wav_24000`.
Source: [Text to speech › Convert](https://elevenlabs.io/docs/api-reference/text-to-speech/convert).

### 1.5 Rate limits

Limits are on concurrency, not requests per minute. Text to speech: Free 2,
Starter 3, Creator 5, Pro 10, Scale 15, Business 15 concurrent requests.
Exceeding it returns HTTP 429 with `too_many_concurrent_requests`; `system_busy`
is the other 429 body, for platform load.
Source: [API error code 429](https://elevenlabs.io/docs/help-center/technical/api-error-code-429).

A script that generates the 28 clips sequentially never touches the limit.

### 1.6 Free tier budget

- Free: $0, "10k credits per month". Starter: $6, "30k credits per month",
  includes "Commercial License".
- "For V2 Multilingual models, 1 text character equals 1 credit."
- "Rollover does not apply to the Free plan."
Source: [Pricing](https://elevenlabs.io/pricing).

### 1.7 Licence for a hackathon game

- Terms §1(c): "if you access or use our Services free of charge (such a user,
  a 'Free User'), you may only use the Services for non-commercial purposes."
- Terms §4(c)(ii): "you retain all rights in and to your Output."
- Help centre: "The free plan does not include a commercial license and cannot
  be used for any commercial purpose." Free-plan content can be published
  non-commercially with attribution to ElevenLabs by including "elevenlabs.io"
  (or "11.ai") in the title. "All paid plans include a commercial license,
  provided you're not using Beta Services."
- Billing docs restate it: "If you are on the free plan, you can use the
  content non-commercially with attribution."

Sources: [Terms of use](https://elevenlabs.io/terms-of-use),
[Can I publish the content I generate?](https://elevenlabs.io/docs/help-center/legal/can-i-publish-the-content-i-generate-on-the-platform),
[Billing](https://elevenlabs.io/docs/overview/administration/billing).

For All In: a free hackathon entry is non-commercial. Put "Narration generated
with ElevenLabs (elevenlabs.io)" in the game's credits/title screen and the
README. If the game is later sold or put behind a paywall, regenerate every
clip on a paid plan (content "generated outside a paid subscription" stays
non-commercial).

## 2. Integration shapes

### 2.1 (a) Pre-generate offline, ship under `assets/`

A one-off script (Python with the official SDK, or plain `curl`) walks the
constants, POSTs each to `/v1/text-to-speech/{voice_id}` with
`output_format=mp3_44100_128`, transcodes with
`ffmpeg -i NAME.mp3 -c:a libvorbis -q:a 4 NAME.ogg`, and writes
`assets/narration/NAME.ogg`. The game loads them like any asset. No new crate,
no network, no key, deterministic every run.

Cost: 5,045 credits per full pass (§4). Files: 28 short Ogg clips, a few MB.

### 2.2 (b) Call the API at runtime

What the game would need, none of which exists in the repo today
(`Cargo.toml` has one dependency, `bevy`):

- An HTTP client. No official Rust SDK (§1.1). Either a blocking client such
  as `ureq` ("uses blocking I/O instead of async I/O", [docs.rs](https://docs.rs/ureq/latest/ureq/))
  run on Bevy's `IoTaskPool` ("a task pool for IO-intensive work",
  [docs.rs](https://docs.rs/bevy/0.19.1/bevy/tasks/struct.IoTaskPool.html))
  so it does not block the frame, or an async client (`reqwest`) which drags
  in a tokio runtime that Bevy does not run for you.
- Task plumbing: spawn a `Task<Result<Vec<u8>>>`, poll it each frame in a
  system, then hand the bytes to the asset system as a runtime-created
  `AudioSource`, then spawn the `AudioPlayer`. Plus timeouts, retries, and a
  "no audio yet" state for every screen.
- Key handling: `ELEVENLABS_API_KEY` must exist on the demo machine, and
  anything compiled into the binary is extractable. A leaked key spends the
  team's credits.
- Budget exposure: every screen transition spends live credits; a judge
  replaying the opening a few times eats the month's 10k.

Why it is a poor fit for a demo on conference wifi: each narration line is a
synchronous dependency on a remote service over a network you do not control.
Captive portals, packet loss, DNS hiccups, or a 429 `system_busy` all turn into
either a stall before the text appears or a silent screen, and the failure is
non-reproducible so you cannot fix it on the day. The text never changes, so
there is nothing runtime generation adds that offline generation lacks.

### 2.3 Verdict

(a). It is the only shape whose failure mode is "the clip is missing", which
Bevy already handles by logging and moving on.

## 3. Bevy 0.19.1 audio

### 3.1 Formats out of the box

From the pinned crate sources (`~/.cargo/registry/src/*/bevy-0.19.1/Cargo.toml`
and `bevy_audio-0.19.1/Cargo.toml`), mirrored on docs.rs:

- `bevy` default features: `["2d", "3d", "ui", "audio"]`; `audio = ["bevy_audio", "vorbis"]`.
  [docs.rs bevy 0.19.1 features](https://docs.rs/crate/bevy/0.19.1/features)
- `bevy_audio` features: `vorbis = ["rodio/lewton"]`, `mp3 = ["rodio/mp3"]`,
  `wav = ["rodio/hound"]`, `flac = ["rodio/claxon"]`, plus `symphonia-vorbis`,
  `symphonia-wav`, `symphonia-flac`, `aac`, `mp4`. None enabled by default at
  the `bevy_audio` level; `bevy` turns on `vorbis` via `audio`.
  [docs.rs bevy_audio 0.19.1 features](https://docs.rs/crate/bevy_audio/0.19.1/features)
- `AudioLoader::extensions()` (`bevy_audio-0.19.1/src/audio_source.rs`)
  registers `ogg` / `oga` only when `vorbis` or `symphonia-vorbis` is on,
  `mp3` only under `mp3`, `wav` under `wav`/`symphonia-wav`, `flac` under
  `flac`/`symphonia-flac`.

So with the current `Cargo.toml`: **Ogg Vorbis plays; MP3, WAV, FLAC do not**
until the matching feature is added to `bevy = { version = "0.19.1", features = [...] }`.
Ogg Opus does not play either: lewton is "A `vorbis` decoder, written in Rust"
([docs.rs](https://docs.rs/lewton/0.10.2/lewton/)), so ElevenLabs'
`opus_48000_*` output is not usable directly.

### 3.2 Playing a one-shot clip from a system

`AudioPlayer` is "A component for playing a sound. Insert this component onto
an entity to trigger an audio source to begin playing." It `#[require(PlaybackSettings)]`
and, once playback starts, an `AudioSink` is added to the entity. Changes to
`PlaybackSettings` after the fact do not affect already-playing audio.
`PlaybackSettings::DESPAWN` = `PlaybackMode::Despawn`, volume 1.0, speed 1.0,
which removes the entity when the clip ends.
Source: [docs.rs AudioPlayer](https://docs.rs/bevy/0.19.1/bevy/audio/struct.AudioPlayer.html),
`bevy_audio-0.19.1/src/audio.rs` lines 35–115 and 239–253.

```rust
fn narrate(asset_server: Res<AssetServer>, mut commands: Commands) {
    commands.spawn((
        AudioPlayer::new(asset_server.load("narration/OPENING.ogg")),
        PlaybackSettings::DESPAWN,
    ));
}
```

Run it in the same `OnEnter(...)` schedule that spawns the screen's text. To
cut a clip when the player skips, query the `AudioSink` on the entity and
despawn it, or just despawn the entity.

### 3.3 Conflicts with `DefaultPlugins`

None. `src/main.rs` adds `DefaultPlugins`, and `bevy_internal-0.19.1/src/default_plugins.rs`
line 80–81 includes `bevy_audio::AudioPlugin` under `#[cfg(feature = "bevy_audio")]`,
which is on by default. `AudioPlugin::build` (`bevy_audio-0.19.1/src/lib.rs`
lines 81–101) inserts `GlobalVolume` and `DefaultSpatialScale`, registers the
`AudioSource` asset and `AudioLoader`, and gates its systems on
`audio_output_available`. Nothing else in `main.rs` touches audio.

### 3.4 No audio device

`AudioOutput::default()` (`bevy_audio-0.19.1/src/audio_output.rs` lines 20–33)
calls `DeviceSinkBuilder::open_default_sink()`, and on error logs
`warn!("No audio device found.")` and stores `None`; `audio_output_available`
then returns `false` and the playback systems do not run. No panic, so a
demo laptop with no output device still runs the game silently.

## 4. Time and budget for the generation script

Counted from `src/overworld/narrative.rs` (28 `pub const ... &str`, line
continuations and `\"` escapes resolved):

| Constant | Chars | | Constant | Chars |
|---|---:|---|---|---:|
| OPENING | 644 | | ENC_THE_HOUSE | 204 |
| TUTORIAL | 532 | | LOBBY | 179 |
| ENDING | 549 | | ENC_SLOTZ | 179 |
| INFO_ROOM | 419 | | BIG_SHOTS_INTRO | 173 |
| ENC_PIT_BOSS | 300 | | WIN_PIT_BOSS | 162 |
| PIT_INTRO | 254 | | ENC_PIT_MINION | 149 |
| FLOOR_INTRO | 216 | | ENC_FLOOR_MINION | 146 |
| WIN_SLOTZ | 128 | | PERK_PICK | 118 |
| FOLD | 116 | | FIGHT_OR_FOLD | 106 |
| LOSE | 104 | | WIN_THE_HOUSE | 101 |
| ITEM_DROP | 97 | | WIN_MINION | 75 |
| ANY_KEY_BACK | 25 | | LOBBY_OPT_TUTORIAL | 21 |
| LOBBY_OPT_INFO | 14 | | LOBBY_OPT_BEGIN | 14 |
| ANY_KEY | 14 | | GAME_OVER | 6 |

Total: **5,045 characters / 952 words**. Prose only (drop the seven UI labels
`LOBBY_OPT_*`, `FIGHT_OR_FOLD`, `GAME_OVER`, `ANY_KEY`, `ANY_KEY_BACK`):
**4,845 characters across 21 constants**.

Budget on Free (10,000 credits, 1 credit/char on Multilingual v2, no rollover):

- One full pass: 5,045 credits, 50 % of the month. Yes, it fits.
- Two full passes: 10,090 credits, 90 over. Two prose-only passes: 9,690, fits.
- Flash v2.5 at 0.5–1 credit/char: two to four full passes.

So the plan is: one pass on `eleven_multilingual_v2`, listen, and re-take only
the lines that need it. A second account or the $6 Starter tier removes the
constraint entirely if the voice needs a couple of full re-takes.

Time estimate, one person:

| Step | Estimate |
|---|---:|
| Create account, API key, pick a `premade` voice by listening to samples | 30 min |
| Script: parse constants (or a hand-written list), POST each, save MP3, ffmpeg to Ogg | 1 h |
| Run it (28 sequential requests; concurrency limit 2 is irrelevant) | 10 min |
| Listen to all 28 clips, note mispronunciations, re-take | 1 h |
| Bevy: `AudioPlayer` spawn in the existing `OnEnter` systems, skip-to-stop, credits line | 1–2 h |
| **Total** | **about half a day** |

The script does not belong in the Rust build; keep it under `tools/` or as a
one-off and commit only the resulting `assets/narration/*.ogg`.

## Open questions

- Which lines get voiced. The seven UI labels are not worth a clip; the
  `ENDING` and `OPENING` are the obvious wins.
- Whether the tutorial text reads well aloud ("Draw 7 and play up to 5") or
  should stay text-only.
- Whether the hackathon rules require disclosing AI-generated assets beyond the
  ElevenLabs attribution.
