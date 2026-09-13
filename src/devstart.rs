//! The dev entry point: start a run partway up the casino, with a pinned seed
//! and a pocket already full, so one encounter can be playtested on its own
//! (#33). Debug builds only — `main` does not declare this module in release,
//! so a shipped binary has no flags to find.
//!
//! ```text
//! cargo run -- --encounter pit-boss --seed 42 --with sixplays,dice
//! ```
//!
//! Everything here goes in through the doors the game already uses:
//! `Progress::at` for where the walk resumes, and `RunState::apply` for the
//! pocket, which is the one sanctioned way run state ever changes. Nothing
//! reaches past a seam the real game respects, so what you playtest is what
//! ships.
//!
//! One thing to know: Folding or dying still drops you in the Lobby, and the
//! Lobby still resets the run (`end_the_run`). The flags set up the *first*
//! encounter, not a mode — a second run from that Lobby is an ordinary one.

use bevy::prelude::*;

use crate::combat::DuelSeed;
use crate::overworld::progression::Progress;
use crate::run::{EncounterId, Reward, RunState};

const USAGE: &str = "\
All In — dev entry point (debug builds only)

    cargo run -- [--encounter <id>] [--seed <n>] [--with <a,b,...>]

    --encounter <id>  Sit down at this encounter instead of walking from the
                      Lobby: floor-minion, slotz, pit-minion, pit-boss,
                      the-house.
    --seed <n>        Pin the shuffle, the deal and the Push Your Luck coin,
                      so a Hand that went wrong can be played again.
    --with <a,b,...>  Rewards to start holding, as if they had been won:
                      dice, streak, pyl, sixplays, pack. Needs --encounter,
                      since starting in the Lobby ends the run.
    --help            This.

With no flags the game opens in the Lobby, exactly as it ships.";

/// The pack of cards the Pit Boss reward rolls has to come from somewhere even
/// when no seed is given. Any constant will do; it only has to be the same one
/// every launch.
const DEFAULT_REWARD_SEED: u64 = 0x5eed_1337_c0ff_ee01;

/// What the command line asked for.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct DevStart {
    encounter: Option<EncounterId>,
    seed: Option<u64>,
    rewards: Vec<Reward>,
}

/// What to do about it.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// No flags: the game as it ships.
    Play,
    Start(DevStart),
    Usage,
}

/// Read the command line, before `main` adds a single plugin.
///
/// Reading has to come first because `WinitPlugin` builds the event loop in
/// its `build`: parse any later and a typo costs you a window before it is
/// told it was a typo, and `--help` needs a display to print itself.
/// Installing is the other half, and has to come last - see `install`.
pub fn read() -> Option<DevStart> {
    match parse(std::env::args().skip(1)) {
        Ok(Outcome::Play) => None,
        Ok(Outcome::Start(dev)) => Some(dev),
        Ok(Outcome::Usage) => {
            println!("{USAGE}");
            std::process::exit(0);
        }
        Err(problem) => {
            eprintln!("{problem}\n\n{USAGE}");
            std::process::exit(2);
        }
    }
}

fn parse(args: impl IntoIterator<Item = String>) -> Result<Outcome, String> {
    let mut dev = DevStart::default();
    let mut args = args.into_iter();
    let mut saw_a_flag = false;

    while let Some(arg) = args.next() {
        // `--flag value` and `--flag=value` both read the same way.
        let (flag, inline) = match arg.split_once('=') {
            Some((flag, value)) => (flag.to_string(), Some(value.to_string())),
            None => (arg, None),
        };
        saw_a_flag = true;

        match flag.as_str() {
            "--help" | "-h" => return Ok(Outcome::Usage),
            "--encounter" => {
                dev.encounter = Some(encounter(&value(&flag, inline, &mut args)?)?);
            }
            "--seed" => {
                let given = value(&flag, inline, &mut args)?;
                dev.seed = Some(
                    given
                        .parse()
                        .map_err(|_| format!("`--seed` wants a whole number, not `{given}`."))?,
                );
            }
            "--with" => {
                for name in value(&flag, inline, &mut args)?.split(',') {
                    dev.rewards.push(reward(name.trim())?);
                }
            }
            other => return Err(format!("`{other}` is not a flag this game knows.")),
        }
    }

    // A pocket with nowhere to start is a pocket that gets emptied: without
    // `--encounter` the game opens in the Lobby, and the Lobby ends the run
    // (`end_the_run`) before the first hand is dealt. Better to say so than to
    // hand over a run that has quietly lost what was asked for.
    if !dev.rewards.is_empty() && dev.encounter.is_none() {
        return Err(
            "`--with` needs an `--encounter` to go with it: starting in the Lobby ends \
             the run, and the Lobby is where the game opens without one."
                .to_string(),
        );
    }

    Ok(if saw_a_flag {
        Outcome::Start(dev)
    } else {
        Outcome::Play
    })
}

/// The value belonging to `flag`, whether it came after an `=` or as the next
/// argument along.
fn value(
    flag: &str,
    inline: Option<String>,
    rest: &mut impl Iterator<Item = String>,
) -> Result<String, String> {
    inline
        .or_else(|| rest.next())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("`{flag}` needs a value after it."))
}

fn encounter(name: &str) -> Result<EncounterId, String> {
    match name {
        "floor-minion" => Ok(EncounterId::FloorMinion),
        "slotz" => Ok(EncounterId::Slotz),
        "pit-minion" => Ok(EncounterId::PitMinion),
        "pit-boss" => Ok(EncounterId::PitBoss),
        "the-house" => Ok(EncounterId::TheHouse),
        other => Err(format!("Nobody called `{other}` sits down in this casino.")),
    }
}

fn reward(name: &str) -> Result<Reward, String> {
    match name {
        "dice" => Ok(Reward::LoadedDice),
        "streak" => Ok(Reward::SlotzStreakCards),
        "pyl" => Ok(Reward::SlotzPylBestTwoOfThree),
        "sixplays" => Ok(Reward::PitBossSixPlays),
        "pack" => Ok(Reward::PitBossRandomCards),
        other => Err(format!("There is no reward called `{other}`.")),
    }
}

impl DevStart {
    /// Set the app up as the flags asked. Called from `main` after the
    /// plugins, so what it writes lands on top of the resources and the state
    /// `OverworldPlugin` puts in during `build`.
    pub fn install(self, app: &mut App) {
        let mut said = Vec::new();

        if !self.rewards.is_empty() {
            let mut run = RunState::new();
            for reward in &self.rewards {
                run.apply(*reward, self.seed.unwrap_or(DEFAULT_REWARD_SEED));
            }
            said.push(format!(
                "holding {} ({} cards, {} Plays)",
                self.rewards.len(),
                run.deck.len(),
                run.plays()
            ));
            app.insert_resource(run);
        }

        if let Some(seed) = self.seed {
            said.push(format!("seed {seed}"));
            app.insert_resource(DuelSeed::new(seed));
        }

        if let Some(id) = self.encounter {
            let progress = Progress::at(id);
            // Where the walk would have put the player on reaching this
            // encounter: the floor's prose if it is the first one on its
            // floor, otherwise straight to Fight or Fold.
            let arrival = progress.arrival();
            said.push(format!("starting at {id:?}"));
            app.insert_resource(progress);
            // After the plugins, so this overwrites the state the overworld
            // initialised and replaces its opening transition. The Lobby is
            // never entered, which is the point: entering it would reset the
            // run that was just set up.
            app.insert_state(arrival);
        }

        println!("dev entry point: {}.", said.join(", "));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::Perk;

    /// One argument per element, the way a shell hands them over.
    fn parsed(args: &[&str]) -> Result<Outcome, String> {
        parse(args.iter().map(|arg| arg.to_string()))
    }

    fn started(args: &[&str]) -> DevStart {
        match parsed(args) {
            Ok(Outcome::Start(dev)) => dev,
            other => panic!("expected a dev start from `{args:?}`, got {other:?}"),
        }
    }

    #[test]
    fn no_flags_is_the_game_as_it_ships() {
        assert_eq!(parsed(&[]), Ok(Outcome::Play));
    }

    #[test]
    fn an_encounter_can_be_named() {
        assert_eq!(
            started(&["--encounter", "pit-boss"]).encounter,
            Some(EncounterId::PitBoss)
        );
    }

    #[test]
    fn every_encounter_in_the_run_has_a_name_on_the_command_line() {
        for (name, id) in [
            ("floor-minion", EncounterId::FloorMinion),
            ("slotz", EncounterId::Slotz),
            ("pit-minion", EncounterId::PitMinion),
            ("pit-boss", EncounterId::PitBoss),
            ("the-house", EncounterId::TheHouse),
        ] {
            assert_eq!(encounter(name), Ok(id));
        }
    }

    #[test]
    fn a_value_can_come_after_a_space_or_an_equals() {
        assert_eq!(started(&["--seed", "42"]), started(&["--seed=42"]));
        assert_eq!(started(&["--seed", "42"]).seed, Some(42));
    }

    #[test]
    fn rewards_come_as_a_comma_separated_list() {
        assert_eq!(
            started(&["--encounter", "pit-boss", "--with", "sixplays,dice"]).rewards,
            vec![Reward::PitBossSixPlays, Reward::LoadedDice]
        );
    }

    #[test]
    fn spaces_around_a_reward_name_do_not_count() {
        assert_eq!(
            started(&["--encounter", "pit-boss", "--with", "sixplays, dice"]).rewards,
            vec![Reward::PitBossSixPlays, Reward::LoadedDice]
        );
    }

    #[test]
    fn the_flags_can_be_combined_in_any_order() {
        let one = started(&["--encounter", "pit-boss", "--seed", "7", "--with", "dice"]);
        let two = started(&["--with", "dice", "--seed", "7", "--encounter", "pit-boss"]);

        assert_eq!(one, two);
        assert_eq!(one.encounter, Some(EncounterId::PitBoss));
        assert_eq!(one.seed, Some(7));
        assert_eq!(one.rewards, vec![Reward::LoadedDice]);
    }

    #[test]
    fn help_asks_for_the_usage() {
        assert_eq!(parsed(&["--help"]), Ok(Outcome::Usage));
        assert_eq!(parsed(&["-h"]), Ok(Outcome::Usage));
    }

    #[test]
    fn a_bad_flag_says_so_rather_than_starting_a_run() {
        assert!(parsed(&["--encounters", "pit-boss"]).is_err());
        assert!(parsed(&["--encounter", "the-cloakroom"]).is_err());
        assert!(parsed(&["--with", "dentures"]).is_err());
        assert!(parsed(&["--seed", "soon"]).is_err());
    }

    #[test]
    fn a_flag_with_nothing_after_it_says_so() {
        assert!(parsed(&["--encounter"]).is_err());
        assert!(parsed(&["--seed"]).is_err());
        assert!(parsed(&["--with"]).is_err());
    }

    #[test]
    fn a_pocket_with_nowhere_to_start_is_refused() {
        // The Lobby would empty it before the first hand.
        assert!(parsed(&["--with", "dice"]).is_err());
        assert!(parsed(&["--with", "dice", "--encounter", "pit-boss"]).is_ok());
    }

    #[test]
    fn a_seed_on_its_own_is_fine() {
        // Nothing resets it, so pinning the shuffle from the Lobby works.
        assert_eq!(started(&["--seed", "42"]).seed, Some(42));
    }

    #[test]
    fn the_pocket_is_built_through_the_same_door_the_game_uses() {
        // Not a parse test: the point is that `--with` produces exactly the
        // run state that winning those rewards would have.
        let dev = started(&["--encounter", "pit-boss", "--with", "sixplays,dice,streak"]);
        let mut run = RunState::new();
        for reward in &dev.rewards {
            run.apply(*reward, DEFAULT_REWARD_SEED);
        }

        assert_eq!(run.perks, vec![Perk::SixPlaysSteepBlinds]);
        assert_eq!(run.plays(), 6);
        assert_eq!(run.loaded_dice(), 2);
        assert_eq!(run.deck.len(), RunState::new().deck.len() + 3);
    }
}
