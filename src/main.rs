//! App builder. Owned by Dev 2. Plugins are added here and nowhere else.

mod overworld;
mod run;
mod state;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(overworld::OverworldPlugin)
        // Stands in for combat until #9 and #11 land. Delete this line then.
        .add_plugins(overworld::combat_stub::CombatStubPlugin)
        .run();
}
