//! Shared run-state and seam types. Vocabulary follows `CONTEXT.md`.
//!
//! Owned by Dev 1 (combat). Overworld reads these and calls the constructors
//! and `RunState::apply`; changes to this file go through a PR to Dev 1.
//! `Encounter` and `CombatOutcome` are the whole combat ↔ overworld seam.

#![allow(dead_code)] // perks, items, rewards and enemy numbers land with #12 and #8

use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Cards
// ---------------------------------------------------------------------------

/// A single passive keyword on a card. At most one per card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tell {
    /// Doubles the card's Stack if the previous card played this turn had any Tell.
    Streak,
    /// Sacrifice another card from the Draw to add its Stack to The Hand.
    AllIn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub name: &'static str,
    /// The chips this card contributes to The Hand.
    pub stack: u32,
    pub tell: Option<Tell>,
}

// ---------------------------------------------------------------------------
// Run-long modifiers
// ---------------------------------------------------------------------------

/// Chosen 1-of-2 after beating a boss. Persists for the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Perk {
    /// Slotz option 1: Push Your Luck is best 2-of-3 at 49/51.
    PylBestTwoOfThree,
    /// Pit Boss option 1: 6 Plays per turn, but Rising Blinds are +4 every turn.
    SixPlaysSteepBlinds,
}

/// Dropped after beating a minion. Consumed by combat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    /// +5 to The Hand for the next `hands_left` Hands.
    LoadedDice { hands_left: u8 },
}

/// What the reward screen hands out. Overworld renders the choice and calls
/// `RunState::apply`; deck-changing rewards mutate the deck here, not in combat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reward {
    LoadedDice,
    /// Slotz option 2: add 3 Streak cards to the deck.
    SlotzStreakCards,
    SlotzPylBestTwoOfThree,
    PitBossSixPlays,
    /// Pit Boss option 2: add 4 random cards (2 with random Tells, 2 vanilla).
    PitBossRandomCards,
}

// ---------------------------------------------------------------------------
// Run state
// ---------------------------------------------------------------------------

/// Everything that persists across encounters within one run. Reset on Fold or
/// death via `RunState::new()`.
#[derive(Resource, Debug, Clone)]
pub struct RunState {
    /// The player's chips. Combat mutates this in place; 0 means the run is over.
    pub stack: u32,
    pub deck: Vec<Card>,
    pub perks: Vec<Perk>,
    pub items: Vec<Item>,
}

impl RunState {
    /// A fresh run with the fixed starter deck and starting Stack.
    pub fn new() -> Self {
        RunState {
            stack: STARTING_STACK,
            deck: starter_deck(),
            perks: Vec::new(),
            items: Vec::new(),
        }
    }

    /// Grant a reward. The only way perks, items, or the deck change between
    /// encounters.
    pub fn apply(&mut self, reward: Reward) {
        let _ = reward;
        todo!("#12")
    }

    /// Plays per turn after perks. 5 by default; the Pit Boss perk is #12.
    pub fn plays(&self) -> u8 {
        5
    }
}

impl Default for RunState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Enemies and encounters
// ---------------------------------------------------------------------------

/// Which encounter the overworld is entering. Overworld's floor table is a
/// sequence of these; Dev 1 owns what each one means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EncounterId {
    FloorMinion,
    Slotz,
    PitMinion,
    PitBoss,
    TheHouse,
}

impl EncounterId {
    /// Bosses hand out a perk pick; minions drop an item.
    pub fn is_boss(self) -> bool {
        !matches!(self, Self::FloorMinion | Self::PitMinion)
    }
}

/// How House Edge escalates as combat goes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RisingBlinds {
    pub every_turns: u8,
    pub increase: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enemy {
    pub name: &'static str,
    /// The enemy's chips; 0 means the encounter is won.
    pub stack: u32,
    /// Starting House Edge. The House overrides this per turn (#5).
    pub house_edge: u32,
    pub blinds: RisingBlinds,
}

impl Enemy {
    /// The single place enemy numbers live (#8).
    pub fn for_encounter(id: EncounterId) -> Self {
        let _ = id;
        todo!("#8")
    }
}

/// Inserted by overworld before `NextState(AppState::Combat)`.
/// Combat removes it on exit.
#[derive(Resource, Debug, Clone)]
pub struct Encounter {
    pub id: EncounterId,
    pub enemy: Enemy,
}

/// Inserted by combat immediately before `NextState(AppState::PostCombat)`.
/// `RunState.stack` has already been updated by then. Overworld removes it
/// once it has routed.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatOutcome {
    Won,
    Lost,
}

/// Placeholder until #8 sets the real number.
pub const STARTING_STACK: u32 = 40;

/// The fixed starter deck (#3): 18 cards, 44% with a Tell. Ten vanilla
/// cards 2..=8, four Streak 3..=6, four All In 2..=5. Prototyped on the
/// `prototype/starter-deck` branch; a careless player fires Streak about a
/// third of the time, a careful one nearly always, and that gap is the game.
pub fn starter_deck() -> Vec<Card> {
    let vanilla = [
        ("Two of Clubs", 2),
        ("Cheap Seat", 3),
        ("Comped Drink", 3),
        ("Four of Hearts", 4),
        ("Bus Ticket Home", 4),
        ("Five of Spades", 5),
        ("Borrowed Watch", 5),
        ("Six of Diamonds", 6),
        ("Marked Card", 7),
        ("Pawned Ring", 8),
    ];
    let streak = [("Hot Streak", 3), ("Lucky Seat", 4), ("Dealer Blinks", 5), ("Table Runs Hot", 6)];
    let all_in = [("Last Dollar", 2), ("Car Keys", 3), ("Deed to the House", 4), ("Firstborn", 5)];
    vanilla
        .into_iter()
        .map(|(name, stack)| Card { name, stack, tell: None })
        .chain(streak.into_iter().map(|(name, stack)| Card { name, stack, tell: Some(Tell::Streak) }))
        .chain(all_in.into_iter().map(|(name, stack)| Card { name, stack, tell: Some(Tell::AllIn) }))
        .collect()
}
