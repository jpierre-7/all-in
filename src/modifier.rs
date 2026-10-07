//! The one way anything bends a duel (ADR-0004). A boss's Table Rule, a Perk
//! and an Item are the same kind of thing to the duel: a [`Modifier`] that
//! answers some of a fixed set of hooks and leaves the rest alone.
//!
//! The duel takes one list of them, in a fixed order: the Table Rule, then
//! Perks in the order taken, then what The Wheel adds, then Items, each
//! seeing the answer the one before it gave. A hook is added here only when a chosen boss, Perk or Item
//! needs it, and taken out again when nothing answers it.

use std::fmt::Debug;

use crate::combat::duel::{Coin, Opposing, Outcome, Placed, Played};

/// Which side of the table a hook is asking about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Player,
    Enemy,
}

/// Where the duel stands as a turn starts: what a hook that bends the turn
/// may read about the turns before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Turn {
    /// Counting from 1.
    pub number: u32,
    /// What the turn before dealt, once Push Your Luck was settled. `None`
    /// on the first turn.
    pub last: Option<Outcome>,
}

impl Turn {
    /// The first turn of a duel.
    pub const FIRST: Turn = Turn {
        number: 1,
        last: None,
    };

    /// The player dealt a Payout last turn. A tie is not a win.
    pub fn player_won_last(&self) -> bool {
        matches!(self.last, Some(Outcome::Payout(n)) if n > 0)
    }

    /// The player took a Whiff last turn. A tie, or a Whiff a Push forgave,
    /// is not a loss.
    pub fn player_lost_last(&self) -> bool {
        matches!(self.last, Some(Outcome::Whiff(n)) if n > 0)
    }
}

/// A Table Rule, a Perk or an Item. Every hook defaults to doing nothing.
pub trait Modifier: Debug + Sync {
    /// `side`'s Blind on `turn`, given the answer so far. The duel never lets
    /// it drop below 1.
    fn blind(&self, _side: Side, blind: u8, _turn: Turn) -> u8 {
        blind
    }

    /// How many cards `side`'s Draw refills to, given the answer so far.
    fn draw(&self, _side: Side, size: usize) -> usize {
        size
    }

    /// The slots of the Opposing Cards to turn face up before Confirm, as
    /// they lie after the modifiers before this one have had their say, on
    /// `turn` of the duel, counting from 1. Asked once the enemy has
    /// committed, and again whenever an Item is spent.
    fn reveal(&self, _turn: u32, _opposing: &[Opposing]) -> Vec<usize> {
        Vec::new()
    }

    /// `side`'s row as it turns over, before any Tell resolves, so every Tell
    /// reads the bent Face Values in the one pass. A card may only come off
    /// the right-hand end: its slot is then empty, and the card is Mucked.
    fn before_showdown(&self, _side: Side, row: Vec<Placed>) -> Vec<Placed> {
        row
    }

    /// The coin Push Your Luck is flipped with.
    fn coin(&self, coin: Coin) -> Coin {
        coin
    }

    /// The Hand once every Tell has resolved, given the answer so far, the
    /// House Edge it will meet, and the player's resolved row.
    fn after_showdown(&self, hand: u32, _house_edge: u32, _row: &[Played]) -> u32 {
        hand
    }

    /// The Whiff The Hand is short by, given the answer so far. Asked after
    /// every modifier's `after_showdown`, so it bends the Whiff that is
    /// actually left, whatever order the modifiers came in.
    fn whiff(&self, whiff: u32) -> u32 {
        whiff
    }
}
