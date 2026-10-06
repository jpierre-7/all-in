//! The Item pool (#153). An Item is a modifier (ADR-0004) with a set number of
//! uses, spent on demand: a use lasts that Hand, or the rest of the
//! encounter where the Item says so. A Perk never runs out; an Item does.
//!
//! Each Item is one static here, listed in [`ITEMS`]. Its uses and its offer
//! weight are starting points: the balance sim (#94) sets the real numbers,
//! and reads them off these fields.

use crate::combat::duel::{Coin, Opposing, Placed, Played};
use crate::modifier::{Modifier, Side};
use crate::run::xorshift64;

/// How long one use of an Item bends the duel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lasts {
    /// The Hand it was spent on.
    Hand,
    /// The rest of the encounter.
    Encounter,
}

/// When an Item can be spent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum When {
    /// While building the row, before Confirm.
    Row,
    /// At the Push / Hold prompt.
    Prompt,
}

#[derive(Debug)]
pub struct Item {
    pub name: &'static str,
    /// What one use does, for the screen.
    pub text: &'static str,
    /// The uses it comes with.
    pub uses: u8,
    pub lasts: Lasts,
    pub when: When,
    /// Its chance of being offered, against the other Items' weights.
    /// Stronger Items are rarer.
    pub weight: u32,
    pub modifier: &'static dyn Modifier,
}

/// An Item is the one definition it was taken from.
impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// An Item the player holds, and how many uses it has left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Held {
    pub item: &'static Item,
    pub uses: u8,
}

impl Held {
    /// A fresh one, with every use it comes with.
    pub fn new(item: &'static Item) -> Self {
        Held {
            item,
            uses: item.uses,
        }
    }
}

/// Every Item in the game. Fixed for the run: beating a boss adds none.
pub static ITEMS: [&Item; 10] = [
    &LOADED_DICE,
    &SUNGLASSES,
    &TWO_WAY_MIRROR,
    &ACE_UP_THE_SLEEVE,
    &SHAVED_CARD,
    &SLEIGHT_OF_HAND,
    &INSURANCE,
    &WEIGHTED_COIN,
    &SHINY_CARD_SLEEVE,
    &DEEP_POCKETS,
];

/// Up to `count` different Items, none of them already `held`, each drawn by
/// its weight. Fewer when fewer are left to offer.
pub fn offer(held: &[Held], count: usize, seed: u64) -> Vec<&'static Item> {
    let mut left: Vec<&'static Item> = ITEMS
        .iter()
        .copied()
        .filter(|item| held.iter().all(|h| h.item != *item))
        .collect();
    let mut rng = seed | 1;
    let mut offered = Vec::with_capacity(count);
    while offered.len() < count && !left.is_empty() {
        let total: u64 = left.iter().map(|item| u64::from(item.weight)).sum();
        let at = if total == 0 {
            0
        } else {
            let mut roll = xorshift64(&mut rng) % total;
            left.iter()
                .position(|item| {
                    let weight = u64::from(item.weight);
                    if roll < weight {
                        true
                    } else {
                        roll -= weight;
                        false
                    }
                })
                .expect("the roll is under the total")
        };
        offered.push(left.remove(at));
    }
    offered
}

// ---------------------------------------------------------------------------
// The ten
// ---------------------------------------------------------------------------

/// Loaded Dice: this much on The Hand.
pub const LOADED_DICE_BONUS: u32 = 5;
/// Shaved Card: this much on the first slot's Face Value.
pub const SHAVED_CARD_BONUS: u32 = 3;

pub static LOADED_DICE: Item = Item {
    name: "Loaded Dice",
    text: "+5 to The Hand.",
    uses: 2,
    lasts: Lasts::Hand,
    when: When::Row,
    weight: 10,
    modifier: &LoadedDice,
};

#[derive(Debug)]
struct LoadedDice;

impl Modifier for LoadedDice {
    fn after_showdown(&self, hand: u32, _house_edge: u32, _row: &[Played]) -> u32 {
        hand + LOADED_DICE_BONUS
    }
}

pub static SUNGLASSES: Item = Item {
    name: "Card Shark's Sunglasses",
    text: "Turn the leftmost face-down Opposing Card face up.",
    uses: 3,
    lasts: Lasts::Hand,
    when: When::Row,
    weight: 10,
    modifier: &Sunglasses,
};

#[derive(Debug)]
struct Sunglasses;

impl Modifier for Sunglasses {
    fn reveal(&self, opposing: &[Opposing]) -> Vec<usize> {
        opposing
            .iter()
            .position(|o| !o.face_up)
            .into_iter()
            .collect()
    }
}

pub static TWO_WAY_MIRROR: Item = Item {
    name: "Two-Way Mirror",
    text: "Turn every Opposing Card face up.",
    uses: 1,
    lasts: Lasts::Hand,
    when: When::Row,
    weight: 5,
    modifier: &TwoWayMirror,
};

#[derive(Debug)]
struct TwoWayMirror;

impl Modifier for TwoWayMirror {
    fn reveal(&self, opposing: &[Opposing]) -> Vec<usize> {
        (0..opposing.len()).collect()
    }
}

pub static ACE_UP_THE_SLEEVE: Item = Item {
    name: "Ace Up the Sleeve",
    text: "+1 to your Blind.",
    uses: 2,
    lasts: Lasts::Hand,
    when: When::Row,
    weight: 6,
    modifier: &AceUpTheSleeve,
};

#[derive(Debug)]
struct AceUpTheSleeve;

impl Modifier for AceUpTheSleeve {
    fn blind(&self, side: Side, blind: u8) -> u8 {
        match side {
            Side::Player => blind.saturating_add(1),
            Side::Enemy => blind,
        }
    }
}

pub static SHAVED_CARD: Item = Item {
    name: "Shaved Card",
    text: "+3 to the Face Value of the card in your first slot, before any Tell reads it.",
    uses: 2,
    lasts: Lasts::Hand,
    when: When::Row,
    weight: 8,
    modifier: &ShavedCard,
};

#[derive(Debug)]
struct ShavedCard;

impl Modifier for ShavedCard {
    fn before_showdown(&self, side: Side, mut row: Vec<Placed>) -> Vec<Placed> {
        if side == Side::Player
            && let Some(first) = row.first_mut()
        {
            first.card.face_value += SHAVED_CARD_BONUS;
        }
        row
    }
}

pub static SLEIGHT_OF_HAND: Item = Item {
    name: "Sleight of Hand",
    text: "The enemy's rightmost Opposing Card is Mucked.",
    uses: 1,
    lasts: Lasts::Hand,
    when: When::Row,
    weight: 5,
    modifier: &SleightOfHand,
};

#[derive(Debug)]
struct SleightOfHand;

impl Modifier for SleightOfHand {
    fn before_showdown(&self, side: Side, mut row: Vec<Placed>) -> Vec<Placed> {
        if side == Side::Enemy {
            row.pop();
        }
        row
    }
}

pub static INSURANCE: Item = Item {
    name: "Insurance",
    text: "A Whiff is halved, rounded down.",
    uses: 2,
    lasts: Lasts::Hand,
    when: When::Row,
    weight: 8,
    modifier: &Insurance,
};

#[derive(Debug)]
struct Insurance;

impl Modifier for Insurance {
    /// Raises The Hand by half the shortfall, rounded up, so the Whiff left
    /// is half of it rounded down and the House Edge still says what the
    /// Opposing Cards came to.
    fn after_showdown(&self, hand: u32, house_edge: u32, _row: &[Played]) -> u32 {
        let short = house_edge.saturating_sub(hand);
        hand + short.div_ceil(2)
    }
}

pub static WEIGHTED_COIN: Item = Item {
    name: "Weighted Coin",
    text: "Push Your Luck flips even instead of in the House's favour. Spent at the prompt.",
    uses: 2,
    lasts: Lasts::Hand,
    when: When::Prompt,
    weight: 8,
    modifier: &WeightedCoin,
};

#[derive(Debug)]
struct WeightedCoin;

impl Modifier for WeightedCoin {
    fn coin(&self, coin: Coin) -> Coin {
        Coin {
            player_pct: coin.player_pct.max(50),
            ..coin
        }
    }
}

pub static SHINY_CARD_SLEEVE: Item = Item {
    name: "Shiny Card Sleeve",
    text: "For the rest of the encounter, what your first slot resolves to counts twice. No Tell sees it.",
    uses: 1,
    lasts: Lasts::Encounter,
    when: When::Row,
    weight: 3,
    modifier: &ShinyCardSleeve,
};

#[derive(Debug)]
struct ShinyCardSleeve;

impl Modifier for ShinyCardSleeve {
    fn after_showdown(&self, hand: u32, _house_edge: u32, row: &[Played]) -> u32 {
        hand + row.first().map_or(0, |p| p.value)
    }
}

pub static DEEP_POCKETS: Item = Item {
    name: "Deep Pockets",
    text: "Draw a card now, and your Draw refills to 8 for the rest of the encounter.",
    uses: 1,
    lasts: Lasts::Encounter,
    when: When::Row,
    weight: 4,
    modifier: &DeepPockets,
};

#[derive(Debug)]
struct DeepPockets;

impl Modifier for DeepPockets {
    fn draw(&self, side: Side, size: usize) -> usize {
        match side {
            Side::Player => size + 1,
            Side::Enemy => size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_item_has_a_use_a_name_and_a_chance_of_being_offered() {
        for item in ITEMS {
            assert!(item.uses > 0, "{}", item.name);
            assert!(item.weight > 0, "{}", item.name);
            assert!(!item.name.is_empty() && !item.text.is_empty());
        }
    }

    #[test]
    fn no_two_items_share_a_name() {
        for (i, a) in ITEMS.iter().enumerate() {
            for b in &ITEMS[i + 1..] {
                assert_ne!(a.name, b.name);
            }
        }
    }

    #[test]
    fn the_offer_is_three_different_items() {
        for seed in 1..200 {
            let offered = offer(&[], 3, seed);
            assert_eq!(offered.len(), 3);
            assert_ne!(offered[0], offered[1]);
            assert_ne!(offered[1], offered[2]);
            assert_ne!(offered[0], offered[2]);
        }
    }

    #[test]
    fn the_offer_skips_what_is_already_held() {
        let held: Vec<Held> = ITEMS[..8].iter().map(|item| Held::new(item)).collect();
        for seed in 1..50 {
            let mut offered = offer(&held, 3, seed);
            offered.sort_by_key(|item| item.name);
            assert_eq!(offered.len(), 2, "only two are left to offer");
            assert!(
                offered
                    .iter()
                    .all(|item| held.iter().all(|h| h.item != *item))
            );
        }
    }

    #[test]
    fn the_rarer_items_turn_up_less_often() {
        let first = |item: &Item| {
            (1..4000)
                .filter(|&seed| offer(&[], 1, seed)[0] == item)
                .count()
        };
        assert!(first(&SHINY_CARD_SLEEVE) < first(&LOADED_DICE));
    }

    #[test]
    fn insurance_leaves_half_the_whiff_rounded_down() {
        for (hand, edge, whiff) in [(0, 7, 3), (4, 10, 3), (9, 10, 0), (12, 10, 0)] {
            let bent = Insurance.after_showdown(hand, edge, &[]);
            assert_eq!(edge.saturating_sub(bent), whiff, "{hand} against {edge}");
        }
    }
}
