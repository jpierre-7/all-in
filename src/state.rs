//! The one flat app state machine. Frozen after the seam decision (ADR-0001):
//! overworld owns every transition except `Combat -> PostCombat`, which combat
//! sets when a duel ends.

use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// Lucky Jack's backstory, shown once on launch.
    #[default]
    Opening,
    /// Three options: Info Room, Tutorial, Begin Run.
    Lobby,
    InfoRoom,
    Tutorial,
    /// Arrival prose for the current floor.
    FloorIntro,
    /// The Fight or Fold prompt for the next encounter.
    FightOrFold,
    /// Owned entirely by `combat`. Enter with an `Encounter` resource present.
    Combat,
    /// Set by `combat` on exit. Overworld reads `CombatOutcome` here and routes
    /// to `Reward`, `Ending`, or `GameOver`.
    PostCombat,
    /// Pick-1-of-2 perk after a boss, or the item drop after a minion.
    Reward,
    /// The son's reveal. Reached only by beating The House.
    Ending,
    /// Player Stack hit 0. Back to the Lobby from here.
    GameOver,
}
