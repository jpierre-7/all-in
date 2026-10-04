//! THE HOUSE, the last boss. Its Boss Tell is Flop.
//!
//! The Deck is a stand-in until its Table Rule is picked: the scraps it used
//! to be dealt from (1 to 4, Streak and Flop), written out as cards. Its
//! Table Rule is still the Hole Card, until that is removed.

use super::{Boss, card};
use crate::combat::duel::{Placed, resolve_row, row_value};
use crate::modifier::{Modifier, Rows};
use crate::run::{Card, Tell};

const STREAK: Option<Tell> = Some(Tell::Streak);
const FLOP: Option<Tell> = Some(Tell::Flop);

pub static THE_HOUSE: Boss = Boss {
    name: "THE HOUSE",
    chips: 35,
    portrait: "portraits/the_house.png",
    intro: "\
THE HOUSE. No face, just a pair of hands resting on the felt and a
voice that comes from the walls. \"Sit down, Jack. Let's see what you
learned. Play four. I'll set the line. Then show me your last card.\"",
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
    table_rule: Some(&HoleCard { margin: 1 }),
    rewards: None,
    pack_tells: 3,
};

/// The House's Hole Card (#5): at Confirm it sets its last Opposing Card so
/// that its row reads the player's row (everything but the player's own last
/// card) plus the margin. The player's last card is the one The House could
/// not see, and the Payout is whatever it is worth over the margin.
#[derive(Debug)]
pub struct HoleCard {
    /// How far above the row it read The House sets its own.
    pub margin: u32,
}

impl Modifier for HoleCard {
    /// The cards in front of the Hole Card are part of the total, not on top
    /// of it, so the Hole Card is the remainder. When they already make more
    /// than the margin asks for, it is worth nothing: The House can't un-deal
    /// a card to come back down.
    fn before_showdown(&self, rows: &mut Rows) {
        let Some(slot) = rows.enemy.len().checked_sub(1) else {
            return;
        };
        // Everything the player put down but their last card.
        let seen = rows.player.len().saturating_sub(1).min(slot);
        let across: Vec<Option<Card>> = rows.enemy[..seen]
            .iter()
            .map(|p| Some(p.card.clone()))
            .collect();
        let read = row_value(&resolve_row(&rows.player[..seen], &across));
        // What its own cards in front of it already make. They can't move
        // once it is set: The House holds no Copycat, the one Tell that would
        // read the card it hasn't decided on yet.
        let player: Vec<Option<Card>> = rows.player.iter().map(|p| Some(p.card.clone())).collect();
        let mine = row_value(&resolve_row(&rows.enemy[..slot], &player));

        rows.enemy[slot] = Placed::plain(Card {
            name: "The House's Hole Card",
            face_value: (read + self.margin).saturating_sub(mine),
            tell: None,
        });
    }
}
