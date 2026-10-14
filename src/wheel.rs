//! The Wheel (#167): where Golden Chips are placed between runs, and the spin
//! that picks a Hot Square at the start of each one. Vocabulary follows
//! `GLOSSARY.md`; the design is #143's resolution.
//!
//! The board lives in the [`SaveSlot`]. A run locks it as a [`Spun`], which
//! the run carries and asks for each square's rank. What each square does at
//! each rank is here; the numbers are starting points for the balance sim.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::boss::BOSSES;
use crate::combat::duel::Opposing;
use crate::item::{self, Item, Rarity};
use crate::modifier::Modifier;
use crate::run::{
    Card, Encounter, MINION_PACK_SIZE, Reward, RunState, Tell, deal_card, xorshift64,
};
use crate::save::SaveSlot;

/// The most Golden Chips one square takes.
pub const MAX_RANK: u8 = 3;

/// Golden Chips paid for each win over The House.
pub const GOLDEN_CHIPS_PER_WIN: u32 = 3;

/// The twelve squares of The Wheel. The green zero takes no chips, so it
/// isn't one.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Square {
    Bankroll,
    SeedMoney,
    Compound,
    Sweetener,
    Regular,
    FatPack,
    PocketChange,
    SecondLook,
    LuckyCoin,
    InsideMan,
    Trim,
    EarlyRead,
}

impl Square {
    /// Every square, in the order the board lists them.
    pub const ALL: [Square; 12] = [
        Square::Bankroll,
        Square::SeedMoney,
        Square::Compound,
        Square::Sweetener,
        Square::Regular,
        Square::FatPack,
        Square::PocketChange,
        Square::SecondLook,
        Square::LuckyCoin,
        Square::InsideMan,
        Square::Trim,
        Square::EarlyRead,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Square::Bankroll => "Bankroll",
            Square::SeedMoney => "Seed Money",
            Square::Compound => "Compound",
            Square::Sweetener => "Sweetener",
            Square::Regular => "Regular",
            Square::FatPack => "Fat Pack",
            Square::PocketChange => "Pocket Change",
            Square::SecondLook => "Second Look",
            Square::LuckyCoin => "Lucky Coin",
            Square::InsideMan => "Inside Man",
            Square::Trim => "Trim",
            Square::EarlyRead => "Early Read",
        }
    }

    /// Whether the square does anything yet. One that doesn't shows as
    /// "coming soon" and takes no chips: the economy squares wait on Cash,
    /// Interest, Side Bets and the Cage.
    pub fn ready(self) -> bool {
        !matches!(
            self,
            Square::SeedMoney | Square::Compound | Square::Sweetener | Square::Regular
        )
    }

    /// What the square does at `rank`, 1 to 4. Rank 4 is only ever reached by
    /// the spin.
    pub fn describe(self, rank: u8) -> String {
        let rank = rank.clamp(1, 4);
        let n = usize::from(rank);
        let times = |n: usize, one: &str, many: &str| {
            if n == 1 {
                format!("{n} {one}")
            } else {
                format!("{n} {many}")
            }
        };
        match self {
            Square::Bankroll => format!("Start with +{} Chips.", bankroll(rank)),
            Square::SeedMoney => format!("Start with +{} Cash.", [3, 6, 10, 15][n - 1]),
            Square::Compound => format!("Interest cap +{n}."),
            Square::Sweetener => format!("Each Side Bet pays +{n} Cash."),
            Square::Regular => format!("Cage prices -{}%.", n * 10),
            Square::FatPack => format!("Minion Packs show {} cards.", fat_pack(rank)),
            Square::PocketChange => match pocket_change(rank) {
                Some(PocketChange::Random(Rarity::Common)) => {
                    "Start with a random common Item.".into()
                }
                Some(PocketChange::Random(_)) => "Start with a random uncommon Item.".into(),
                Some(PocketChange::Pick { any_rarity: false }) => {
                    "Start with your pick of 3 Items, none of them rare.".into()
                }
                _ => "Start with your pick of 3 Items, of any rarity.".into(),
            },
            Square::SecondLook => format!(
                "Reroll a minion reward offer {} a run.",
                times(n, "time", "times")
            ),
            Square::LuckyCoin => format!(
                "{} a run: a lost Push flips again.",
                times(n, "Push Your Luck re-flip", "Push Your Luck re-flips")
            ),
            Square::InsideMan => {
                let bosses: Vec<String> = BOSSES[..n.min(BOSSES.len())]
                    .iter()
                    .map(|boss| {
                        if Tell::OPEN.contains(&boss.tell) {
                            boss.name.to_string()
                        } else {
                            format!("{} ({})", boss.name, boss.tell.name())
                        }
                    })
                    .collect();
                let card = if inside_man_card(rank) {
                    ", and one Boss Tell card in your starting deck"
                } else {
                    ""
                };
                format!(
                    "Start with the Boss Tell of {} open{card}.",
                    bosses.join(", ")
                )
            }
            Square::Trim => format!(
                "Take up to {} out of your starting deck.",
                times(trims(rank), "card", "cards")
            ),
            Square::EarlyRead => match early_read(rank) {
                Some((true, _)) => {
                    "Reveal the leftmost Opposing Card on turn 1 of boss fights.".into()
                }
                Some((false, 1)) => {
                    "Reveal the leftmost Opposing Card on turn 1 of every encounter.".into()
                }
                Some((false, turns)) => format!(
                    "Reveal the leftmost Opposing Card on turns 1-{turns} of every encounter."
                ),
                None => String::new(),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// What each square does at each rank
// ---------------------------------------------------------------------------

/// Bankroll: Chips added to the run's starting Chips.
fn bankroll(rank: u8) -> u32 {
    5 * u32::from(rank.min(4))
}

/// Fat Pack: how many cards a minion's Pack shows, one more a rank than
/// without it.
fn fat_pack(rank: u8) -> usize {
    MINION_PACK_SIZE + usize::from(rank.min(4))
}

/// What Pocket Change starts a run with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PocketChange {
    /// One Item of this rarity, drawn by weight.
    Random(Rarity),
    /// The player's pick of three. Rare ones only at rank 4.
    Pick { any_rarity: bool },
}

fn pocket_change(rank: u8) -> Option<PocketChange> {
    match rank {
        0 => None,
        1 => Some(PocketChange::Random(Rarity::Common)),
        2 => Some(PocketChange::Random(Rarity::Uncommon)),
        3 => Some(PocketChange::Pick { any_rarity: false }),
        _ => Some(PocketChange::Pick { any_rarity: true }),
    }
}

/// Second Look: rerolls of a minion reward offer, a run.
fn second_look(rank: u8) -> u8 {
    rank.min(4)
}

/// Lucky Coin: re-flips of a lost Push, a run.
fn lucky_coin(rank: u8) -> u8 {
    rank.min(4)
}

/// Inside Man opens the Boss Tells of the first `rank` bosses, in the order
/// they're met, as if each were already beaten. Minions read the open pool
/// too, so it cuts both ways.
fn inside_man(rank: u8) -> Vec<Tell> {
    BOSSES[..usize::from(rank).min(BOSSES.len())]
        .iter()
        .map(|boss| boss.tell)
        .collect()
}

/// Rank 4 of Inside Man also deals a Boss Tell card into the starting deck.
fn inside_man_card(rank: u8) -> bool {
    rank >= 4
}

/// Trim: how many starting cards may come out at run start.
fn trims(rank: u8) -> usize {
    usize::from(rank.min(4))
}

/// Early Read: whether it's boss fights only, and for how many turns.
fn early_read(rank: u8) -> Option<(bool, u32)> {
    match rank {
        0 => None,
        1 => Some((true, 1)),
        2 => Some((false, 1)),
        3 => Some((false, 2)),
        _ => Some((false, 3)),
    }
}

/// The Face Values Inside Man's Boss Tell card prints at: the starter deck's
/// range for a Boss Tell, as the Boss Pack deals them.
const INSIDE_MAN_FACES: std::ops::RangeInclusive<u32> = 2..=5;

/// Early Read in the duel: the leftmost Opposing Card turns face up on each
/// of the first `turns` turns.
#[derive(Debug)]
struct EarlyRead {
    turns: u32,
}

static EARLY_READ: [EarlyRead; 3] = [
    EarlyRead { turns: 1 },
    EarlyRead { turns: 2 },
    EarlyRead { turns: 3 },
];

impl Modifier for EarlyRead {
    fn reveal(&self, turn: u32, opposing: &[Opposing]) -> Vec<usize> {
        if turn <= self.turns && !opposing.is_empty() {
            vec![0]
        } else {
            Vec::new()
        }
    }
}

// ---------------------------------------------------------------------------
// Arranging the board, between runs
// ---------------------------------------------------------------------------

/// Why a Golden Chip couldn't go on, or come off, a square.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// The Wheel opens once The House is beaten.
    Closed,
    /// The square does nothing yet.
    ComingSoon,
    NoGoldenChips,
    /// The square already holds [`MAX_RANK`].
    Full,
    /// Nothing on the square to take off.
    Empty,
}

/// Move a Golden Chip from hand onto `square`.
pub fn place(slot: &mut SaveSlot, square: Square) -> Result<(), Refused> {
    if !slot.house_beaten {
        return Err(Refused::Closed);
    }
    if !square.ready() {
        return Err(Refused::ComingSoon);
    }
    let rank = slot.wheel.get(&square).copied().unwrap_or(0);
    if rank >= MAX_RANK {
        return Err(Refused::Full);
    }
    if slot.golden_chips == 0 {
        return Err(Refused::NoGoldenChips);
    }
    slot.golden_chips -= 1;
    slot.wheel.insert(square, rank + 1);
    Ok(())
}

/// Move a Golden Chip off `square` and back into hand.
pub fn take_off(slot: &mut SaveSlot, square: Square) -> Result<(), Refused> {
    if !slot.house_beaten {
        return Err(Refused::Closed);
    }
    let Some(rank) = slot.wheel.get(&square).copied().filter(|&r| r > 0) else {
        return Err(Refused::Empty);
    };
    if rank == 1 {
        slot.wheel.remove(&square);
    } else {
        slot.wheel.insert(square, rank - 1);
    }
    slot.golden_chips += 1;
    Ok(())
}

// ---------------------------------------------------------------------------
// The spin, and the run it starts
// ---------------------------------------------------------------------------

/// Spin The Wheel: one of the twelve squares, or `None` for the green zero,
/// each 1 in 13.
pub fn spin(seed: u64) -> Option<Square> {
    let mut rng = seed | 1;
    let pocket = (xorshift64(&mut rng) % (Square::ALL.len() as u64 + 1)) as usize;
    Square::ALL.get(pocket).copied()
}

/// The Wheel as one run sees it: the board locked at run start, and the
/// square the spin made hot.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Spun {
    board: BTreeMap<Square, u8>,
    hot: Option<Square>,
}

impl Spun {
    /// Lock `board` for a run with `hot` as its Hot Square. A square that
    /// doesn't do anything yet counts as empty, whatever a file says.
    pub fn new(board: &BTreeMap<Square, u8>, hot: Option<Square>) -> Self {
        Spun {
            board: board
                .iter()
                .filter(|(square, rank)| square.ready() && **rank > 0)
                .map(|(&square, &rank)| (square, rank.min(MAX_RANK)))
                .collect(),
            hot,
        }
    }

    /// The Hot Square, or `None` when the spin landed on the green zero.
    pub fn hot(&self) -> Option<Square> {
        self.hot
    }

    /// `square`'s rank this run: its chips, and one more if it's hot. An
    /// empty square stays empty, hot or not.
    pub fn rank(&self, square: Square) -> u8 {
        match self.board.get(&square).copied().unwrap_or(0) {
            0 => 0,
            rank if self.hot == Some(square) => rank + 1,
            rank => rank,
        }
    }

    /// Every square doing something this run, with its rank, in board order.
    pub fn working(&self) -> Vec<(Square, u8)> {
        Square::ALL
            .into_iter()
            .map(|square| (square, self.rank(square)))
            .filter(|&(_, rank)| rank > 0)
            .collect()
    }

    /// What The Wheel adds to the duel against `encounter`, after the Perks.
    pub fn modifiers(&self, encounter: Encounter) -> Vec<&'static dyn Modifier> {
        let read = match early_read(self.rank(Square::EarlyRead)) {
            Some((true, turns)) if matches!(encounter, Encounter::Boss { .. }) => Some(turns),
            Some((false, turns)) if encounter != Encounter::Practice => Some(turns),
            _ => None,
        };
        read.and_then(|turns| EARLY_READ.get(turns as usize - 1))
            .map(|m| m as &'static dyn Modifier)
            .into_iter()
            .collect()
    }

    /// How many cards a minion's Pack shows: Fat Pack's.
    pub fn minion_pack_size(&self) -> usize {
        fat_pack(self.rank(Square::FatPack))
    }

    /// How many starting cards Trim lets the player take out.
    pub fn trims(&self) -> usize {
        trims(self.rank(Square::Trim))
    }

    /// The three Items Pocket Change lets the player pick from, or none when
    /// it isn't a pick.
    pub fn pocket_pick(&self, seed: u64) -> Vec<&'static Item> {
        match pocket_change(self.rank(Square::PocketChange)) {
            Some(PocketChange::Pick { any_rarity }) => item::offer_where(&[], 3, seed, |item| {
                any_rarity || item.rarity() != Rarity::Rare
            }),
            _ => Vec::new(),
        }
    }
}

/// `run` started under `wheel`: Bankroll's Chips, Inside Man's Tells and
/// card, Lucky Coin's re-flips, Second Look's rerolls, and a random Pocket
/// Change Item. `run` is fresh, or shaped by a Legacy Perk already
/// (`legacy::apply`), so Inside Man's card joins a Victory Lap deck rather
/// than being lost to it. The choices are the caller's to put to the player
/// afterwards: [`Spun::trims`] and [`Spun::pocket_pick`].
pub fn start_run(mut run: RunState, wheel: Spun, seed: u64) -> RunState {
    let mut rng = seed | 1;
    run.chips += bankroll(wheel.rank(Square::Bankroll));
    run.reflips = lucky_coin(wheel.rank(Square::LuckyCoin));
    run.rerolls = second_look(wheel.rank(Square::SecondLook));

    let inside = wheel.rank(Square::InsideMan);
    run.open_tells = inside_man(inside);
    if inside_man_card(inside) {
        let boss_tells: Vec<Tell> = run
            .open_tells
            .iter()
            .copied()
            .filter(|tell| !Tell::OPEN.contains(tell))
            .collect();
        run.deck
            .push(deal_card(INSIDE_MAN_FACES, 100, &boss_tells, &mut rng));
    }

    if let Some(PocketChange::Random(rarity)) = pocket_change(wheel.rank(Square::PocketChange))
        && let Some(&item) =
            item::offer_where(&[], 1, xorshift64(&mut rng), |item| item.rarity() == rarity).first()
    {
        run.apply(Reward::Item(item), 0);
    }

    run.wheel = wheel;
    run
}

/// Take the starting cards at `picks` out of `run`'s deck: up to as many as
/// Trim allows, each once, or nothing at all.
pub fn trim(run: &mut RunState, picks: &[usize]) -> bool {
    let mut sorted = picks.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    if sorted.len() != picks.len()
        || picks.len() > run.wheel.trims()
        || sorted.last().is_some_and(|&i| i >= run.deck.len())
    {
        return false;
    }
    for &i in sorted.iter().rev() {
        run.deck.remove(i);
    }
    true
}

/// One card as the Trim screen lists it.
pub fn card_line(card: &Card) -> String {
    match card.tell {
        Some(tell) => format!("{}  {}  {}", card.name, card.face_value, tell.name()),
        None => format!("{}  {}", card.name, card.face_value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss::{PIT_BOSS, SLOTZ, THE_HOUSE};
    use crate::combat::duel::Duel;
    use crate::run::{Floor, STARTING_CHIPS, starter_deck};

    const SEED: u64 = 0x1234_5678_9abc_def0;

    fn open_slot(golden_chips: u32) -> SaveSlot {
        SaveSlot {
            golden_chips,
            house_beaten: true,
            ..SaveSlot::default()
        }
    }

    fn board(squares: &[(Square, u8)]) -> BTreeMap<Square, u8> {
        squares.iter().copied().collect()
    }

    /// A run under `squares`, nothing hot.
    fn run_with(squares: &[(Square, u8)]) -> RunState {
        start_run(RunState::new(), Spun::new(&board(squares), None), SEED)
    }

    const BOSS: Encounter = Encounter::Boss {
        boss: &SLOTZ,
        floor: Floor::TheFloor,
    };
    const MINION: Encounter = Encounter::Minion {
        floor: Floor::TheFloor,
    };

    // --- arranging the board ---

    #[test]
    fn a_golden_chip_moves_from_hand_onto_a_square_and_back() {
        let mut slot = open_slot(2);

        place(&mut slot, Square::Bankroll).unwrap();
        place(&mut slot, Square::Bankroll).unwrap();
        assert_eq!(slot.golden_chips, 0);
        assert_eq!(slot.wheel[&Square::Bankroll], 2);

        take_off(&mut slot, Square::Bankroll).unwrap();
        take_off(&mut slot, Square::Bankroll).unwrap();
        assert_eq!(slot.golden_chips, 2);
        assert!(slot.wheel.is_empty(), "an empty square isn't listed");
    }

    #[test]
    fn a_square_takes_three_chips_at_most() {
        let mut slot = open_slot(5);
        for _ in 0..3 {
            place(&mut slot, Square::Trim).unwrap();
        }
        assert_eq!(place(&mut slot, Square::Trim), Err(Refused::Full));
        assert_eq!(slot.golden_chips, 2);
    }

    #[test]
    fn no_chip_goes_down_without_one_in_hand() {
        let mut slot = open_slot(0);
        assert_eq!(
            place(&mut slot, Square::Bankroll),
            Err(Refused::NoGoldenChips)
        );
        assert_eq!(take_off(&mut slot, Square::Bankroll), Err(Refused::Empty));
    }

    #[test]
    fn the_wheel_is_closed_until_the_house_is_beaten() {
        let mut slot = SaveSlot {
            golden_chips: 3,
            ..SaveSlot::default()
        };
        assert_eq!(place(&mut slot, Square::Bankroll), Err(Refused::Closed));
    }

    #[test]
    fn a_coming_soon_square_takes_no_chips() {
        let mut slot = open_slot(3);
        for square in Square::ALL.into_iter().filter(|s| !s.ready()) {
            assert_eq!(place(&mut slot, square), Err(Refused::ComingSoon));
        }
        assert_eq!(slot.golden_chips, 3);
    }

    #[test]
    fn every_square_but_the_economys_is_ready_now() {
        let ready: Vec<Square> = Square::ALL.into_iter().filter(|s| s.ready()).collect();
        assert_eq!(
            ready,
            [
                Square::Bankroll,
                Square::FatPack,
                Square::PocketChange,
                Square::SecondLook,
                Square::LuckyCoin,
                Square::InsideMan,
                Square::Trim,
                Square::EarlyRead,
            ],
        );
    }

    #[test]
    fn every_square_says_what_it_does_at_every_rank() {
        for square in Square::ALL {
            for rank in 1..=4 {
                assert!(!square.describe(rank).is_empty(), "{square:?} {rank}");
            }
            assert_ne!(square.describe(1), square.describe(4), "{square:?}");
        }
    }

    // --- the spin ---

    #[test]
    fn the_spin_lands_on_every_square_and_the_zero_about_as_often() {
        let mut counts = BTreeMap::new();
        for seed in 1..13_001u64 {
            *counts.entry(spin(seed * 7919)).or_insert(0) += 1;
        }
        assert_eq!(counts.len(), 13);
        for (pocket, count) in counts {
            assert!((800..1200).contains(&count), "{pocket:?}: {count}");
        }
    }

    #[test]
    fn the_spin_is_seeded() {
        assert_eq!(spin(SEED), spin(SEED));
    }

    #[test]
    fn the_hot_square_is_one_rank_higher_and_reaches_four() {
        let spun = Spun::new(
            &board(&[(Square::Bankroll, 3), (Square::Trim, 1)]),
            Some(Square::Bankroll),
        );
        assert_eq!(spun.rank(Square::Bankroll), 4);
        assert_eq!(spun.rank(Square::Trim), 1);
    }

    #[test]
    fn an_empty_hot_square_does_nothing() {
        let spun = Spun::new(&board(&[(Square::Trim, 1)]), Some(Square::Bankroll));
        assert_eq!(spun.rank(Square::Bankroll), 0);
        assert_eq!(spun.working(), vec![(Square::Trim, 1)]);
    }

    #[test]
    fn a_board_from_a_file_cant_go_past_three_or_onto_a_coming_soon_square() {
        let spun = Spun::new(
            &board(&[(Square::Bankroll, 9), (Square::SeedMoney, 2)]),
            None,
        );
        assert_eq!(spun.rank(Square::Bankroll), 3);
        assert_eq!(spun.rank(Square::SeedMoney), 0);
    }

    // --- the squares ---

    #[test]
    fn an_empty_board_starts_the_same_run_as_no_wheel_at_all() {
        let run = start_run(RunState::new(), Spun::default(), SEED);
        let fresh = RunState::new();
        assert_eq!(run.chips, fresh.chips);
        assert_eq!(run.deck, fresh.deck);
        assert!(run.items.is_empty());
        assert_eq!(run.tell_pool(), fresh.tell_pool());
        assert_eq!(run.reflips, 0);
        assert_eq!(run.rerolls, 0);
        assert_eq!(run.wheel.minion_pack_size(), MINION_PACK_SIZE);
    }

    #[test]
    fn bankroll_adds_five_chips_a_rank() {
        assert_eq!(
            run_with(&[(Square::Bankroll, 2)]).chips,
            STARTING_CHIPS + 10
        );
        let hot = start_run(
            RunState::new(),
            Spun::new(&board(&[(Square::Bankroll, 3)]), Some(Square::Bankroll)),
            SEED,
        );
        assert_eq!(hot.chips, STARTING_CHIPS + 20);
    }

    #[test]
    fn lucky_coin_hands_the_run_its_re_flips_and_the_duel_takes_them() {
        let run = run_with(&[(Square::LuckyCoin, 3)]);
        assert_eq!(run.reflips, 3);
        assert_eq!(Duel::for_run(&run, MINION, SEED).reflips_left(), 3);
    }

    #[test]
    fn fat_pack_shows_one_more_card_a_rank_in_a_minions_pack() {
        for (rank, cards) in [(1, 4), (2, 5), (3, 6)] {
            let run = run_with(&[(Square::FatPack, rank)]);
            let pack = crate::run::Pack::minion(&run, SEED);
            assert_eq!(pack.cards.len(), cards, "rank {rank}");
            assert_eq!(pack.keep, 1, "still keep one");
        }
        let hot = start_run(
            RunState::new(),
            Spun::new(&board(&[(Square::FatPack, 3)]), Some(Square::FatPack)),
            SEED,
        );
        assert_eq!(crate::run::Pack::minion(&hot, SEED).cards.len(), 7);
    }

    #[test]
    fn fat_pack_leaves_the_boss_pack_alone() {
        let run = run_with(&[(Square::FatPack, 3)]);
        let pack = BOSS.boss_pack(&run, SEED).expect("a boss deals one");
        assert_eq!(pack.cards.len(), crate::run::BOSS_PACK_SIZE);
    }

    #[test]
    fn second_look_hands_the_run_a_reroll_a_rank() {
        assert_eq!(run_with(&[(Square::SecondLook, 2)]).rerolls, 2);
        let hot = start_run(
            RunState::new(),
            Spun::new(&board(&[(Square::SecondLook, 3)]), Some(Square::SecondLook)),
            SEED,
        );
        assert_eq!(hot.rerolls, 4);
    }

    #[test]
    fn pocket_change_at_rank_one_and_two_puts_an_item_in_your_pocket() {
        for seed in 1..50 {
            let board = board(&[(Square::PocketChange, 1)]);
            let run = start_run(RunState::new(), Spun::new(&board, None), seed);
            assert_eq!(run.items.len(), 1);
            assert_eq!(run.items[0].item.rarity(), Rarity::Common);
            assert_eq!(run.items[0].uses, run.items[0].item.uses);

            let board = self::board(&[(Square::PocketChange, 2)]);
            let run = start_run(RunState::new(), Spun::new(&board, None), seed);
            assert_eq!(run.items[0].item.rarity(), Rarity::Uncommon);
            assert!(Spun::new(&board, None).pocket_pick(seed).is_empty());
        }
    }

    #[test]
    fn pocket_change_at_rank_three_is_a_pick_of_three_none_of_them_rare() {
        let spun = Spun::new(&board(&[(Square::PocketChange, 3)]), None);
        assert!(
            start_run(RunState::new(), spun.clone(), SEED)
                .items
                .is_empty(),
            "nothing yet"
        );
        for seed in 1..100 {
            let pick = spun.pocket_pick(seed);
            assert_eq!(pick.len(), 3);
            assert!(pick.iter().all(|item| item.rarity() != Rarity::Rare));
        }
    }

    #[test]
    fn pocket_change_at_rank_four_can_offer_a_rare() {
        let spun = Spun::new(
            &board(&[(Square::PocketChange, 3)]),
            Some(Square::PocketChange),
        );
        assert!(
            (1..200)
                .flat_map(|seed| spun.pocket_pick(seed))
                .any(|item| item.rarity() == Rarity::Rare)
        );
    }

    #[test]
    fn inside_man_opens_the_boss_tells_in_the_order_the_bosses_are_met() {
        let pool = |rank| run_with(&[(Square::InsideMan, rank)]).tell_pool();
        assert!(pool(1).contains(&SLOTZ.tell));
        assert!(!pool(1).contains(&THE_HOUSE.tell));
        assert!(pool(2).contains(&PIT_BOSS.tell));
        assert!(pool(3).contains(&THE_HOUSE.tell));
        assert_eq!(run_with(&[(Square::InsideMan, 3)]).deck, starter_deck());
    }

    #[test]
    fn inside_man_opens_its_tells_to_minions_too() {
        let run = run_with(&[(Square::InsideMan, 1)]);
        let dealt: Vec<Card> = (1..50)
            .flat_map(|seed| MINION.enemy(&run, seed).deck)
            .collect();
        assert!(dealt.iter().any(|c| c.tell == Some(SLOTZ.tell)));
    }

    #[test]
    fn inside_man_opens_its_tells_to_minion_packs_too() {
        let run = run_with(&[(Square::InsideMan, 1)]);
        let dealt: Vec<Card> = (1..300)
            .flat_map(|seed| crate::run::Pack::minion(&run, seed).cards)
            .collect();
        assert!(dealt.iter().any(|c| c.tell == Some(SLOTZ.tell)));
    }

    #[test]
    fn inside_man_at_rank_four_deals_one_boss_tell_card_into_the_deck() {
        for seed in 1..50 {
            let spun = Spun::new(&board(&[(Square::InsideMan, 3)]), Some(Square::InsideMan));
            let run = start_run(RunState::new(), spun, seed);
            assert_eq!(run.deck.len(), starter_deck().len() + 1);
            let card = run.deck.last().unwrap();
            assert!(
                matches!(card.tell, Some(t) if !Tell::OPEN.contains(&t)),
                "{card:?}"
            );
        }
    }

    #[test]
    fn trim_takes_out_up_to_its_rank_of_starting_cards() {
        let mut run = run_with(&[(Square::Trim, 2)]);
        let deck = run.deck.clone();

        assert!(!trim(&mut run, &[0, 1, 2]), "one too many");
        assert!(!trim(&mut run, &[4, 4]), "the same card twice");
        assert!(!trim(&mut run, &[99]), "no such card");
        assert_eq!(run.deck, deck);

        assert!(trim(&mut run, &[3, 0]));
        assert_eq!(run.deck.len(), deck.len() - 2);
        assert!(!run.deck.contains(&deck[0]));
        assert!(!run.deck.contains(&deck[3]));
    }

    #[test]
    fn trim_can_take_out_fewer_or_none() {
        let mut run = run_with(&[(Square::Trim, 3)]);
        assert!(trim(&mut run, &[]));
        assert!(trim(&mut run, &[5]));
        assert_eq!(run.deck.len(), starter_deck().len() - 1);
    }

    #[test]
    fn without_trim_nothing_comes_out() {
        let mut run = run_with(&[]);
        assert!(!trim(&mut run, &[0]));
    }

    /// Whether the leftmost Opposing Card is face up at the start of `turn`
    /// of a duel against `encounter` under `squares`.
    fn read_on(squares: &[(Square, u8)], hot: bool, encounter: Encounter, turn: u32) -> bool {
        let hot = hot.then_some(Square::EarlyRead);
        let run = start_run(RunState::new(), Spun::new(&board(squares), hot), SEED);
        let mut duel = Duel::for_run(&run, encounter, SEED);
        while duel.turn() < turn {
            duel.confirm();
            duel.hold();
        }
        duel.opposing().first().is_some_and(|o| o.face_up)
    }

    #[test]
    fn early_read_at_rank_one_reads_turn_one_of_boss_fights_only() {
        let r1 = [(Square::EarlyRead, 1)];
        assert!(read_on(&r1, false, BOSS, 1));
        assert!(!read_on(&r1, false, BOSS, 2));
        assert!(!read_on(&r1, false, MINION, 1));
        assert!(!read_on(&[], false, BOSS, 1));
    }

    #[test]
    fn early_read_grows_to_every_encounter_and_more_turns() {
        assert!(read_on(&[(Square::EarlyRead, 2)], false, MINION, 1));
        assert!(!read_on(&[(Square::EarlyRead, 2)], false, MINION, 2));
        assert!(read_on(&[(Square::EarlyRead, 3)], false, MINION, 2));
        assert!(!read_on(&[(Square::EarlyRead, 3)], false, MINION, 3));
        assert!(read_on(&[(Square::EarlyRead, 3)], true, MINION, 3));
    }
}
