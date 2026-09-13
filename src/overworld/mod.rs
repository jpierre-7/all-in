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
    Anchor, Backdrop, Screen, any_key, apply_backdrop, confirm, digit_pressed, load_overworld_art,
    space,
};

use crate::run::{CombatOutcome, Encounter, EncounterId, Enemy, RewardOffer, RunState};
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
                        in_state(AppState::InfoRoom)
                            .or_else(in_state(AppState::Ending))
                            .or_else(in_state(AppState::GameOver)),
                    ),
                    // Paging redraws the screen mid-state, so it has to land
                    // before the backdrop is hung or the new frame would spend
                    // a tick on bare felt.
                    page_opening
                        .before(apply_backdrop)
                        .run_if(in_state(AppState::Opening)),
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

/// Which frame of the Opening is on the felt. Lives on the screen root, so it
/// is cleared with the screen and there is no way to be between frames.
#[derive(Component)]
struct OpeningFrame(usize);

fn show_opening(mut commands: Commands) {
    draw_opening_frame(&mut commands, 0);
}

/// One backstory frame: its art behind it, its prose in the dark band the art
/// leaves along the bottom.
fn draw_opening_frame(commands: &mut Commands, index: usize) {
    let screen = Screen::new()
        .backdrop(Backdrop::Opening(index))
        .anchor(Anchor::LowerThird)
        .prose(narrative::OPENING[index])
        .footer(narrative::ANY_KEY_OR_SKIP)
        .spawn(commands, AppState::Opening);
    commands.entity(screen).insert(OpeningFrame(index));
}

/// Any key turns to the next frame; the last one opens the Lobby door. Esc is
/// the skip, so a player who has read it once is two keys from the table.
fn page_opening(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    frames: Query<(Entity, &OpeningFrame)>,
    mut next: ResMut<NextState<AppState>>,
) {
    // Esc is answered before the screen is looked up, so the skip still works
    // if there is somehow no frame on the felt to page.
    if keys.just_pressed(KeyCode::Escape) {
        next.set(AppState::Lobby);
        return;
    }

    let Ok((screen, &OpeningFrame(index))) = frames.single() else {
        return;
    };
    if !any_key(&keys) {
        return;
    }

    let onward = index + 1;
    if onward == narrative::OPENING.len() {
        next.set(AppState::Lobby);
        return;
    }

    commands.entity(screen).despawn();
    draw_opening_frame(&mut commands, onward);
}

/// Dismisses the two side rooms and both endings.
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
            "Your Stack: {} — Enter sits down. I: what the words mean.",
            run.stack
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
    let body = if tutorial.is_some() {
        match *outcome {
            CombatOutcome::Won => narrative::TUTORIAL_DONE,
            CombatOutcome::Lost => narrative::TUTORIAL_LEFT,
        }
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

fn show_reward(mut commands: Commands, progress: Res<Progress>) {
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
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut run: ResMut<RunState>,
    mut progress: ResMut<Progress>,
    mut next: ResMut<NextState<AppState>>,
) {
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

    use super::combat_stub::CombatStubPlugin;
    use super::progression::Progress;
    use super::{OverworldPlugin, narrative};
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

    /// The shell in the Lobby, with the Opening skipped — where every test
    /// below that is not about the Opening wants to start.
    fn in_the_lobby() -> App {
        let mut app = opened();
        press(&mut app, KeyCode::Escape);
        assert_eq!(state(&app), AppState::Lobby);
        app
    }

    /// Which Opening frame is on screen, by its index in `narrative::OPENING`.
    fn frame(app: &mut App) -> usize {
        app.world_mut()
            .query::<&super::OpeningFrame>()
            .single(app.world())
            .expect("the Opening always has exactly one frame up")
            .0
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

    /// The Opening is five frames, not one screen: it takes as many keypresses
    /// as there are frames to reach the Lobby, and the last one is what opens
    /// the door.
    #[test]
    fn the_opening_pages_one_frame_at_a_time_into_the_lobby() {
        let mut app = opened();

        for expected in 0..narrative::OPENING.len() {
            assert_eq!(state(&app), AppState::Opening, "frame {expected}");
            assert_eq!(frame(&mut app), expected);
            press(&mut app, KeyCode::Enter);
        }

        assert_eq!(state(&app), AppState::Lobby);
    }

    /// Any key advances, the same as every other prose screen in the shell.
    #[test]
    fn any_key_turns_an_opening_frame() {
        let mut app = opened();

        for key in [KeyCode::KeyA, KeyCode::Space, KeyCode::Digit1] {
            let before = frame(&mut app);
            press(&mut app, key);
            assert_eq!(frame(&mut app), before + 1, "{key:?} turns the frame");
        }
    }

    #[test]
    fn esc_skips_the_opening_straight_to_the_lobby() {
        let mut app = opened();

        press(&mut app, KeyCode::Escape);

        assert_eq!(state(&app), AppState::Lobby);
    }

    /// Esc is a skip from wherever you have got to, not only from the first
    /// frame.
    #[test]
    fn esc_skips_from_halfway_through_too() {
        let mut app = opened();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Enter);
        assert_eq!(frame(&mut app), 2);

        press(&mut app, KeyCode::Escape);

        assert_eq!(state(&app), AppState::Lobby);
    }

    /// One frame on the felt at a time: the old one is cleared before the next
    /// is drawn, or the prose would stack up on the backdrop.
    #[test]
    fn only_one_frame_is_on_screen_at_a_time() {
        let mut app = opened();

        for _ in 1..narrative::OPENING.len() {
            press(&mut app, KeyCode::Enter);
            let frames = app
                .world_mut()
                .query::<&super::OpeningFrame>()
                .iter(app.world())
                .count();
            assert_eq!(frames, 1);
        }
    }

    /// The Opening runs once, on the way in from the Title. Every later trip
    /// through the Lobby — Fold, death, the Arcade — skips it.
    #[test]
    fn the_opening_does_not_replay_on_the_way_back_to_the_lobby() {
        let mut app = in_the_lobby();
        begin_run(&mut app);

        press(&mut app, KeyCode::Digit2); // Fold

        assert_eq!(state(&app), AppState::Lobby);
    }

    #[test]
    fn the_lobby_side_rooms_come_back_to_the_lobby() {
        let mut app = in_the_lobby();

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
        app.world_mut().resource_mut::<RunState>().stack = 7;
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

    #[test]
    fn enter_takes_the_option_each_menu_leads_with() {
        let mut app = in_the_lobby();

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
        let mut app = in_the_lobby();
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
        let mut app = in_the_lobby();
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
        let mut app = in_the_lobby();
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
        let mut app = in_the_lobby();
        begin_run(&mut app);

        duel(&mut app, false);

        assert_eq!(state(&app), AppState::GameOver);
        assert_eq!(app.world().resource::<RunState>().stack, 0);

        press(&mut app, KeyCode::Enter);
        assert_eq!(state(&app), AppState::Lobby);
    }

    #[test]
    fn beating_a_minion_drops_the_loaded_dice() {
        let mut app = in_the_lobby();
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
        let mut app = in_the_lobby();
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
        let mut app = in_the_lobby();
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
        let mut app = in_the_lobby();
        begin_run(&mut app);

        press(&mut app, KeyCode::Digit1);

        let encounter = app.world().resource::<Encounter>();
        assert_eq!(encounter.id, EncounterId::FloorMinion);
        assert!(encounter.enemy.stack > 0);
        assert!(app.world().get_resource::<CombatOutcome>().is_none());
    }
}
