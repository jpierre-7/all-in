//! THE PIT BOSS, the boss of The Pit. Its Boss Tell is Counterweight.
//!
//! A brass scale: its Deck is light cards that weigh whatever you put across
//! them. The Counterweights match your heavy cards, so the only way round
//! them is to play fewer, and you can't see where they sit without a reveal.
//! Its Table Rule, The Beam Swings Back, raises its Blind the turn after you
//! win one.

use super::{Boss, card};
use crate::combat::duel::Opposing;
use crate::modifier::{Modifier, Side, Turn};
use crate::run::{Perk, Reward, Tell};

const COUNTERWEIGHT: Option<Tell> = Some(Tell::Counterweight);
const STREAK: Option<Tell> = Some(Tell::Streak);

pub static PIT_BOSS: Boss = Boss {
    name: "THE PIT BOSS",
    chips: 35,
    portrait: "portraits/pit_boss.png",
    intro: "\
THE PIT BOSS. A brass balance scale the height of a man, two pans
hanging off a beam that creaks when it turns to look at you. One pan is
already piled with chips; the other holds nothing yet. \"Jack. Twenty-five
years, and you came back with those Chips? Put it on the pan. Let's see
what it's worth.\"",
    win: "\
The beam tips your way and stays there, and for a long moment the only
sound in the Pit is brass settling. \"Upstairs,\" it says at last. \"He's
been expecting you.\"",
    deck: &[
        card("Brass Weight", 4, None),
        card("Full Pan", 5, None),
        card("Tipped Beam", 6, None),
        card("Heavy Stack", 7, None),
        card("Counterweight", 3, COUNTERWEIGHT),
        card("Balanced Books", 3, COUNTERWEIGHT),
        card("Even Money", 4, COUNTERWEIGHT),
        card("Equal Measure", 4, COUNTERWEIGHT),
        card("Thumb on the Scale", 5, COUNTERWEIGHT),
        // A Counterweight carries a Tell, so a Streak after one doubles.
        card("Tipping Point", 4, STREAK),
        card("Double Weight", 5, STREAK),
    ],
    tell: Tell::Counterweight,
    table_rule: Some(&BeamSwingsBack),
    rewards: Some([
        Reward::Perk(&BEAM_SWINGS_YOUR_WAY),
        Reward::Perk(&READ_THE_PAN),
    ]),
    pack_tells: 3,
};

/// The Beam Swings Back: the turn after the player wins one, the Pit Boss's
/// Blind is 1 higher. Two wins in a row are still only 1.
#[derive(Debug)]
struct BeamSwingsBack;

impl Modifier for BeamSwingsBack {
    fn blind(&self, side: Side, blind: u8, turn: Turn) -> u8 {
        match side {
            Side::Enemy if turn.player_won_last() => blind.saturating_add(1),
            _ => blind,
        }
    }
}

/// The turn after the player loses one, their Blind is 1 higher.
pub static BEAM_SWINGS_YOUR_WAY: Perk = Perk {
    label: "Beam Swings Your Way: lose a turn, and your Blind is one card higher the next.",
    modifier: &BeamSwingsYourWay,
};

#[derive(Debug)]
struct BeamSwingsYourWay;

impl Modifier for BeamSwingsYourWay {
    fn blind(&self, side: Side, blind: u8, turn: Turn) -> u8 {
        match side {
            Side::Player if turn.player_lost_last() => blind.saturating_add(1),
            _ => blind,
        }
    }
}

/// The heaviest Opposing Card turns face up at the start of every turn.
pub static READ_THE_PAN: Perk = Perk {
    label: "Read the Pan: the heaviest Opposing Card turns face up every turn. Not against The House.",
    modifier: &ReadThePan,
};

#[derive(Debug)]
struct ReadThePan;

impl Modifier for ReadThePan {
    /// The highest Face Value, the leftmost of a tie. The House lays nothing
    /// down until Confirm, so against it there is nothing here to turn.
    fn reveal(&self, _turn: u32, opposing: &[Opposing]) -> Vec<usize> {
        let heaviest = opposing
            .iter()
            .enumerate()
            .rev()
            .max_by_key(|(_, o)| o.card.face_value)
            .map(|(slot, _)| slot);
        heaviest.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::duel::Outcome;
    use crate::run::Card;

    /// `modifier`'s answer for `side` at a floor Blind of 3, the turn after
    /// one that dealt `last`.
    fn blind_after(modifier: &dyn Modifier, side: Side, last: Option<Outcome>) -> u8 {
        let turn = match last {
            None => Turn::FIRST,
            last => Turn { number: 2, last },
        };
        modifier.blind(side, 3, turn)
    }

    #[test]
    fn the_beam_swings_back_after_a_payout_and_only_then() {
        let enemy = |last| blind_after(&BeamSwingsBack, Side::Enemy, last);
        assert_eq!(enemy(None), 3, "turn one");
        assert_eq!(enemy(Some(Outcome::Payout(5))), 4);
        assert_eq!(enemy(Some(Outcome::Payout(0))), 3, "a tie");
        assert_eq!(enemy(Some(Outcome::Whiff(5))), 3);
        assert_eq!(
            blind_after(&BeamSwingsBack, Side::Player, Some(Outcome::Payout(5))),
            3
        );
    }

    #[test]
    fn beam_swings_your_way_after_a_whiff_and_only_then() {
        let player = |last| blind_after(&BeamSwingsYourWay, Side::Player, last);
        assert_eq!(player(None), 3, "turn one");
        assert_eq!(player(Some(Outcome::Whiff(5))), 4);
        assert_eq!(player(Some(Outcome::Whiff(0))), 3, "a Push forgave it");
        assert_eq!(player(Some(Outcome::Payout(5))), 3);
        assert_eq!(
            blind_after(&BeamSwingsYourWay, Side::Enemy, Some(Outcome::Whiff(5))),
            3
        );
    }

    fn face_down(face_value: u32) -> Opposing {
        Opposing {
            card: Card {
                name: "weight",
                face_value,
                tell: None,
            },
            sacrifice: None,
            face_up: false,
        }
    }

    #[test]
    fn read_the_pan_turns_the_heaviest_the_leftmost_of_a_tie() {
        let row = [face_down(4), face_down(8), face_down(2), face_down(8)];
        assert_eq!(ReadThePan.reveal(1, &row), vec![1]);
        assert_eq!(ReadThePan.reveal(1, &[]), Vec::<usize>::new());
    }
}
