//! The Item pool (#153). An Item is a modifier (ADR-0004) with a set number of
//! uses, spent on demand: a use lasts that Hand, or the rest of the
//! encounter where the Item says so. A Perk never runs out; an Item does.
//!
//! Each Item is one static here, listed in [`ITEMS`]. Its uses and its offer
//! weight are starting points: the balance sim (#94) sets the real numbers,
//! and reads them off these fields.

use crate::combat::duel::{Coin, Opposing, Placed, Played};
use crate::modifier::{Modifier, Side, Turn};
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

/// How often an Item turns up, read off its weight, so retuning a weight on
/// the sim moves the Item between rarities with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
}

/// The lightest weight a common Item has.
pub const COMMON_WEIGHT: u32 = 8;
/// The lightest weight an uncommon Item has. Anything lighter is rare.
pub const UNCOMMON_WEIGHT: u32 = 5;

impl Item {
    pub fn rarity(&self) -> Rarity {
        if self.weight >= COMMON_WEIGHT {
            Rarity::Common
        } else if self.weight >= UNCOMMON_WEIGHT {
            Rarity::Uncommon
        } else {
            Rarity::Rare
        }
    }
}

/// Up to `count` different Items, none of them already `held`, each drawn by
/// its weight. Fewer when fewer are left to offer.
pub fn offer(held: &[Held], count: usize, seed: u64) -> Vec<&'static Item> {
    offer_where(held, count, seed, |_| true)
}

/// [`offer`], from only the Items `keep` lets through.
pub fn offer_where(
    held: &[Held],
    count: usize,
    seed: u64,
    keep: impl Fn(&Item) -> bool,
) -> Vec<&'static Item> {
    let mut left: Vec<&'static Item> = ITEMS
        .iter()
        .copied()
        .filter(|item| keep(item) && held.iter().all(|h| h.item != *item))
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
    fn reveal(&self, _turn: u32, opposing: &[Opposing]) -> Vec<usize> {
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
    fn reveal(&self, _turn: u32, opposing: &[Opposing]) -> Vec<usize> {
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
    fn blind(&self, side: Side, _turn: Turn, blind: u8) -> u8 {
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
    fn whiff(&self, whiff: u32) -> u32 {
        whiff / 2
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
    fn every_rarity_has_an_item_in_it() {
        for rarity in [Rarity::Common, Rarity::Uncommon, Rarity::Rare] {
            assert!(
                ITEMS.iter().any(|item| item.rarity() == rarity),
                "{rarity:?}"
            );
        }
        assert_eq!(LOADED_DICE.rarity(), Rarity::Common);
        assert_eq!(SHINY_CARD_SLEEVE.rarity(), Rarity::Rare);
    }

    #[test]
    fn a_narrowed_offer_only_holds_what_it_lets_through() {
        for seed in 1..100 {
            let offered = offer_where(&[], 3, seed, |item| item.rarity() == Rarity::Uncommon);
            assert_eq!(offered.len(), 3, "there are three uncommons");
            assert!(offered.iter().all(|item| item.rarity() == Rarity::Uncommon));
        }
    }

    #[test]
    fn insurance_leaves_half_the_whiff_rounded_down() {
        for (short, left) in [(7, 3), (6, 3), (1, 0), (0, 0)] {
            assert_eq!(Insurance.whiff(short), left, "short by {short}");
        }
    }
}
