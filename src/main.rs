//! App builder. Owned by Dev 2. Plugins are added here and nowhere else.

mod combat;
mod overworld;
mod run;
mod state;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(overworld::OverworldPlugin)
        .add_plugins(combat::CombatPlugin)
        .run();
}
