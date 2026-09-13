//! Where the run is. A pure model of the linear floor table, with no Bevy in
//! it beyond the `Resource` derive, so the routing rules can be tested on
//! their own.

use bevy::prelude::*;

use crate::overworld::narrative;
use crate::run::{CombatOutcome, EncounterId, RewardOffer};
use crate::state::AppState;

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

/// Who sits down across from you.
pub fn encounter_intro(id: EncounterId) -> &'static str {
    match id {
        EncounterId::FloorMinion => narrative::ENC_FLOOR_MINION,
        EncounterId::Slotz => narrative::ENC_SLOTZ,
        EncounterId::PitMinion => narrative::ENC_PIT_MINION,
        EncounterId::PitBoss => narrative::ENC_PIT_BOSS,
        EncounterId::TheHouse => narrative::ENC_THE_HOUSE,
        EncounterId::Tutorial => narrative::TUTORIAL_INTRO,
    }
}

/// What the player reads on clearing this encounter's Stack.
pub fn win_line(id: EncounterId) -> &'static str {
    match id {
        EncounterId::FloorMinion | EncounterId::PitMinion => narrative::WIN_MINION,
        EncounterId::Slotz => narrative::WIN_SLOTZ,
        EncounterId::PitBoss => narrative::WIN_PIT_BOSS,
        EncounterId::TheHouse => narrative::WIN_THE_HOUSE,
        EncounterId::Tutorial => narrative::TUTORIAL_DONE,
    }
}

/// Every encounter in a run, in order. The whole progression is this list.
const RUN: [EncounterId; 5] = [
    EncounterId::FloorMinion,
    EncounterId::Slotz,
    EncounterId::PitMinion,
    EncounterId::PitBoss,
    EncounterId::TheHouse,
];

/// Which floor the encounter at `index` sits on.
fn floor_of(index: usize) -> Floor {
    match RUN.get(index) {
        Some(EncounterId::FloorMinion | EncounterId::Slotz) => Floor::TheFloor,
        Some(EncounterId::PitMinion | EncounterId::PitBoss) => Floor::ThePit,
        _ => Floor::BigShotsTable,
    }
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
    pub fn at(id: EncounterId) -> Self {
        let next = RUN
            .iter()
            .position(|&encounter| encounter == id)
            .expect("every EncounterId is somewhere in RUN");
        Self { next }
    }

    /// The encounter about to be played, or `None` once The House is beaten.
    pub fn encounter(&self) -> Option<EncounterId> {
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
            (CombatOutcome::Won, Some(EncounterId::TheHouse)) => AppState::Ending,
            (CombatOutcome::Won, _) => AppState::Reward,
        }
    }

    /// What the encounter just won pays. `None` once The House is beaten, or
    /// after The House itself, which pays in an ending.
    pub fn reward_offer(&self) -> Option<RewardOffer> {
        self.encounter().and_then(RewardOffer::for_encounter)
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
        let mut progress = Progress::at(EncounterId::PitBoss);

        assert_eq!(progress.encounter(), Some(EncounterId::PitBoss));
        // Same floor as the Pit minion before it, so no arrival prose.
        assert_eq!(progress.arrival(), AppState::FightOrFold);
        assert_eq!(progress.floor(), Floor::ThePit);
        assert!(matches!(
            progress.reward_offer(),
            Some(RewardOffer::Pick(..))
        ));

        progress.advance();
        assert_eq!(progress.encounter(), Some(EncounterId::TheHouse));
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

        assert_eq!(progress.encounter(), Some(EncounterId::TheHouse));
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
                (Floor::TheFloor, EncounterId::FloorMinion),
                (Floor::TheFloor, EncounterId::Slotz),
                (Floor::ThePit, EncounterId::PitMinion),
                (Floor::ThePit, EncounterId::PitBoss),
                (Floor::BigShotsTable, EncounterId::TheHouse),
            ]
        );
    }
}
