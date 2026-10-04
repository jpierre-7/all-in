//! THE HOUSE, the last boss. Its Boss Tell is Flop.
//!
//! The Deck is a stand-in until its Table Rule is picked: the scraps it used
//! to be dealt from (1 to 4, Streak and Flop), written out as cards. Its
//! Table Rule waits on that pick too: until then it plays by the same rules
//! as everyone.

use super::{Boss, card};
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
learned. Same table, same rules as everybody else.\"",
    win: "\
The House's hands go still on the felt.

For the first time in twenty-five years, the table is yours.",
    deck: &[
        card("Comp Ticket", 1, None),
        card("Matchbook", 1, None),
        card("Valet Stub", 2, None),
        card("Ashtray", 2, None),
        card("Keno Slip", 3, None),
        card("Bent Chip", 3, None),
        card("Room Key", 4, None),
        card("Pit Note", 4, None),
        card("House Always", 2, STREAK),
        card("House Wins", 3, STREAK),
        card("The Turn", 2, FLOP),
        card("The River", 3, FLOP),
        card("The Board", 4, FLOP),
    ],
    tell: Tell::Flop,
    table_rule: None,
    rewards: None,
    pack_tells: 3,
};
