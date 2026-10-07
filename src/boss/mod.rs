//! Every boss, one file each (ADR-0004). A boss is its Deck, its Boss Tell,
//! its Table Rule, what beating it pays, and the words and face it comes
//! with. Adding one is a new file here and a line in [`BOSSES`]; nothing
//! outside this module asks which boss it is.
//!
//! A boss doesn't know its floor: the floor layout lists bosses.

mod pit_boss;
mod slotz;
mod the_house;

pub use pit_boss::PIT_BOSS;
pub use slotz::SLOTZ;
pub use the_house::THE_HOUSE;

use crate::modifier::Modifier;
use crate::run::{Card, Reward, Tell};

/// Every boss in the game.
pub static BOSSES: [&Boss; 3] = [&SLOTZ, &PIT_BOSS, &THE_HOUSE];

#[derive(Debug)]
pub struct Boss {
    pub name: &'static str,
    pub chips: u32,
    /// The portrait's path under `assets/`.
    pub portrait: &'static str,
    /// What the player reads as it sits down.
    pub intro: &'static str,
    /// What the player reads on clearing its Chips.
    pub win: &'static str,
    /// Written by hand, and carrying its Boss Tell.
    pub deck: &'static [Card],
    /// The Tell its Deck is built around. Nobody else has it until this boss
    /// is beaten in the run.
    pub tell: Tell,
    /// The rule it bends its own fight with, if it has one yet.
    pub table_rule: Option<&'static dyn Modifier>,
    /// The 1-of-2 it pays after its Boss Pack. `None` for The House, which
    /// pays in an ending. The card-giving ones stand until the boss
    /// proposals (#146) replace them with Perks.
    pub rewards: Option<[Reward; 2]>,
    /// How many of the Boss Pack's seven cards carry its Boss Tell.
    pub pack_tells: u8,
}

/// Two bosses are the same boss when they are the same definition.
impl PartialEq for Boss {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// A card of a boss's hand-written Deck.
const fn card(name: &'static str, face_value: u32, tell: Option<Tell>) -> Card {
    Card {
        name,
        face_value,
        tell,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_deck_carries_its_own_boss_tell_and_no_one_elses() {
        for boss in BOSSES {
            assert!(
                boss.deck.iter().any(|c| c.tell == Some(boss.tell)),
                "{}'s Deck has no {}",
                boss.name,
                boss.tell.name()
            );
            for other in BOSSES.iter().filter(|other| *other != &boss) {
                assert!(
                    boss.deck.iter().all(|c| c.tell != Some(other.tell)),
                    "{}'s Deck holds {}'s {}",
                    boss.name,
                    other.name,
                    other.tell.name()
                );
            }
        }
    }

    #[test]
    fn no_boss_tell_is_open_from_the_start() {
        for boss in BOSSES {
            assert!(
                !Tell::OPEN.contains(&boss.tell),
                "{}'s {} is open from the start, so beating it unlocks nothing",
                boss.name,
                boss.tell.name()
            );
        }
    }

    #[test]
    fn no_boss_pack_holds_more_boss_tells_than_cards() {
        for boss in BOSSES {
            assert!(boss.pack_tells <= 7, "{}", boss.name);
        }
    }
}
