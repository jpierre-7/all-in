//! The one way anything bends a duel (ADR-0004). A boss's Table Rule, a Perk
//! and an Item are the same kind of thing to the duel: a [`Modifier`] that
//! answers some of a fixed set of hooks and leaves the rest alone.
//!
//! The duel takes one list of them, in a fixed order: the Table Rule, then
//! Perks in the order taken, then Items, each seeing the answer the one
//! before it gave. A hook is added here only when a chosen boss, Perk or Item
//! needs it, and taken out again when nothing answers it.

use std::fmt::Debug;

use crate::combat::duel::{Coin, Placed};

/// Which side of the table a hook is asking about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Player,
    Enemy,
}

/// Both rows as they turn over at Confirm, before any Tell resolves. Slot `i`
/// of one faces slot `i` of the other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rows {
    pub player: Vec<Placed>,
    pub enemy: Vec<Placed>,
}

/// A Table Rule, a Perk or an Item. Every hook defaults to doing nothing.
pub trait Modifier: Debug + Sync {
    /// `side`'s Blind, given the answer so far. The duel never lets it drop
    /// below 1.
    fn blind(&self, _side: Side, blind: u8) -> u8 {
        blind
    }

    /// Bends Face Values or slots once both rows are down and before any Tell
    /// reads them, so every Tell still resolves in one pass.
    fn before_showdown(&self, _rows: &mut Rows) {}

    /// The coin Push Your Luck is flipped with.
    fn coin(&self, coin: Coin) -> Coin {
        coin
    }
}
