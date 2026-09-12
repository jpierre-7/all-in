//! App builder. Owned by Dev 2. Plugins are added here and nowhere else.

mod combat;
mod overworld;
mod run;
mod state;
mod theme;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "ALL-IN".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(theme::ThemePlugin)
        .add_plugins(overworld::OverworldPlugin)
        .add_plugins(combat::CombatPlugin)
        .run();
}
