//! The one flat app state machine. Frozen after the seam decision (ADR-0001):
//! overworld owns every transition except `Combat -> PostCombat`, which combat
//! sets when a duel ends.

use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// The marquee. Space, and only Space, starts the game.
    #[default]
    Title,
    /// Lucky Jack's backstory, shown once, on the way in from the Title.
    Opening,
    /// Three options: Info Room, Tutorial, Begin Run. A fourth, The Wheel,
    /// once The House has been beaten.
    Lobby,
    /// The Wheel: Golden Chips placed and moved between runs.
    Wheel,
    /// The start of a run once The Wheel is open: the spin, then Trim and
    /// Pocket Change if they have a choice to put.
    Spin,
    InfoRoom,
    Tutorial,
    /// Arrival prose for the current floor.
    FloorIntro,
    /// The Fight or Fold prompt for the next encounter.
    FightOrFold,
    /// Owned entirely by `combat`. Enter with an `Encounter` resource present.
    Combat,
    /// Set by `combat` on exit. Overworld reads `CombatOutcome` here and routes
    /// to `Reward` or `GameOver`.
    PostCombat,
    /// Pick-1-of-2 perk after a boss, or the item drop after a minion. After
    /// The House, its Flop Pack and the Legacy pick.
    Reward,
    /// The son's reveal. Reached only by beating The House, after its Legacy
    /// pick.
    Ending,
    /// Player Chips hit 0. Back to the Lobby from here.
    GameOver,
}
