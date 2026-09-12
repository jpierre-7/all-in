//! The overworld: the app shell, the lobby, and the linear walk up the casino.
//!
//! Owns every `AppState` transition except `Combat -> PostCombat` (ADR-0001).
//! Combat is a black box: the overworld inserts an `Encounter`, hands over, and
//! routes on the `CombatOutcome` it gets back.

pub mod combat_stub;
pub mod narrative;
pub mod placeholder;
pub mod progression;
pub mod screens;

use bevy::prelude::*;

use progression::{Progress, encounter_intro, win_line};
use screens::{Screen, any_key, digit_pressed};

use crate::run::{CombatOutcome, Encounter, RunState};
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
            .insert_resource(placeholder::new_run_state())
            .add_systems(Startup, spawn_camera)
            .add_systems(OnEnter(AppState::Opening), show_opening)
            .add_systems(OnEnter(AppState::Lobby), show_lobby)
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
                    leave_opening.run_if(in_state(AppState::Opening)),
                    pick_from_lobby.run_if(in_state(AppState::Lobby)),
                    back_to_lobby
                        .run_if(in_state(AppState::InfoRoom).or_else(in_state(AppState::Tutorial))),
                    leave_floor_intro.run_if(in_state(AppState::FloorIntro)),
                    fight_or_fold.run_if(in_state(AppState::FightOrFold)),
                    leave_outcome.run_if(in_state(AppState::PostCombat)),
                    take_reward.run_if(in_state(AppState::Reward)),
                    end_the_night
                        .run_if(in_state(AppState::Ending).or_else(in_state(AppState::GameOver))),
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

fn leave_opening(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
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
        .footer("Press 1, 2 or 3.")
        .spawn(&mut commands, AppState::Lobby);
}

fn pick_from_lobby(
    keys: Res<ButtonInput<KeyCode>>,
    mut run: ResMut<RunState>,
    mut progress: ResMut<Progress>,
    mut next: ResMut<NextState<AppState>>,
) {
    match digit_pressed(&keys) {
        Some(1) => next.set(AppState::InfoRoom),
        Some(2) => next.set(AppState::Tutorial),
        Some(3) => {
            // A run always starts clean: Fold and death both leave the old one
            // behind.
            *run = placeholder::new_run_state();
            *progress = Progress::new();
            next.set(progress.arrival());
        }
        _ => {}
    }
}

fn show_info_room(mut commands: Commands) {
    Screen::new()
        .title("The Info Room")
        .prose(narrative::INFO_ROOM)
        .footer(narrative::ANY_KEY)
        .spawn(&mut commands, AppState::InfoRoom);
}

fn show_tutorial(mut commands: Commands) {
    Screen::new()
        .title("The Arcade")
        .prose(narrative::TUTORIAL)
        .footer(narrative::ANY_KEY)
        .spawn(&mut commands, AppState::Tutorial);
}

fn back_to_lobby(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if any_key(&keys) {
        next.set(AppState::Lobby);
    }
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
        .footer(format!("Your Stack: {}", run.stack))
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

    match digit_pressed(&keys) {
        Some(1) => {
            // The whole handover: an `Encounter` and the state. Combat takes it
            // from here and comes back at `PostCombat`.
            commands.insert_resource(Encounter {
                id,
                enemy: placeholder::enemy_for(id),
            });
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
    let body = match (*outcome, progress.encounter()) {
        (CombatOutcome::Lost, _) => narrative::LOSE,
        (CombatOutcome::Won, Some(id)) => win_line(id),
        (CombatOutcome::Won, None) => narrative::WIN_THE_HOUSE,
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
    let boss = progress.encounter().is_some_and(|id| id.is_boss());

    Screen::new()
        .title(if boss { "A perk" } else { "Something drops" })
        .prose(if boss {
            "You pick one thing to carry up the stairs."
        } else {
            "Somebody left something on the felt, and you pocket it."
        })
        .footer(format!(
            "{} (the pick itself lands with #12)",
            narrative::ANY_KEY
        ))
        .spawn(&mut commands, AppState::Reward);
}

fn take_reward(
    keys: Res<ButtonInput<KeyCode>>,
    mut progress: ResMut<Progress>,
    mut next: ResMut<NextState<AppState>>,
) {
    if any_key(&keys) {
        progress.advance();
        next.set(progress.arrival());
    }
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

fn end_the_night(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if any_key(&keys) {
        next.set(AppState::Lobby);
    }
}

#[cfg(test)]
mod tests {
    use bevy::input::ButtonInput;
    use bevy::prelude::*;
    use bevy::state::app::StatesPlugin;

    use super::OverworldPlugin;
    use super::combat_stub::CombatStubPlugin;
    use super::progression::Progress;
    use crate::run::{CombatOutcome, Encounter, EncounterId, RunState};
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
