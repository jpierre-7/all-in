//! App builder. Owned by Dev 2. Plugins are added here and nowhere else.

mod run;
mod state;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<state::AppState>()
        .run();
}
