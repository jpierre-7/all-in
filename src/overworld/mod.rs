//! The overworld: the app shell, the lobby, and the linear walk up the casino.
//!
//! Owns every `AppState` transition except `Combat -> PostCombat` (ADR-0001).
//! Combat is a black box: the overworld inserts an `Encounter`, hands over, and
//! routes on the `CombatOutcome` it gets back.

#[allow(dead_code)] // superseded by combat::CombatPlugin; the tests below still drive it
pub mod combat_stub;
pub mod narrative;
pub mod progression;
pub mod screens;

use bevy::prelude::*;

use progression::{Progress, encounter_intro, win_line};
use screens::{Screen, any_key, confirm, digit_pressed};

use crate::run::{CombatOutcome, Encounter, Enemy, RewardOffer, RunState};
use crate::state::AppState;

pub struct OverworldPlugin;

/// Set when the player Folds, so the Lobby can say so. Cleared on arrival.
#[derive(Resource)]
struct Folded;

impl Plugin for OverworldPlugin {
    fn build(&self, app: &mut App) {
        // The overworld owns the state machine; combat only ever sets
        // `PostCombat` (ADR-0001).
        app.init_state::<AppState>()
            .insert_resource(Progress::new())
            .insert_resource(RunState::new())
            .add_systems(Startup, spawn_camera)
            .add_systems(OnEnter(AppState::Opening), show_opening)
            .add_systems(OnEnter(AppState::Lobby), (end_the_run, show_lobby))
            .add_systems(OnEnter(AppState::InfoRoom), show_info_room)
            .add_systems(OnEnter(AppState::Tutorial), show_tutorial)
            .add_systems(OnEnter(AppState::FloorIntro), show_floor_intro)
            .add_systems(OnEnter(AppState::FightOrFold), show_fight_or_fold)
            .add_systems(OnEnter(AppState::PostCombat), show_outcome)
            .add_systems(OnEnter(AppState::Reward), show_reward)
            .add_systems(OnEnter(AppState::Ending), show_ending)
            .add_systems(OnEnter(AppState::GameOver), show_game_over)
            .add_systems(
                Update,
                (
                    // Every screen that only needs dismissing goes the same
                    // place: back to the Lobby.
                    back_to_lobby.run_if(
                        in_state(AppState::Opening)
                            .or_else(in_state(AppState::InfoRoom))
                            .or_else(in_state(AppState::Tutorial))
                            .or_else(in_state(AppState::Ending))
                            .or_else(in_state(AppState::GameOver)),
                    ),
                    pick_from_lobby.run_if(in_state(AppState::Lobby)),
                    leave_floor_intro.run_if(in_state(AppState::FloorIntro)),
                    fight_or_fold.run_if(in_state(AppState::FightOrFold)),
                    leave_outcome.run_if(in_state(AppState::PostCombat)),
                    take_reward.run_if(in_state(AppState::Reward)),
                ),
            );
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

// ---------------------------------------------------------------------------
// Opening
// ---------------------------------------------------------------------------

fn show_opening(mut commands: Commands) {
    Screen::new()
        .title("ALL IN")
        .prose(narrative::OPENING)
        .footer(narrative::ANY_KEY)
        .spawn(&mut commands, AppState::Opening);
}

/// Dismisses the Opening, the two side rooms, and both endings.
fn back_to_lobby(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if any_key(&keys) {
        next.set(AppState::Lobby);
    }
}

// ---------------------------------------------------------------------------
// Lobby
// ---------------------------------------------------------------------------

fn show_lobby(mut commands: Commands, folded: Option<Res<Folded>>) {
    let mut screen = Screen::new().title("The Lobby");

    if folded.is_some() {
        screen = screen.prose(narrative::FOLD);
        commands.remove_resource::<Folded>();
    }

    screen
        .prose(narrative::LOBBY)
        .option(1, narrative::LOBBY_OPT_INFO)
        .option(2, narrative::LOBBY_OPT_TUTORIAL)
        .option(3, narrative::LOBBY_OPT_BEGIN)
        .footer("Press 1, 2 or 3 — or Enter to walk the Floor.")
        .spawn(&mut commands, AppState::Lobby);
}

/// Every way back into the Lobby ends a run, so the reset lives here: Fold and
/// death both leave behind the perks, items and deck changes of the old one.
fn end_the_run(mut run: ResMut<RunState>, mut progress: ResMut<Progress>) {
    *run = RunState::new();
    *progress = Progress::new();
}

fn pick_from_lobby(
    keys: Res<ButtonInput<KeyCode>>,
    progress: Res<Progress>,
    mut next: ResMut<NextState<AppState>>,
) {
    match digit_pressed(&keys) {
        Some(1) => next.set(AppState::InfoRoom),
        Some(2) => next.set(AppState::Tutorial),
        Some(3) => next.set(progress.arrival()),
        _ if confirm(&keys) => next.set(progress.arrival()),
        _ => {}
    }
}

fn show_info_room(mut commands: Commands) {
    Screen::new()
        .title("The Info Room")
        .prose(narrative::INFO_ROOM)
        .footer(narrative::ANY_KEY_BACK)
        .spawn(&mut commands, AppState::InfoRoom);
}

fn show_tutorial(mut commands: Commands) {
    Screen::new()
        .title("The Arcade")
        .prose(narrative::TUTORIAL)
        .footer(narrative::ANY_KEY_BACK)
        .spawn(&mut commands, AppState::Tutorial);
}

// ---------------------------------------------------------------------------
// Walking the floors
// ---------------------------------------------------------------------------

fn show_floor_intro(mut commands: Commands, progress: Res<Progress>) {
    let floor = progress.floor();
    Screen::new()
        .title(floor.name())
        .prose(floor.intro())
        .footer(narrative::ANY_KEY)
        .spawn(&mut commands, AppState::FloorIntro);
}

fn leave_floor_intro(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if any_key(&keys) {
        next.set(AppState::FightOrFold);
    }
}

fn show_fight_or_fold(mut commands: Commands, progress: Res<Progress>, run: Res<RunState>) {
    let Some(id) = progress.encounter() else {
        return;
    };

    Screen::new()
        .prose(encounter_intro(id))
        .prose(narrative::FIGHT_OR_FOLD)
        .option(1, "Fight")
        .option(2, "Fold")
        .footer(format!("Your Stack: {} — Enter sits down.", run.stack))
        .spawn(&mut commands, AppState::FightOrFold);
}

fn fight_or_fold(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    progress: Res<Progress>,
    mut next: ResMut<NextState<AppState>>,
) {
    let Some(id) = progress.encounter() else {
        return;
    };

    match digit_pressed(&keys).or(confirm(&keys).then_some(1)) {
        Some(1) => {
            // The whole handover: an `Encounter` and the state. Combat takes it
            // from here and comes back at `PostCombat`.
            commands.insert_resource(Encounter { id, enemy: Enemy::for_encounter(id) });
            next.set(AppState::Combat);
        }
        Some(2) => {
            commands.insert_resource(Folded);
            next.set(AppState::Lobby);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Coming back out of combat
// ---------------------------------------------------------------------------

fn show_outcome(mut commands: Commands, progress: Res<Progress>, outcome: Res<CombatOutcome>) {
    // The run only advances on the reward screen, so the encounter just
    // played is still the current one.
    let Some(id) = progress.encounter() else {
        return;
    };

    let body = match *outcome {
        CombatOutcome::Lost => narrative::LOSE,
        CombatOutcome::Won => win_line(id),
    };

    Screen::new()
        .prose(body)
        .footer(narrative::ANY_KEY)
        .spawn(&mut commands, AppState::PostCombat);
}

fn leave_outcome(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    progress: Res<Progress>,
    outcome: Res<CombatOutcome>,
    mut next: ResMut<NextState<AppState>>,
) {
    if any_key(&keys) {
        next.set(progress.route(*outcome));
        commands.remove_resource::<CombatOutcome>();
    }
}

// ---------------------------------------------------------------------------
// Reward
// ---------------------------------------------------------------------------

fn show_reward(mut commands: Commands, progress: Res<Progress>) {
    let Some(offer) = progress.encounter().and_then(RewardOffer::for_encounter) else {
        return;
    };

    let screen = match offer {
        RewardOffer::Drop(reward) => Screen::new()
            .title("Something drops")
            .prose(narrative::ITEM_DROP)
            .prose(reward.label())
            .footer(narrative::ANY_KEY),
        RewardOffer::Pick(one, two) => Screen::new()
            .title("A perk")
            .prose(narrative::PERK_PICK)
            .option(1, one.label())
            .option(2, two.label())
            .footer("Press 1 or 2 — or Enter to take the first."),
    };

    screen.spawn(&mut commands, AppState::Reward);
}

/// The reward is granted here rather than on the way in, so there is one place
/// the run changes and it is the same place for a drop and a pick.
fn take_reward(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut run: ResMut<RunState>,
    mut progress: ResMut<Progress>,
    mut next: ResMut<NextState<AppState>>,
) {
    let Some(offer) = progress.encounter().and_then(RewardOffer::for_encounter) else {
        return;
    };

    let taken = match offer {
        RewardOffer::Drop(reward) => any_key(&keys).then_some(reward),
        RewardOffer::Pick(one, two) => match digit_pressed(&keys).or(confirm(&keys).then_some(1)) {
            Some(1) => Some(one),
            Some(2) => Some(two),
            _ => None,
        },
    };
    let Some(reward) = taken else { return };

    // The only roll the overworld makes: the Pit Boss card pack.
    run.apply(reward, time.elapsed_secs_f64().to_bits());
    progress.advance();
    next.set(progress.arrival());
}

// ---------------------------------------------------------------------------
// The two ways a night ends
// ---------------------------------------------------------------------------

fn show_ending(mut commands: Commands) {
    Screen::new()
        .prose(narrative::ENDING)
        .footer(narrative::ANY_KEY)
        .spawn(&mut commands, AppState::Ending);
}

fn show_game_over(mut commands: Commands) {
    Screen::new()
        .title(narrative::GAME_OVER)
        .footer(narrative::ANY_KEY)
        .spawn(&mut commands, AppState::GameOver);
}

#[cfg(test)]
mod tests {
    use bevy::input::ButtonInput;
    use bevy::prelude::*;
    use bevy::state::app::StatesPlugin;

    use super::OverworldPlugin;
    use super::combat_stub::CombatStubPlugin;
    use super::progression::Progress;
    use crate::run::{CombatOutcome, Encounter, EncounterId, Perk, RunState, Tell};
    use crate::state::AppState;

    /// The shell with no window, no renderer and no real input device: enough
    /// to drive every transition the overworld owns.
    fn shell() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(StatesPlugin)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(OverworldPlugin)
            .add_plugins(CombatStubPlugin);
        app.update();
        app
    }

    fn press(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        // Let go of the key, or the next press never counts as a fresh one.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.update();
    }

    fn progress(app: &App) -> Progress {
        *app.world().resource::<Progress>()
    }

    fn state(app: &App) -> AppState {
        *app.world().resource::<State<AppState>>().get()
    }

    /// Walk from the Lobby into the first Fight or Fold prompt.
    fn begin_run(app: &mut App) {
        press(app, KeyCode::Digit3);
        assert_eq!(state(app), AppState::FloorIntro);
        press(app, KeyCode::Enter);
        assert_eq!(state(app), AppState::FightOrFold);
    }

    /// Fight the encounter on offer and take the stubbed outcome.
    fn duel(app: &mut App, win: bool) {
        press(app, KeyCode::Digit1);
        assert_eq!(state(app), AppState::Combat);
        press(
            app,
            if win {
                KeyCode::Digit1
            } else {
                KeyCode::Digit2
            },
        );
        assert_eq!(state(app), AppState::PostCombat);
        press(app, KeyCode::Enter);
    }

    #[test]
    fn the_opening_leads_into_the_lobby() {
        let mut app = shell();
        assert_eq!(state(&app), AppState::Opening);

        press(&mut app, KeyCode::Enter);

        assert_eq!(state(&app), AppState::Lobby);
    }

    #[test]
    fn the_lobby_side_rooms_come_back_to_the_lobby() {
        let mut app = shell();
        press(&mut app, KeyCode::Enter);

        for (key, room) in [
            (KeyCode::Digit1, AppState::InfoRoom),
            (KeyCode::Digit2, AppState::Tutorial),
        ] {
            press(&mut app, key);
            assert_eq!(state(&app), room);
            press(&mut app, KeyCode::Enter);
            assert_eq!(state(&app), AppState::Lobby);
        }
    }

    #[test]
    fn enter_takes_the_option_each_menu_leads_with() {
        let mut app = shell();
        press(&mut app, KeyCode::Enter);

        // The Lobby leads with Walk the Floor...
        press(&mut app, KeyCode::Enter);
        assert_eq!(state(&app), AppState::FloorIntro);
        press(&mut app, KeyCode::Enter);

        // ...and Fight or Fold leads with Fight.
        assert_eq!(state(&app), AppState::FightOrFold);
        press(&mut app, KeyCode::Enter);
        assert_eq!(state(&app), AppState::Combat);
    }

    #[test]
    fn the_lobby_is_where_a_run_ends() {
        let mut app = shell();
        press(&mut app, KeyCode::Enter);
        begin_run(&mut app);
        duel(&mut app, true);
        press(&mut app, KeyCode::Enter);

        // Deep enough into the run for a reset to show.
        assert_eq!(progress(&app).encounter(), Some(EncounterId::Slotz));
        app.world_mut().resource_mut::<RunState>().stack = 7;

        press(&mut app, KeyCode::Digit2); // Fold
        assert_eq!(state(&app), AppState::Lobby);
        assert_eq!(progress(&app).encounter(), Some(EncounterId::FloorMinion));
        assert_ne!(app.world().resource::<RunState>().stack, 7);
    }

    #[test]
    fn winning_every_encounter_walks_three_floors_and_reaches_the_ending() {
        let mut app = shell();
        press(&mut app, KeyCode::Enter);
        begin_run(&mut app);

        // The Floor: a minion, then Slotz.
        duel(&mut app, true);
        assert_eq!(state(&app), AppState::Reward);
        press(&mut app, KeyCode::Enter);
        assert_eq!(state(&app), AppState::FightOrFold);
        duel(&mut app, true);
        press(&mut app, KeyCode::Enter);

        // The Pit announces itself, then a minion and the Pit Boss.
        assert_eq!(state(&app), AppState::FloorIntro);
        press(&mut app, KeyCode::Enter);
        duel(&mut app, true);
        press(&mut app, KeyCode::Enter);
        assert_eq!(state(&app), AppState::FightOrFold);
        duel(&mut app, true);
        press(&mut app, KeyCode::Enter);

        // The Big Shots Table: The House, and no reward after it.
        assert_eq!(state(&app), AppState::FloorIntro);
        press(&mut app, KeyCode::Enter);
        duel(&mut app, true);
        assert_eq!(state(&app), AppState::Ending);

        press(&mut app, KeyCode::Enter);
        assert_eq!(state(&app), AppState::Lobby);
    }

    #[test]
    fn folding_walks_back_to_the_lobby_and_the_next_run_starts_over() {
        let mut app = shell();
        press(&mut app, KeyCode::Enter);
        begin_run(&mut app);

        // Get one encounter deep, then Fold.
        duel(&mut app, true);
        press(&mut app, KeyCode::Enter);
        assert_eq!(progress(&app).encounter(), Some(EncounterId::Slotz));
        press(&mut app, KeyCode::Digit2);
        assert_eq!(state(&app), AppState::Lobby);

        begin_run(&mut app);
        assert_eq!(progress(&app).encounter(), Some(EncounterId::FloorMinion));
    }

    #[test]
    fn losing_ends_the_night_and_sends_you_back_to_the_lobby() {
        let mut app = shell();
        press(&mut app, KeyCode::Enter);
        begin_run(&mut app);

        duel(&mut app, false);

        assert_eq!(state(&app), AppState::GameOver);
        assert_eq!(app.world().resource::<RunState>().stack, 0);

        press(&mut app, KeyCode::Enter);
        assert_eq!(state(&app), AppState::Lobby);
    }

    #[test]
    fn beating_a_minion_drops_the_loaded_dice() {
        let mut app = shell();
        press(&mut app, KeyCode::Enter);
        begin_run(&mut app);

        duel(&mut app, true);
        assert_eq!(state(&app), AppState::Reward);
        assert_eq!(app.world().resource::<RunState>().loaded_dice(), 0);
        press(&mut app, KeyCode::Enter);

        assert_eq!(app.world().resource::<RunState>().loaded_dice(), 2);
        assert_eq!(state(&app), AppState::FightOrFold);
    }

    /// Walk to the Slotz reward screen and take the option `key` picks.
    fn slotz_reward(key: KeyCode) -> App {
        let mut app = shell();
        press(&mut app, KeyCode::Enter);
        begin_run(&mut app);
        duel(&mut app, true); // the Floor minion
        press(&mut app, KeyCode::Enter); // pocket the drop
        duel(&mut app, true); // Slotz
        assert_eq!(state(&app), AppState::Reward);

        press(&mut app, key);
        app
    }

    #[test]
    fn a_boss_pays_out_the_perk_you_pressed_and_not_the_other_one() {
        let coin = slotz_reward(KeyCode::Digit1);
        let run = coin.world().resource::<RunState>();
        assert_eq!(run.perks, vec![Perk::PylBestTwoOfThree]);
        assert_eq!(run.deck.len(), crate::run::starter_deck().len());

        let cards = slotz_reward(KeyCode::Digit2);
        let run = cards.world().resource::<RunState>();
        assert!(run.perks.is_empty());
        let added = &run.deck[crate::run::starter_deck().len()..];
        assert_eq!(added.len(), 3);
        assert!(added.iter().all(|c| c.tell == Some(Tell::Streak)));
    }

    #[test]
    fn taking_a_perk_walks_on_to_the_next_encounter() {
        let app = slotz_reward(KeyCode::Digit2);

        assert_eq!(progress(&app).encounter(), Some(EncounterId::PitMinion));
        assert_eq!(state(&app), AppState::FloorIntro);
    }

    #[test]
    fn folding_leaves_every_reward_behind() {
        let mut app = slotz_reward(KeyCode::Digit1);
        assert_eq!(app.world().resource::<RunState>().loaded_dice(), 2);

        press(&mut app, KeyCode::Enter); // onto the Pit
        assert_eq!(state(&app), AppState::FightOrFold);
        press(&mut app, KeyCode::Digit2); // Fold

        assert_eq!(state(&app), AppState::Lobby);
        let run = app.world().resource::<RunState>();
        assert!(run.perks.is_empty());
        assert!(run.items.is_empty());
        assert_eq!(run.deck.len(), crate::run::starter_deck().len());
    }

    #[test]
    fn the_fight_key_hands_combat_an_encounter_and_nothing_else() {
        let mut app = shell();
        press(&mut app, KeyCode::Enter);
        begin_run(&mut app);

        press(&mut app, KeyCode::Digit1);

        let encounter = app.world().resource::<Encounter>();
        assert_eq!(encounter.id, EncounterId::FloorMinion);
        assert!(encounter.enemy.stack > 0);
        assert!(app.world().get_resource::<CombatOutcome>().is_none());
    }
}
