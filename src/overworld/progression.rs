//! Where the run is. A pure model of the linear floor table, with no Bevy in
//! it beyond the `Resource` derive, so the routing rules can be tested on
//! their own.

use bevy::prelude::*;

use crate::boss::{PIT_BOSS, SLOTZ, THE_HOUSE};
use crate::overworld::narrative;
pub use crate::run::Floor;
use crate::run::{CombatOutcome, Encounter, RewardOffer};
use crate::state::AppState;

impl Floor {
    /// The arrival prose the player reads on stepping onto this floor.
    pub fn intro(self) -> &'static str {
        match self {
            Self::TheFloor => narrative::FLOOR_INTRO,
            Self::ThePit => narrative::PIT_INTRO,
            Self::BigShotsTable => narrative::BIG_SHOTS_INTRO,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::TheFloor => "The Floor",
            Self::ThePit => "The Pit",
            Self::BigShotsTable => "The Big Shots Table",
        }
    }
}

/// Who sits down across from you. A boss brings its own words.
pub fn encounter_intro(encounter: Encounter) -> &'static str {
    match encounter {
        Encounter::Boss { boss, .. } => boss.intro,
        Encounter::Minion { floor } => match floor {
            Floor::TheFloor => narrative::ENC_FLOOR_MINION,
            Floor::ThePit => narrative::ENC_PIT_MINION,
            Floor::BigShotsTable => narrative::ENC_BIG_SHOTS_MINION,
        },
        Encounter::Practice => narrative::TUTORIAL_INTRO,
    }
}

/// What the player reads on clearing this encounter's Chips.
pub fn win_line(encounter: Encounter) -> &'static str {
    match encounter {
        Encounter::Boss { boss, .. } => boss.win,
        Encounter::Minion { .. } => narrative::WIN_MINION,
        Encounter::Practice => narrative::TUTORIAL_DONE,
    }
}

/// Every encounter in a run, in order: the floor layout. The whole
/// progression is this list; the balance sim (`tools/sim`) walks it too.
pub const RUN: [Encounter; 5] = [
    Encounter::Minion {
        floor: Floor::TheFloor,
    },
    Encounter::Boss {
        boss: &SLOTZ,
        floor: Floor::TheFloor,
    },
    Encounter::Minion {
        floor: Floor::ThePit,
    },
    Encounter::Boss {
        boss: &PIT_BOSS,
        floor: Floor::ThePit,
    },
    Encounter::Boss {
        boss: &THE_HOUSE,
        floor: Floor::BigShotsTable,
    },
];

/// Which floor the encounter at `index` sits on.
fn floor_of(index: usize) -> Floor {
    RUN.get(index)
        .and_then(|encounter| encounter.floor())
        .unwrap_or(Floor::BigShotsTable)
}

/// How far into `RUN` the player is. Reset with `Progress::new` on Fold or death.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Progress {
    next: usize,
}

impl Progress {
    pub fn new() -> Self {
        Self { next: 0 }
    }

    /// Start the walk at `id` rather than at the door. Only the dev entry
    /// point (`--encounter`, #33) uses this; a real run always starts at
    /// `new()`. Going through `Progress` rather than dropping straight into
    /// `Combat` is the point: the encounter after it, the floor prose, and the
    /// reward screen all still come out right.
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    pub fn at(encounter: Encounter) -> Self {
        let next = RUN
            .iter()
            .position(|&e| e == encounter)
            .expect("the encounter is somewhere in RUN");
        Self { next }
    }

    /// The encounter about to be played, or `None` once The House is beaten.
    pub fn encounter(&self) -> Option<Encounter> {
        RUN.get(self.next).copied()
    }

    /// The floor the current encounter sits on.
    pub fn floor(&self) -> Floor {
        floor_of(self.next)
    }

    /// The screen the player meets on reaching the current encounter: the
    /// floor's arrival prose the first time, then straight to Fight or Fold.
    pub fn arrival(&self) -> AppState {
        match self.next.checked_sub(1) {
            Some(previous) if floor_of(previous) == self.floor() => AppState::FightOrFold,
            _ => AppState::FloorIntro,
        }
    }

    /// Where the overworld goes once combat hands back an outcome. Losing ends
    /// the run; winning pays out, except against The House, which ends it.
    pub fn route(&self, outcome: CombatOutcome) -> AppState {
        match (outcome, self.encounter()) {
            (CombatOutcome::Lost, _) => AppState::GameOver,
            (CombatOutcome::Won, _) if self.next + 1 == RUN.len() => AppState::Ending,
            (CombatOutcome::Won, _) => AppState::Reward,
        }
    }

    /// What the encounter just won pays. `None` once The House is beaten, or
    /// after The House itself, which pays in an ending.
    pub fn reward_offer(&self) -> Option<RewardOffer> {
        self.encounter().and_then(Encounter::reward_offer)
    }

    /// Move past the encounter just won.
    pub fn advance(&mut self) {
        self.next += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_house_is_the_last_encounter_of_the_run() {
        assert_eq!(
            RUN.last(),
            Some(&Encounter::Boss {
                boss: &THE_HOUSE,
                floor: Floor::BigShotsTable
            })
        );
    }

    #[test]
    fn every_floor_has_arrival_prose() {
        assert!(Floor::TheFloor.intro().contains("rows of slots"));
        assert!(Floor::ThePit.intro().contains("velvet rope"));
        assert!(Floor::BigShotsTable.intro().contains("one lamp"));
    }

    #[test]
    fn every_encounter_introduces_itself_and_has_a_line_for_beating_it() {
        let lines: Vec<_> = RUN
            .iter()
            .map(|&id| (encounter_intro(id), win_line(id)))
            .collect();

        assert!(lines[0].0.contains("rented tux"));
        assert!(lines[1].0.contains("SLOTZ"));
        assert!(lines[2].0.contains("scar under one eye"));
        assert!(lines[3].0.contains("THE PIT BOSS"));
        assert!(lines[4].0.contains("THE HOUSE"));

        assert!(lines[0].1.contains("chair scrapes back"));
        assert!(lines[1].1.contains("neon goes out"));
        assert!(lines[2].1.contains("chair scrapes back"));
        assert!(lines[3].1.contains("Upstairs"));
        assert!(lines[4].1.contains("the table is yours"));
    }

    #[test]
    fn arriving_on_a_new_floor_reads_its_prose_first() {
        let mut progress = Progress::new();
        let mut arrivals = Vec::new();

        while progress.encounter().is_some() {
            arrivals.push(progress.arrival());
            progress.advance();
        }

        assert_eq!(
            arrivals,
            vec![
                AppState::FloorIntro,  // The Floor
                AppState::FightOrFold, // still The Floor
                AppState::FloorIntro,  // The Pit
                AppState::FightOrFold, // still The Pit
                AppState::FloorIntro,  // The Big Shots Table
            ]
        );
    }

    #[test]
    fn starting_at_an_encounter_leaves_the_rest_of_the_walk_intact() {
        let mut progress = Progress::at(RUN[3]);

        assert_eq!(progress.encounter(), Some(RUN[3]));
        // Same floor as the Pit minion before it, so no arrival prose.
        assert_eq!(progress.arrival(), AppState::FightOrFold);
        assert_eq!(progress.floor(), Floor::ThePit);
        assert!(matches!(
            progress.reward_offer(),
            Some(RewardOffer::Pick(..))
        ));

        progress.advance();
        assert_eq!(progress.encounter(), Some(RUN[4]));
    }

    #[test]
    fn every_encounter_can_be_started_at() {
        for (index, &id) in RUN.iter().enumerate() {
            let mut walked = Progress::new();
            for _ in 0..index {
                walked.advance();
            }

            assert_eq!(
                Progress::at(id),
                walked,
                "starting at {id:?} lands where walking there does"
            );
        }
    }

    #[test]
    fn losing_ends_the_run_wherever_it_happens() {
        let mut progress = Progress::new();

        while progress.encounter().is_some() {
            assert_eq!(progress.route(CombatOutcome::Lost), AppState::GameOver);
            progress.advance();
        }
    }

    #[test]
    fn beating_a_minion_or_a_boss_goes_to_the_reward_screen() {
        let mut progress = Progress::new();

        for _ in 0..4 {
            assert_eq!(progress.route(CombatOutcome::Won), AppState::Reward);
            progress.advance();
        }
    }

    #[test]
    fn the_reward_on_offer_is_the_one_for_the_encounter_just_won() {
        let mut progress = Progress::new();
        let mut offers = Vec::new();

        while progress.encounter().is_some() {
            offers.push(progress.reward_offer());
            progress.advance();
        }

        assert!(matches!(offers[0], Some(RewardOffer::Drop(_))));
        assert!(matches!(offers[1], Some(RewardOffer::Pick(..))));
        assert!(matches!(offers[2], Some(RewardOffer::Drop(_))));
        assert!(matches!(offers[3], Some(RewardOffer::Pick(..))));
        assert_eq!(offers[4], None, "The House pays in an ending");
        assert_eq!(
            progress.reward_offer(),
            None,
            "and there is nothing after it"
        );
    }

    #[test]
    fn beating_the_house_goes_to_the_ending() {
        let mut progress = Progress::new();
        for _ in 0..4 {
            progress.advance();
        }

        assert_eq!(progress.encounter(), Some(RUN[4]));
        assert_eq!(progress.route(CombatOutcome::Won), AppState::Ending);
    }

    #[test]
    fn a_run_walks_the_three_floors_in_order() {
        let mut progress = Progress::new();
        let mut walked = Vec::new();

        while let Some(encounter) = progress.encounter() {
            walked.push((progress.floor(), encounter));
            progress.advance();
        }

        assert_eq!(
            walked,
            vec![
                (Floor::TheFloor, RUN[0]),
                (Floor::TheFloor, RUN[1]),
                (Floor::ThePit, RUN[2]),
                (Floor::ThePit, RUN[3]),
                (Floor::BigShotsTable, RUN[4]),
            ]
        );
    }
}
