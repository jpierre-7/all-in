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
///
/// Every Tell reads the row by position, not by the order the cards were
/// picked up: Streak looks one slot to its left, Copycat one slot to its
/// right, Flop straight across at the Opposing Card. All of them read
/// *printed* Stacks, so no Tell ever depends on another Tell resolving first
/// and the two rows can be worked out in either order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tell {
    /// Doubles the card's Stack if the card in the slot to its left has any Tell.
    Streak,
    /// Sacrifice another card from the Draw to add its Stack to this card's.
    AllIn,
    /// Takes the printed Stack of the card in the slot to its right, and none
    /// of its Tell; its own if it is the last card in the row.
    Copycat,
    /// Takes the printed Stack of the Opposing Card across from it; its own
    /// if there is nothing across.
    Flop,
}

impl Tell {
    pub fn rule_text(&self) -> Vec<&'static str> {
        match self {
            Tell::Streak => vec![
                "Doubles this card's",
                "Stack",
                "if the card in the slot to its left has any",
                "Tell",
                ".",
            ],
            Tell::AllIn => vec![
                "Burns",
                "another card from your",
                "Draw",
                "and adds its",
                "Stack",
                "to",
                "The Hand",
                ".",
            ],
            Tell::Copycat => vec![
                "Takes the printed",
                "Stack",
                "of the card in the slot to its right, and none of its",
                "Tell",
                ". Its own if nothing follows it.",
            ],
            Tell::Flop => vec![
                "Takes the printed",
                "Stack",
                "of the",
                "Opposing Card",
                "across from it, and none of its",
                "Tell",
                ". Its own if nothing is across.",
            ],
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Tell::Streak => "Streak",
            Tell::AllIn => "All In",
            Tell::Copycat => "Copycat",
            Tell::Flop => "Flop",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub name: &'static str,
    /// The chips this card contributes to its row's Stack Sum.
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
            Self::PitBossSixPlays => {
                "A sixth Play every turn - but the Blinds rise every turn, not every third, and each rise is another Opposing Card."
            }
            Self::PitBossRandomCards => {
                "Four cards off the Pit's table: two with Tells, two plain."
            }
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
            EncounterId::FloorMinion | EncounterId::PitMinion => {
                Some(Self::Drop(Reward::LoadedDice))
            }
            EncounterId::Slotz => Some(Self::Pick(
                Reward::SlotzPylBestTwoOfThree,
                Reward::SlotzStreakCards,
            )),
            EncounterId::PitBoss => Some(Self::Pick(
                Reward::PitBossSixPlays,
                Reward::PitBossRandomCards,
            )),
            EncounterId::TheHouse | EncounterId::Tutorial => None,
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
        self.items
            .retain(|item| !matches!(item, Item::LoadedDice { .. }));
        if hands_left > 0 {
            self.items.push(Item::LoadedDice { hands_left });
        }
    }

    /// Plays per turn after perks: the most cards the player may put in the
    /// row. An enemy row longer than this can't be covered slot for slot, and
    /// what isn't covered is free chips for the enemy.
    pub fn plays(&self) -> u8 {
        if self.perks.contains(&Perk::SixPlaysSteepBlinds) {
            6
        } else {
            5
        }
    }

    /// Rising Blinds after perks. The Pit Boss perk states its own price
    /// rather than borrowing the enemy's step: another Opposing Card every
    /// turn, three times the base rate, and the same price whoever is sitting
    /// across the table. The sixth Play buys one turn of cover against it.
    pub fn blinds(&self, base: RisingBlinds) -> RisingBlinds {
        if self.perks.contains(&Perk::SixPlaysSteepBlinds) {
            RisingBlinds {
                every_turns: 1,
                cards: STEEP_BLINDS_CARDS,
            }
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
    /// The Arcade's practice duel (#40). Never part of a run.
    Tutorial,
}

/// How the enemy's side escalates as combat goes on: every `every_turns`
/// turns it lays `cards` more Opposing Cards. The House is the exception —
/// it raises its margin instead, by `HoleCard::step`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RisingBlinds {
    pub every_turns: u8,
    pub cards: u8,
}

/// What an enemy deals its Opposing Cards from. Ordinary enemies keep no
/// named deck: a Stack range and the odds of a Tell are the whole of them, so
/// a new enemy is six numbers rather than eighteen cards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deal {
    /// Opposing Cards laid down on turn one. Rising Blinds add more.
    pub row: u8,
    /// The Stack range a card is dealt from, inclusive at both ends.
    pub low: u32,
    pub high: u32,
    /// Chance in 100 that a dealt card carries a Tell at all.
    pub tell_pct: u32,
    /// The Tells this enemy plays; one is drawn at random when `tell_pct`
    /// hits. **Never All In** — an enemy has no Draw to burn a card from, so
    /// an All In dealt here would just be worth its printed Stack.
    pub tells: &'static [Tell],
    /// Chance in 100 that a card is dealt face down. The first Opposing Card
    /// is always face up whatever this says.
    pub hidden_pct: u32,
}

/// The names the house deals under. Flavour only: an Opposing Card is a
/// Stack and a Tell, and the name is what the Peek puts at the top of the tag.
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

impl Deal {
    /// One Opposing Card off the enemy's table. Rolls the Stack, then whether
    /// it carries a Tell, then which — always in that order, so a change to
    /// the Tell list doesn't re-deal the Stacks of a pinned seed.
    pub fn card(&self, rng: &mut u64) -> Card {
        let span = u64::from(self.high.saturating_sub(self.low) + 1);
        let stack = self.low + (xorshift64(rng) % span) as u32;
        let carries = !self.tells.is_empty() && xorshift64(rng) % 100 < u64::from(self.tell_pct);
        let tell =
            carries.then(|| self.tells[(xorshift64(rng) % self.tells.len() as u64) as usize]);
        let name = HOUSE_CARDS[(xorshift64(rng) % HOUSE_CARDS.len() as u64) as usize];
        Card { name, stack, tell }
    }
}

/// The House's Hole Card rule (#5). It deals its last Opposing Card face
/// down and leaves it blank until the showdown, then sets it so its row reads
/// the player's row — everything but the player's own last card — plus the
/// margin. The player's last card is the Hole Card: the one card The House
/// could not see. Rising Blinds raise the margin rather than the row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoleCard {
    /// How far above the row it read The House sets its own.
    pub margin: u32,
    /// What each Blinds tick adds to the margin.
    pub step: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enemy {
    pub name: &'static str,
    /// The enemy's chips; 0 means the encounter is won.
    pub stack: u32,
    /// How it fills the row across from the player.
    pub deal: Deal,
    pub blinds: RisingBlinds,
    /// Set for The House alone (#5); `None` for every ordinary enemy.
    pub hole_card: Option<HoleCard>,
}

impl Enemy {
    /// The single place enemy numbers live (#8). 
    pub fn for_encounter(id: EncounterId) -> Self {
        let blinds = RisingBlinds {
            every_turns: 5,
            cards: 1,
        };
        match id {
            EncounterId::FloorMinion => Enemy {
                name: "A shill in a rented tux",
                stack: 25,
                deal: Deal {
                    row: 3,
                    low: 2,
                    high: 5,
                    tell_pct: 20,
                    tells: &[Tell::Streak],
                    hidden_pct: 50,
                },
                blinds,
                hole_card: None,
            },
            EncounterId::Slotz => Enemy {
                name: "SLOTZ",
                stack: 32,
                deal: Deal {
                    row: 3,
                    low: 3,
                    high: 7,
                    tell_pct: 35,
                    tells: &[Tell::Streak, Tell::Copycat],
                    hidden_pct: 50,
                },
                blinds,
                hole_card: None,
            },
            EncounterId::PitMinion => Enemy {
                name: "A dealer with a scar",
                stack: 32,
                deal: Deal {
                    row: 4,
                    low: 4,
                    high: 7,
                    tell_pct: 30,
                    tells: &[Tell::Streak, Tell::Flop],
                    hidden_pct: 50,
                },
                blinds,
                hole_card: None,
            },
            EncounterId::PitBoss => Enemy {
                name: "THE PIT BOSS",
                stack: 40,
                deal: Deal {
                    row: 4,
                    low: 4,
                    high: 9,
                    tell_pct: 40,
                    tells: &[Tell::Streak, Tell::Copycat, Tell::Flop],
                    hidden_pct: 55,
                },
                blinds,
                hole_card: None,
            },
            // A practice hand, not a clock: everything face up, no Tells, no
            // Blinds. The Arcade overrides the row outright (#40).
            EncounterId::Tutorial => Enemy {
                name: "THE DEMO DEALER",
                stack: 30,
                deal: Deal {
                    row: 5,
                    low: 4,
                    high: 4,
                    tell_pct: 0,
                    tells: &[],
                    hidden_pct: 0,
                },
                blinds: RisingBlinds {
                    every_turns: u8::MAX,
                    cards: 0,
                },
                hole_card: None,
            },
            // The House deals itself scraps and keeps the last card blank
            // until the showdown, so its row lands wherever the Hole Card
            // rule says it lands. Streak and Flop only: both read a slot
            // that is already settled when the Hole Card is worked out,
            // where a Copycat would have to read the blank.
            EncounterId::TheHouse => Enemy {
                name: "THE HOUSE",
                stack: 35,
                deal: Deal {
                    row: 4,
                    low: 1,
                    high: 4,
                    tell_pct: 35,
                    tells: &[Tell::Streak, Tell::Flop],
                    hidden_pct: 40,
                },
                blinds: RisingBlinds {
                    every_turns: 2,
                    cards: 0,
                },
                hole_card: Some(HoleCard { margin: 1, step: 2 }),
            },
        }
    }
}

/// Inserted by overworld before `NextState(AppState::Combat)`.
/// Combat removes it on exit.
#[derive(Resource, Debug, Clone)]
pub struct Encounter {
    /// Which encounter this is. Combat reads it for the Arcade's fixed deal
    /// and for the portrait the screen hangs across the table; the Hole Card
    /// rule rides on `enemy.hole_card` rather than on the id.
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

/// What the Pit Boss perk's sixth Play costs: this many more Opposing Cards
/// every single turn, against a base of the same step every three (#12).
const STEEP_BLINDS_CARDS: u8 = 1;

/// Slotz Option 2 (#12): three more Streak cards, all middling, so the reward
/// is a better chance of firing Streak rather than a higher ceiling.
fn streak_reward_cards() -> Vec<Card> {
    ["Loose Slot", "Second Cherry", "Jackpot Bell"]
        .into_iter()
        .map(|name| Card {
            name,
            stack: 4,
            tell: Some(Tell::Streak),
        })
        .collect()
}

/// Pit Boss Option 2 (#12): four cards off the Pit's own table, two with a
/// random Tell (Streak, All In, Copycat, or Flop) and two plain. Stacks stay
/// inside the starter deck's ranges so the pack thickens the deck without
/// rewriting its maths.
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
        // sequence rather than the last slot. Flop prints low for the same
        // reason — its print only counts opposite an empty slot (#88).
        let (tell, stack) = match roll(4) {
            0 => (Tell::Streak, 3 + roll(4) as u32),
            1 => (Tell::AllIn, 2 + roll(4) as u32),
            2 => (Tell::Copycat, 2 + roll(4) as u32),
            _ => (Tell::Flop, 2 + roll(4) as u32),
        };
        cards.push(Card {
            name,
            stack,
            tell: Some(tell),
        });
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
        .map(|(name, stack)| Card {
            name,
            stack,
            tell: None,
        })
        .chain(streak.into_iter().map(|(name, stack)| Card {
            name,
            stack,
            tell: Some(Tell::Streak),
        }))
        .chain(all_in.into_iter().map(|(name, stack)| Card {
            name,
            stack,
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
    .map(|(name, stack, face_up)| {
        (
            Card {
                name,
                stack,
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
        let base = RisingBlinds {
            every_turns: 3,
            cards: 1,
        };
        assert_eq!(run.blinds(base), base);

        run.apply(Reward::PitBossSixPlays, SEED);

        assert_eq!(run.plays(), 6);
        assert_eq!(
            run.blinds(base),
            RisingBlinds {
                every_turns: 1,
                cards: 1
            }
        );
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
            Some(RewardOffer::Pick(
                Reward::SlotzPylBestTwoOfThree,
                Reward::SlotzStreakCards
            ))
        );
        assert_eq!(
            RewardOffer::for_encounter(PitBoss),
            Some(RewardOffer::Pick(
                Reward::PitBossSixPlays,
                Reward::PitBossRandomCards
            ))
        );
        // The House pays out in an ending, not a perk.
        assert_eq!(RewardOffer::for_encounter(TheHouse), None);
    }

    #[test]
    fn an_opposing_card_is_dealt_inside_the_enemys_range() {
        let deal = Deal {
            row: 3,
            low: 4,
            high: 8,
            tell_pct: 100,
            tells: &[Tell::Streak],
            hidden_pct: 50,
        };
        let mut rng = SEED;

        for _ in 0..500 {
            let card = deal.card(&mut rng);
            assert!(
                (4..=8).contains(&card.stack),
                "{} is off the range",
                card.stack
            );
            assert_eq!(card.tell, Some(Tell::Streak));
            assert!(!card.name.is_empty());
        }
    }

    #[test]
    fn the_tell_odds_are_what_the_enemy_says_they_are() {
        let never = Deal {
            tell_pct: 0,
            ..Enemy::for_encounter(EncounterId::PitBoss).deal
        };
        let mut rng = SEED;
        assert!((0..200).all(|_| never.card(&mut rng).tell.is_none()));

        // An enemy with no Tells listed plays none, whatever the odds say.
        let none = Deal {
            tell_pct: 100,
            tells: &[],
            ..never
        };
        assert!((0..200).all(|_| none.card(&mut rng).tell.is_none()));

        // A third of the deal, near enough, over five hundred cards.
        let some = Deal {
            tell_pct: 33,
            tells: &[Tell::Flop],
            ..never
        };
        let telled = (0..500)
            .filter(|_| some.card(&mut rng).tell.is_some())
            .count();
        assert!(
            (120..=210).contains(&telled),
            "{telled} of 500 carried a Tell"
        );
    }

    #[test]
    fn no_enemy_deals_itself_an_all_in() {
        use EncounterId::*;

        // An enemy has no Draw to burn from, so an All In across the table
        // would silently be worth its printed Stack and nothing else.
        for id in [FloorMinion, Slotz, PitMinion, PitBoss, TheHouse, Tutorial] {
            let deal = Enemy::for_encounter(id).deal;
            assert!(
                !deal.tells.contains(&Tell::AllIn),
                "{id:?} deals itself an All In"
            );
        }
    }

    #[test]
    fn the_house_is_the_only_one_holding_a_hole_card() {
        use EncounterId::*;

        assert_eq!(
            Enemy::for_encounter(TheHouse).hole_card,
            Some(HoleCard { margin: 1, step: 2 })
        );
        for id in [FloorMinion, Slotz, PitMinion, PitBoss, Tutorial] {
            assert_eq!(Enemy::for_encounter(id).hole_card, None, "{id:?}");
        }
    }

    #[test]
    fn the_arcades_opposing_row_is_twenty_chips_with_eight_of_them_face_down() {
        let row = tutorial_opposing();

        assert_eq!(row.len(), 5, "one slot per card the script plays");
        assert_eq!(row.iter().map(|(c, _)| c.stack).sum::<u32>(), 20);
        let hidden: u32 = row
            .iter()
            .filter(|(_, face_up)| !face_up)
            .map(|(c, _)| c.stack)
            .sum();
        assert_eq!(hidden, 8);
        assert!(row[0].1, "the first Opposing Card is always face up");
        assert!(row.iter().all(|(c, _)| c.tell.is_none()), "a practice hand");
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
