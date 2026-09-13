# Music for All In

Research for [#66](https://github.com/jpierre-7/all-in/issues/66) (part of #1).
Question: can a looping casino-noir track ship this weekend without licence
trouble or a build risk?

Pinned versions checked: `bevy = "0.19.1"` in `Cargo.toml`; `Cargo.lock` resolves
`bevy_audio 0.19.1`, `rodio 0.22.2`, `lewton 0.10.2`. Bevy audio facts are read
from the local registry copy (`~/.cargo/registry/src/*/bevy_audio-0.19.1/`) and
cross-checked on docs.rs; the format facts (Ogg Vorbis plays out of the box, no
Cargo change) are established in
[`elevenlabs-narration.md` §3.1](https://github.com/jpierre-7/all-in/blob/research/elevenlabs/docs/research/elevenlabs-narration.md)
and reused here. Every claim below cites the page or file it came from.
Researched 2026-09-13.

## Recommendation

**Take one Kevin MacLeod track from incompetech under CC BY 4.0, transcode it to
Ogg Vorbis at `-q:a 3` (about 2 MB), commit it as `assets/music/<name>.ogg`,
and spawn it once at `Startup` with `AudioPlayer` + `PlaybackSettings::LOOP` on
an entity with no `DespawnOnExit`. Mute with `AudioSink::toggle_mute` on M.**

First choice: **"Deadly Roulette"** (2:39, "Jazzy! Gumshoe-y! Mystery-y!",
Dark / Grooving / Relaxed). Backups: **"Hard Boiled"** (3:01, private-eye noir)
and **"Bass Walker"** (2:41, solo walking bass, "Noir!"). All three are in §1.1.

The attribution line, in the exact form incompetech asks for, goes in the README
and on a credits line the player can find (incompetech's FAQ says a game's
credits screen or settings menu is the expected place):

```
"Deadly Roulette" Kevin MacLeod (incompetech.com)
Licensed under Creative Commons: By Attribution 4.0 License
http://creativecommons.org/licenses/by/4.0/
```

Four facts drive this:

1. **incompetech is the only library found that is CC-BY throughout, has a
   genre catalogue with searchable mood text, and returned tracks that are
   actually casino-noir.** The catalogue JSON lists 1,442 pieces; a search of
   title, description and feel for *noir / lounge / casino / vegas / detective*
   returns 24 hits, of which six fit this game's brief (§1.1). Pixabay's top
   "jazz noir" hits are AI-generated and unattributed; OpenGameArt's are
   one-minute intro stingers; freesound is a sound-effects site whose search
   was returning "Freesound is busy" during this research (§1.2–1.4). FreePD
   closed in 2025 (§1.5).
2. **CC BY 4.0 is the easiest licence to comply with.** It permits sharing and
   adapting "for any purpose, even commercially"; the only condition is
   credit, a link to the licence, and a note of changes, "in any reasonable
   manner" ([CC BY 4.0 deed](https://creativecommons.org/licenses/by/4.0/)).
   Transcoding MP3 to Ogg is a change, and the deed asks that changes be
   indicated, so add "(transcoded to Ogg Vorbis)" after the first line of the
   credit. No non-commercial clause, so a later paid release needs no
   relicensing.
3. **Bevy 0.19.1 has everything needed with no new dependency.**
   `PlaybackSettings::LOOP` sets `PlaybackMode::Loop`, which `bevy_audio`
   implements as rodio `repeat_infinite()`; `AudioSink` implements
   `AudioSinkPlayback` with `mute`, `unmute`, `toggle_mute`, `set_volume`, and
   `Volume::fade_towards` exists for cross-fades. Bevy's own
   `examples/audio/audio_control.rs` binds `KeyCode::KeyM` to `toggle_mute`
   (§2). An entity without `DespawnOnExit` is never touched by a state change,
   so the loop survives every transition (§2.3).
4. **A real transcode measured 1.96 MB.** "Deadly Roulette" (2:39, 256 kbps
   MP3, 5.1 MB) at `ffmpeg -c:a libvorbis -q:a 3` is 1,963,887 bytes at
   98.6 kbps, and lewton 0.10.2 decodes all 159.29 s of it (§3). That is the
   size of one `assets/backstory/opening_N.png`, and the repo's own rule is
   "keep each file near or under ~1.5MB — there is no git-lfs here"; one music
   file slightly over that line is a 2 % addition to a 13 MB `assets/` tree.
   Commit it.

Rough cost: about two hours for one person, including a listen-through of the
three candidates (§4). Cut it by dropping the mute key (twenty minutes) rather
than the loop.

## 1. Sources

Licence terms, attribution text, and what a "jazz noir / lounge / casino"
search actually surfaces.

### 1.1 incompetech (Kevin MacLeod)

- **Licence:** "Creative Commons: By Attribution 4.0". The licences page offers
  two options: "Creative Commons — Free", which "Requires that you credit the
  music", and a paid "Standard License" for cases where attribution is
  "impractical or unwanted".
  Source: [licences](https://incompetech.com/music/royalty-free/licenses/),
  [FAQ](https://incompetech.com/music/royalty-free/faq.html).
- **Attribution text** (verbatim from the FAQ, where the title is the piece's):

  ```
  "Title" Kevin MacLeod (incompetech.com)
  Licensed under Creative Commons: By Attribution 4.0 License
  http://creativecommons.org/licenses/by/4.0/
  ```

  Placement, from the same FAQ: the credit goes wherever "someone seeking the
  music source can locate it without difficulty"; for video games that is "a
  'Credits' screen found in the settings menu".
  Source: [FAQ](https://incompetech.com/music/royalty-free/faq.html).
- **Search:** the site's browser is driven by a JSON feed,
  [`pieces.json`](https://incompetech.com/music/royalty-free/pieces.json)
  (1,442 entries with `title`, `length`, `feel`, `description`,
  `instruments`, `isrc`). Filtering `title + description + feel` for
  *noir|lounge|casino|vegas|smoky|speakeasy|detective|film noir|cocktail|saloon|poker|gambl*
  gives 24 hits. The ones that fit the game, with the track page
  (`index.html?isrc=<ISRC>`) and the direct MP3:

  | Title | ISRC | Length | Feel | Kevin's description | MP3 |
  |---|---|---:|---|---|---:|
  | Deadly Roulette | USUAN1600033 | 2:39 | Dark, Grooving, Relaxed | "Jazzy! Gumshoe-y! Mystery-y!" Drums, bass, synth bass, guitar, vibes | 5.1 MB |
  | Hard Boiled | USUAN1700076 | 3:01 | Grooving, Mysterious, Relaxed | "You're looking for the low-down, huh? ... Ten bucks a day plus expenses." Bass, piano, drums | 5.8 MB |
  | Bass Walker | USUAN1200071 | 2:41 | Dark, Grooving, Mysterious | "Just your average work-a-day upright walking bass line. Add reverb to taste. Noir!" | 6.5 MB |
  | Night on the Docks - Sax | USUAN1100137 | 2:54 | Dark, Somber, Relaxed | "Sad and smooth; Think 1950's detective film." EP, tenor sax | 5.9 MB |
  | Spy Glass | USUAN1500058 | 3:47 | Grooving, Mysterious | "Super cool jazz for your hardcore detectives!" Piano, bass, drums, vibes, saxes | 7.3 MB |
  | Dances and Dames | USUAN1100595 | 2:27 | Grooving, Mysterious, Suspenseful | "Off-center feeling film noir-like piece with a continually changing time signature." | 6.0 MB |

  Also relevant but off-brief: "Aces High" (3:13, "a night out on the Vegas
  strip", but Bouncy/Driving), "Lobby Time" (3:13, lounge vibes, Calming),
  "Deuces" (3:10, 1960s club jazz, Bright).

  Track page pattern: `https://incompetech.com/music/royalty-free/index.html?isrc=USUAN1600033`.
  MP3 pattern: `https://incompetech.com/music/royalty-free/mp3-royaltyfree/Deadly%20Roulette.mp3`
  (HEAD returned 200 and the `Content-Length` above for each; the MP3s are
  44.1 kHz stereo at 256 kbps per `ffprobe`).
  Source: `pieces.json` fetched 2026-09-13; sizes from HTTP `Content-Length`.

### 1.2 OpenGameArt

- **Licences:** submitters choose from CC0, CC-BY 3.0/4.0, CC-BY-SA 3.0/4.0,
  OGA-BY 3.0/4.0, GPL 2.0/3.0; the licence is shown per asset. Suggested
  credit form: "[Asset name] by [author name] licensed [license]: [asset url]",
  and artists may override it in a "Copyright/Attribution Notice" field.
  Source: [OGA FAQ](https://opengameart.org/content/faq).
- **Search:** the music-type search for "noir jazz", "casino" and "lounge jazz"
  surfaces mostly short pieces. Checked individually:

  | Title | Author | Licence | File | Length | Notes | URL |
  |---|---|---|---|---:|---|---|
  | Coffee Black | GrimFrenzy | CC-BY 3.0 | `CoffeeBlack.ogg`, 1.6 MB, Vorbis 213 kbps | 0:59.8 | "a jazzy noir intro theme, slightly on the anxious side"; attribution notice "Free to use, have fun." | [link](https://opengameart.org/content/coffee-black) |
  | Casino Man | Spring Spring | CC-BY 3.0 / OGA-BY 3.0 / CC-BY-SA 3.0 | `Casino Man.ogg`, 2.1 MB, Vorbis 195 kbps | 1:25 | "Retro, brisk, 'casino theme'", tagged silly | [link](https://opengameart.org/content/casino-man) |
  | Jazzy Vibes #89 - Melancholic Jazz Piano | Tri-Tachyon | CC-BY 4.0 | MP3, 916 KB | not stated | Requires "Music by Tri-Tachyon - https://soundcloud.com/tri-tachyon/albums" | [link](https://opengameart.org/content/jazzy-vibes-89-melancholic-jazz-piano) |
  | Velvet Sax (Solo / Band / Synth) | Bogart VGM | CC-BY 4.0 | three MP3s, 2.2–2.5 MB | not stated | "moody", "future noir influenced" | [link](https://opengameart.org/content/velvet-sax-three-arangments-solo-band-synth) |

  Lengths for the two Ogg files are from `ffprobe` on the downloaded file.
  Coffee Black is the best fit tonally, but a one-minute intro theme looped
  for a twenty-minute run is a worse listen than a three-minute piece.
  Source: the four asset pages above, fetched 2026-09-13.

### 1.3 Pixabay Music

- **Licence:** the Pixabay Content License grants an "irrevocable, worldwide,
  perpetual, non-exclusive and royalty-free right" to use content "for
  commercial or non-commercial purposes". Attribution: "You do not need to
  credit Pixabay or the contributor". Prohibited: selling or distributing the
  content "on a Standalone basis", trademark use, misleading use. It is not a
  Creative Commons licence.
  Source: [licence summary](https://pixabay.com/service/license-summary/),
  [terms](https://pixabay.com/service/terms/).
- **Search:** "jazz noir" returns, among others: "Noir Jazz Detective Midnight
  Mystery" (alex-morgan, 2:23), "Gritty Noir (ASMR Noir Jazz)"
  (KonstantinPazuzuStudio, 3:23), "Noir-Alley" (WelbornWorks, 3:10), "Dancing
  Saxophone (dark jazz)" (Surprising_Media, 4:00). The first is labelled
  **"AI generated"** on its page; downloads are MP3.
  Source: [search](https://pixabay.com/music/search/jazz%20noir/),
  [track page](https://pixabay.com/music/crime-scene-noir-jazz-detective-midnight-mystery-587403/).
- **Why not first choice:** no attribution required is convenient, but a
  hackathon entry may have to disclose AI-generated assets, the artist
  identities are pseudonymous and the site's own terms warn that "certain
  Content may be subject to additional intellectual property rights". A named
  composer under a CC licence is easier to stand behind on a judges' table.

### 1.4 freesound

- **Licences:** CC0, CC BY 4.0, CC BY-NC 4.0 (and the retired Sampling+ on
  old uploads). CC0 "you can do pretty much what you want with the sound";
  CC BY and CC BY-NC need credit; BY-NC bars commercial use.
  Source: [FAQ](https://freesound.org/help/faq/).
- **Search:** a CC0 "jazz noir" search returned short loops, e.g. "(Jazz Loop)
  Rusted Maid" by plasterbrain, 0:30.8, CC0
  ([sound 464923](https://freesound.org/people/plasterbrain/sounds/464923/)).
  Half of the sound pages and the second search returned "Freesound is busy
  right now" / "Search is busy" during this research, and the FAQ itself gave
  HTTP 503 twice. It is a sound-effects library; use it for a chip clink or a
  card snap, not the bed.

### 1.5 FreePD

Closed. The site now shows only a notice: "After 17 years of sharing millions
of free-to-use, Public Domain music downloads ... we have officially taken the
service offline. ... [freepd.com] is now permanently closed. ... 2008-2025".
Source: [freepd.com](https://freepd.com/), fetched 2026-09-13.

### 1.6 The licences themselves

- **CC BY 4.0:** free to "copy and redistribute the material in any medium or
  format for any purpose, even commercially" and to "remix, transform, and
  build upon the material". Condition: "give appropriate credit, provide a
  link to the license, and indicate if changes were made ... in any reasonable
  manner, but not in any way that suggests the licensor endorses you or your
  use". "No additional restrictions".
  Source: [deed](https://creativecommons.org/licenses/by/4.0/).
- **CC0 1.0:** the author "has dedicated the work to the public domain by
  waiving all of his or her rights to the work worldwide under copyright law".
  No attribution required; "you should not imply endorsement by the author".
  Patent, trademark, publicity and privacy rights are not affected.
  Source: [deed](https://creativecommons.org/publicdomain/zero/1.0/).
- **DRM note from OGA's FAQ:** CC-BY's "no additional restrictions" clause is
  read by OpenGameArt as barring distribution on DRM stores (App Store, Xbox
  Live, PSN) without the artist's permission; OGA-BY and CC0 do not have that
  issue. Irrelevant for an itch.io / GitHub demo, worth knowing if the game
  ever goes to a console store.
  Source: [OGA FAQ](https://opengameart.org/content/faq).

## 2. Bevy 0.19.1 audio

File references are to `~/.cargo/registry/src/index.crates.io-*/bevy_audio-0.19.1/src/`.
docs.rs links are for the same version.

### 2.1 Looping

- `PlaybackSettings::LOOP` is `PlaybackSettings { mode: PlaybackMode::Loop, ..PlaybackSettings::ONCE }`
  (`audio.rs` lines 100–103). `ONCE` is volume `Volume::Linear(1.0)`, speed
  1.0, not paused, not muted, not spatial, `start_position: None`,
  `duration: None` (lines 87–97). `Default` is `ONCE` (line 73–77).
  `PlaybackMode::Loop` is documented "Repeat the sound forever." (line 21).
  Source: [`PlaybackSettings`](https://docs.rs/bevy/0.19.1/bevy/audio/struct.PlaybackSettings.html),
  [`PlaybackMode`](https://docs.rs/bevy/0.19.1/bevy/audio/enum.PlaybackMode.html).
- Implementation: `play_queued_audio_system` matches `PlaybackMode::Loop` and
  appends `decoder.repeat_infinite()` to the rodio sink (with
  `skip_duration` / `take_duration` if `start_position` / `duration` are set),
  then inserts the `AudioSink` component on the entity (`audio_output.rs`
  lines 145–165 and 200). The sink volume is set to
  `settings.volume * global_volume.volume` at that moment (line 193).
  Source: [audio_output.rs @ v0.19.1](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_audio/src/audio_output.rs).
- Helpers: `.with_volume(Volume)`, `.paused()`, `.muted()` are `const fn`
  builders on `PlaybackSettings` (`audio.rs` lines 118–134). `AudioPlayer`
  is `#[require(PlaybackSettings)]` and `AudioPlayer::new(handle)` builds it
  (lines 250–272).
  Source: [`AudioPlayer`](https://docs.rs/bevy/0.19.1/bevy/audio/struct.AudioPlayer.html).
- Loop seam: Vorbis carries an exact end position ("A granule position on the
  final page in a stream that indicates less audio data than the final packet
  would normally return is used to end the stream on other than even frame
  boundaries"), so `repeat_infinite` restarts at the true last sample; whether
  the join is audible depends only on the track's own ending. "Deadly
  Roulette" and "Hard Boiled" both end on a sustained chord, so expect a
  seam; pick a loop point (`start_position` + `duration` on
  `PlaybackSettings`) or fade the ends with ffmpeg if it grates.
  Source: [Vorbis I spec §A.2](https://xiph.org/vorbis/doc/Vorbis_I_spec.html).

### 2.2 Volume and mute

- `Volume` is an enum, `Linear(f32)` or `Decibels(f32)`, with `SILENT =
  Linear(0.0)`, `to_linear()`, `to_decibels()`, `increase_by_percentage()`,
  `decrease_by_percentage()`, `scale_to_factor()`, and
  `fade_towards(target, factor)` (linear interpolation, factor clamped to
  0..=1) (`volume.rs` lines 36–247).
  Source: [`Volume`](https://docs.rs/bevy/0.19.1/bevy/audio/enum.Volume.html).
- `AudioSink` is inserted by Bevy once playback starts; it implements
  `AudioSinkPlayback`: `volume()`, `set_volume(&mut self, Volume)`, `speed()`,
  `set_speed()`, `play()`, `pause()`, `toggle_playback()`, `is_paused()`,
  `stop()`, `empty()`, `position()`, `try_seek()`, `is_muted()`,
  `mute(&mut self)`, `unmute(&mut self)`, `toggle_mute(&mut self)`
  (`sinks.rs` lines 10–126). Mute "sets the volume to 0", unmute "Restores the
  volume to the value it was before it was muted", and `set_volume` while
  muted is remembered and applied on unmute. `mute`/`unmute`/`set_volume`
  take `&mut self`, so the query is `Query<&mut AudioSink, With<Music>>`.
  Source: [`AudioSinkPlayback`](https://docs.rs/bevy/0.19.1/bevy/audio/trait.AudioSinkPlayback.html),
  [`AudioSink`](https://docs.rs/bevy/0.19.1/bevy/audio/struct.AudioSink.html).
- `GlobalVolume` is a resource with one field `volume: Volume`, but "Changing
  `GlobalVolume` does not affect already playing audio" (`volume.rs` lines
  5–13). So a global mute via `GlobalVolume` would not silence the running
  loop; mute the sink instead.
  Source: [`GlobalVolume`](https://docs.rs/bevy/0.19.1/bevy/audio/struct.GlobalVolume.html).
- `PlaybackSettings` is initial-only: "Changes to this component will *not*
  be applied to already-playing audio" (`audio.rs` line 32). Same reason.
- Bevy's own example does exactly the M-key toggle:

  ```rust
  fn mute(
      keyboard_input: Res<ButtonInput<KeyCode>>,
      mut music_controller: Query<&mut AudioSink, With<MyMusic>>,
  ) {
      let Ok(mut sink) = music_controller.single_mut() else { return; };
      if keyboard_input.just_pressed(KeyCode::KeyM) {
          sink.toggle_mute();
      }
  }
  ```

  Source: [`examples/audio/audio_control.rs` @ v0.19.1](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/audio/audio_control.rs)
  (also in the registry at `bevy-0.19.1/examples/audio/audio_control.rs`).

### 2.3 Surviving state transitions

- State-scoped despawn in 0.19.1 is opt-in per entity: `DespawnOnExit<S>(pub S)`
  "despawns the entity when exiting the given state", and the doc example
  spawns it explicitly alongside the entity (`bevy_state-0.19.1/src/state_scoped.rs`
  line 149 and the doc above it). Every overworld screen and the combat UI
  attach it (`src/overworld/screens.rs` line 168, `src/combat/ui.rs` line
  162, `src/combat/info.rs` line 143). An entity spawned without it is never
  touched by `AppState` changes.
  Source: [`DespawnOnExit`](https://docs.rs/bevy/0.19.1/bevy/state/state_scoped/struct.DespawnOnExit.html).
- Where to spawn it: `AppState::Title` is the `#[default]` and nothing
  transitions back to it (`grep AppState::Title src/` finds only `OnEnter`,
  `in_state`, the screen spawn and tests), so `OnEnter(AppState::Title)` runs
  once. `Startup` is simpler and already hosts `load_overworld_art`
  (`src/overworld/mod.rs` line 36). Either works; `Startup` cannot double up.
- Test harness: the overworld tests build `MinimalPlugins + StatesPlugin`
  with no `AssetPlugin` (`src/overworld/mod.rs` lines 391–400), so a `Startup`
  system with `Res<AssetServer>` would panic there. `load_overworld_art`
  already handles this with `Option<Res<AssetServer>>`
  (`src/overworld/screens.rs` line 29); the music spawn should copy that.
- Spawn shape:

  ```rust
  #[derive(Component)]
  struct Music;

  fn start_music(mut commands: Commands, assets: Option<Res<AssetServer>>) {
      let Some(assets) = assets else { return };
      commands.spawn((
          AudioPlayer::new(assets.load("music/deadly_roulette.ogg")),
          PlaybackSettings::LOOP.with_volume(Volume::Linear(0.6)),
          Music,
      ));
  }
  ```

  The 0.6 is a guess; the combat UI has no other sound to balance against yet.

### 2.4 Switching or fading per floor

Not needed for the ticket's minimum, but the pinned version supports it
without extra crates. Bevy's `examples/audio/soundtrack.rs` does the whole
thing: on a state change it tags every entity `With<AudioSink>` with a
`FadeOut` marker, spawns the new track with
`PlaybackSettings { mode: PlaybackMode::Loop, volume: Volume::SILENT, ..default() }`
plus a `FadeIn` marker, and two `Update` systems call
`sink.set_volume(Volume::SILENT.fade_towards(Volume::Linear(1.0), t / FADE_TIME))`
(and the reverse), despawning the old entity when its fade completes.
Source: [`examples/audio/soundtrack.rs` @ v0.19.1](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/audio/soundtrack.rs).

For All In that would be `OnEnter(AppState::FloorIntro)` reading
`Progress` to pick a per-floor handle. Cost is roughly one more hour and one
more track per floor (each another ~2 MB); do not do it this weekend.

### 2.5 Formats

Unchanged from `elevenlabs-narration.md` §3.1: `bevy`'s default `audio`
feature is `["bevy_audio", "vorbis"]`, `AudioLoader::extensions()` registers
`ogg`/`oga` under `vorbis`, and MP3 / WAV / FLAC each need a feature flag. So
the incompetech MP3 must be transcoded, not committed as-is.
Source: [bevy 0.19.1 features](https://docs.rs/crate/bevy/0.19.1/features),
[bevy_audio 0.19.1 features](https://docs.rs/crate/bevy_audio/0.19.1/features),
`bevy_audio-0.19.1/src/audio_source.rs`.

### 2.6 No audio device

Also unchanged: `AudioOutput::default()` logs `No audio device found.` and the
playback systems are gated off; no panic (`elevenlabs-narration.md` §3.4,
`bevy_audio-0.19.1/src/audio_output.rs` lines 20–33).

## 3. Asset size

Measured on "Deadly Roulette" (source MP3: 5,099,663 bytes, 44.1 kHz stereo,
256 kbps, 159.29 s) with `ffmpeg n9.0.1`, `-c:a libvorbis`:

| `-q:a` | Bytes | Reported bitrate |
|---:|---:|---:|
| 2 | 1,651,987 | 83 kbps |
| 3 | 1,963,887 | 99 kbps |
| 4 | 2,216,562 | 111 kbps |
| 5 | 2,781,608 | 140 kbps |

`-b:a 96k` and `-b:a 128k` produced byte-identical files to `-q:a 2` and
`-q:a 4` respectively. Rule of thumb from these numbers: **about 0.75 MB per
minute at q3, 0.85 MB per minute at q4**, so a three-minute track is 2.2–2.5 MB.

Decode check: a scratch crate on `lewton = "=0.10.2"` (the version in
`Cargo.lock`) read the q3 file end to end with `OggStreamReader` and
`read_dec_packet_itl`: 2 channels, 44,100 Hz, 7,024,896 frames = 159.29 s,
matching `ffprobe`. So the file Bevy will load is the file that was tested.

Commit it or fetch it?

- The repo already carries 13 MB under `assets/`, five backstory PNGs at
  ~1.6 MB each, and states its own policy: "Keep each file near or under
  ~1.5MB — there is no git-lfs here" (`assets/README.md`). One 2 MB Ogg is in
  the same class as one backstory frame.
- Fetching at build or run time would add a network dependency to the exact
  thing the ElevenLabs research rejected a network dependency for (demo
  machine, conference wifi), and would need a download step nobody has
  written. The art is not fetched; the music should not be either.
- Git stores the file once; re-encodes are the only thing that grow history.
  Pick the quality once (q3) and do not iterate it in-repo.

**Recommendation: commit `assets/music/deadly_roulette.ogg` at `-q:a 3`.** If
the size rule is felt to be binding, q2 at 1.65 MB is still fine for a jazz
bed under a UI with no other sound.

## 4. Time cost

One ambient loop from the Title screen through the game, plus M to mute. One
person, nothing parallel.

| Step | Estimate |
|---|---:|
| Listen to Deadly Roulette, Hard Boiled, Bass Walker; pick one | 15 min |
| `ffmpeg -i X.mp3 -c:a libvorbis -q:a 3 assets/music/x.ogg`; add a row to `assets/README.md` | 10 min |
| `Music` marker + `start_music` in `Startup` with `Option<Res<AssetServer>>` (§2.3) | 20 min |
| `toggle_music` on `KeyCode::KeyM` in `Update`, `Query<&mut AudioSink, With<Music>>` | 15 min |
| Keep M out of `any_key` (see below) | 15 min |
| Attribution line in README and on the Title footer or a credits line | 15 min |
| `cargo test` (shell harness must still pass with no `AssetServer`), `cargo run`, listen through one full run for the loop seam | 30 min |
| **Total** | **about 2 h** |

The one design snag: `screens::any_key` returns true for *any* just-pressed
key (`src/overworld/screens.rs` line 207), and every prose screen dismisses on
it, so pressing M to mute mid-Opening would also page the story. Either filter
`KeyCode::KeyM` out of `any_key`, or accept that mute is a Title-screen /
Lobby / combat action (Title takes Space only, Lobby takes digits, combat takes
P/H/I). The filter is the fifteen-minute row above.

Drop the mute for a 1 h 20 min version. Do not drop the loop for the mute.

## Open questions

- Which of the three. Deadly Roulette has vibes and a synth bass, which sits
  closer to the neon backdrops; Hard Boiled is the purer noir trio. Both loop
  with an audible seam at the ending chord; whoever implements should try
  `start_position`/`duration` once and give up if it is not clean in ten
  minutes.
- Whether the hackathon requires an in-game credits screen or a README line
  suffices. incompetech's FAQ expects an in-game credits screen for games; a
  line on the Title footer covers it either way.
- Sound effects (card snap, chip clink) are a separate ticket; freesound CC0 is
  the place, when it is up.
