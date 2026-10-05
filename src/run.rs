//! Shared run-state and seam types. Vocabulary follows `CONTEXT.md`.
//!
//! Owned by Dev 1 (combat). Overworld reads these and calls the constructors
//! and `RunState::apply`; changes to this file go through a PR to Dev 1.
//! `Encounter` and `CombatOutcome` are the whole combat ↔ overworld seam.

use bevy::prelude::*;

use crate::boss::Boss;
use crate::modifier::Modifier;

// ---------------------------------------------------------------------------
// Cards
// ---------------------------------------------------------------------------

/// A single passive keyword on a card. At most one per card.
///
/// Every Tell reads the row by position, not by the order the cards were
/// picked up: Streak looks one slot to its left, Copycat one slot to its
/// right, Flop across at the Opposing Card and its neighbours. All of them read
/// Face Values, so no Tell ever depends on another Tell resolving first
/// and the two rows can be worked out in either order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tell {
    /// Doubles the card's Face Value if the card in the slot to its left has any Tell.
    Streak,
    /// Sacrifice another card from the Draw to add its Face Value to this card's.
    AllIn,
    /// Takes the Face Value of the card in the slot to its right, and none
    /// of its Tell; its own if it is the last card in the row.
    Copycat,
    /// Takes the Face Values of the Opposing Card across from it and that
    /// card's two neighbours, never its own.
    Flop,
    /// At the Showdown, set against the card across from it: the side with
    /// the lower Face Value loses the difference off its Chips. Counts its
    /// own Face Value toward its row.
    Bluff,
}

impl Tell {
    /// The Tells every run starts with. Every other Tell is a Boss Tell,
    /// locked until its boss is beaten.
    pub const OPEN: [Tell; 3] = [Tell::Streak, Tell::AllIn, Tell::Bluff];

    pub fn rule_text(&self) -> Vec<&'static str> {
        match self {
            Tell::Streak => vec![
                "Doubles this card's",
                "Face Value",
                "if the card in the slot to its left has any",
                "Tell",
                ".",
            ],
            Tell::AllIn => vec![
                "Burns",
                "another card from your",
                "Draw",
                "and adds its",
                "Face Value",
                "to",
                "The Hand",
                ".",
            ],
            Tell::Copycat => vec![
                "Takes the",
                "Face Value",
                "of the card in the slot to its right, and none of its",
                "Tell",
                ". Its own if nothing follows it.",
            ],
            Tell::Flop => vec![
                "Takes the",
                "Face Value",
                "of the",
                "Opposing Card",
                "across from it and of the cards either side of that. Never its own.",
            ],
            Tell::Bluff => vec![
                "At",
                "Confirm",
                ", set against the card across from it. The lower",
                "Face Value",
                "loses the difference off its side's",
                "Chips",
                ".",
            ],
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Tell::Streak => "Streak",
            Tell::AllIn => "All In",
            Tell::Copycat => "Copycat",
            Tell::Flop => "Flop",
            Tell::Bluff => "Bluff",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub name: &'static str,
    /// The number printed on the card: what it adds to its row before its Tell.
    pub face_value: u32,
    pub tell: Option<Tell>,
}

// ---------------------------------------------------------------------------
// Run-long modifiers
// ---------------------------------------------------------------------------

/// Chosen 1-of-2 after beating a boss, and kept for the run. Each one lives
/// in its boss's file and bends the duel through its [`Modifier`].
#[derive(Debug)]
pub struct Perk {
    /// The line the reward screen puts against the key that takes it.
    pub label: &'static str,
    pub modifier: &'static dyn Modifier,
}

/// A Perk is the one definition it was taken from.
impl PartialEq for Perk {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// A boss reward that adds cards to the deck. Today's card-giving Perks,
/// until the boss proposals (#146) replace them.
#[derive(Debug)]
pub struct CardReward {
    pub label: &'static str,
    /// The cards it adds, rolled off the seed the caller hands over.
    pub cards: fn(u64) -> Vec<Card>,
}

/// Dropped after beating a minion. Consumed by combat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    /// +5 to The Hand for the next `hands_left` Hands.
    LoadedDice { hands_left: u8 },
}

/// What the reward screen hands out. Overworld renders the choice and calls
/// `RunState::apply`; deck-changing rewards mutate the deck here, not in combat.
#[derive(Debug, Clone, Copy)]
pub enum Reward {
    LoadedDice,
    Perk(&'static Perk),
    Cards(&'static CardReward),
}

impl PartialEq for Reward {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::LoadedDice, Self::LoadedDice) => true,
            (Self::Perk(a), Self::Perk(b)) => std::ptr::eq(*a, *b),
            (Self::Cards(a), Self::Cards(b)) => std::ptr::eq(*a, *b),
            _ => false,
        }
    }
}

impl Reward {
    /// The line the reward screen puts against the key that takes it.
    pub fn label(self) -> &'static str {
        match self {
            Self::LoadedDice => "Loaded Dice. +5 to each of your next two Hands.",
            Self::Perk(perk) => perk.label,
            Self::Cards(cards) => cards.label,
        }
    }
}

/// What is on the table once an encounter is won. Overworld renders it;
/// `RunState::apply` does the mutating.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RewardOffer {
    /// A minion's drop: no choice, it is already in your pocket.
    Drop(Reward),
    /// A boss's pick, 1 of 2.
    Pick(Reward, Reward),
}

/// A sealed set of cards: the player keeps `keep` of them and the rest are
/// gone. After a boss it is the Boss Pack, opened before the Perk pick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pack {
    pub cards: Vec<Card>,
    pub keep: usize,
}

/// The Boss Pack: seven cards, keep two.
pub const BOSS_PACK_SIZE: usize = 7;
pub const BOSS_PACK_KEEP: usize = 2;

/// The Face Values Pack cards are dealt in, and how often one that isn't a
/// guaranteed Boss Tell carries a Tell. The starter deck's ranges: 2..=8
/// plain, and 2..=5 for a Boss Tell, which Copycat and Flop both print at
/// (#110, #113). Untuned: the balance sim sets these.
const PACK_FACES: std::ops::RangeInclusive<u32> = 2..=8;
const PACK_TELL_FACES: std::ops::RangeInclusive<u32> = 2..=5;
const PACK_TELL_PCT: u32 = 40;

impl Pack {
    /// What beating `boss` deals: `pack_tells` cards carrying its Boss Tell,
    /// and the rest regular cards off the run's pool, which by now holds
    /// that Tell too.
    pub fn boss(boss: &'static Boss, run: &RunState, seed: u64) -> Self {
        let mut pool = run.tell_pool();
        if !pool.contains(&boss.tell) {
            pool.push(boss.tell);
        }
        let tells = usize::from(boss.pack_tells).min(BOSS_PACK_SIZE);
        let mut rng = seed | 1;
        let mut cards: Vec<Card> = (0..tells)
            .map(|_| deal_card(PACK_TELL_FACES, 100, &[boss.tell], &mut rng))
            .collect();
        cards.extend(
            (tells..BOSS_PACK_SIZE).map(|_| deal_card(PACK_FACES, PACK_TELL_PCT, &pool, &mut rng)),
        );
        // Shuffled, so the Boss Tells aren't always the first keys.
        for i in (1..cards.len()).rev() {
            let j = (xorshift64(&mut rng) % (i as u64 + 1)) as usize;
            cards.swap(i, j);
        }
        Pack {
            cards,
            keep: BOSS_PACK_KEEP,
        }
    }
}

// ---------------------------------------------------------------------------
// Run state
// ---------------------------------------------------------------------------

/// Everything that persists across encounters within one run. Reset on Fold or
/// death via `RunState::new()`.
#[derive(Resource, Debug, Clone)]
pub struct RunState {
    /// The player's chips. Combat mutates this in place; 0 means the run is over.
    pub chips: u32,
    pub deck: Vec<Card>,
    /// In the order taken, which is the order the duel applies them in.
    pub perks: Vec<&'static Perk>,
    pub items: Vec<Item>,
    /// Every boss beaten this run. The Tells they unlocked follow from it.
    pub bosses_beaten: Vec<&'static Boss>,
}

impl RunState {
    /// A fresh run with the fixed starter deck and starting Chips.
    pub fn new() -> Self {
        RunState {
            chips: STARTING_CHIPS,
            deck: starter_deck(),
            perks: Vec::new(),
            items: Vec::new(),
            bosses_beaten: Vec::new(),
        }
    }

    /// Grant a reward. The only way perks, items, or the deck change between
    /// encounters.
    ///
    /// `seed` feeds the rewards that roll dice. The caller owns the
    /// randomness here for the same reason it owns the duel's shuffle: it
    /// keeps this whole file deterministic under test.
    pub fn apply(&mut self, reward: Reward, seed: u64) {
        match reward {
            Reward::LoadedDice => {
                // A fresh pair on top of an existing one adds Hands, not clutter.
                self.set_loaded_dice(self.loaded_dice().saturating_add(LOADED_DICE_HANDS));
            }
            Reward::Perk(perk) => self.take_perk(perk),
            Reward::Cards(cards) => self.deck.extend((cards.cards)(seed)),
        }
    }

    /// Keep the cards of `pack` at `picks` and let the rest go. Takes
    /// exactly as many as the Pack allows, each once, or nothing at all.
    pub fn keep(&mut self, pack: &Pack, picks: &[usize]) -> bool {
        let mut seen = Vec::with_capacity(picks.len());
        for &i in picks {
            if i >= pack.cards.len() || seen.contains(&i) {
                return false;
            }
            seen.push(i);
        }
        if picks.len() != pack.keep.min(pack.cards.len()) {
            return false;
        }
        self.deck
            .extend(picks.iter().map(|&i| pack.cards[i].clone()));
        true
    }

    /// Record a boss beaten, which unlocks its Boss Tell for the rest of the run.
    pub fn beat(&mut self, boss: &'static Boss) {
        if !self.bosses_beaten.contains(&boss) {
            self.bosses_beaten.push(boss);
        }
    }

    /// Every Tell this run has unlocked: the ones open from the start, then
    /// each beaten boss's Boss Tell. Minion decks, Packs and the Cage deal
    /// from this.
    pub fn tell_pool(&self) -> Vec<Tell> {
        let mut pool = Tell::OPEN.to_vec();
        for boss in &self.bosses_beaten {
            if !pool.contains(&boss.tell) {
                pool.push(boss.tell);
            }
        }
        pool
    }

    /// Hands still carrying the Loaded Dice bonus. Combat reads this on the
    /// way in and writes back what is left with `set_loaded_dice`.
    pub fn loaded_dice(&self) -> u8 {
        self.items
            .iter()
            .map(|Item::LoadedDice { hands_left }| *hands_left)
            .sum()
    }

    /// Write back what combat left of the Loaded Dice. A spent pair is thrown
    /// away rather than kept around at zero.
    pub fn set_loaded_dice(&mut self, hands_left: u8) {
        self.items
            .retain(|item| !matches!(item, Item::LoadedDice { .. }));
        if hands_left > 0 {
            self.items.push(Item::LoadedDice { hands_left });
        }
    }

    /// A perk is a thing you either have or don't; taking it twice is a no-op.
    fn take_perk(&mut self, perk: &'static Perk) {
        if !self.perks.contains(&perk) {
            self.perks.push(perk);
        }
    }
}

impl Default for RunState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Floors, encounters and enemies
// ---------------------------------------------------------------------------

/// The three floors of the casino, in the order Lucky Jack walks them.
// The first one really is called The Floor; the narrative names them.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Floor {
    TheFloor,
    ThePit,
    BigShotsTable,
}

impl Floor {
    /// The most cards either side may play in a turn here: 2 on The Floor
    /// and one more on each floor after it.
    pub fn blind(self) -> u8 {
        match self {
            Self::TheFloor => 2,
            Self::ThePit => 3,
            Self::BigShotsTable => 4,
        }
    }
}

/// The Arcade's practice hand plays every card the script walks through.
pub const PRACTICE_BLIND: u8 = 5;

/// Who is across the table. Inserted by overworld before
/// `NextState(AppState::Combat)`; combat removes it on exit.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub enum Encounter {
    Boss {
        boss: &'static Boss,
        floor: Floor,
    },
    Minion {
        floor: Floor,
    },
    /// The Arcade's demo dealer (#40). Never part of a run: it unlocks
    /// nothing, pays nothing, and doesn't read the Tell pool.
    Practice,
}

impl Encounter {
    /// The floor it is fought on. `None` for the Arcade.
    pub fn floor(self) -> Option<Floor> {
        match self {
            Self::Boss { floor, .. } | Self::Minion { floor } => Some(floor),
            Self::Practice => None,
        }
    }

    /// The Blind both sides start the duel with.
    pub fn blind(self) -> u8 {
        self.floor().map_or(PRACTICE_BLIND, Floor::blind)
    }

    /// Who sits down. A minion's Deck is dealt here off `seed`, from every
    /// Tell `run` has unlocked so far.
    pub fn enemy(self, run: &RunState, seed: u64) -> Enemy {
        match self {
            Self::Boss { boss, .. } => Enemy {
                name: boss.name,
                chips: boss.chips,
                deck: boss.deck.to_vec(),
                table_rule: boss.table_rule,
            },
            Self::Minion { floor } => {
                let table = MinionTable::on(floor);
                Enemy {
                    name: table.name,
                    chips: table.chips,
                    deck: minion_deck(floor, &run.tell_pool(), seed),
                    table_rule: None,
                }
            }
            Self::Practice => Enemy {
                name: "THE DEMO DEALER",
                chips: 30,
                deck: Vec::new(),
                table_rule: None,
            },
        }
    }

    /// The Boss Pack beating it deals, opened before its Perk pick. `None`
    /// for a minion and the Arcade. The House has one, its Flop Pack, but
    /// its win goes to the ending, so nothing opens it yet.
    pub fn boss_pack(self, run: &RunState, seed: u64) -> Option<Pack> {
        match self {
            Self::Boss { boss, .. } => Some(Pack::boss(boss, run, seed)),
            Self::Minion { .. } | Self::Practice => None,
        }
    }

    /// What beating it pays. `None` for The House, which pays in an ending,
    /// and for the Arcade.
    pub fn reward_offer(self) -> Option<RewardOffer> {
        match self {
            Self::Boss { boss, .. } => boss.rewards.map(|[one, two]| RewardOffer::Pick(one, two)),
            Self::Minion { .. } => Some(RewardOffer::Drop(Reward::LoadedDice)),
            Self::Practice => None,
        }
    }
}

/// Whoever is across the table, as the duel needs them.
#[derive(Debug, Clone)]
pub struct Enemy {
    pub name: &'static str,
    /// The enemy's chips; 0 means the encounter is won.
    pub chips: u32,
    /// What it draws from, in draw order before the duel shuffles it.
    pub deck: Vec<Card>,
    /// A boss's rule for its own fight, applied before anything else.
    pub table_rule: Option<&'static dyn Modifier>,
}

/// The names the house deals under. Flavour only: a dealt card is a Face
/// Value and a Tell, and the name is what the Peek puts at the top of the tag.
const HOUSE_CARDS: [&str; 10] = [
    "Dealer's Nod",
    "Chip Rack",
    "Table Limit",
    "Floorman's Eye",
    "Comped Suite",
    "Shoe of Eights",
    "The Rake",
    "Cut Card",
    "Pit Marker",
    "Eye in the Sky",
];

/// One card off the house's table: a Face Value in `faces`, and a Tell drawn
/// uniformly from `pool` `tell_pct` times in 100. Rolls the Face Value, then
/// whether it carries a Tell, then which, always in that order, so a change
/// to the pool doesn't re-deal the Face Values of a pinned seed. Minion decks
/// and Packs are dealt with it.
pub fn deal_card(
    faces: std::ops::RangeInclusive<u32>,
    tell_pct: u32,
    pool: &[Tell],
    rng: &mut u64,
) -> Card {
    let span = u64::from(faces.end().saturating_sub(*faces.start()) + 1);
    let face_value = faces.start() + (xorshift64(rng) % span) as u32;
    let carries = !pool.is_empty() && xorshift64(rng) % 100 < u64::from(tell_pct);
    let tell = carries.then(|| pool[(xorshift64(rng) % pool.len() as u64) as usize]);
    let name = HOUSE_CARDS[(xorshift64(rng) % HOUSE_CARDS.len() as u64) as usize];
    Card {
        name,
        face_value,
        tell,
    }
}

/// The standard template a minion's Deck is dealt from, one row per floor.
/// Untuned: the balance sim sets these.
struct MinionTable {
    name: &'static str,
    chips: u32,
    size: usize,
    faces: std::ops::RangeInclusive<u32>,
    /// Chance in 100 that a card carries a Tell.
    tell_pct: u32,
}

impl MinionTable {
    fn on(floor: Floor) -> Self {
        match floor {
            Floor::TheFloor => MinionTable {
                name: "A shill in a rented tux",
                chips: 25,
                size: 14,
                faces: 2..=5,
                tell_pct: 20,
            },
            Floor::ThePit => MinionTable {
                name: "A dealer with a scar",
                chips: 32,
                size: 14,
                faces: 4..=7,
                tell_pct: 30,
            },
            Floor::BigShotsTable => MinionTable {
                name: "A high roller's minder",
                chips: 36,
                size: 16,
                faces: 4..=8,
                tell_pct: 35,
            },
        }
    }
}

/// A minion's Deck for `floor`, its Tells drawn uniformly from `pool`.
pub fn minion_deck(floor: Floor, pool: &[Tell], seed: u64) -> Vec<Card> {
    let table = MinionTable::on(floor);
    let mut rng = seed | 1;
    (0..table.size)
        .map(|_| deal_card(table.faces.clone(), table.tell_pct, pool, &mut rng))
        .collect()
}

/// Inserted by combat immediately before `NextState(AppState::PostCombat)`.
/// `RunState.chips` has already been updated by then. Overworld removes it
/// once it has routed.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatOutcome {
    Won,
    Lost,
}

/// Lucky Jack sits down with this many chips (#8).
pub const STARTING_CHIPS: u32 = 50;

/// The one xorshift64 the whole game rolls on: the duel's reshuffle, the
/// deal into the Draw, the Push Your Luck coin, minion decks, Packs, and the
/// Pit Boss's card Perk.
/// Not cryptography - it is a card game, and one stream is easier to reason
/// about than three.
pub fn xorshift64(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// Loaded Dice: this much on The Hand, for this many Hands (#12).
pub const LOADED_DICE_BONUS: u32 = 5;
pub const LOADED_DICE_HANDS: u8 = 2;

/// The fixed starter deck (#3): 18 cards, 44% with a Tell. Ten vanilla
/// cards 2..=8, four Streak 3..=6, four All In 2..=5. Prototyped on the
/// `prototype/starter-deck` branch; a careless player fires Streak about a
/// third of the time, a careful one nearly always, and that gap is the game.
pub fn starter_deck() -> Vec<Card> {
    let vanilla = [
        ("Two of Clubs", 2),
        ("Cheap Seat", 3),
        ("Comped Drink", 3),
        ("Four of Hearts", 4),
        ("Bus Ticket Home", 4),
        ("Five of Spades", 5),
        ("Borrowed Watch", 5),
        ("Six of Diamonds", 6),
        ("Marked Card", 7),
        ("Pawned Ring", 8),
    ];
    let streak = [
        ("Hot Streak", 3),
        ("Lucky Seat", 4),
        ("Dealer Blinks", 5),
        ("Table Runs Hot", 6),
    ];
    let all_in = [
        ("Last Dollar", 2),
        ("Car Keys", 3),
        ("Deed to the House", 4),
        ("Firstborn", 5),
    ];
    vanilla
        .into_iter()
        .map(|(name, face_value)| Card {
            name,
            face_value,
            tell: None,
        })
        .chain(streak.into_iter().map(|(name, face_value)| Card {
            name,
            face_value,
            tell: Some(Tell::Streak),
        }))
        .chain(all_in.into_iter().map(|(name, face_value)| Card {
            name,
            face_value,
            tell: Some(Tell::AllIn),
        }))
        .collect()
}

/// The Arcade's fixed deal (#40), in Draw order: every mechanic fires once
/// in the scripted first turn. Never shuffled.
pub fn tutorial_deal() -> Vec<Card> {
    let deck = starter_deck();
    let pick = |name: &str| {
        deck.iter()
            .find(|c| c.name == name)
            .expect("tutorial card is in the starter deck")
            .clone()
    };
    let draw = [
        "Pawned Ring",
        "Last Dollar",
        "Two of Clubs",
        "Hot Streak",
        "Dealer Blinks",
        "Four of Hearts",
        "Cheap Seat",
    ];
    // `Duel` draws from the end, so the first card dealt goes last; the rest
    // of the deck follows in starter order for the free-play turn.
    let mut rest: Vec<Card> = deck
        .iter()
        .filter(|c| !draw.contains(&c.name))
        .cloned()
        .collect();
    rest.extend(draw.iter().rev().map(|n| pick(n)));
    rest
}

/// The Arcade's fixed Opposing Cards (#40), left to right, with whether the
/// cabinet deals each one face up. Five slots for the five cards the script
/// walks the player through, adding up to 20 — the number the old House Edge
/// used to be handed as a flat total — with 12 of it showing and 8 face down,
/// so the first thing the tutorial teaches about the enemy's row is that you
/// never see all of it.
pub fn tutorial_opposing() -> Vec<(Card, bool)> {
    [
        ("Cut Card", 5, true),
        ("Chip Rack", 4, false),
        ("Dealer's Nod", 4, true),
        ("The Rake", 4, false),
        ("Table Limit", 3, true),
    ]
    .into_iter()
    .map(|(name, face_value, face_up)| {
        (
            Card {
                name,
                face_value,
                tell: None,
            },
            face_up,
        )
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss::{PIT_BOSS, SLOTZ, THE_HOUSE};

    /// Any old seed; the rewards that roll dice only have to be deterministic.
    const SEED: u64 = 0x1234_5678_9abc_def0;

    /// The two rewards Slotz puts on the table, perk first.
    fn slotz() -> [Reward; 2] {
        SLOTZ.rewards.expect("Slotz pays a 1-of-2")
    }

    fn pit_boss() -> [Reward; 2] {
        PIT_BOSS.rewards.expect("the Pit Boss pays a 1-of-2")
    }

    fn tells(cards: &[Card]) -> usize {
        cards.iter().filter(|c| c.tell.is_some()).count()
    }

    #[test]
    fn a_fresh_run_carries_nothing_from_the_last_one() {
        let run = RunState::new();

        assert_eq!(run.chips, STARTING_CHIPS);
        assert_eq!(run.deck.len(), starter_deck().len());
        assert!(run.perks.is_empty());
        assert!(run.items.is_empty());
        assert!(run.bosses_beaten.is_empty());
        assert_eq!(run.loaded_dice(), 0);
    }

    #[test]
    fn a_fresh_run_has_only_the_open_tells_unlocked() {
        assert_eq!(
            RunState::new().tell_pool(),
            vec![Tell::Streak, Tell::AllIn, Tell::Bluff]
        );
    }

    #[test]
    fn beating_a_boss_unlocks_its_boss_tell_and_nothing_else() {
        let mut run = RunState::new();

        run.beat(&SLOTZ);

        assert_eq!(
            run.tell_pool(),
            vec![Tell::Streak, Tell::AllIn, Tell::Bluff, Tell::Copycat]
        );
    }

    #[test]
    fn a_boss_tell_already_in_the_pool_is_not_listed_twice() {
        let mut run = RunState::new();

        // The Pit Boss carries Streak until its own Boss Tell is picked.
        run.beat(&PIT_BOSS);
        run.beat(&THE_HOUSE);
        run.beat(&THE_HOUSE);

        assert_eq!(
            run.tell_pool(),
            vec![Tell::Streak, Tell::AllIn, Tell::Bluff, Tell::Flop]
        );
    }

    #[test]
    fn a_minion_deck_is_dealt_inside_its_floors_range() {
        let pool = [Tell::Streak, Tell::AllIn];

        for seed in 1..50 {
            let deck = minion_deck(Floor::TheFloor, &pool, seed);
            assert_eq!(deck.len(), 14);
            assert!(deck.iter().all(|c| (2..=5).contains(&c.face_value)));

            let pit = minion_deck(Floor::ThePit, &pool, seed);
            assert!(pit.iter().all(|c| (4..=7).contains(&c.face_value)));
        }
    }

    #[test]
    fn a_minion_only_holds_tells_the_run_has_unlocked() {
        let pool = [Tell::Copycat];

        let dealt: Vec<Card> = (1..50)
            .flat_map(|seed| minion_deck(Floor::ThePit, &pool, seed))
            .collect();

        assert!(tells(&dealt) > 0);
        assert!(
            dealt
                .iter()
                .all(|c| c.tell.is_none() || c.tell == Some(Tell::Copycat))
        );
        assert!(
            minion_deck(Floor::ThePit, &[], SEED)
                .iter()
                .all(|c| c.tell.is_none()),
            "an empty pool deals plain cards"
        );
    }

    #[test]
    fn a_minion_draws_every_unlocked_tell_about_as_often() {
        let pool = [Tell::Streak, Tell::AllIn, Tell::Copycat, Tell::Flop];
        let dealt: Vec<Card> = (1..400)
            .flat_map(|seed| minion_deck(Floor::BigShotsTable, &pool, seed))
            .collect();

        let count = |tell| dealt.iter().filter(|c| c.tell == Some(tell)).count();
        let telled = tells(&dealt);
        for tell in pool {
            let share = count(tell) * 100 / telled;
            assert!(
                (18..=32).contains(&share),
                "{tell:?} was {share}% of the Tells"
            );
        }
    }

    #[test]
    fn a_minion_deck_is_dealt_off_its_seed() {
        let pool = RunState::new().tell_pool();

        assert_eq!(
            minion_deck(Floor::TheFloor, &pool, SEED),
            minion_deck(Floor::TheFloor, &pool, SEED)
        );
        assert_ne!(
            minion_deck(Floor::TheFloor, &pool, SEED),
            minion_deck(Floor::TheFloor, &pool, SEED ^ 0xffff)
        );
    }

    #[test]
    fn a_minion_is_dealt_from_the_runs_pool() {
        let mut run = RunState::new();
        run.beat(&THE_HOUSE);

        let enemy = Encounter::Minion {
            floor: Floor::ThePit,
        }
        .enemy(&run, SEED);

        assert_eq!(
            enemy.deck,
            minion_deck(Floor::ThePit, &run.tell_pool(), SEED)
        );
    }

    #[test]
    fn a_boss_sits_down_with_its_own_deck() {
        let enemy = Encounter::Boss {
            boss: &SLOTZ,
            floor: Floor::TheFloor,
        }
        .enemy(&RunState::new(), SEED);

        assert_eq!(enemy.name, SLOTZ.name);
        assert_eq!(enemy.chips, SLOTZ.chips);
        assert_eq!(enemy.deck, SLOTZ.deck);
    }

    #[test]
    fn the_blind_is_two_on_the_floor_and_one_more_each_floor_up() {
        assert_eq!(Floor::TheFloor.blind(), 2);
        assert_eq!(Floor::ThePit.blind(), 3);
        assert_eq!(Floor::BigShotsTable.blind(), 4);
    }

    #[test]
    fn every_encounter_worth_beating_puts_something_on_the_table() {
        let minion = Encounter::Minion {
            floor: Floor::TheFloor,
        };
        let boss = |boss| Encounter::Boss {
            boss,
            floor: Floor::ThePit,
        };

        assert_eq!(
            minion.reward_offer(),
            Some(RewardOffer::Drop(Reward::LoadedDice))
        );
        assert_eq!(
            boss(&SLOTZ).reward_offer(),
            Some(RewardOffer::Pick(slotz()[0], slotz()[1]))
        );
        // The House pays out in an ending, not a perk, and the Arcade in nothing.
        assert_eq!(boss(&THE_HOUSE).reward_offer(), None);
        assert_eq!(Encounter::Practice.reward_offer(), None);
    }

    fn slotz_pack(run: &RunState, seed: u64) -> Pack {
        Encounter::Boss {
            boss: &SLOTZ,
            floor: Floor::TheFloor,
        }
        .boss_pack(run, seed)
        .expect("a boss deals a Boss Pack")
    }

    #[test]
    fn a_boss_pack_is_seven_cards_keep_two() {
        let pack = slotz_pack(&RunState::new(), SEED);

        assert_eq!(pack.cards.len(), 7);
        assert_eq!(pack.keep, 2);
    }

    #[test]
    fn a_boss_pack_carries_at_least_its_share_of_the_boss_tell() {
        for seed in 1..200 {
            let pack = slotz_pack(&RunState::new(), seed);
            let copycats = pack
                .cards
                .iter()
                .filter(|c| c.tell == Some(Tell::Copycat))
                .count();
            assert!(copycats >= usize::from(SLOTZ.pack_tells), "seed {seed}");
        }
    }

    #[test]
    fn the_rest_of_a_boss_pack_comes_off_the_runs_pool() {
        let mut run = RunState::new();
        run.beat(&SLOTZ);

        let dealt: Vec<Card> = (1..200)
            .flat_map(|seed| slotz_pack(&run, seed).cards)
            .collect();

        assert!(dealt.iter().any(|c| c.tell.is_none()), "some regular cards");
        assert!(dealt.iter().any(|c| c.tell == Some(Tell::Streak)));
        assert!(
            dealt.iter().all(|c| c.tell != Some(Tell::Flop)),
            "The House isn't beaten, so its Boss Tell stays locked"
        );
        assert!(dealt.iter().all(|c| (2..=8).contains(&c.face_value)));
    }

    #[test]
    fn a_boss_pack_is_dealt_off_its_seed() {
        let run = RunState::new();

        assert_eq!(slotz_pack(&run, SEED), slotz_pack(&run, SEED));
        assert_ne!(slotz_pack(&run, SEED), slotz_pack(&run, SEED ^ 0xffff));
    }

    #[test]
    fn only_a_boss_deals_a_boss_pack() {
        let run = RunState::new();

        assert_eq!(
            Encounter::Minion {
                floor: Floor::TheFloor
            }
            .boss_pack(&run, SEED),
            None
        );
        assert_eq!(Encounter::Practice.boss_pack(&run, SEED), None);
    }

    #[test]
    fn keeping_two_adds_those_two_and_lets_the_rest_go() {
        let mut run = RunState::new();
        let pack = slotz_pack(&run, SEED);
        let before = run.deck.len();

        assert!(run.keep(&pack, &[4, 1]));

        assert_eq!(
            run.deck[before..],
            [pack.cards[4].clone(), pack.cards[1].clone()]
        );
    }

    #[test]
    fn a_pack_takes_exactly_its_keep_each_card_once_or_nothing() {
        let mut run = RunState::new();
        let pack = slotz_pack(&run, SEED);
        let deck = run.deck.clone();

        for picks in [&[][..], &[0], &[0, 1, 2], &[3, 3], &[0, 7]] {
            assert!(!run.keep(&pack, picks), "{picks:?}");
        }
        assert_eq!(run.deck, deck);
    }

    #[test]
    fn loaded_dice_are_good_for_the_next_two_hands() {
        let mut run = RunState::new();

        run.apply(Reward::LoadedDice, SEED);

        assert_eq!(run.loaded_dice(), LOADED_DICE_HANDS);
    }

    #[test]
    fn a_second_pair_of_loaded_dice_adds_hands_rather_than_items() {
        let mut run = RunState::new();

        run.apply(Reward::LoadedDice, SEED);
        run.apply(Reward::LoadedDice, SEED);

        assert_eq!(run.items.len(), 1);
        assert_eq!(run.loaded_dice(), LOADED_DICE_HANDS * 2);
    }

    #[test]
    fn spent_loaded_dice_leave_nothing_in_the_pocket() {
        let mut run = RunState::new();
        run.apply(Reward::LoadedDice, SEED);

        run.set_loaded_dice(1);
        assert_eq!(run.loaded_dice(), 1);

        run.set_loaded_dice(0);
        assert_eq!(run.loaded_dice(), 0);
        assert!(run.items.is_empty());
    }

    #[test]
    fn the_slotz_card_reward_adds_three_streak_cards() {
        let mut run = RunState::new();
        let before = run.deck.len();

        run.apply(slotz()[1], SEED);

        let added = &run.deck[before..];
        assert_eq!(added.len(), 3);
        assert!(added.iter().all(|c| c.tell == Some(Tell::Streak)));
    }

    #[test]
    fn a_perk_is_the_only_thing_that_changes_and_is_taken_once() {
        let mut run = RunState::new();
        let deck = run.deck.len();

        run.apply(slotz()[0], SEED);
        run.apply(slotz()[0], SEED);

        assert_eq!(run.perks.len(), 1);
        assert_eq!(Reward::Perk(run.perks[0]), slotz()[0]);
        assert_eq!(run.deck.len(), deck);
    }

    #[test]
    fn the_pit_boss_card_reward_adds_two_tells_and_two_plain_cards() {
        let mut run = RunState::new();
        let before = run.deck.len();

        run.apply(pit_boss()[1], SEED);

        let added = &run.deck[before..];
        assert_eq!(added.len(), 4);
        assert_eq!(tells(added), 2);
        assert!(added.iter().all(|c| (2..=8).contains(&c.face_value)));

        let names: std::collections::HashSet<_> = added.iter().map(|c| c.name).collect();
        assert_eq!(names.len(), 4, "no pack deals the same card twice");
    }

    #[test]
    fn the_pit_boss_cards_are_rolled_from_the_seed() {
        let mut one = RunState::new();
        let mut two = RunState::new();
        let mut same = RunState::new();

        one.apply(pit_boss()[1], SEED);
        two.apply(pit_boss()[1], SEED ^ 0xffff);
        same.apply(pit_boss()[1], SEED);

        assert_eq!(one.deck, same.deck, "the same seed deals the same cards");
        assert_ne!(one.deck, two.deck, "a different seed deals different ones");
    }

    #[test]
    fn the_arcades_opposing_row_is_twenty_chips_with_eight_of_them_face_down() {
        let row = tutorial_opposing();

        assert_eq!(row.len(), 5, "one slot per card the script plays");
        assert_eq!(row.iter().map(|(c, _)| c.face_value).sum::<u32>(), 20);
        let hidden: u32 = row
            .iter()
            .filter(|(_, face_up)| !face_up)
            .map(|(c, _)| c.face_value)
            .sum();
        assert_eq!(hidden, 8);
        assert!(row.iter().all(|(c, _)| c.tell.is_none()), "a practice hand");
    }

    #[test]
    fn every_reward_says_what_it_is() {
        let mut rewards = vec![Reward::LoadedDice];
        rewards.extend(slotz());
        rewards.extend(pit_boss());
        for reward in rewards {
            assert!(!reward.label().is_empty());
        }
    }
}
