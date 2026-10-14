//! SLOTZ, the boss of The Floor. Its Boss Tell is Copycat.
//!
//! Its Deck is reels: plain symbols, and Copycats that land on whatever sits
//! beside them. Its Table Rule, All Night, gives it more reels the longer
//! the duel runs, and every extra slot is room for another Copycat.
//!
//! The prints are low on purpose. At its full Blind of 4 Slotz lays twice
//! the player's row, so the Deck, its Chips and All Night are tuned together
//! on the balance sim: a fight that runs long gets hard, not hopeless.

use super::{Boss, card};
use crate::combat::duel::{Coin, Played};
use crate::modifier::{Modifier, Side, Turn};
use crate::run::{Perk, Reward, Tell};

const COPYCAT: Option<Tell> = Some(Tell::Copycat);

pub static SLOTZ: Boss = Boss {
    name: "SLOTZ",
    chips: 26,
    portrait: "portraits/slotz.png",
    intro: "\
SLOTZ. Three feet of chrome and neon on a rolling base, arms spinning,
grinning the way only a machine can. It doesn't want your money; it wants
your time, and it's got all night.",
    win: "\
The reels spin once more, land on nothing, and the neon goes out. Somewhere
off in the dark, something bigger clears its throat.",
    deck: &[
        card("Cherry", 1, None),
        card("Lemon", 2, None),
        card("Plum", 2, None),
        card("Orange", 3, None),
        card("Melon", 3, None),
        card("Bell", 4, None),
        card("Bar", 4, None),
        card("Double Bar", 5, None),
        card("Lucky Seven", 7, None),
        card("Mirror Reel", 1, COPYCAT),
        card("Matching Pair", 1, COPYCAT),
        card("Nudge", 2, COPYCAT),
        card("Hold", 2, COPYCAT),
        card("Three of a Kind", 3, COPYCAT),
    ],
    tell: Tell::Copycat,
    table_rule: Some(&ALL_NIGHT),
    rewards: Some([Reward::Perk(&BEST_TWO_OF_THREE), Reward::Perk(&JACKPOT)]),
    pack_tells: 3,
};

/// How many turns Slotz plays at each Blind before All Night raises it.
pub const ALL_NIGHT_EVERY: u32 = 3;
/// The most All Night raises Slotz's Blind above the floor's.
pub const ALL_NIGHT_CAP: u8 = 2;

/// All Night: Slotz's Blind goes up by 1 every [`ALL_NIGHT_EVERY`] turns, to
/// at most [`ALL_NIGHT_CAP`] above the floor's. As the Table Rule it bends
/// first, so the Blind it's handed is the floor's.
#[derive(Debug)]
struct AllNight;

static ALL_NIGHT: AllNight = AllNight;

impl Modifier for AllNight {
    fn blind(&self, side: Side, turn: Turn, blind: u8) -> u8 {
        match side {
            Side::Player => blind,
            Side::Enemy => {
                let raised =
                    (turn.number.saturating_sub(1) / ALL_NIGHT_EVERY).min(u32::from(ALL_NIGHT_CAP));
                blind.saturating_add(raised as u8)
            }
        }
    }
}

/// Push Your Luck becomes best 2 of 3, at 49/51, or better if a Perk taken
/// before it already weighted the coin further (Lucky Streak).
pub static BEST_TWO_OF_THREE: Perk = Perk {
    label: "Push Your Luck becomes best 2 of 3, at 49/51.",
    modifier: &BestTwoOfThree,
};

#[derive(Debug)]
struct BestTwoOfThree;

impl Modifier for BestTwoOfThree {
    fn coin(&self, coin: Coin) -> Coin {
        // 3p² - 2p³ at 49% is about 48.5%: nearly a fair coin.
        Coin {
            player_pct: coin.player_pct.max(49),
            best_of: 3,
        }
    }
}

/// What Jackpot adds to The Hand.
pub const JACKPOT_BONUS: u32 = 5;

/// Jackpot: three cards in the row that came to the same number, Tells
/// resolved, pay [`JACKPOT_BONUS`] on top of The Hand. A Copycat counts as
/// what it copied. A card that came to nothing, mucked or a Flop across
/// empty slots, never lines up.
pub static JACKPOT: Perk = Perk {
    label: "Jackpot: +5 to The Hand when three cards in your row come to the same number.",
    modifier: &Jackpot,
};

#[derive(Debug)]
struct Jackpot;

impl Modifier for Jackpot {
    fn after_showdown(&self, hand: u32, _house_edge: u32, row: &[Played]) -> u32 {
        let lined_up = row.iter().any(|played| {
            played.value > 0 && row.iter().filter(|p| p.value == played.value).count() >= 3
        });
        if lined_up { hand + JACKPOT_BONUS } else { hand }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::Card;

    fn played(value: u32) -> Played {
        Played {
            card: card("Reel", value, None),
            value,
            mucked: false,
        }
    }

    /// What came of the turn before doesn't matter to All Night.
    fn turn(number: u32) -> Turn {
        Turn { number, last: None }
    }

    fn enemy_blind(number: u32, floor_blind: u8) -> u8 {
        ALL_NIGHT.blind(Side::Enemy, turn(number), floor_blind)
    }

    #[test]
    fn all_night_raises_slotz_blind_every_three_turns_up_to_two() {
        let blinds: Vec<u8> = (1..=12).map(|turn| enemy_blind(turn, 2)).collect();
        assert_eq!(blinds, [2, 2, 2, 3, 3, 3, 4, 4, 4, 4, 4, 4]);
    }

    #[test]
    fn all_night_leaves_the_players_blind_alone() {
        assert_eq!(ALL_NIGHT.blind(Side::Player, turn(9), 2), 2);
    }

    #[test]
    fn jackpot_pays_on_three_alike() {
        let row = [played(6), played(2), played(6), played(6)];
        assert_eq!(Jackpot.after_showdown(20, 0, &row), 20 + JACKPOT_BONUS);
    }

    #[test]
    fn jackpot_pays_once_however_many_line_up() {
        let row: Vec<Played> = (0..6).map(|_| played(4)).collect();
        assert_eq!(Jackpot.after_showdown(24, 0, &row), 24 + JACKPOT_BONUS);
    }

    #[test]
    fn jackpot_needs_three() {
        let row = [played(6), played(6), played(5)];
        assert_eq!(Jackpot.after_showdown(17, 0, &row), 17);
    }

    #[test]
    fn jackpot_reads_what_a_copycat_resolved_to() {
        let copycat = Played {
            card: Card {
                name: "Mirror Reel",
                face_value: 2,
                tell: COPYCAT,
            },
            value: 6,
            mucked: false,
        };
        let row = [copycat, played(6), played(6)];
        assert_eq!(Jackpot.after_showdown(18, 0, &row), 18 + JACKPOT_BONUS);
    }

    #[test]
    fn cards_that_came_to_nothing_never_line_up() {
        let mucked = Played {
            mucked: true,
            ..played(0)
        };
        let row = [mucked.clone(), mucked.clone(), mucked];
        assert_eq!(Jackpot.after_showdown(0, 0, &row), 0);
    }
}
