//! SLOTZ, the boss of The Floor. Its Boss Tell is Copycat.
//!
//! The Deck is a stand-in until the boss proposals pick its real one: the
//! Face Values and Tells it used to be dealt from (3 to 7, Streak and
//! Copycat), written out as cards.

use super::{Boss, card};
use crate::combat::duel::Coin;
use crate::modifier::Modifier;
use crate::run::{Card, CardReward, Perk, Reward, Tell};

const STREAK: Option<Tell> = Some(Tell::Streak);
const COPYCAT: Option<Tell> = Some(Tell::Copycat);

pub static SLOTZ: Boss = Boss {
    name: "SLOTZ",
    chips: 32,
    portrait: "portraits/slotz.png",
    intro: "\
SLOTZ. Three feet of chrome and neon on a rolling base, arms spinning,
grinning the way only a machine can. It doesn't want your money; it wants
your time, and it's got all night.",
    win: "\
The reels spin once more, land on nothing, and the neon goes out. Somewhere
off in the dark, something bigger clears its throat.",
    deck: &[
        card("Cherry", 3, None),
        card("Lemon", 4, None),
        card("Plum", 4, None),
        card("Orange", 5, None),
        card("Melon", 5, None),
        card("Bar", 6, None),
        card("Double Bar", 7, None),
        card("Lucky Seven", 7, None),
        card("Hot Reel", 4, STREAK),
        card("Spin Again", 5, STREAK),
        card("Free Spins", 6, STREAK),
        card("Mirror Reel", 3, COPYCAT),
        card("Matching Pair", 4, COPYCAT),
        card("Three of a Kind", 5, COPYCAT),
    ],
    tell: Tell::Copycat,
    table_rule: None,
    rewards: Some([
        Reward::Perk(&BEST_TWO_OF_THREE),
        Reward::Cards(&STREAK_CARDS),
    ]),
    pack_tells: 3,
};

/// Push Your Luck becomes best 2 of 3, at 49/51.
pub static BEST_TWO_OF_THREE: Perk = Perk {
    label: "Push Your Luck becomes best 2 of 3, at 49/51.",
    modifier: &BestTwoOfThree,
};

#[derive(Debug)]
struct BestTwoOfThree;

impl Modifier for BestTwoOfThree {
    fn coin(&self, _coin: Coin) -> Coin {
        // 3p² - 2p³ at 49% is about 48.5%: nearly a fair coin.
        Coin {
            player_pct: 49,
            best_of: 3,
        }
    }
}

/// Three more Streak cards, all middling, so the reward is a better chance
/// of firing Streak rather than a higher ceiling.
pub static STREAK_CARDS: CardReward = CardReward {
    label: "Three more Streak cards in the deck.",
    cards: streak_cards,
};

fn streak_cards(_seed: u64) -> Vec<Card> {
    ["Loose Slot", "Second Cherry", "Jackpot Bell"]
        .into_iter()
        .map(|name| card(name, 4, STREAK))
        .collect()
}
