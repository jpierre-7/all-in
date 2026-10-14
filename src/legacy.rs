//! Legacy Perks (#168): picked after a House win, for the next run only.
//! Vocabulary follows `GLOSSARY.md`; the design is #143's resolution.
//!
//! The pick waits in the [`SaveSlot`] between runs. The next run takes it out
//! as it starts ([`apply`]), so it's spent however that run ends: a win,
//! death, a Fold, or quitting mid-run, which loses the run. Three of them
//! bend the duel the way any Perk does, as a [`Modifier`]; Victory Lap swaps
//! the starting deck instead.
//!
//! [`SaveSlot`]: crate::save::SaveSlot

use serde::{Deserialize, Serialize};

use crate::combat::duel::{Coin, Opposing};
use crate::modifier::{Modifier, Side, Turn};
use crate::run::{Card, Perk, RunState, xorshift64};
use crate::save::SavedCard;

/// How many Legacy Perks a House win puts on the table.
pub const OFFERED: usize = 3;

/// The Legacy pool. Victory Lap carries the deck The House was beaten with,
/// so the deck is only ever saved when that's the perk.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum LegacyPerk {
    HouseMoney,
    HighLimit,
    MarkedCards,
    VictoryLap { deck: Vec<SavedCard> },
    Comped,
    LuckyStreak,
}

impl LegacyPerk {
    /// Every Legacy Perk, in the order the pool lists them, Victory Lap
    /// carrying `deck`.
    pub fn all(deck: &[Card]) -> [LegacyPerk; 6] {
        [
            LegacyPerk::HouseMoney,
            LegacyPerk::HighLimit,
            LegacyPerk::MarkedCards,
            LegacyPerk::victory_lap(deck),
            LegacyPerk::Comped,
            LegacyPerk::LuckyStreak,
        ]
    }

    /// Victory Lap, carrying `deck`: the deck The House was beaten with.
    pub fn victory_lap(deck: &[Card]) -> Self {
        LegacyPerk::VictoryLap {
            deck: deck.iter().map(SavedCard::from).collect(),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            LegacyPerk::HouseMoney => "House Money",
            LegacyPerk::HighLimit => "High Limit",
            LegacyPerk::MarkedCards => "Marked Cards",
            LegacyPerk::VictoryLap { .. } => "Victory Lap",
            LegacyPerk::Comped => "Comped",
            LegacyPerk::LuckyStreak => "Lucky Streak",
        }
    }

    /// Whether it does anything yet. House Money and Comped wait on Cash and
    /// the Cage, and stay out of the draw until then.
    pub fn ready(&self) -> bool {
        !matches!(self, LegacyPerk::HouseMoney | LegacyPerk::Comped)
    }

    /// The line the pick puts against its key.
    pub fn describe(&self) -> String {
        let does = match self {
            LegacyPerk::HouseMoney => "start with +15 Cash.".to_string(),
            LegacyPerk::HighLimit => format!(
                "your Blind is one card higher on the first {HIGH_LIMIT_TURNS} turns of every duel."
            ),
            LegacyPerk::MarkedCards => {
                "the leftmost Opposing Card turns face up every turn.".to_string()
            }
            LegacyPerk::VictoryLap { deck } => format!(
                "start with the {} cards you beat The House with.",
                deck.len()
            ),
            LegacyPerk::Comped => "the first purchase in each Cage is free.".to_string(),
            LegacyPerk::LuckyStreak => {
                format!("Push Your Luck's coin falls your way {LUCKY_STREAK_PCT} times in 100.")
            }
        };
        format!("{}: {does}", self.name())
    }
}

/// What a House win offers: [`OFFERED`] different Legacy Perks drawn at
/// random from the ones that do something yet, Victory Lap carrying `deck`,
/// the deck The House was just beaten with.
pub fn offer(deck: &[Card], seed: u64) -> Vec<LegacyPerk> {
    let mut pool: Vec<LegacyPerk> = LegacyPerk::all(deck)
        .into_iter()
        .filter(LegacyPerk::ready)
        .collect();
    let mut rng = seed | 1;
    for i in (1..pool.len()).rev() {
        let j = (xorshift64(&mut rng) % (i as u64 + 1)) as usize;
        pool.swap(i, j);
    }
    pool.truncate(OFFERED);
    pool
}

/// Start `run`, still fresh, under `perk`. It's taken before any boss Perk,
/// so the duel applies it first of them. One that does nothing yet changes
/// nothing.
pub fn apply(run: &mut RunState, perk: &LegacyPerk) {
    match perk {
        LegacyPerk::HighLimit => run.perks.push(&HIGH_LIMIT),
        LegacyPerk::MarkedCards => run.perks.push(&MARKED_CARDS),
        LegacyPerk::LuckyStreak => run.perks.push(&LUCKY_STREAK),
        LegacyPerk::VictoryLap { deck } => run.deck = deck.iter().map(SavedCard::card).collect(),
        LegacyPerk::HouseMoney | LegacyPerk::Comped => {}
    }
}

// ---------------------------------------------------------------------------
// The ones that bend the duel
// ---------------------------------------------------------------------------

/// How many turns of each duel High Limit raises the Blind for. Tuned on
/// the balance sim: all run took smart's win rate from 8% to 40%; two turns
/// takes it to 18%, beside Lucky Streak.
pub const HIGH_LIMIT_TURNS: u32 = 2;

/// High Limit: the player's Blind is one higher on the first
/// [`HIGH_LIMIT_TURNS`] turns of every duel.
pub static HIGH_LIMIT: Perk = Perk {
    label: "High Limit: your Blind is one card higher on the first 2 turns of every duel.",
    modifier: &HighLimit,
};

#[derive(Debug)]
struct HighLimit;

impl Modifier for HighLimit {
    fn blind(&self, side: Side, turn: Turn, blind: u8) -> u8 {
        match side {
            Side::Player if turn.number <= HIGH_LIMIT_TURNS => blind.saturating_add(1),
            _ => blind,
        }
    }
}

/// Marked Cards: a Reveal of the leftmost Opposing Card at the start of
/// every turn. Against The House, which plays last, it has nothing to show.
pub static MARKED_CARDS: Perk = Perk {
    label: "Marked Cards: the leftmost Opposing Card turns face up every turn.",
    modifier: &MarkedCards,
};

#[derive(Debug)]
struct MarkedCards;

impl Modifier for MarkedCards {
    fn reveal(&self, _turn: u32, opposing: &[Opposing]) -> Vec<usize> {
        if opposing.is_empty() {
            Vec::new()
        } else {
            vec![0]
        }
    }
}

/// Lucky Streak's chance of taking one flip, in percent. The coin is 45 out
/// of 100 without it.
pub const LUCKY_STREAK_PCT: u32 = 60;

/// Lucky Streak: Push Your Luck's coin is weighted at least 60/40 the
/// player's way. Only the weight changes, so a best-of coin stays best-of.
pub static LUCKY_STREAK: Perk = Perk {
    label: "Lucky Streak: Push Your Luck's coin is weighted 60/40 your way.",
    modifier: &LuckyStreak,
};

#[derive(Debug)]
struct LuckyStreak;

impl Modifier for LuckyStreak {
    fn coin(&self, coin: Coin) -> Coin {
        Coin {
            player_pct: coin.player_pct.max(LUCKY_STREAK_PCT),
            ..coin
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss::{SLOTZ, THE_HOUSE};
    use crate::combat::duel::Duel;
    use crate::run::{Encounter, Floor, Tell, starter_deck};
    use crate::wheel::{self, Spun};

    const SEED: u64 = 0x1234_5678_9abc_def0;

    const MINION: Encounter = Encounter::Minion {
        floor: Floor::TheFloor,
    };
    const HOUSE: Encounter = Encounter::Boss {
        boss: &THE_HOUSE,
        floor: Floor::BigShotsTable,
    };

    fn winning_deck() -> Vec<Card> {
        vec![
            Card {
                name: "Hot Streak",
                face_value: 3,
                tell: Some(Tell::Streak),
            },
            Card {
                name: "Pawned Ring",
                face_value: 8,
                tell: None,
            },
            Card {
                name: "The River",
                face_value: 2,
                tell: Some(Tell::Flop),
            },
        ]
    }

    /// A fresh run started under `perk`.
    fn run_under(perk: &LegacyPerk) -> RunState {
        let mut run = RunState::new();
        apply(&mut run, perk);
        run
    }

    // --- the draw ---

    #[test]
    fn a_house_win_offers_three_different_perks() {
        for seed in 1..200 {
            let offered = offer(&winning_deck(), seed * 7919);
            assert_eq!(offered.len(), 3);
            for (i, perk) in offered.iter().enumerate() {
                assert!(
                    offered[i + 1..]
                        .iter()
                        .all(|other| other.name() != perk.name()),
                    "{offered:?}"
                );
            }
        }
    }

    #[test]
    fn house_money_and_comped_stay_out_of_the_draw() {
        for seed in 1..200 {
            for perk in offer(&winning_deck(), seed * 7919) {
                assert!(
                    !matches!(perk, LegacyPerk::HouseMoney | LegacyPerk::Comped),
                    "{perk:?} was offered"
                );
            }
        }
    }

    #[test]
    fn every_ready_perk_turns_up_and_the_draw_is_seeded() {
        let mut seen = std::collections::BTreeSet::new();
        for seed in 1..200 {
            for perk in offer(&winning_deck(), seed * 7919) {
                seen.insert(perk.name());
            }
        }
        assert_eq!(
            seen.into_iter().collect::<Vec<_>>(),
            ["High Limit", "Lucky Streak", "Marked Cards", "Victory Lap"]
        );
        assert_eq!(offer(&winning_deck(), SEED), offer(&winning_deck(), SEED));
    }

    #[test]
    fn victory_lap_is_offered_with_the_deck_that_beat_the_house() {
        let lap = (1..200)
            .flat_map(|seed| offer(&winning_deck(), seed * 7919))
            .find(|perk| matches!(perk, LegacyPerk::VictoryLap { .. }))
            .expect("Victory Lap turns up");
        assert_eq!(
            lap.describe(),
            "Victory Lap: start with the 3 cards you beat The House with."
        );
        assert_eq!(run_under(&lap).deck, winning_deck());
    }

    // --- the next run ---

    #[test]
    fn victory_lap_starts_the_run_with_that_deck_and_the_wheel_adds_to_it() {
        let lap = LegacyPerk::victory_lap(&winning_deck());
        let board = [(wheel::Square::InsideMan, 3)].into_iter().collect();
        let run = wheel::start_run(
            run_under(&lap),
            Spun::new(&board, Some(wheel::Square::InsideMan)),
            SEED,
        );
        assert_eq!(run.deck.len(), winning_deck().len() + 1);
        assert_eq!(run.deck[..3], winning_deck()[..]);
    }

    #[test]
    fn a_perk_that_does_nothing_yet_leaves_the_run_fresh() {
        for perk in [LegacyPerk::HouseMoney, LegacyPerk::Comped] {
            let run = run_under(&perk);
            assert!(run.perks.is_empty());
            assert_eq!(run.deck, starter_deck());
        }
    }

    #[test]
    fn lucky_streak_and_the_slotz_coin_stack() {
        // Lucky Streak is taken first; Slotz's best of three comes later in
        // the run and keeps its weight.
        let mut run = run_under(&LegacyPerk::LuckyStreak);
        let [best_of_three, _] = SLOTZ.rewards.expect("Slotz pays a 1-of-2");
        run.apply(best_of_three, 0);
        assert_eq!(
            Duel::for_run(&run, MINION, SEED).coin(),
            Coin {
                player_pct: 60,
                best_of: 3
            }
        );
    }

    #[test]
    fn high_limit_raises_the_players_blind_on_the_first_two_turns_only() {
        let mut plain = Duel::for_run(&RunState::new(), MINION, SEED);
        let mut duel = Duel::for_run(&run_under(&LegacyPerk::HighLimit), MINION, SEED);
        for (turn, raised) in [(1, 1), (2, 1), (3, 0), (4, 0)] {
            assert_eq!(duel.turn(), turn);
            assert_eq!(
                duel.blind(Side::Player),
                plain.blind(Side::Player) + raised,
                "turn {turn}"
            );
            assert_eq!(duel.blind(Side::Enemy), plain.blind(Side::Enemy));
            for duel in [&mut duel, &mut plain] {
                duel.confirm();
                duel.hold();
            }
        }
    }

    #[test]
    fn marked_cards_turns_the_leftmost_opposing_card_every_turn() {
        let mut duel = Duel::for_run(&run_under(&LegacyPerk::MarkedCards), MINION, SEED);
        for _ in 0..4 {
            let opposing = duel.opposing();
            assert!(opposing[0].face_up, "turn {}", duel.turn());
            assert!(opposing[1..].iter().all(|o| !o.face_up));
            duel.confirm();
            duel.hold();
        }
        let plain = Duel::for_run(&RunState::new(), MINION, SEED);
        assert!(!plain.opposing()[0].face_up);
    }

    #[test]
    fn marked_cards_has_nothing_to_show_against_the_house() {
        let duel = Duel::for_run(&run_under(&LegacyPerk::MarkedCards), HOUSE, SEED);
        assert!(duel.opposing().iter().all(|o| !o.face_up));
    }

    #[test]
    fn lucky_streak_weights_the_coin_sixty_forty() {
        let duel = Duel::for_run(&run_under(&LegacyPerk::LuckyStreak), MINION, SEED);
        assert_eq!(
            duel.coin(),
            Coin {
                player_pct: 60,
                best_of: 1
            }
        );
    }

    #[test]
    fn every_pick_reads_as_its_name_and_what_it_does() {
        for perk in LegacyPerk::all(&winning_deck()) {
            assert!(perk.describe().starts_with(perk.name()), "{perk:?}");
        }
    }
}
