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
mod wheel;

use bevy::prelude::*;

use progression::{Progress, encounter_intro, win_line};
use screens::{
    Backdrop, Screen, any_key, apply_backdrop, confirm, digit_pressed, load_overworld_art, space,
};

use crate::boss::SLOTZ;
use crate::item::{self, Item};
use crate::run::{Card, CombatOutcome, Encounter, Pack, Reward, RewardOffer, RunState};
use crate::save::SaveSlot;
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
            // `SavePlugin` loads the real one over this; without it the game
            // plays on a fresh slot.
            .init_resource::<SaveSlot>()
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
        wheel::add_systems(app);
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

fn show_lobby(mut commands: Commands, folded: Option<Res<Folded>>, slot: Res<SaveSlot>) {
    let mut screen = Screen::new().title("The Lobby");

    if folded.is_some() {
        screen = screen.prose(narrative::FOLD);
        commands.remove_resource::<Folded>();
    }

    screen = screen
        .prose(narrative::LOBBY)
        .option(1, narrative::LOBBY_OPT_INFO)
        .option(2, narrative::LOBBY_OPT_TUTORIAL)
        .option(3, narrative::LOBBY_OPT_BEGIN);
    // The Wheel opens once The House has been beaten.
    let footer = if slot.house_beaten {
        screen = screen.option(
            4,
            &format!(
                "{}  ({} Golden Chips in hand)",
                narrative::LOBBY_OPT_WHEEL,
                slot.golden_chips
            ),
        );
        "Press 1 to 4 — or Enter to walk the Floor."
    } else {
        "Press 1, 2 or 3 — or Enter to walk the Floor."
    };
    screen.footer(footer).spawn(&mut commands, AppState::Lobby);
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
    slot: Res<SaveSlot>,
    mut next: ResMut<NextState<AppState>>,
) {
    // Once The Wheel is open, every run starts with its spin.
    let begin = if slot.house_beaten {
        AppState::Spin
    } else {
        progress.arrival()
    };
    match digit_pressed(&keys) {
        Some(1) => next.set(AppState::InfoRoom),
        Some(2) => next.set(AppState::Tutorial),
        Some(3) => next.set(begin),
        Some(4) if slot.house_beaten => next.set(AppState::Wheel),
        _ if confirm(&keys) => next.set(begin),
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
    commands.insert_resource(Encounter::Practice);
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
    let Some(encounter) = progress.encounter() else {
        return;
    };

    Screen::new()
        .prose(encounter_intro(encounter))
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
    let Some(encounter) = progress.encounter() else {
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
            commands.insert_resource(encounter);
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
        let [one, two] = SLOTZ.rewards.expect("Slotz pays a 1-of-2");
        Screen::new()
            .title("A perk")
            .prose(narrative::PERK_PICK)
            .option(1, &one.label())
            .option(2, &two.label())
            .footer("Press 1 or 2. There is no going back.")
            .spawn(&mut commands, AppState::PostCombat);
        return;
    }
    let body = if tutorial.is_some() {
        narrative::TUTORIAL_LEFT
    } else {
        // The run only advances on the reward screen, so the encounter just
        // played is still the current one.
        let Some(encounter) = progress.encounter() else {
            return;
        };
        match *outcome {
            CombatOutcome::Lost => narrative::LOSE,
            CombatOutcome::Won => win_line(encounter),
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

/// A Pack on the table, and which of its cards the player has marked to
/// keep. Only there between a boss's win and the Perk pick, or after
/// choosing a minion's cards.
#[derive(Resource)]
struct OpenPack {
    pack: Pack,
    picked: Vec<usize>,
    title: &'static str,
    prose: &'static str,
}

/// The Items a minion's reward put on the table, after choosing Items.
#[derive(Resource)]
struct OpenItems(Vec<&'static Item>);

/// A minion's three, in the order the screen lists the keys.
const MINION_ITEMS: usize = 3;

/// The reward screen on the felt now, so a mark in a Pack, opening a
/// minion's reward or moving on to the Perk pick can redraw it.
#[derive(Component)]
struct RewardScreen;

fn show_reward(
    mut commands: Commands,
    progress: Res<Progress>,
    run: Res<RunState>,
    time: Res<Time>,
    tutorial: Option<Res<InTutorial>>,
) {
    // The Arcade's sign-off (#73). This gate is load-bearing: during the
    // Arcade, Progress still sits at the first encounter, so falling through
    // would render its minion reward.
    if tutorial.is_some() {
        Screen::new()
            .prose(narrative::TUTORIAL_PERK_TAKEN)
            .prose(narrative::TUTORIAL_DONE)
            .footer(narrative::ANY_KEY)
            .spawn(&mut commands, AppState::Reward);
        return;
    }
    let Some(encounter) = progress.encounter() else {
        return;
    };

    // A boss's Pack comes first, then its Perk pick. The overworld's other
    // roll, alongside the one `take_reward` hands to card-giving rewards.
    if let Some(pack) = encounter.boss_pack(&run, time.elapsed_secs_f64().to_bits()) {
        let open = OpenPack {
            pack,
            picked: Vec::new(),
            title: "The Boss Pack",
            prose: narrative::BOSS_PACK,
        };
        spawn_pack(&mut commands, &open);
        commands.insert_resource(open);
        return;
    }
    spawn_offer(&mut commands, &progress);
}

fn spawn_pack(commands: &mut Commands, open: &OpenPack) {
    let mut screen = Screen::new().title(open.title).prose(open.prose);
    for (i, card) in open.pack.cards.iter().enumerate() {
        let mark = if open.picked.contains(&i) {
            "KEEP "
        } else {
            ""
        };
        screen = screen.option(i as u8 + 1, &format!("{mark}{}", card_line(card)));
    }
    let them = if open.pack.keep == 1 { "it" } else { "them" };
    let footer = format!(
        "Press 1 to {} to mark {} to keep, again to put one back. Enter takes {them}. ({} of {})",
        open.pack.cards.len(),
        open.pack.keep,
        open.picked.len(),
        open.pack.keep,
    );
    let root = screen.footer(footer).spawn(commands, AppState::Reward);
    commands.entity(root).insert(RewardScreen);
}

/// One card as the Pack shows it: its name, Face Value and Tell.
fn card_line(card: &Card) -> String {
    match card.tell {
        Some(tell) => format!("{}  {}  {}", card.name, card.face_value, tell.name()),
        None => format!("{}  {}", card.name, card.face_value),
    }
}

/// The minion's choice or the Perk pick for the encounter just won.
fn spawn_offer(commands: &mut Commands, progress: &Progress) {
    let Some(offer) = progress.reward_offer() else {
        return;
    };

    let screen = match offer {
        RewardOffer::ItemsOrPack => Screen::new()
            .title("Something's left on the felt")
            .prose(narrative::MINION_REWARD)
            .option(1, "Items: three of them, keep one.")
            .option(2, "Cards: a Pack of three, keep one.")
            .option(3, "Leave it.")
            .footer("Press 1, 2 or 3. You don't get to look first."),
        RewardOffer::Pick(one, two) => Screen::new()
            .title("A perk")
            .prose(narrative::PERK_PICK)
            .option(1, &one.label())
            .option(2, &two.label())
            .footer("Press 1 or 2. There is no going back."),
    };

    let root = screen.spawn(commands, AppState::Reward);
    commands.entity(root).insert(RewardScreen);
}

fn spawn_items(commands: &mut Commands, open: &OpenItems) {
    let mut screen = Screen::new().title("Items").prose(narrative::MINION_ITEMS);
    for (i, &item) in open.0.iter().enumerate() {
        screen = screen.option(i as u8 + 1, &Reward::Item(item).label());
    }
    let footer = format!("Press 1 to {}. The rest stay on the felt.", open.0.len());
    let root = screen.footer(footer).spawn(commands, AppState::Reward);
    commands.entity(root).insert(RewardScreen);
}

/// The reward is granted here rather than on the way in, so there is one place
/// the run changes and it is the same place for an Item, a Pack and a pick.
#[allow(clippy::too_many_arguments)]
fn take_reward(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    tutorial: Option<Res<InTutorial>>,
    open: Option<ResMut<OpenPack>>,
    items: Option<Res<OpenItems>>,
    shown: Query<Entity, With<RewardScreen>>,
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

    // The Boss Pack: a number marks or unmarks a card, Enter takes the marked
    // ones once there are as many as the Pack lets you keep.
    if let Some(mut open) = open {
        if let Some(n) = digit_pressed(&keys) {
            let i = usize::from(n) - 1;
            if let Some(at) = open.picked.iter().position(|&p| p == i) {
                open.picked.remove(at);
            } else if i < open.pack.cards.len() && open.picked.len() < open.pack.keep {
                open.picked.push(i);
            } else {
                return;
            }
            redraw(&mut commands, &shown);
            spawn_pack(&mut commands, &open);
        } else if confirm(&keys) && run.keep(&open.pack, &open.picked) {
            commands.remove_resource::<OpenPack>();
            // A minion's Pack is the whole reward; a boss's comes before
            // its Perk pick.
            if progress.reward_offer() == Some(RewardOffer::ItemsOrPack) {
                progress.advance();
                next.set(progress.arrival());
            } else {
                redraw(&mut commands, &shown);
                spawn_offer(&mut commands, &progress);
            }
        }
        return;
    }

    // A minion's Items: a number keeps that one and the rest are gone.
    if let Some(items) = items {
        let picked = digit_pressed(&keys).and_then(|n| items.0.get(usize::from(n) - 1));
        if let Some(&item) = picked {
            commands.remove_resource::<OpenItems>();
            run.apply(Reward::Item(item), time.elapsed_secs_f64().to_bits());
            progress.advance();
            next.set(progress.arrival());
        }
        return;
    }

    let Some(offer) = progress.reward_offer() else {
        return;
    };

    let taken = match offer {
        // Opening one swaps the screen; only leaving it walks on. Enter
        // does nothing, so the choice is never made for you.
        RewardOffer::ItemsOrPack => {
            let seed = time.elapsed_secs_f64().to_bits();
            match digit_pressed(&keys) {
                Some(1) => {
                    let open = OpenItems(item::offer(&run.items, MINION_ITEMS, seed));
                    // Every Item already held: nothing to offer, so the
                    // choice stays on the table.
                    if open.0.is_empty() {
                        return;
                    }
                    redraw(&mut commands, &shown);
                    spawn_items(&mut commands, &open);
                    commands.insert_resource(open);
                }
                Some(2) => {
                    let open = OpenPack {
                        pack: Pack::minion(&run, seed),
                        picked: Vec::new(),
                        title: "A Pack",
                        prose: narrative::MINION_PACK,
                    };
                    redraw(&mut commands, &shown);
                    spawn_pack(&mut commands, &open);
                    commands.insert_resource(open);
                }
                Some(3) => {
                    progress.advance();
                    next.set(progress.arrival());
                }
                _ => {}
            }
            return;
        }
        // Enter advances every other screen in the shell, so it deliberately
        // does nothing here: a perk is picked once and never given back.
        RewardOffer::Pick(one, two) => match digit_pressed(&keys) {
            Some(1) => Some(one),
            Some(2) => Some(two),
            _ => None,
        },
    };
    let Some(reward) = taken else { return };

    // A reward that deals cards rolls here.
    run.apply(reward, time.elapsed_secs_f64().to_bits());
    progress.advance();
    next.set(progress.arrival());
}

/// Clear the reward screen off the felt so the next one can go down.
fn redraw(commands: &mut Commands, shown: &Query<Entity, With<RewardScreen>>) {
    for screen in shown {
        commands.entity(screen).despawn();
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

    use super::combat_stub::CombatStubPlugin;
    use super::progression::{Progress, RUN};
    use super::{OpenItems, OpenPack, OverworldPlugin};
    use crate::boss::SLOTZ;
    use crate::run::{CombatOutcome, Encounter, Floor, Reward, RunState, Tell};
    use crate::save::SaveSlot;
    use crate::state::AppState;
    use crate::wheel::Square;

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
        assert_eq!(*app.world().resource::<Encounter>(), Encounter::Practice);

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
        assert_eq!(progress(&app).encounter(), Some(RUN[0]));
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
        press(&mut app, KeyCode::Digit3);

        // Deep enough into the run for a reset to show.
        assert_eq!(progress(&app).encounter(), Some(RUN[1]));
        app.world_mut().resource_mut::<RunState>().chips = 7;

        press(&mut app, KeyCode::Digit2); // Fold
        assert_eq!(state(&app), AppState::Lobby);
        assert_eq!(progress(&app).encounter(), Some(RUN[0]));
        assert_ne!(app.world().resource::<RunState>().chips, 7);
    }

    #[test]
    fn winning_every_encounter_walks_three_floors_and_reaches_the_ending() {
        let mut app = opened();
        press(&mut app, KeyCode::Enter);
        begin_run(&mut app);

        // The Floor: a minion, then Slotz. The minion pays an Item; a boss
        // pick only answers to 1 or 2.
        duel(&mut app, true);
        assert_eq!(state(&app), AppState::Reward);
        press(&mut app, KeyCode::Digit1);
        press(&mut app, KeyCode::Digit1);
        assert_eq!(state(&app), AppState::FightOrFold);
        duel(&mut app, true);
        keep_two(&mut app);
        press(&mut app, KeyCode::Digit1);

        // The Pit announces itself, then a minion and the Pit Boss.
        assert_eq!(state(&app), AppState::FloorIntro);
        press(&mut app, KeyCode::Enter);
        duel(&mut app, true);
        press(&mut app, KeyCode::Digit2);
        press(&mut app, KeyCode::Digit1);
        press(&mut app, KeyCode::Enter);
        assert_eq!(state(&app), AppState::FightOrFold);
        duel(&mut app, true);
        keep_two(&mut app);
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
        press(&mut app, KeyCode::Digit3);
        assert_eq!(progress(&app).encounter(), Some(RUN[1]));
        press(&mut app, KeyCode::Digit2);
        assert_eq!(state(&app), AppState::Lobby);

        begin_run(&mut app);
        assert_eq!(progress(&app).encounter(), Some(RUN[0]));
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

    /// Walk to the Floor minion's reward, still unchosen.
    fn minion_reward() -> App {
        let mut app = opened();
        press(&mut app, KeyCode::Enter);
        begin_run(&mut app);
        duel(&mut app, true);
        assert_eq!(state(&app), AppState::Reward);
        app
    }

    #[test]
    fn skipping_a_minions_reward_takes_nothing_and_walks_on() {
        let mut app = minion_reward();
        let deck = deck_len(&app);

        // Enter picks nothing for you: the choice is sealed once made.
        press(&mut app, KeyCode::Enter);
        assert_eq!(state(&app), AppState::Reward);

        press(&mut app, KeyCode::Digit3);

        assert_eq!(state(&app), AppState::FightOrFold);
        assert!(app.world().resource::<RunState>().items.is_empty());
        assert_eq!(deck_len(&app), deck);
    }

    #[test]
    fn choosing_items_offers_three_and_keeps_the_one_pressed() {
        let mut app = minion_reward();

        press(&mut app, KeyCode::Digit1);
        let offered = app.world().resource::<OpenItems>().0.clone();
        assert_eq!(offered.len(), 3);
        assert_eq!(state(&app), AppState::Reward);

        press(&mut app, KeyCode::Digit2);

        let run = app.world().resource::<RunState>();
        assert_eq!(run.items.len(), 1);
        assert_eq!(run.uses(offered[1]), Some(offered[1].uses));
        assert!(app.world().get_resource::<OpenItems>().is_none());
        assert_eq!(state(&app), AppState::FightOrFold);
    }

    #[test]
    fn choosing_cards_opens_a_pack_of_three_and_keeps_the_one_marked() {
        let mut app = minion_reward();
        let deck = deck_len(&app);

        press(&mut app, KeyCode::Digit2);
        let cards = pack_cards(&app);
        assert_eq!(cards.len(), 3);

        press(&mut app, KeyCode::Enter);
        assert_eq!(state(&app), AppState::Reward, "nothing marked yet");
        press(&mut app, KeyCode::Digit3);
        press(&mut app, KeyCode::Digit1);
        assert_eq!(
            app.world().resource::<OpenPack>().picked,
            vec![2],
            "only one can be marked"
        );
        press(&mut app, KeyCode::Enter);

        let run = app.world().resource::<RunState>();
        assert_eq!(run.deck.len(), deck + 1);
        assert_eq!(run.deck.last(), Some(&cards[2]));
        assert!(run.items.is_empty());
        assert_eq!(state(&app), AppState::FightOrFold);
    }

    /// Keep the Boss Pack's first two cards and take them.
    fn keep_two(app: &mut App) {
        assert_eq!(state(app), AppState::Reward);
        press(app, KeyCode::Digit1);
        press(app, KeyCode::Digit2);
        press(app, KeyCode::Enter);
        assert!(app.world().get_resource::<OpenPack>().is_none());
    }

    /// Walk to Slotz's Boss Pack, still unopened.
    fn slotz_pack() -> App {
        let mut app = opened();
        press(&mut app, KeyCode::Enter);
        begin_run(&mut app);
        duel(&mut app, true); // the Floor minion
        press(&mut app, KeyCode::Digit3); // skip its reward
        duel(&mut app, true); // Slotz
        assert_eq!(state(&app), AppState::Reward);
        app
    }

    /// The cards of the Boss Pack on the table.
    fn pack_cards(app: &App) -> Vec<crate::run::Card> {
        app.world().resource::<OpenPack>().pack.cards.clone()
    }

    fn deck_len(app: &App) -> usize {
        app.world().resource::<RunState>().deck.len()
    }

    /// Walk past the Slotz Boss Pack and take the Perk option `key` picks.
    fn slotz_reward(key: KeyCode) -> App {
        let mut app = slotz_pack();
        keep_two(&mut app);
        press(&mut app, key);
        app
    }

    #[test]
    fn a_boss_opens_its_pack_before_the_perk_pick() {
        let mut app = slotz_pack();
        let cards = pack_cards(&app);
        let before = deck_len(&app);
        assert_eq!(cards.len(), 7);

        press(&mut app, KeyCode::Digit5);
        press(&mut app, KeyCode::Digit2);
        press(&mut app, KeyCode::Enter);

        let run = app.world().resource::<RunState>();
        assert_eq!(run.deck[before..], [cards[4].clone(), cards[1].clone()]);
        assert!(run.perks.is_empty(), "the Perk pick is still to come");
        assert_eq!(state(&app), AppState::Reward);
        assert_eq!(progress(&app).encounter(), Some(RUN[1]));
    }

    #[test]
    fn enter_takes_nothing_until_two_cards_are_marked() {
        let mut app = slotz_pack();
        let before = deck_len(&app);

        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Digit3);
        press(&mut app, KeyCode::Enter);

        assert_eq!(deck_len(&app), before);
        assert!(app.world().get_resource::<OpenPack>().is_some());
    }

    #[test]
    fn a_marked_card_is_put_back_with_its_key_and_a_third_mark_is_refused() {
        let mut app = slotz_pack();
        let cards = pack_cards(&app);
        let before = deck_len(&app);

        press(&mut app, KeyCode::Digit1);
        press(&mut app, KeyCode::Digit1); // put back
        press(&mut app, KeyCode::Digit3);
        press(&mut app, KeyCode::Digit6);
        press(&mut app, KeyCode::Digit7); // already holding two
        press(&mut app, KeyCode::Enter);

        let run = app.world().resource::<RunState>();
        assert_eq!(run.deck[before..], [cards[2].clone(), cards[5].clone()]);
    }

    #[test]
    fn a_boss_pays_out_the_perk_you_pressed_and_not_the_other_one() {
        let coin = slotz_reward(KeyCode::Digit1);
        let run = coin.world().resource::<RunState>();
        assert_eq!(
            run.perks
                .iter()
                .map(|&p| Reward::Perk(p))
                .collect::<Vec<_>>(),
            vec![SLOTZ.rewards.expect("a 1-of-2")[0]]
        );
        // The two kept from the Boss Pack, and nothing from the Perk.
        assert_eq!(run.deck.len(), crate::run::starter_deck().len() + 2);

        let cards = slotz_reward(KeyCode::Digit2);
        let run = cards.world().resource::<RunState>();
        assert!(run.perks.is_empty());
        let added = &run.deck[crate::run::starter_deck().len() + 2..];
        assert_eq!(added.len(), 3);
        assert!(added.iter().all(|c| c.tell == Some(Tell::Streak)));
    }

    #[test]
    fn enter_does_not_pick_a_perk_for_you() {
        let mut app = slotz_pack();
        keep_two(&mut app);
        let deck = deck_len(&app);

        press(&mut app, KeyCode::Enter);

        assert_eq!(
            state(&app),
            AppState::Reward,
            "the pick is still on the table"
        );
        let run = app.world().resource::<RunState>();
        assert!(run.perks.is_empty());
        assert_eq!(run.deck.len(), deck);
    }

    #[test]
    fn taking_a_perk_walks_on_to_the_next_encounter() {
        let app = slotz_reward(KeyCode::Digit2);

        assert_eq!(progress(&app).encounter(), Some(RUN[2]));
        assert_eq!(state(&app), AppState::FloorIntro);
    }

    #[test]
    fn folding_leaves_every_reward_behind() {
        let mut app = minion_reward();
        press(&mut app, KeyCode::Digit1); // Items
        press(&mut app, KeyCode::Digit1); // keep the first
        duel(&mut app, true); // Slotz
        keep_two(&mut app);
        press(&mut app, KeyCode::Digit1);
        let run = app.world().resource::<RunState>();
        assert_eq!(run.items.len(), 1);
        assert_eq!(run.perks.len(), 1);

        press(&mut app, KeyCode::Enter); // onto the Pit
        assert_eq!(state(&app), AppState::FightOrFold);
        press(&mut app, KeyCode::Digit2); // Fold

        assert_eq!(state(&app), AppState::Lobby);
        let run = app.world().resource::<RunState>();
        assert!(run.perks.is_empty());
        assert!(run.items.is_empty());
        assert_eq!(run.deck.len(), crate::run::starter_deck().len());
    }

    // --- The Wheel (#167) ---

    fn slot(app: &mut App) -> Mut<'_, SaveSlot> {
        app.world_mut().resource_mut::<SaveSlot>()
    }

    /// In the Lobby, with The House beaten and `chips` Golden Chips in hand.
    fn lobby_with_the_wheel(chips: u32) -> App {
        let mut app = opened();
        *slot(&mut app) = SaveSlot {
            golden_chips: chips,
            house_beaten: true,
            ..SaveSlot::default()
        };
        press(&mut app, KeyCode::Enter);
        assert_eq!(state(&app), AppState::Lobby);
        app
    }

    #[test]
    fn the_wheel_stays_shut_until_the_house_is_beaten() {
        let mut app = opened();
        press(&mut app, KeyCode::Enter);

        press(&mut app, KeyCode::Digit4);
        assert_eq!(state(&app), AppState::Lobby);

        // And no spin: the run starts on the Floor, as it always has.
        press(&mut app, KeyCode::Digit3);
        assert_eq!(state(&app), AppState::FloorIntro);
    }

    #[test]
    fn golden_chips_go_on_and_come_off_a_square_at_the_wheel() {
        let mut app = lobby_with_the_wheel(2);

        press(&mut app, KeyCode::Digit4);
        assert_eq!(state(&app), AppState::Wheel);
        // The cursor starts on Bankroll.
        press(&mut app, KeyCode::ArrowRight);
        press(&mut app, KeyCode::ArrowRight);
        press(&mut app, KeyCode::ArrowRight); // none left in hand
        assert_eq!(slot(&mut app).wheel[&Square::Bankroll], 2);
        assert_eq!(slot(&mut app).golden_chips, 0);

        press(&mut app, KeyCode::ArrowLeft);
        press(&mut app, KeyCode::ArrowDown); // Seed Money: coming soon
        press(&mut app, KeyCode::ArrowRight);
        assert_eq!(slot(&mut app).wheel[&Square::Bankroll], 1);
        assert_eq!(slot(&mut app).golden_chips, 1);

        press(&mut app, KeyCode::Escape);
        assert_eq!(state(&app), AppState::Lobby);
    }

    #[test]
    fn once_the_wheel_is_open_a_run_starts_with_the_spin() {
        let mut app = lobby_with_the_wheel(0);
        slot(&mut app).wheel.insert(Square::Bankroll, 2);

        press(&mut app, KeyCode::Enter);
        assert_eq!(state(&app), AppState::Spin);
        let run = app.world().resource::<RunState>();
        let bankroll = 5 * u32::from(run.wheel.rank(Square::Bankroll));
        assert!(bankroll >= 10, "rank 2, or 3 if it came up hot");
        assert_eq!(run.chips, crate::run::STARTING_CHIPS + bankroll);

        press(&mut app, KeyCode::Enter);
        assert_eq!(state(&app), AppState::FloorIntro);
        assert_eq!(
            app.world().resource::<RunState>().chips,
            crate::run::STARTING_CHIPS + bankroll,
            "the Floor keeps what the spin gave"
        );
    }

    #[test]
    fn trim_takes_the_marked_cards_out_before_the_floor() {
        let mut app = lobby_with_the_wheel(0);
        slot(&mut app).wheel.insert(Square::Trim, 1);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Enter); // past the spin
        let deck = app.world().resource::<RunState>().deck.clone();

        press(&mut app, KeyCode::ArrowDown);
        press(&mut app, KeyCode::Space);
        press(&mut app, KeyCode::Enter);

        assert_eq!(state(&app), AppState::FloorIntro);
        let run = app.world().resource::<RunState>();
        assert_eq!(run.deck.len(), deck.len() - 1);
        assert!(!run.deck.contains(&deck[1]));
    }

    #[test]
    fn pocket_change_at_rank_three_puts_its_pick_of_three() {
        let mut app = lobby_with_the_wheel(0);
        slot(&mut app).wheel.insert(Square::PocketChange, 3);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Enter); // past the spin

        assert_eq!(state(&app), AppState::Spin, "the pick is up");
        assert!(app.world().resource::<RunState>().items.is_empty());
        press(&mut app, KeyCode::Digit2);

        assert_eq!(state(&app), AppState::FloorIntro);
        assert_eq!(app.world().resource::<RunState>().items.len(), 1);
    }

    #[test]
    fn the_fight_key_hands_combat_an_encounter_and_nothing_else() {
        let mut app = opened();
        press(&mut app, KeyCode::Enter);
        begin_run(&mut app);

        press(&mut app, KeyCode::Digit1);

        let encounter = *app.world().resource::<Encounter>();
        assert_eq!(
            encounter,
            Encounter::Minion {
                floor: Floor::TheFloor
            }
        );
        assert!(encounter.enemy(&RunState::new(), 1).chips > 0);
        assert!(app.world().get_resource::<CombatOutcome>().is_none());
    }
}
