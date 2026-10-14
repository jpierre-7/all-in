//! THE HOUSE, the last boss. Its Boss Tell is Flop.
//!
//! **Table Rule: The House Plays Last.** Everyone else commits face down at
//! the start of the turn. The House waits for Confirm, looks at the row you
//! put down, and lays its own across from it, so every Flop it plays reads
//! exactly what you gave it. There is nothing to reveal before Confirm.
//!
//! The Deck is built around Flop: Flops to read your row, Streaks that
//! double beside them, and plain cards to fill a slot when your row is too
//! thin for a Flop to be worth it.

use super::{Boss, card};
use crate::modifier::Modifier;
use crate::run::Tell;

const STREAK: Option<Tell> = Some(Tell::Streak);
const FLOP: Option<Tell> = Some(Tell::Flop);

pub static THE_HOUSE: Boss = Boss {
    name: "THE HOUSE",
    chips: 35,
    portrait: "portraits/the_house.png",
    intro: "\
THE HOUSE. No face, just a pair of hands resting on the felt and a
voice that comes from the walls. \"Sit down, Jack. Let's see what you
learned. You go first. The House always plays last.\"",
    win: "\
The House's hands go still on the felt.

For the first time in twenty-five years, the table is yours.",
    deck: &[
        card("Comp Ticket", 2, None),
        card("Matchbook", 2, None),
        card("Valet Stub", 3, None),
        card("Ashtray", 3, None),
        card("Room Key", 4, None),
        card("Pit Note", 4, None),
        card("House Always", 3, STREAK),
        card("House Wins", 4, STREAK),
        card("The River", 2, FLOP),
        card("The Board", 3, FLOP),
    ],
    tell: Tell::Flop,
    table_rule: Some(&PLAYS_LAST),
    rewards: None,
    pack_tells: 3,
};

/// The House Plays Last: it commits at Confirm, after seeing your row.
#[derive(Debug)]
struct PlaysLast;

static PLAYS_LAST: PlaysLast = PlaysLast;

impl Modifier for PlaysLast {
    fn plays_last(&self, _plays_last: bool) -> bool {
        true
    }
}
