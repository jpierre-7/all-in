//! THE PIT BOSS, the boss of The Pit. Its Boss Tell isn't picked yet, so it
//! carries Streak as a stand-in.
//!
//! The Deck is a stand-in until the boss proposals pick its real one: the
//! Face Values it used to be dealt from (4 to 9), with only the Tells open
//! from the start, since the other two belong to Slotz and The House.

use super::{Boss, card};
use crate::modifier::{Modifier, Side};
use crate::run::{Card, CardReward, Perk, Reward, Tell, xorshift64};

const STREAK: Option<Tell> = Some(Tell::Streak);
const ALL_IN: Option<Tell> = Some(Tell::AllIn);

pub static PIT_BOSS: Boss = Boss {
    name: "THE PIT BOSS",
    chips: 40,
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
        card("Counterweight", 5, None),
        card("Full Pan", 5, None),
        card("Tipped Beam", 6, None),
        card("Heavy Stack", 7, None),
        card("Lead Chip", 8, None),
        card("Thumb on the Scale", 9, None),
        card("Balanced Books", 4, STREAK),
        card("Even Money", 5, STREAK),
        card("Double Weight", 6, STREAK),
        card("Tipping Point", 7, STREAK),
        card("House Marker", 3, ALL_IN),
        card("Collateral", 4, ALL_IN),
        card("Everything on the Pan", 5, ALL_IN),
    ],
    tell: Tell::Streak,
    table_rule: None,
    rewards: Some([Reward::Perk(&SIXTH_CARD), Reward::Cards(&RANDOM_CARDS)]),
    pack_tells: 3,
};

/// One more card in the player's row every turn.
pub static SIXTH_CARD: Perk = Perk {
    label: "Your Blind is one card higher.",
    modifier: &BlindPlusOne,
};

#[derive(Debug)]
struct BlindPlusOne;

impl Modifier for BlindPlusOne {
    fn blind(&self, side: Side, blind: u8) -> u8 {
        match side {
            Side::Player => blind.saturating_add(1),
            Side::Enemy => blind,
        }
    }
}

/// Four cards off the Pit's own table: two with a random Tell and two plain.
pub static RANDOM_CARDS: CardReward = CardReward {
    label: "Four cards off the Pit's table: two with Tells, two plain.",
    cards: random_cards,
};

/// Each card's Tell is any of the four, whatever the run has unlocked. Face
/// Values stay inside the starter deck's ranges so the pack thickens the deck
/// without rewriting its maths.
fn random_cards(seed: u64) -> Vec<Card> {
    const TELLED: [&str; 6] = [
        "Sleeve Ace",
        "Tipped Dealer",
        "Marker from the Pit",
        "Cooler Deck",
        "Late Bet",
        "Chip on the Rail",
    ];
    const PLAIN: [&str; 6] = [
        "House Matchbook",
        "Parking Stub",
        "Cocktail Napkin",
        "Loose Change",
        "Cigarette Burn",
        "Plastic Chip",
    ];

    let mut rng = seed | 1;
    let mut roll = |n: u64| xorshift64(&mut rng) % n;
    // Names come out of the hat rather than off it, so a pack never holds the
    // same card twice.
    let mut telled = TELLED.to_vec();
    let mut plain = PLAIN.to_vec();

    let mut cards = Vec::with_capacity(4);
    for _ in 0..2 {
        let name = telled.swap_remove(roll(telled.len() as u64) as usize);
        // Each Tell keeps the range the starter deck gives it: Streak 3..=6,
        // All In 2..=5. Copycat prints 2..=5 too (#110): the print only
        // counts when no card follows, so a low one pushes it into the
        // sequence rather than the last slot. Flop's print never counts for
        // itself, only for a Copycat or an enemy Flop reading it (#113).
        let (tell, face_value) = match roll(4) {
            0 => (Tell::Streak, 3 + roll(4) as u32),
            1 => (Tell::AllIn, 2 + roll(4) as u32),
            2 => (Tell::Copycat, 2 + roll(4) as u32),
            _ => (Tell::Flop, 2 + roll(4) as u32),
        };
        cards.push(Card {
            name,
            face_value,
            tell: Some(tell),
        });
    }
    for _ in 0..2 {
        cards.push(Card {
            name: plain.swap_remove(roll(plain.len() as u64) as usize),
            face_value: 2 + roll(7) as u32, // 2..=8, the starter deck's vanilla range
            tell: None,
        });
    }
    cards
}
