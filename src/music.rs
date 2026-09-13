//! The bed: one looping track, from the Title screen to the end of the night.
//!
//! One entity, spawned once at `Startup` and never state-scoped, so no screen
//! transition can reach it and the loop plays through the whole run. Mute is
//! **M**, and it goes through the sink: `GlobalVolume` does not touch audio
//! that is already playing.

use bevy::audio::Volume;
use bevy::prelude::*;

/// Kevin MacLeod, "Deadly Roulette", CC BY 4.0, transcoded to Ogg Vorbis.
/// Vorbis is the one format Bevy decodes without a Cargo feature.
const TRACK: &str = "music/deadly_roulette.ogg";

/// Under the prose rather than over it. Nothing else in the game makes a sound
/// yet, so there is nothing to balance against — this is a listening call.
const VOLUME: f32 = 0.6;

pub struct MusicPlugin;

impl Plugin for MusicPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_music)
            .add_systems(Update, toggle_mute);
    }
}

/// The one long-lived audio entity.
#[derive(Component)]
pub struct Music;

/// `AssetServer` is an `Option` because the headless test harnesses build
/// `MinimalPlugins` with no `AssetPlugin`, and asking for a missing resource
/// is a panic. No server: the game runs silent.
///
/// The track is *not* checked for on disk first. `AssetServer` resolves its
/// root from `BEVY_ASSET_ROOT`, then `CARGO_MANIFEST_DIR`, then the
/// executable's own directory — so a `Path::new("assets")` probe agrees with
/// it under `cargo run` and disagrees with it in a shipped build launched from
/// anywhere else, which would silence a game whose ogg is sitting right beside
/// the binary. `assets.load` on a genuinely missing file logs and plays
/// nothing, which is the same outcome without the false negative.
/// `the_track_is_committed_and_is_really_vorbis` is what guards the file.
fn start_music(mut commands: Commands, assets: Option<Res<AssetServer>>) {
    let Some(assets) = assets else {
        return;
    };
    commands.spawn((
        AudioPlayer::new(assets.load(TRACK)),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(VOLUME)),
        Music,
    ));
}

/// M. The sink appears only once playback has actually started, so an empty
/// query is the ordinary state for the first frames and for a machine with no
/// audio device — not an error.
///
/// Untested, and not for want of trying: `AudioSink` wraps a live rodio sink
/// and cannot be constructed without opening the machine's audio device, which
/// is not a thing a test suite should do. What *is* tested is the half that
/// has been wrong before — that M reaches this system at all, rather than
/// being eaten by `screens::any_key` (see `screens::tests::but_not_on_the_mute`).
fn toggle_mute(keys: Res<ButtonInput<KeyCode>>, mut music: Query<&mut AudioSink, With<Music>>) {
    if !keys.just_pressed(MUTE) {
        return;
    }
    for mut sink in &mut music {
        sink.toggle_mute();
    }
}

/// M, the one key in the game that is not a game input. `screens::any_key`
/// reads it from here to filter it out, so muting mid-story does not also page
/// the story.
pub const MUTE: KeyCode = KeyCode::KeyM;

#[cfg(test)]
mod tests {
    use bevy::asset::AssetPlugin;
    use bevy::audio::{AudioSource, PlaybackMode};
    use bevy::prelude::*;

    use super::{Music, MusicPlugin};

    /// The real shell's plugins, minus the window and the renderer: enough for
    /// `AssetServer` to hand out a handle.
    fn with_assets() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(AssetPlugin::default())
            // What `AudioPlugin` would register. The plugin itself is not
            // added: it opens the machine's real audio device, which a test
            // has no business doing.
            .init_asset::<AudioSource>()
            .add_plugins(MusicPlugin);
        app.update();
        app
    }

    /// What the overworld and combat test harnesses look like: no `AssetPlugin`,
    /// so no `AssetServer` to ask.
    fn without_assets() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(MusicPlugin);
        app.update();
        app
    }

    fn music(app: &mut App) -> Vec<Entity> {
        app.world_mut()
            .query_filtered::<Entity, With<Music>>()
            .iter(app.world())
            .collect()
    }

    /// The track is the feature; losing it to a bad merge should be a red test
    /// and not a silent game. `OggS` is the Vorbis container's magic — Bevy
    /// decodes no other format without a Cargo feature, so an MP3 renamed to
    /// `.ogg` would load as nothing.
    #[test]
    fn the_track_is_committed_and_is_really_vorbis() {
        let path = std::path::Path::new("assets").join(super::TRACK);

        let bytes = std::fs::read(&path).unwrap_or_else(|e| {
            panic!(
                "{} is the whole feature, and it is gone: {e}",
                path.display()
            )
        });

        assert!(bytes.starts_with(b"OggS"), "not an Ogg container");
    }

    #[test]
    fn one_track_starts_at_startup() {
        let mut app = with_assets();

        assert_eq!(music(&mut app).len(), 1);
    }

    #[test]
    fn the_track_loops_forever() {
        let mut app = with_assets();
        let entity = music(&mut app)[0];

        let settings = app.world().get::<PlaybackSettings>(entity).unwrap();

        assert!(matches!(settings.mode, PlaybackMode::Loop));
    }

    /// No `DespawnOnExit` of any state, so nothing the state machine does can
    /// reach it and the loop plays across every screen.
    #[test]
    fn the_track_is_not_scoped_to_a_state() {
        let mut app = with_assets();
        let entity = music(&mut app)[0];

        assert!(
            app.world()
                .inspect_entity(entity)
                .unwrap()
                .all(|c| !c.name().to_string().contains("DespawnOnExit"))
        );
    }

    /// The headless harnesses have no `AssetServer`; asking for one there is a
    /// panic, so the startup system takes it as an `Option` and stays quiet.
    #[test]
    fn a_shell_without_assets_is_silent_rather_than_broken() {
        let mut app = without_assets();

        assert!(music(&mut app).is_empty());
    }
}
