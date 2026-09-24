//! App builder. Owned by Dev 2. Plugins are added here and nowhere else.
#![windows_subsystem = "windows"]

mod combat;
#[cfg(debug_assertions)]
mod devstart;
mod music;
mod overworld;
mod run;
mod state;
mod theme;

use bevy::prelude::*;

fn main() {
    // The dev entry point brackets the plugins (#33): read the flags before
    // them, because `WinitPlugin` builds an event loop in its `build` and a
    // typo should not cost a window; install after them, so what it sets
    // overwrites what the overworld set. A release build has neither the
    // module nor the flags.
    #[cfg(debug_assertions)]
    let dev = devstart::read();

    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "All In".into(),
            ..default()
        }),
        ..default()
    }))
    .add_plugins(theme::ThemePlugin)
    .add_plugins(music::MusicPlugin)
    .add_plugins(overworld::OverworldPlugin)
    .add_plugins(combat::CombatPlugin);

    #[cfg(debug_assertions)]
    if let Some(dev) = dev {
        dev.install(&mut app);
    }

    app.run();
}
