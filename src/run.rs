//! Shared run-state and seam types. Vocabulary follows `CONTEXT.md`.
//!
//! Owned by Dev 1 (combat). Overworld reads these and calls the constructors
//! and `RunState::apply`; changes to this file go through a PR to Dev 1.
//! `Encounter` and `CombatOutcome` are the whole combat ↔ overworld seam.

use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Cards
// ---------------------------------------------------------------------------

/// A single passive keyword on a card. At most one per card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tell {
    /// Doubles the card's Stack if the previous card played this turn had any Tell.
    Streak,
    /// Sacrifice another card from the Draw to add its Stack to The Hand.
    AllIn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub name: &'static str,
    /// The chips this card contributes to The Hand.
    pub stack: u32,
    pub tell: Option<Tell>,
}

// ---------------------------------------------------------------------------
// Run-long modifiers
// ---------------------------------------------------------------------------

/// Chosen 1-of-2 after beating a boss. Persists for the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Perk {
    /// Slotz option 1: Push Your Luck is best 2-of-3 at 49/51.
    PylBestTwoOfThree,
    /// Pit Boss option 1: 6 Plays per turn, but Rising Blinds are +2 every turn.
    SixPlaysSteepBlinds,
}

/// Dropped after beating a minion. Consumed by combat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    /// +5 to The Hand for the next `hands_left` Hands.
    LoadedDice { hands_left: u8 },
}

/// What the reward screen hands out. Overworld renders the choice and calls
/// `RunState::apply`; deck-changing rewards mutate the deck here, not in combat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reward {
    LoadedDice,
    /// Slotz option 2: add 3 Streak cards to the deck.
    SlotzStreakCards,
    SlotzPylBestTwoOfThree,
    PitBossSixPlays,
    /// Pit Boss option 2: add 4 random cards (2 with random Tells, 2 vanilla).
    PitBossRandomCards,
}

impl Reward {
    /// The line the reward screen puts against the key that takes it.
    pub fn label(self) -> &'static str {
        match self {
            Self::LoadedDice => "Loaded Dice. +5 to each of your next two Hands.",
            Self::SlotzStreakCards => "Three more Streak cards in the deck.",
            Self::SlotzPylBestTwoOfThree => "Push Your Luck becomes best 2 of 3, at 49/51.",
            Self::PitBossSixPlays => "A sixth Play every turn - but the Blinds rise +2 every turn, not every third.",
            Self::PitBossRandomCards => "Four cards off the Pit's table: two with Tells, two plain.",
        }
    }
}

/// What is on the table once an encounter is won. Overworld renders it;
/// `RunState::apply` does the mutating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewardOffer {
    /// A minion's drop: no choice, it is already in your pocket.
    Drop(Reward),
    /// A boss's pick, 1 of 2.
    Pick(Reward, Reward),
}

impl RewardOffer {
    /// What beating `id` pays. The House pays in an ending, not a perk.
    pub fn for_encounter(id: EncounterId) -> Option<Self> {
        match id {
            EncounterId::FloorMinion | EncounterId::PitMinion => Some(Self::Drop(Reward::LoadedDice)),
            EncounterId::Slotz => {
                Some(Self::Pick(Reward::SlotzPylBestTwoOfThree, Reward::SlotzStreakCards))
            }
            EncounterId::PitBoss => {
                Some(Self::Pick(Reward::PitBossSixPlays, Reward::PitBossRandomCards))
            }
            EncounterId::TheHouse => None,
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
    pub stack: u32,
    pub deck: Vec<Card>,
    pub perks: Vec<Perk>,
    pub items: Vec<Item>,
}

impl RunState {
    /// A fresh run with the fixed starter deck and starting Stack.
    pub fn new() -> Self {
        RunState {
            stack: STARTING_STACK,
            deck: starter_deck(),
            perks: Vec::new(),
            items: Vec::new(),
        }
    }

    /// Grant a reward. The only way perks, items, or the deck change between
    /// encounters.
    ///
    /// `seed` feeds the one reward that rolls dice, the Pit Boss card pack.
    /// The caller owns the randomness here for the same reason it owns the
    /// duel's shuffle: it keeps this whole file deterministic under test.
    pub fn apply(&mut self, reward: Reward, seed: u64) {
        match reward {
            Reward::LoadedDice => {
                // A fresh pair on top of an existing one adds Hands, not clutter.
                self.set_loaded_dice(self.loaded_dice().saturating_add(LOADED_DICE_HANDS));
            }
            Reward::SlotzStreakCards => self.deck.extend(streak_reward_cards()),
            Reward::SlotzPylBestTwoOfThree => self.take_perk(Perk::PylBestTwoOfThree),
            Reward::PitBossSixPlays => self.take_perk(Perk::SixPlaysSteepBlinds),
            Reward::PitBossRandomCards => self.deck.extend(random_cards(seed)),
        }
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
        self.items.retain(|item| !matches!(item, Item::LoadedDice { .. }));
        if hands_left > 0 {
            self.items.push(Item::LoadedDice { hands_left });
        }
    }

    /// Plays per turn after perks.
    pub fn plays(&self) -> u8 {
        if self.perks.contains(&Perk::SixPlaysSteepBlinds) { 6 } else { 5 }
    }

    /// Rising Blinds after perks. The Pit Boss perk states its own price
    /// rather than borrowing the enemy's step: +2 every turn, three times the
    /// base rate, and the same price whoever is sitting across the table.
    pub fn blinds(&self, base: RisingBlinds) -> RisingBlinds {
        if self.perks.contains(&Perk::SixPlaysSteepBlinds) {
            RisingBlinds { every_turns: 1, increase: STEEP_BLINDS_INCREASE }
        } else {
            base
        }
    }

    /// A perk is a thing you either have or don't; taking it twice is a no-op.
    fn take_perk(&mut self, perk: Perk) {
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
// Enemies and encounters
// ---------------------------------------------------------------------------

/// Which encounter the overworld is entering. Overworld's floor table is a
/// sequence of these; Dev 1 owns what each one means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EncounterId {
    FloorMinion,
    Slotz,
    PitMinion,
    PitBoss,
    TheHouse,
}

/// How House Edge escalates as combat goes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RisingBlinds {
    pub every_turns: u8,
    pub increase: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enemy {
    pub name: &'static str,
    /// The enemy's chips; 0 means the encounter is won.
    pub stack: u32,
    /// Starting House Edge. The House overrides this per turn (#5).
    pub house_edge: u32,
    pub blinds: RisingBlinds,
}

impl Enemy {
    /// The single place enemy numbers live (#8). Tuned on the
    /// `prototype/starter-deck` sim: fights run 2-5 turns for a decent
    /// player, and Rising Blinds only bite past ~6.
    ///
    /// The House has no fixed Edge (the Hole Card rule, #5): for it,
    /// `house_edge` is the **starting margin** and `blinds` is the margin
    /// ramp. #14 reads it that way.
    pub fn for_encounter(id: EncounterId) -> Self {
        let blinds = RisingBlinds { every_turns: 3, increase: 2 };
        match id {
            EncounterId::FloorMinion => Enemy { name: "A shill in a rented tux", stack: 25, house_edge: 18, blinds },
            EncounterId::Slotz => Enemy { name: "SLOTZ", stack: 32, house_edge: 20, blinds },
            EncounterId::PitMinion => Enemy { name: "A dealer with a scar", stack: 32, house_edge: 22, blinds },
            EncounterId::PitBoss => Enemy { name: "THE PIT BOSS", stack: 40, house_edge: 24, blinds },
            EncounterId::TheHouse => Enemy {
                name: "THE HOUSE",
                stack: 35,
                house_edge: 1,
                blinds: RisingBlinds { every_turns: 2, increase: 2 },
            },
        }
    }
}

/// Inserted by overworld before `NextState(AppState::Combat)`.
/// Combat removes it on exit.
#[derive(Resource, Debug, Clone)]
pub struct Encounter {
    /// Which encounter this is. The Hole Card rule (#14) is the first thing
    /// in combat that has to tell The House from everyone else.
    #[allow(dead_code)]
    pub id: EncounterId,
    pub enemy: Enemy,
}

/// Inserted by combat immediately before `NextState(AppState::PostCombat)`.
/// `RunState.stack` has already been updated by then. Overworld removes it
/// once it has routed.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatOutcome {
    Won,
    Lost,
}

/// Lucky Jack sits down with this many chips (#8).
pub const STARTING_STACK: u32 = 50;

/// The one xorshift64 the whole game rolls on: the duel's reshuffle, the
/// deal into the Draw, the Push Your Luck coin, and the Pit Boss card pack.
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

/// What the Pit Boss perk's sixth Play costs: this much on House Edge every
/// single turn, against a base of the same step every three (#12).
const STEEP_BLINDS_INCREASE: u32 = 2;

/// Slotz Option 2 (#12): three more Streak cards, all middling, so the reward
/// is a better chance of firing Streak rather than a higher ceiling.
fn streak_reward_cards() -> Vec<Card> {
    ["Loose Slot", "Second Cherry", "Jackpot Bell"]
        .into_iter()
        .map(|name| Card { name, stack: 4, tell: Some(Tell::Streak) })
        .collect()
}

/// Pit Boss Option 2 (#12): four cards off the Pit's own table, two with a
/// random Tell and two plain. Stacks stay inside the starter deck's ranges so
/// the pack thickens the deck without rewriting its maths.
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
        // All In 2..=5.
        let (tell, stack) = if roll(2) == 0 {
            (Tell::Streak, 3 + roll(4) as u32)
        } else {
            (Tell::AllIn, 2 + roll(4) as u32)
        };
        cards.push(Card { name, stack, tell: Some(tell) });
    }
    for _ in 0..2 {
        cards.push(Card {
            name: plain.swap_remove(roll(plain.len() as u64) as usize),
            stack: 2 + roll(7) as u32, // 2..=8, the starter deck's vanilla range
            tell: None,
        });
    }
    cards
}

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
    let streak = [("Hot Streak", 3), ("Lucky Seat", 4), ("Dealer Blinks", 5), ("Table Runs Hot", 6)];
    let all_in = [("Last Dollar", 2), ("Car Keys", 3), ("Deed to the House", 4), ("Firstborn", 5)];
    vanilla
        .into_iter()
        .map(|(name, stack)| Card { name, stack, tell: None })
        .chain(streak.into_iter().map(|(name, stack)| Card { name, stack, tell: Some(Tell::Streak) }))
        .chain(all_in.into_iter().map(|(name, stack)| Card { name, stack, tell: Some(Tell::AllIn) }))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Any old seed; the rewards that roll dice only have to be deterministic.
    const SEED: u64 = 0x1234_5678_9abc_def0;

    fn tells(cards: &[Card]) -> usize {
        cards.iter().filter(|c| c.tell.is_some()).count()
    }

    #[test]
    fn a_fresh_run_carries_nothing_from_the_last_one() {
        let run = RunState::new();

        assert_eq!(run.stack, STARTING_STACK);
        assert_eq!(run.deck.len(), starter_deck().len());
        assert!(run.perks.is_empty());
        assert!(run.items.is_empty());
        assert_eq!(run.plays(), 5);
        assert_eq!(run.loaded_dice(), 0);
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

        run.apply(Reward::SlotzStreakCards, SEED);

        let added = &run.deck[before..];
        assert_eq!(added.len(), 3);
        assert!(added.iter().all(|c| c.tell == Some(Tell::Streak)));
    }

    #[test]
    fn the_slotz_perk_is_the_only_thing_that_changes() {
        let mut run = RunState::new();
        let deck = run.deck.len();

        run.apply(Reward::SlotzPylBestTwoOfThree, SEED);

        assert_eq!(run.perks, vec![Perk::PylBestTwoOfThree]);
        assert_eq!(run.deck.len(), deck);
    }

    #[test]
    fn the_pit_boss_card_reward_adds_two_tells_and_two_plain_cards() {
        let mut run = RunState::new();
        let before = run.deck.len();

        run.apply(Reward::PitBossRandomCards, SEED);

        let added = &run.deck[before..];
        assert_eq!(added.len(), 4);
        assert_eq!(tells(added), 2);
        assert!(added.iter().all(|c| (2..=8).contains(&c.stack)));

        let names: std::collections::HashSet<_> = added.iter().map(|c| c.name).collect();
        assert_eq!(names.len(), 4, "no pack deals the same card twice");
    }

    #[test]
    fn the_pit_boss_cards_are_rolled_from_the_seed() {
        let mut one = RunState::new();
        let mut two = RunState::new();
        let mut same = RunState::new();

        one.apply(Reward::PitBossRandomCards, SEED);
        two.apply(Reward::PitBossRandomCards, SEED ^ 0xffff);
        same.apply(Reward::PitBossRandomCards, SEED);

        assert_eq!(one.deck, same.deck, "the same seed deals the same cards");
        assert_ne!(one.deck, two.deck, "a different seed deals different ones");
    }

    #[test]
    fn the_pit_boss_perk_buys_a_sixth_play_with_blinds_every_turn() {
        let mut run = RunState::new();
        let base = RisingBlinds { every_turns: 3, increase: 2 };
        assert_eq!(run.blinds(base), base);

        run.apply(Reward::PitBossSixPlays, SEED);

        assert_eq!(run.plays(), 6);
        assert_eq!(run.blinds(base), RisingBlinds { every_turns: 1, increase: 2 });
    }

    #[test]
    fn every_encounter_worth_beating_puts_something_on_the_table() {
        use EncounterId::*;

        assert_eq!(
            RewardOffer::for_encounter(FloorMinion),
            Some(RewardOffer::Drop(Reward::LoadedDice))
        );
        assert_eq!(
            RewardOffer::for_encounter(PitMinion),
            Some(RewardOffer::Drop(Reward::LoadedDice))
        );
        assert_eq!(
            RewardOffer::for_encounter(Slotz),
            Some(RewardOffer::Pick(Reward::SlotzPylBestTwoOfThree, Reward::SlotzStreakCards))
        );
        assert_eq!(
            RewardOffer::for_encounter(PitBoss),
            Some(RewardOffer::Pick(Reward::PitBossSixPlays, Reward::PitBossRandomCards))
        );
        // The House pays out in an ending, not a perk.
        assert_eq!(RewardOffer::for_encounter(TheHouse), None);
    }

    #[test]
    fn every_reward_says_what_it_is() {
        for reward in [
            Reward::LoadedDice,
            Reward::SlotzStreakCards,
            Reward::SlotzPylBestTwoOfThree,
            Reward::PitBossSixPlays,
            Reward::PitBossRandomCards,
        ] {
            assert!(!reward.label().is_empty());
        }
    }
}
