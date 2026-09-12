//! A stand-in for the duel, so the overworld can be walked end to end before
//! the combat engine (#9) and its UI (#11) land.
//!
//! It honours the seam exactly as ADR-0001 describes it: it consumes the
//! `Encounter`, mutates `RunState.stack`, inserts `CombatOutcome` and makes the
//! one transition combat is allowed to make, `Combat -> PostCombat`. The
//! handover is deleting this file, its `pub mod combat_stub;` line in
//! `mod.rs`, and the `CombatStubPlugin` line in `main.rs`.

use bevy::prelude::*;

use crate::overworld::screens::Screen;
use crate::run::{CombatOutcome, Encounter, RunState};
use crate::state::AppState;

pub struct CombatStubPlugin;

impl Plugin for CombatStubPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Combat), show_stub)
            .add_systems(Update, pick_outcome.run_if(in_state(AppState::Combat)));
    }
}

fn show_stub(mut commands: Commands, encounter: Res<Encounter>, run: Res<RunState>) {
    Screen::new()
        .title("[ combat stub ]")
        .prose(format!(
            "{} sits down with {} chips behind a House Edge of {}.\nYou have {}.",
            encounter.enemy.name, encounter.enemy.stack, encounter.enemy.house_edge, run.stack
        ))
        .option(1, "Win the duel")
        .option(2, "Lose the duel")
        .footer("The real duel arrives with #9 and #11.")
        .spawn(&mut commands, AppState::Combat);
}

fn pick_outcome(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut run: ResMut<RunState>,
    mut next: ResMut<NextState<AppState>>,
) {
    let outcome = match crate::overworld::screens::digit_pressed(&keys) {
        Some(1) => CombatOutcome::Won,
        Some(2) => CombatOutcome::Lost,
        _ => return,
    };

    if outcome == CombatOutcome::Lost {
        run.stack = 0;
    }

    commands.remove_resource::<Encounter>();
    commands.insert_resource(outcome);
    next.set(AppState::PostCombat);
}
