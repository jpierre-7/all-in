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
use screens::{
    Backdrop, Screen, any_key, apply_backdrop, confirm, digit_pressed, load_overworld_art, space,
};

use crate::run::{CombatOutcome, Encounter, EncounterId, Enemy, Reward, RewardOffer, RunState};
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
            .add_systems(Startup, (spawn_camera, load_overworld_art))
            .add_systems(Update, apply_backdrop)
            .add_systems(OnEnter(AppState::Title), show_title)
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
                    leave_title.run_if(in_state(AppState::Title)),
                    // Every screen that only needs dismissing goes the same
                    // place: back to the Lobby.
                    back_to_lobby.run_if(
                        in_state(AppState::Opening)
                            .or_else(in_state(AppState::InfoRoom))
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
// Title
// ---------------------------------------------------------------------------

fn show_title(mut commands: Commands) {
    Screen::new()
        .marquee(narrative::TITLE)
        .backdrop(Backdrop::Title)
        .note(narrative::MUSIC_HINT)
        .footer(narrative::PRESS_SPACE)
        .spawn(&mut commands, AppState::Title);
}

/// Space only. Every other screen in the shell takes any key, so the marquee
/// deliberately does not: a stray keypress on the way to the table should not
/// skip the game's own name.
fn leave_title(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if space(&keys) {
        next.set(AppState::Opening);
    }
}

// ---------------------------------------------------------------------------
// Opening
// ---------------------------------------------------------------------------

fn show_opening(mut commands: Commands) {
    Screen::new()
        .title(narrative::TITLE)
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

/// The Arcade is a scripted duel (#40): hand combat a Tutorial encounter the
/// same way a floor would, and remember to route it home afterwards.
fn show_tutorial(mut commands: Commands, mut next: ResMut<NextState<AppState>>) {
    commands.insert_resource(InTutorial);
    commands.insert_resource(Encounter {
        id: EncounterId::Tutorial,
        enemy: Enemy::for_encounter(EncounterId::Tutorial),
    });
    next.set(AppState::Combat);
}

/// Set while the Arcade's duel runs, so PostCombat knows to go back to the
/// Lobby instead of routing the run. Cleared on the way out.
#[derive(Resource)]
struct InTutorial;

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
        .footer(format!(
            "Your Chips: {} — Enter sits down. I: what the words mean.",
            run.chips
        ))
        .spawn(&mut commands, AppState::FightOrFold);
}

fn fight_or_fold(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    progress: Res<Progress>,
    info: Option<Res<crate::combat::info::InfoOpen>>,
    mut next: ResMut<NextState<AppState>>,
) {
    let Some(id) = progress.encounter() else {
        return;
    };
    // The Info overlay (#43) is up, or about to be: the prompt hears nothing.
    if info.is_some() || crate::combat::info::toggled(&keys) {
        return;
    }

    match digit_pressed(&keys).or(confirm(&keys).then_some(1)) {
        Some(1) => {
            // The whole handover: an `Encounter` and the state. Combat takes it
            // from here and comes back at `PostCombat`.
            commands.insert_resource(Encounter {
                id,
                enemy: Enemy::for_encounter(id),
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

fn show_outcome(
    mut commands: Commands,
    progress: Res<Progress>,
    outcome: Res<CombatOutcome>,
    tutorial: Option<Res<InTutorial>>,
) {
    // The Arcade (#73): on the tutorial path PostCombat and Reward swap
    // roles. Combat owns `Combat -> PostCombat`, so PostCombat is the first
    // screen after the duel; a win renders the real Slotz pick here, and
    // `Reward` then carries the sign-off. Both states do the opposite of
    // their `state.rs` doc comments for this one path, which costs a
    // comment and nothing in the frozen shared file.
    if tutorial.is_some() && *outcome == CombatOutcome::Won {
        Screen::new()
            .title("A perk")
            .prose(narrative::PERK_PICK)
            .option(1, Reward::SlotzPylBestTwoOfThree.label())
            .option(2, Reward::SlotzStreakCards.label())
            .footer("Press 1 or 2. There is no going back.")
            .spawn(&mut commands, AppState::PostCombat);
        return;
    }
    let body = if tutorial.is_some() {
        narrative::TUTORIAL_LEFT
    } else {
        // The run only advances on the reward screen, so the encounter just
        // played is still the current one.
        let Some(id) = progress.encounter() else {
            return;
        };
        match *outcome {
            CombatOutcome::Lost => narrative::LOSE,
            CombatOutcome::Won => win_line(id),
        }
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
    tutorial: Option<Res<InTutorial>>,
    mut next: ResMut<NextState<AppState>>,
) {
    if tutorial.is_some() && *outcome == CombatOutcome::Won {
        // The Arcade's pick (#73): 1 or 2 takes it (to the sign-off), Esc
        // walks away unpicked. Nothing is applied either way.
        if keys.just_pressed(KeyCode::Escape) {
            commands.remove_resource::<InTutorial>();
            commands.remove_resource::<CombatOutcome>();
            next.set(AppState::Lobby);
        } else if matches!(digit_pressed(&keys), Some(1 | 2)) {
            commands.remove_resource::<CombatOutcome>();
            next.set(AppState::Reward);
        }
        return;
    }
    if any_key(&keys) {
        if tutorial.is_some() {
            commands.remove_resource::<InTutorial>();
            next.set(AppState::Lobby);
        } else {
            next.set(progress.route(*outcome));
        }
        commands.remove_resource::<CombatOutcome>();
    }
}

// ---------------------------------------------------------------------------
// Reward
// ---------------------------------------------------------------------------

fn show_reward(mut commands: Commands, progress: Res<Progress>, tutorial: Option<Res<InTutorial>>) {
    // The Arcade's sign-off (#73). This gate is load-bearing: during the
    // Arcade, Progress still sits at the first encounter, so falling through
    // would render its Loaded Dice drop.
    if tutorial.is_some() {
        Screen::new()
            .prose(narrative::TUTORIAL_PERK_TAKEN)
            .prose(narrative::TUTORIAL_DONE)
            .footer(narrative::ANY_KEY)
            .spawn(&mut commands, AppState::Reward);
        return;
    }
    let Some(offer) = progress.reward_offer() else {
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
            .footer("Press 1 or 2. There is no going back."),
    };

    screen.spawn(&mut commands, AppState::Reward);
}

/// The reward is granted here rather than on the way in, so there is one place
/// the run changes and it is the same place for a drop and a pick.
fn take_reward(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    tutorial: Option<Res<InTutorial>>,
    mut run: ResMut<RunState>,
    mut progress: ResMut<Progress>,
    mut next: ResMut<NextState<AppState>>,
) {
    // The Arcade's sign-off (#73): any key, home, nothing applied and the
    // run not advanced. Load-bearing for the same reason as `show_reward`.
    if tutorial.is_some() {
        if any_key(&keys) {
            commands.remove_resource::<InTutorial>();
            next.set(AppState::Lobby);
        }
        return;
    }
    let Some(offer) = progress.reward_offer() else {
        return;
    };

    let taken = match offer {
        RewardOffer::Drop(reward) => any_key(&keys).then_some(reward),
        // Enter advances every other screen in the shell, so it deliberately
        // does nothing here: a perk is picked once and never given back.
        RewardOffer::Pick(one, two) => match digit_pressed(&keys) {
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
    // #69 showed the music credit only when a track was found on disk,
    // because #70 was first to cut and a credit for music nobody hears is
    // worse than none. #70 landed: the track is committed, so the attribution
    // is unconditional, and the test that the ogg is still there is what
    // keeps the two honest.
    Screen::new()
        .title(narrative::GAME_OVER)
        .note(narrative::CREDITS)
        .note(narrative::MUSIC_CREDIT)
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

    /// The shell past the Title screen, where every test but the two below
    /// wants to start.
    fn opened() -> App {
        let mut app = shell();
        press(&mut app, KeyCode::Space);
        assert_eq!(state(&app), AppState::Opening);
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

    /// M is the mute (#70). Every prose screen pages on any key, so the one
    /// key that is not a game input has to be kept out of that.
    #[test]
    fn muting_does_not_page_a_prose_screen() {
        let mut app = opened();

        press(&mut app, KeyCode::KeyM);

        assert_eq!(state(&app), AppState::Opening);
    }

    #[test]
    fn the_game_opens_on_the_title_screen() {
        let app = shell();

        assert_eq!(state(&app), AppState::Title);
    }

    #[test]
    fn only_space_leaves_the_title() {
        let mut app = shell();

        for key in [
            KeyCode::Enter,
            KeyCode::Digit1,
            KeyCode::Escape,
            KeyCode::KeyA,
        ] {
            press(&mut app, key);
            assert_eq!(state(&app), AppState::Title, "{key:?} is not Space");
        }

        press(&mut app, KeyCode::Space);
        assert_eq!(state(&app), AppState::Opening);
    }

    #[test]
    fn the_opening_leads_into_the_lobby() {
        let mut app = opened();
        assert_eq!(state(&app), AppState::Opening);

        press(&mut app, KeyCode::Enter);

        assert_eq!(state(&app), AppState::Lobby);
    }

    #[test]
    fn the_lobby_side_rooms_come_back_to_the_lobby() {
        let mut app = opened();
        press(&mut app, KeyCode::Enter);

        press(&mut app, KeyCode::Digit1);
        assert_eq!(state(&app), AppState::InfoRoom);
        press(&mut app, KeyCode::Enter);
        assert_eq!(state(&app), AppState::Lobby);
    }

    /// The Arcade is a duel (#40): it hands combat a Tutorial encounter and,
    /// whatever happens at the table, comes back to the Lobby without
    /// touching the run.
    #[test]
    fn the_arcade_is_a_duel_that_comes_back_to_the_lobby() {
        let mut app = opened();
        press(&mut app, KeyCode::Enter);
        app.world_mut().resource_mut::<RunState>().chips = 7;
        let before = progress(&app);

        press(&mut app, KeyCode::Digit2);
        // Lobby -> Tutorial -> Combat is two transitions, one per frame.
        app.update();
        assert_eq!(state(&app), AppState::Combat);
        assert_eq!(
            app.world().resource::<Encounter>().id,
            EncounterId::Tutorial
        );

        // The stub stands in for the duel: "lose" here is what Esc sends.
        press(&mut app, KeyCode::Digit2);
        assert_eq!(state(&app), AppState::PostCombat);
        press(&mut app, KeyCode::Enter);

        assert_eq!(state(&app), AppState::Lobby);
        assert_eq!(progress(&app), before);
        assert!(app.world().get_resource::<CombatOutcome>().is_none());
    }

    /// Into the Arcade and through the stubbed duel to its outcome screen.
    /// `win` picks the stub's key: 1 wins, 2 loses (what Esc sends).
    fn arcade_outcome(app: &mut App, win: bool) {
        press(app, KeyCode::Digit2);
        app.update(); // Lobby -> Tutorial -> Combat, one frame each
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
    }

    /// Winning the Arcade shows the real Slotz pick (#73); 1 or 2 goes to
    /// the sign-off, then the Lobby.
    #[test]
    fn winning_the_arcade_offers_the_slotz_perk_then_signs_off() {
        let mut app = opened();
        press(&mut app, KeyCode::Enter);
        arcade_outcome(&mut app, true);

        // Any key is not enough on the pick: it wants 1 or 2.
        press(&mut app, KeyCode::Enter);
        assert_eq!(state(&app), AppState::PostCombat);

        press(&mut app, KeyCode::Digit1);
        assert_eq!(state(&app), AppState::Reward);
        press(&mut app, KeyCode::Enter);
        assert_eq!(state(&app), AppState::Lobby);
        assert!(app.world().get_resource::<CombatOutcome>().is_none());
    }

    #[test]
    fn escape_on_the_arcade_perk_goes_home_unpicked() {
        let mut app = opened();
        press(&mut app, KeyCode::Enter);
        arcade_outcome(&mut app, true);

        press(&mut app, KeyCode::Escape);

        assert_eq!(state(&app), AppState::Lobby);
    }

    /// The one that matters: the Arcade's perk applies nothing. After a full
    /// visit, Begin Run still starts at the first encounter with the starter
    /// deck and an empty pocket.
    #[test]
    fn the_arcade_perk_leaves_the_run_untouched() {
        let mut app = opened();
        press(&mut app, KeyCode::Enter);
        let before = progress(&app);
        let deck_before = app.world().resource::<RunState>().deck.len();

        arcade_outcome(&mut app, true);
        press(&mut app, KeyCode::Digit2); // "three more Streak cards"
        press(&mut app, KeyCode::Enter);
        assert_eq!(state(&app), AppState::Lobby);

        let run = app.world().resource::<RunState>();
        assert_eq!(run.deck.len(), deck_before);
        assert!(run.perks.is_empty());
        assert!(run.items.is_empty());
        assert_eq!(progress(&app), before);
        assert_eq!(progress(&app).encounter(), Some(EncounterId::FloorMinion));
    }

    #[test]
    fn enter_takes_the_option_each_menu_leads_with() {
        let mut app = opened();
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
        let mut app = opened();
        press(&mut app, KeyCode::Enter);
        begin_run(&mut app);
        duel(&mut app, true);
        press(&mut app, KeyCode::Enter);

        // Deep enough into the run for a reset to show.
        assert_eq!(progress(&app).encounter(), Some(EncounterId::Slotz));
        app.world_mut().resource_mut::<RunState>().chips = 7;

        press(&mut app, KeyCode::Digit2); // Fold
        assert_eq!(state(&app), AppState::Lobby);
        assert_eq!(progress(&app).encounter(), Some(EncounterId::FloorMinion));
        assert_ne!(app.world().resource::<RunState>().chips, 7);
    }

    #[test]
    fn winning_every_encounter_walks_three_floors_and_reaches_the_ending() {
        let mut app = opened();
        press(&mut app, KeyCode::Enter);
        begin_run(&mut app);

        // The Floor: a minion, then Slotz. A drop is dismissed with any key;
        // a boss pick only answers to 1 or 2.
        duel(&mut app, true);
        assert_eq!(state(&app), AppState::Reward);
        press(&mut app, KeyCode::Enter);
        assert_eq!(state(&app), AppState::FightOrFold);
        duel(&mut app, true);
        press(&mut app, KeyCode::Digit1);

        // The Pit announces itself, then a minion and the Pit Boss.
        assert_eq!(state(&app), AppState::FloorIntro);
        press(&mut app, KeyCode::Enter);
        duel(&mut app, true);
        press(&mut app, KeyCode::Enter);
        assert_eq!(state(&app), AppState::FightOrFold);
        duel(&mut app, true);
        press(&mut app, KeyCode::Digit1);

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
        let mut app = opened();
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
        let mut app = opened();
        press(&mut app, KeyCode::Enter);
        begin_run(&mut app);

        duel(&mut app, false);

        assert_eq!(state(&app), AppState::GameOver);
        assert_eq!(app.world().resource::<RunState>().chips, 0);

        press(&mut app, KeyCode::Enter);
        assert_eq!(state(&app), AppState::Lobby);
    }

    #[test]
    fn beating_a_minion_drops_the_loaded_dice() {
        let mut app = opened();
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
        let mut app = opened();
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
    fn enter_does_not_pick_a_perk_for_you() {
        let mut app = opened();
        press(&mut app, KeyCode::Enter);
        begin_run(&mut app);
        duel(&mut app, true);
        press(&mut app, KeyCode::Enter); // pocket the drop
        duel(&mut app, true); // Slotz
        assert_eq!(state(&app), AppState::Reward);

        press(&mut app, KeyCode::Enter);

        assert_eq!(
            state(&app),
            AppState::Reward,
            "the pick is still on the table"
        );
        let run = app.world().resource::<RunState>();
        assert!(run.perks.is_empty());
        assert_eq!(run.deck.len(), crate::run::starter_deck().len());
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
        let mut app = opened();
        press(&mut app, KeyCode::Enter);
        begin_run(&mut app);

        press(&mut app, KeyCode::Digit1);

        let encounter = app.world().resource::<Encounter>();
        assert_eq!(encounter.id, EncounterId::FloorMinion);
        assert!(encounter.enemy.chips > 0);
        assert!(app.world().get_resource::<CombatOutcome>().is_none());
    }
}
